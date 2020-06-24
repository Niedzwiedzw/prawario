#![feature(async_closure)]

mod communication;
mod config;
mod game;
mod rendering;

// #![deny(warnings)]
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use game::VectorDef;

use futures_new::{FutureExt, StreamExt};
use serde_json::{from_str, to_string};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use warp::ws::{Message, WebSocket};
use warp::Filter;

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;
pub type GameState = Arc<RwLock<game::Game>>;

async fn run_game(game: GameState) {
    loop {
        tokio::time::delay_for(Duration::from_millis(1000 / 60)).await;
        game.write().await.step();
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // Keep track of all connected users, key is usize, value
    // is a websocket sender.
    let users = Users::default();
    // Turn our "state" into a new Filter...
    let users = warp::any().map(move || users.clone());

    // do the same for the game state
    let game_state: GameState = Default::default();
    let game_running_state = Arc::clone(&game_state);
    let app = async || run_game(game_running_state).await;

    tokio::task::spawn(app());

    let game_state = warp::any().map(move || game_state.clone());
    // GET /chat -> websocket upgrade
    let chat = warp::path("game")
        // The `ws()` filter will prepare Websocket handshake...
        .and(warp::ws())
        .and(users)
        .and(game_state)
        .map(|ws: warp::ws::Ws, users, game_state| {
            // This will call our function if the handshake succeeds.
            ws.on_upgrade(move |socket| user_connected(socket, users, game_state))
        });

    // GET / -> index html
    // let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));
    let index = warp::any()
        .and(warp::fs::dir("dist/"));

    let routes = chat.or(index);

    warp::serve(routes).run(([0, 0, 0, 0], 80)).await;
}

async fn user_connected(ws: WebSocket, users: Users, game_state: GameState) {
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
    game_state.write().await.add(my_id);
    println!("new player ID: {}", my_id);
    // Split the socket into a sender and receive of messages.
    let (user_ws_tx, mut user_ws_rx) = ws.split();

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::task::spawn(rx.forward(user_ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
    }));

    // Save the sender in our list of connected users.
    users.write().await.insert(my_id, tx);

    // for (&uid, tx) in users.read().await.iter() {
    users
        .read()
        .await
        .get(&my_id)
        .expect("cannot find just inserted user...?")
        .send(Ok(Message::text(
            game_state.read().await.state_dump().clone(),
        )));

    // Return a `Future` that is basically a state machine managing
    // this specific user's connection.

    // Make an extra clone to give to our disconnection handler...
    let users2 = users.clone();

    // Every time the user sends a message, broadcast it to
    // all other users...
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error(uid={}): {}", my_id, e);
                break;
            }
        };
        user_message(my_id, msg, &users, &game_state).await;
    }

    // user_ws_rx stream will keep processing as long as the user stays
    // connected. Once they disconnect, then...
    user_disconnected(my_id, &users2).await;
}

async fn user_message(my_id: usize, msg: Message, users: &Users, game_state: &GameState) {
    // Skip any non-Text messages...
    let msg = if let Ok(s) = msg.to_str() {
        s
    } else {
        return;
    };
    if msg == "ping" {
        let hello_message =
            communication::ServerMessage::HelloPlayer(my_id, (*game_state.read().await).clone());
        if let Err(_disconnected) = users
            .read()
            .await
            .get(&my_id)
            .expect(format!("user not found: [#{}]", my_id).as_str())
            .send(Ok(Message::text(
                to_string(&hello_message).expect(
                    format!(
                        "failed to serialize server hello message: {:#?}",
                        hello_message
                    )
                    .as_str(),
                ),
            )))
        {
            // The tx is disconnected, our `user_disconnected` code
            // should be happening in another task, nothing more to
            // do here.
        }
    } else if let Ok(message) = from_str::<communication::ClientMessage>(msg) {
        game_state.write().await.handle_client_message(&message);
        for (&uid, tx) in users.read().await.iter() {
            if let Err(_disconnected) = tx.send(Ok(Message::text(
                to_string(&*game_state.read().await)
                    .expect(format!("failed to serialize user message: {:#?}", message).as_str()),
            ))) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
            }
        }
    } else {
        println!("error: failed to parse message: {:#?}", msg);
    }
    // New message from this user, send it to everyone else (except same uid)...
}

async fn user_disconnected(my_id: usize, users: &Users) {
    eprintln!("good bye user: {}", my_id);

    // Stream closed up, so remove from the user list
    users.write().await.remove(&my_id);
}

static INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <title>Warp Chat</title>
    </head>
    <body>
        <h1>Warp chat</h1>
        <div id="chat">
            <p><em>Connecting...</em></p>
        </div>
        <input type="text" id="text" />
        <button type="button" id="send">Send</button>
        <script type="text/javascript">
        const chat = document.getElementById('chat');
        const text = document.getElementById('text');
        const uri = 'ws://' + location.host + '/chat';
        const ws = new WebSocket(uri);
        function message(data) {
            const line = document.createElement('p');
            line.innerText = data;
            chat.appendChild(line);
        }
        ws.onopen = function() {
            chat.innerHTML = '<p><em>Connected!</em></p>';
        };
        ws.onmessage = function(msg) {
            message(msg.data);
        };
        ws.onclose = function() {
            chat.getElementsByTagName('em')[0].innerText = 'Disconnected!';
        };
        send.onclick = function() {
            const msg = text.value;
            ws.send(msg);
            text.value = '';
            message('<You>: ' + msg);
        };
        </script>
    </body>
</html>
"#;
