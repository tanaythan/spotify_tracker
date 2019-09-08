use super::schema::*;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use serde::Serialize;
use std::time::SystemTime;

pub fn establish_connection(db_url: String) -> PgConnection {
    PgConnection::establish(&db_url).unwrap_or_else(|_| panic!("Error connecting to {}", db_url))
}

#[derive(Clone, Debug, Queryable, Serialize)]
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

pub fn lookup_song_by_name(db_conn: &PgConnection, song: String) -> Option<Vec<SongPlay>> {
    use song_plays::*;
    let res = song_plays::table
        .filter(song_name.eq(song))
        .load::<SongPlay>(db_conn);

    match res {
        Ok(songs) => Some(songs),
        Err(e) => {
            println!("Error retrieving: {}", e);
            None
        }
    }
}

pub fn lookup_song(db_conn: &PgConnection, id: i32) -> Option<SongPlay> {
    let res = song_plays::table.find(id).load::<SongPlay>(db_conn);
    match res {
        Ok(song) => match song.get(0) {
            Some(value) => Some((*value).clone()),
            None => None,
        },
        Err(e) => {
            println!("Error retrieving: {}", e);
            None
        }
    }
}
