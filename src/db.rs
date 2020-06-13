use async_trait::async_trait;
use sqlx::postgres::PgPool;
use sqlx::types::time::PrimitiveDateTime;

async fn establish_connection(db_url: &str) -> PgPool {
    PgPool::builder().max_size(5).build(db_url).await.unwrap()
}

pub struct DB {
    db: PgPool,
}

impl DB {
    pub async fn connect(db_url: &str) -> Self {
        let db = establish_connection(db_url).await;
        Self { db }
    }
}

#[async_trait]
pub trait SongTracker {
    async fn insert_song(&self, name: &str, artists: &[String], album: &str) -> Option<SongPlay>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct SongPlay {
    pub id: i32,
    pub song_name: String,
    pub song_artist: Vec<String>,
    pub song_album: String,
    pub time: Option<PrimitiveDateTime>,
}

#[async_trait]
impl SongTracker for DB {
    async fn insert_song(&self, name: &str, artists: &[String], album: &str) -> Option<SongPlay> {
        let res = sqlx::query_as!(
            SongPlay,
            "INSERT INTO song_plays (song_name, song_artist, song_album) VALUES ($1, $2, $3) RETURNING id, song_name, song_album, song_artist, time",
            name,
            artists,
            album,
        )
        .fetch_one(&self.db)
        .await;
        match res {
            Ok(play) => Some(play),
            Err(e) => {
                eprintln!("Detected error inserting into DB: {}", e);
                None
            }
        }
    }
}
