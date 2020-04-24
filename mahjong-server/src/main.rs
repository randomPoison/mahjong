use crate::{client::*, match_controller::*};
use futures::prelude::*;
use mahjong::{match_state::*, messages::*};
use std::{collections::HashMap, sync::Arc};
use thespian::*;
use tracing::*;
use tracing_futures::Instrument;
use warp::Filter;

mod client;
mod match_controller;

#[tokio::main]
async fn main() {
    // Setup the global logger.
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Create the game state actor and spawn it, holding on to its proxy so that the
    // socket tasks can still communicate with it.
    let stage = GameState::new().into_stage();
    let game = stage.proxy();
    tokio::spawn(stage.run());

    let client_id_generator = Arc::new(ClientIdGenerator::new());

    let client = warp::path("client")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let game = game.clone();
            let id = client_id_generator.next();
            ws.on_upgrade(move |socket| {
                async move {
                    // Perform the handshake sequence with the client in order to initiate the session.
                    let (mut client, mut stream) =
                        match ClientController::perform_handshake(id, socket, game).await {
                            Ok(result) => result,

                            // Log the failed connection attempt and then disconnect from the client.
                            Err(err) => {
                                warn!(%err, "Failed to establish connection with client");
                                return;
                            }
                        };

                    while let Some(message) = stream.next().await {
                        match message {
                            Ok(message) => {
                                if message.is_close() {
                                    info!("Socket connection closed, shutting down client");

                                    // TODO: Actually shut down the client actor.

                                    return;
                                }

                                let result = client
                                    .handle_message(message)
                                    .await
                                    .expect("Failed to communicate with client actor");

                                if let Err(err) = result {
                                    error!("Error handling client message: {:?}", err);
                                }
                            }

                            Err(err) => {
                                error!(%err, "Received error message from socket");
                                break;
                            }
                        }
                    }

                    // TODO: Notify game that the client has disconnected.
                }
                .instrument(trace_span!("Socket message pump", %id))
            })
        });

    let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));

    let routes = index.or(client);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

/// Central storage of state data for the game.
///
/// This struct simulates the role of a database, acting as central storage of state data for the game.
#[derive(Debug, Default, Actor)]
pub struct GameState {
    accounts: HashMap<AccountId, Account>,

    // TODO: These ID counters should be generated by the database rather than the server.
    account_id_counter: u64,
    match_id_counter: u32,
}

impl GameState {
    pub fn new() -> Self {
        Default::default()
    }
}

#[thespian::actor]
impl GameState {
    #[tracing::instrument(skip(self))]
    pub fn create_account(&mut self) -> Account {
        // Increment the account ID counter to get the next unused ID.
        self.account_id_counter += 1;
        let id = AccountId::new(self.account_id_counter);

        info!(%id, "Creating new player account");

        // Create the credentials for the new account. For now we generate dummy
        // credentials, eventually this will be replaced with some system for
        // generating credentials.
        let token = String::from("DUMMY");
        let credentials = Credentials { id, token };

        // Setup initial state for the account. We'll start players out with 10,000
        // points because why not.
        let data = PlayerState { points: 10_000 };

        // Store the new account.
        let account = Account { credentials, data };
        let old = self.accounts.insert(id, account.clone());
        assert!(old.is_none(), "Created duplicate account, id: {:?}", id);

        account
    }

    #[tracing::instrument(skip(self))]
    pub fn start_match(&mut self) -> MatchControllerProxy {
        self.match_id_counter += 1;
        let id = MatchId::new(self.match_id_counter);

        info!(%id, "Starting a new match");

        let stage = MatchController::new(id).into_stage();
        let proxy = stage.proxy();
        tokio::spawn(stage.run());

        proxy
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    credentials: Credentials,
    data: PlayerState,
}

static INDEX_HTML: &str = r#"
<!DOCTYPE html>
<html>
    <head>
        <title>Warp Chat</title>
    </head>
    <body>
        <h1>warp chat</h1>
        <div id="chat">
            <p><em>Connecting...</em></p>
        </div>
        <input type="text" id="text" />
        <button type="button" id="send">Send</button>
        <script type="text/javascript">
        var uri = 'ws://' + location.host + '/client';
        var ws = new WebSocket(uri);
        function message(data) {
            var line = document.createElement('p');
            line.innerText = data;
            chat.appendChild(line);
        }
        ws.onopen = function() {
            chat.innerHTML = "<p><em>Connected!</em></p>";
        }
        ws.onmessage = function(msg) {
            message(msg.data);
        };
        send.onclick = function() {
            var msg = text.value;
            ws.send(msg);
            text.value = '';
            message('<You>: ' + msg);
        };
        </script>
    </body>
</html>
"#;
