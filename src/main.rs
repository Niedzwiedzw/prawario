mod config;

use std::net::TcpListener;
use std::thread::spawn;
use tungstenite::server::accept;

fn main() {
    println!("starting server at {}", config::BACKEND_ADDRESS);
    let server = TcpListener::bind(config::BACKEND_ADDRESS).unwrap();
    for stream in server.incoming() {
        spawn(move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {
                let msg = websocket.read_message().unwrap();

                // We do not want to send back ping/pong messages.
                if msg.is_binary() || msg.is_text() {
                    websocket.write_message(msg).unwrap();
                }
            }
        });
    }
}
