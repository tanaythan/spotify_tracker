use async_trait::async_trait;
use rspotify::client::Spotify;
use rspotify::model::context::SimplifiedPlayingContext;
use rspotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::util::get_token;
use std::error::Error;
use std::fmt;

#[async_trait]
pub trait SpotifyClient {
    async fn current_playing(&mut self) -> Option<SongData>;
}

#[derive(Clone, Default)]
pub struct SongData {
    pub progress_ms: Option<u32>,
    pub name: Option<String>,
    pub artists: Option<Vec<String>>,
    pub album: Option<String>,
}

pub struct SpotifyWrapper {
    spotify: Option<Spotify>,
    config: OAuthConfig,
}

struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub callback_url: String,
}

#[derive(Debug)]
pub enum SpotifyWrapperError {
    UnauthenticatedClient,
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

pub type SpotifyWrapperResult<T> = Result<T, SpotifyWrapperError>;

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
        Ok(SpotifyWrapper {
            spotify: None,
            config,
        })
    }

    async fn authenticate_spotify(config: &OAuthConfig) -> SpotifyWrapperResult<Spotify> {
        let mut oauth = SpotifyOAuth::default()
            .client_id(&config.client_id)
            .client_secret(&config.client_secret)
            .redirect_uri(&config.callback_url)
            .scope("user-read-currently-playing")
            .build();

        match get_token(&mut oauth).await {
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

    pub async fn connect(&mut self) -> SpotifyWrapperResult<()> {
        self.spotify = Some(Self::authenticate_spotify(&self.config).await?);
        Ok(())
    }

    fn convert_context_to_song_data(ctx: Option<SimplifiedPlayingContext>) -> Option<SongData> {
        if let Some(ctx) = ctx {
            let mut song_data = SongData::default();
            song_data.progress_ms = ctx.progress_ms;
            if let Some(track) = ctx.item {
                song_data.name = Some(track.name);
                song_data.artists = Some(
                    track
                        .artists
                        .iter()
                        .map(|artist| artist.name.clone())
                        .collect(),
                );
                song_data.album = Some(track.album.name);
            }
            Some(song_data)
        } else {
            None
        }
    }
}

#[async_trait]
impl SpotifyClient for SpotifyWrapper {
    async fn current_playing(&mut self) -> Option<SongData> {
        match &self.spotify {
            Some(spotify) => match spotify.current_playing(None).await {
                Ok(spc) => Self::convert_context_to_song_data(spc),
                Err(e) => {
                    println!("Detected error from spotify API: {}", e);
                    let spotify = match Self::authenticate_spotify(&self.config).await {
                        Ok(s) => s,
                        Err(e) => {
                            println!("Received err {}", e);
                            return None;
                        }
                    };
                    println!("Reauthenticated client!");
                    let res = spotify.current_playing(None).await.unwrap_or(None);
                    self.spotify = Some(spotify);
                    Self::convert_context_to_song_data(res)
                }
            },
            None => None,
        }
    }
}
