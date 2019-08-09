table! {
    song_plays (id) {
        id -> Int4,
        song_name -> Varchar,
        song_artist -> Varchar,
        time -> Nullable<Timestamp>,
    }
}
