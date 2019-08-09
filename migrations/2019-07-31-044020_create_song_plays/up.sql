-- Your SQL goes here
CREATE TABLE IF NOT EXISTS song_plays (
  id SERIAL PRIMARY KEY,
  song_name VARCHAR NOT NULL,
  song_artist VARCHAR NOT NULL,
  time TIMESTAMP DEFAULT NOW()
);
