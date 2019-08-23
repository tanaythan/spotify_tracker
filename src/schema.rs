table! {
    song_plays (id) {
        id -> Int4,
        song_name -> Varchar,
        song_artist -> Array<Text>,
        song_album -> Varchar,
        time -> Nullable<Timestamp>,
    }
}
