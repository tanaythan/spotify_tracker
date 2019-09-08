use crate::db::{establish_connection, lookup_song, lookup_song_by_name};
use diesel::PgConnection;
use gotham::helpers::http::response::{create_empty_response, create_response};
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::single::single_pipeline;
use gotham::pipeline::single_middleware;
use gotham::router::{builder::*, Router};
use gotham::state::{FromState, State};
use gotham_derive::*;
use hyper::{Body, Response, StatusCode};
use mime;
use serde::Deserialize;
use serde_json;
use std::net::SocketAddrV4;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone, StateData)]
struct RequestConnection {
    conn: Arc<Mutex<PgConnection>>,
}

impl RequestConnection {
    pub fn new(db_url: String) -> Self {
        RequestConnection {
            conn: Arc::new(Mutex::new(establish_connection(db_url))),
        }
    }

    pub fn get_conn(&self) -> MutexGuard<PgConnection> {
        self.conn.lock().unwrap()
    }
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct SongPath {
    song: String,
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct SongIdPath {
    id: i32,
}

pub struct Server {
    addr: SocketAddrV4,
    state: RequestConnection,
}

impl Server {
    pub fn new(db_url: String) -> Server {
        Server {
            addr: "127.0.0.1:8888".parse().unwrap(),
            state: RequestConnection::new(db_url),
        }
    }

    pub fn run(&self) {
        gotham::start(self.addr, self.router());
    }

    fn router(&self) -> Router {
        let middleware = StateMiddleware::new(self.state.clone());

        let pipeline = single_middleware(middleware);

        let (chain, pipelines) = single_pipeline(pipeline);

        build_router(chain, pipelines, |route| {
            route
                .get("/api/song/:song")
                .with_path_extractor::<SongPath>()
                .to(Server::fetch_songs);
            route
                .get("/api/id/:id")
                .with_path_extractor::<SongIdPath>()
                .to(Server::fetch_song_by_id);
        })
    }

    fn fetch_songs(state: State) -> (State, Response<Body>) {
        let message = {
            let conn_wrapper = RequestConnection::borrow_from(&state);
            let conn = conn_wrapper.get_conn();

            let query_param = SongPath::borrow_from(&state);
            let songs = lookup_song_by_name(&(*conn), query_param.song.clone());

            create_response(
                &state,
                StatusCode::OK,
                mime::APPLICATION_JSON,
                serde_json::to_vec(&songs).expect("Serialized Songs"),
            )
        };

        // return message
        (state, message)
    }

    fn fetch_song_by_id(state: State) -> (State, Response<Body>) {
        let message = {
            let conn_wrapper = RequestConnection::borrow_from(&state);
            let conn = conn_wrapper.get_conn();

            let query_param = SongIdPath::borrow_from(&state);
            let song = lookup_song(&(*conn), query_param.id);
            match song {
                Some(s) => create_response(
                    &state,
                    StatusCode::OK,
                    mime::APPLICATION_JSON,
                    serde_json::to_string(&s).expect("Serialized"),
                ),
                None => create_empty_response(&state, StatusCode::NOT_FOUND),
            }
        };
        println!("Message: {:?}", message);

        (state, message)
    }
}
