use super::db::{SongPlay, SongTracker, DB};
use super::spotify::{SongData, SpotifyClient, SpotifyWrapper, SpotifyWrapperResult};

struct CachedData {
    previous: Option<SongData>,
    has_uploaded: bool,
}

pub struct Worker<D: SongTracker, S: SpotifyClient> {
    spotify: S,
    db: D,
    cache: CachedData,
}

impl CachedData {
    pub fn new() -> Self {
        CachedData {
            previous: None,
            has_uploaded: false,
        }
    }

    pub fn should_upload(&self, song: &SongData) -> bool {
        if let Some(previous_item) = &self.previous {
            let item = song.progress_ms;
            let old_item = previous_item.progress_ms;

            match (item, old_item) {
                (Some(item), Some(old_item)) => {
                    item > old_item && !self.has_uploaded && item >= 30000
                }
                _ => false,
            }
        } else {
            let song_duration = song.progress_ms;
            return match song_duration {
                Some(ms) => ms >= 30000,
                None => false,
            };
        }
    }

    pub fn update(&mut self, song: SongData, uploaded: bool) {
        if let Some(previous) = &self.previous {
            if !previous.name.eq(&song.name) {
                self.has_uploaded = false;
            } else if self.has_uploaded {
                let prev_ms = previous.progress_ms.unwrap_or(0u32);
                let curr_ms = song.progress_ms.unwrap_or(0u32);
                self.has_uploaded = prev_ms <= curr_ms;
            }
        }
        self.previous = Some(song);

        if !self.has_uploaded {
            self.has_uploaded = uploaded;
        }
    }
}

impl Worker<DB, SpotifyWrapper> {
    pub async fn with_config(config: super::WorkerConfig) -> SpotifyWrapperResult<Self> {
        let db = DB::connect(&config.db_url).await;
        let mut spotify =
            SpotifyWrapper::new(config.client_id, config.client_secret, config.callback_url)?;
        spotify.connect().await?;
        Self::new(db, spotify).await
    }
}

impl<D: SongTracker, S: SpotifyClient> Worker<D, S> {
    pub async fn new(db: D, spotify: S) -> SpotifyWrapperResult<Self> {
        let cache = CachedData::new();
        Ok(Worker { db, cache, spotify })
    }
    pub async fn maybe_add_song(&mut self) -> Option<SongPlay> {
        let current_song = self.spotify.current_playing().await?;
        if self.cache.should_upload(&current_song) {
            let value = self.insert_song(&current_song).await;
            self.cache.update(current_song, value.is_some());
            value
        } else {
            self.cache.update(current_song, false);
            None
        }
    }

    async fn insert_song(&self, song: &SongData) -> Option<SongPlay> {
        match (&song.name, &song.artists, &song.album) {
            (Some(name), Some(artists), Some(album)) => {
                self.db.insert_song(&name, &artists, &album).await
            }
            _ => None,
        }
    }

    pub async fn run(&mut self) {
        println!("{:?}", self.maybe_add_song().await);
    }
}

#[cfg(test)]
mod test {
    use super::{SongData, SongPlay, SongTracker, SpotifyClient, Worker};
    use async_trait::async_trait;
    use std::sync::Mutex;

    type FakeDB = Mutex<Vec<SongPlay>>;

    #[derive(Default)]
    struct FakeSpotify {
        pub val_to_return: Option<SongData>,
    }

    #[async_trait]
    impl SpotifyClient for FakeSpotify {
        async fn current_playing(&mut self) -> Option<SongData> {
            self.val_to_return.clone()
        }
    }

    #[async_trait]
    impl SongTracker for FakeDB {
        async fn insert_song(
            &self,
            name: &str,
            artist: &[String],
            album: &str,
        ) -> Option<SongPlay> {
            let mut inner_vec = self.lock().expect("Able to unwrap LockResult in tests");
            let id = inner_vec.len() as i32;
            let song_play = SongPlay {
                id,
                song_name: name.into(),
                song_artist: artist.into(),
                song_album: album.into(),
                time: None,
            };
            inner_vec.push(song_play.clone());
            Some(song_play)
        }
    }

    fn create_playing_context(progress_ms: Option<u32>) -> SongData {
        SongData {
            name: Some("".into()),
            artists: Some(Vec::default()),
            album: Some("".into()),
            progress_ms,
        }
    }

    #[tokio::test]
    async fn test_cache() {
        let fake_db = FakeDB::new(Vec::new());
        let fake_spotify = FakeSpotify::default();
        let mut worker = Worker::new(fake_db, fake_spotify).await.unwrap();
        let song = create_playing_context(Some(10));
        worker.spotify.val_to_return = Some(song);
        assert_eq!(false, worker.maybe_add_song().await.is_some());

        worker.spotify.val_to_return = Some(create_playing_context(Some(100)));
        assert_eq!(false, worker.maybe_add_song().await.is_some());

        worker.spotify.val_to_return = Some(create_playing_context(Some(30000)));
        assert_eq!(true, worker.maybe_add_song().await.is_some());
        assert_eq!(false, worker.maybe_add_song().await.is_some());

        worker.spotify.val_to_return = Some(create_playing_context(Some(29000)));
        assert_eq!(false, worker.maybe_add_song().await.is_some());

        worker.spotify.val_to_return = Some(create_playing_context(Some(30000)));
        assert_eq!(true, worker.maybe_add_song().await.is_some());

        worker.spotify.val_to_return = Some(create_playing_context(Some(33000)));
        assert_eq!(false, worker.maybe_add_song().await.is_some());

        worker.spotify.val_to_return = Some(create_playing_context(Some(34000)));
        assert_eq!(false, worker.maybe_add_song().await.is_some());

        worker.spotify.val_to_return = Some(create_playing_context(Some(34000)));
        assert_eq!(false, worker.maybe_add_song().await.is_some());
    }
}
