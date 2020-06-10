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
    async fn insert_song(&self, name: &str, artist: Vec<String>, album: &str) -> Option<SongPlay>;
}

#[derive(Clone, Debug)]
pub struct SongPlay {
    pub id: i32,
    pub song_name: String,
    pub song_artist: Vec<String>,
    pub song_album: String,
    pub time: Option<PrimitiveDateTime>,
}

pub struct NewSongPlay<'a> {
    pub song_name: &'a str,
    pub song_artist: Vec<&'a str>,
    pub song_album: &'a str,
}

#[async_trait]
impl SongTracker for DB {
    async fn insert_song(&self, name: &str, artist: Vec<String>, album: &str) -> Option<SongPlay> {
        let res = sqlx::query!(
            "INSERT INTO song_plays (song_name, song_artist, song_album) VALUES ($1, $2, $3)",
            name,
            &artist,
            album,
        )
        .execute(&self.db)
        .await;
        match res {
            Ok(_) => Some(SongPlay {
                id: 0,
                song_name: name.into(),
                song_artist: artist,
                song_album: album.into(),
                time: None,
            }),
            Err(_) => None,
        }
    }
}

pub async fn lookup_song_by_name<'a>(db: &PgPool, song: &'a str) -> Option<Vec<SongPlay>> {
    let res = sqlx::query_as!(
        SongPlay,
        "SELECT * from song_plays where song_name = $1",
        song,
    )
    .fetch_all(db)
    .await;
    match res {
        Ok(songs) => Some(songs),
        Err(e) => {
            println!("Error retrieving: {}", e);
            None
        }
    }
}

pub async fn lookup_song(db: &PgPool, id: i32) -> Option<SongPlay> {
    let res = sqlx::query_as!(SongPlay, "SELECT * from song_plays where id = $1", id)
        .fetch_one(db)
        .await;

    match res {
        Ok(song) => Some(song.clone()),
        Err(e) => {
            println!("Error retrieving: {}", e);
            None
        }
    }
}
