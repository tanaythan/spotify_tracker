use super::schema::*;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::time::SystemTime;

pub fn establish_connection(db_url: String) -> PgConnection {
    PgConnection::establish(&db_url).unwrap_or_else(|_| panic!("Error connecting to {}", db_url))
}

#[derive(Debug, Queryable)]
pub struct SongPlay {
    pub id: i32,
    pub song_name: String,
    pub song_artist: Vec<String>,
    pub song_album: String,
    pub time: Option<SystemTime>,
}

#[derive(Insertable)]
#[table_name = "song_plays"]
pub struct NewSongPlay<'a> {
    pub song_name: &'a str,
    pub song_artist: Vec<&'a str>,
    pub song_album: &'a str,
}

pub fn insert_song<'a>(
    db_conn: &PgConnection,
    song_name: &'a str,
    song_artist: Vec<&'a str>,
    song_album: &'a str,
) -> Option<SongPlay> {
    let song_play = NewSongPlay {
        song_name,
        song_artist,
        song_album,
    };

    let res = diesel::insert_into(song_plays::table)
        .values(&song_play)
        .get_result(db_conn);
    match res {
        Ok(songplay) => Some(songplay),
        Err(e) => {
            println!("Error inserting: {}", e);
            None
        }
    }
}
