use super::db::{establish_connection, insert_song, SongPlay};

use diesel::PgConnection;
use rspotify::spotify::client::Spotify;
use rspotify::spotify::model::context::SimplifiedPlayingContext;
use rspotify::spotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::spotify::senum::Country;
use rspotify::spotify::util::get_token;
use std::error::Error;
use std::{fmt, thread, time};

struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub callback_url: String,
}

struct SpotifyWrapper {
    spotify: Spotify,
    config: OAuthConfig,
}

#[derive(Debug)]
pub enum SpotifyWrapperError {
    UnauthenticatedClient,
}

struct CachedData {
    previous: Option<SimplifiedPlayingContext>,
    has_uploaded: bool,
}

pub struct Worker {
    spotify: SpotifyWrapper,
    db_conn: PgConnection,
    cache: CachedData,
}

type SpotifyWrapperResult<T> = Result<T, SpotifyWrapperError>;

impl SpotifyWrapper {
    pub fn new(
        client_id: String,
        client_secret: String,
        callback_url: String,
    ) -> SpotifyWrapperResult<Self> {
        let config = OAuthConfig {
            client_id,
            client_secret,
            callback_url,
        };
        let spotify = Self::authenticate_spotify(&config)?;
        Ok(SpotifyWrapper { spotify, config })
    }

    fn authenticate_spotify(config: &OAuthConfig) -> SpotifyWrapperResult<Spotify> {
        let mut oauth = SpotifyOAuth::default()
            .client_id(&config.client_id)
            .client_secret(&config.client_secret)
            .redirect_uri(&config.callback_url)
            .scope("user-read-currently-playing")
            .build();

        match get_token(&mut oauth) {
            Some(token_info) => {
                let client_credential = SpotifyClientCredentials::default()
                    .token_info(token_info)
                    .build();
                Ok(Spotify::default()
                    .client_credentials_manager(client_credential)
                    .build())
            }
            None => Err(SpotifyWrapperError::UnauthenticatedClient),
        }
    }

    pub fn current_playing(&mut self, market: Option<Country>) -> Option<SimplifiedPlayingContext> {
        match self.spotify.current_playing(market.clone()) {
            Ok(spc) => spc,
            Err(e) => {
                println!("Detected error from spotify API: {}", e);
                self.spotify = match Self::authenticate_spotify(&self.config) {
                    Ok(s) => s,
                    Err(e) => {
                        println!("Received err {}", e);
                        return None;
                    }
                };
                println!("Reauthenticated client!");
                self.current_playing(market)
            }
        }
    }
}

impl CachedData {
    pub fn new() -> Self {
        CachedData {
            previous: None,
            has_uploaded: false,
        }
    }

    pub fn has_uploaded(&mut self, has_uploaded: bool) {
        self.has_uploaded = has_uploaded;
    }

    pub fn should_upload(&mut self, song: &SimplifiedPlayingContext) -> bool {
        if self.previous.is_none() {
            self.previous = Some(song.clone());
            let song_duration = song.clone().progress_ms;
            self.has_uploaded = match song_duration {
                Some(ms) => ms >= 30000,
                None => false,
            };
            return self.has_uploaded;
        }
        let item = song.clone().progress_ms;
        let old_item = self.previous.clone().unwrap().progress_ms;

        if item.is_none() || old_item.is_none() {
            return false;
        }

        let item = item.unwrap();
        let old_item = old_item.unwrap();

        if item > old_item && self.has_uploaded {
            return false;
        } else if item > old_item && !self.has_uploaded {
            self.previous = Some(song.clone());
            return item >= 30000;
        } else if item <= old_item {
            self.previous = Some(song.clone());
            self.has_uploaded = false;
            return false;
        }
        false
    }
}

impl Worker {
    pub fn new(config: &super::Config) -> SpotifyWrapperResult<Self> {
        let db_conn = establish_connection(config.db_url.clone());
        let cache = CachedData::new();
        let spotify = SpotifyWrapper::new(
            config.client_id.clone(),
            config.client_secret.clone(),
            config.callback_url.clone(),
        )?;

        Ok(Worker {
            db_conn,
            cache,
            spotify,
        })
    }

    pub fn maybe_add_song(&mut self) -> Option<SongPlay> {
        let current_song = self.spotify.current_playing(Some(Country::UnitedStates));
        match current_song {
            Some(s) => {
                if self.cache.should_upload(&s) {
                    let value = self.insert_song(&s);
                    self.cache.has_uploaded(value.is_some());
                    value
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn insert_song(&self, song: &SimplifiedPlayingContext) -> Option<SongPlay> {
        let full_track = song.item.clone().unwrap();
        insert_song(
            &self.db_conn,
            &full_track.name,
            full_track
                .artists
                .iter()
                .map(|artist| artist.name.as_str())
                .collect(),
            &full_track.album.name,
        )
    }

    pub fn run(&mut self) {
        loop {
            println!("{:?}", self.maybe_add_song());
            thread::sleep(time::Duration::from_secs(5));
        }
    }
}

impl fmt::Display for SpotifyWrapperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpotifyWrapperError::UnauthenticatedClient => write!(f, "Unauthenticated client!"),
        }
    }
}

impl Error for SpotifyWrapperError {
    fn description(&self) -> &str {
        match self {
            SpotifyWrapperError::UnauthenticatedClient => "Unauthenticated client",
        }
    }
}

#[cfg(test)]
mod test {
    use super::CachedData;
    use super::SimplifiedPlayingContext;

    fn create_playing_context(progress_ms: Option<u32>) -> SimplifiedPlayingContext {
        SimplifiedPlayingContext {
            context: None,
            is_playing: true,
            progress_ms,
            timestamp: 0,
            item: None,
        }
    }

    #[test]
    fn test_cache() {
        let mut cache = CachedData::new();
        let song = create_playing_context(Some(10));
        assert_eq!(false, cache.should_upload(&song));

        let song = create_playing_context(Some(100));
        assert_eq!(false, cache.should_upload(&song));

        let song = create_playing_context(Some(30000));
        assert_eq!(true, cache.should_upload(&song));

        cache.has_uploaded(true);

        let song = create_playing_context(Some(31000));
        assert_eq!(false, cache.should_upload(&song));

        let song = create_playing_context(Some(29000));
        assert_eq!(false, cache.should_upload(&song));

        let song = create_playing_context(Some(30000));
        assert_eq!(true, cache.should_upload(&song));

        let song = create_playing_context(Some(33000));
        assert_eq!(true, cache.should_upload(&song));

        let song = create_playing_context(Some(34000));
        assert_eq!(true, cache.should_upload(&song));

        let song = create_playing_context(Some(34000));
        assert_eq!(false, cache.should_upload(&song));
    }
}
