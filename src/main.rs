mod db;
mod schema;
mod server;
mod worker;

#[macro_use]
extern crate diesel;
extern crate tokio;

use dotenv;
use server::Server;
use worker::Worker;

use std::{thread, time};

#[derive(Clone, Debug)]
pub struct WorkerConfig {
    pub client_id: String,
    pub client_secret: String,
    pub callback_url: String,
    pub db_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().expect("Environment variables loaded");
    let config = WorkerConfig {
        client_id: dotenv::var("CLIENT_ID").unwrap(),
        client_secret: dotenv::var("CLIENT_SECRET").unwrap(),
        callback_url: dotenv::var("CALLBACK_URL").unwrap(),
        db_url: dotenv::var("DATABASE_URL").unwrap(),
    };

    let server = Server::new(config.db_url.clone());

    tokio::spawn(async {
        loop {
            let config = WorkerConfig {
                client_id: dotenv::var("CLIENT_ID").unwrap(),
                client_secret: dotenv::var("CLIENT_SECRET").unwrap(),
                callback_url: dotenv::var("CALLBACK_URL").unwrap(),
                db_url: dotenv::var("DATABASE_URL").unwrap(),
            };
            let mut worker = Worker::new(&config).unwrap();
            worker.connect().await.unwrap();
            worker.run().await;
            thread::sleep(time::Duration::from_secs(5));
        }
    });

    server.run().await;
    Ok(())
}
