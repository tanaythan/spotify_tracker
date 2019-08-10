mod db;
mod schema;

#[macro_use]
extern crate diesel;

use diesel::pg::PgConnection;
use dotenv;
use rspotify::spotify::client::Spotify;
use rspotify::spotify::model::context::SimplifiedPlayingContext;
use rspotify::spotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::spotify::senum::Country;
use rspotify::spotify::util::get_token;
use std::{thread, time};

use db::{establish_connection, insert_song, SongPlay};

struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub callback_url: String,
    pub db_url: String,
}

struct CachedData {
    previous: Option<SimplifiedPlayingContext>,
    has_uploaded: bool,
}

struct State {
    spotify: Spotify,
    db_conn: PgConnection,
    cache: CachedData,
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
            self.has_uploaded = false;
            return false;
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
        } else if item < old_item {
            self.previous = Some(song.clone());
            self.has_uploaded = false;
            return false;
        }
        false
    }
}

impl State {
    pub fn maybe_add_song(&mut self) -> Option<SongPlay> {
        let current_song = self.spotify.current_playing(Some(Country::UnitedStates));
        match current_song {
            Ok(song) => match song {
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
            },
            Err(e) => {
                println!("Error: {}\n from Spotify API", e);
                None
            }
        }
    }

    fn insert_song(&self, song: &SimplifiedPlayingContext) -> Option<SongPlay> {
        let full_track = song.item.clone().unwrap();
        insert_song(
            &self.db_conn,
            &full_track.name,
            &full_track.artists.first().unwrap().name,
        )
    }
}

fn main() {
    dotenv::dotenv().ok();
    let config = Config {
        client_id: dotenv::var("CLIENT_ID").unwrap(),
        client_secret: dotenv::var("CLIENT_SECRET").unwrap(),
        callback_url: dotenv::var("CALLBACK_URL").unwrap(),
        db_url: dotenv::var("DATABASE_URL").unwrap(),
    };

    let mut oauth = SpotifyOAuth::default()
        .client_id(&config.client_id)
        .client_secret(&config.client_secret)
        .redirect_uri(&config.callback_url)
        .scope("user-read-currently-playing")
        .build();

    let spotify = match get_token(&mut oauth) {
        Some(token_info) => {
            let client_credential = SpotifyClientCredentials::default()
                .token_info(token_info)
                .build();
            Spotify::default()
                .client_credentials_manager(client_credential)
                .build()
        }
        None => panic!("Spotify client must be credentialed!"),
    };

    let db_conn = establish_connection(config.db_url);
    let cache = CachedData::new();
    let mut state = State {
        spotify,
        db_conn,
        cache,
    };
    loop {
        println!("{:?}", state.maybe_add_song());
        thread::sleep(time::Duration::from_secs(5));
    }
}
