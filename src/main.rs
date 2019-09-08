mod db;
mod schema;
mod server;
mod worker;

#[macro_use]
extern crate diesel;

use dotenv;
use server::Server;
use worker::Worker;

#[derive(Clone, Debug)]
pub struct Config {
    pub client_id: String,
    pub client_secret: String,
    pub callback_url: String,
    pub db_url: String,
}

fn main() {
    dotenv::dotenv().ok();
    let config = Config {
        client_id: dotenv::var("CLIENT_ID").unwrap(),
        client_secret: dotenv::var("CLIENT_SECRET").unwrap(),
        callback_url: dotenv::var("CALLBACK_URL").unwrap(),
        db_url: dotenv::var("DATABASE_URL").unwrap(),
    };

    let mut worker = Worker::new(&config);

    let server = Server::new(config.db_url.clone());

    std::thread::spawn(move || {
        worker.run();
    });

    server.run();
}
