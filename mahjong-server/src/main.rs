use derive_more::From;
use futures::{
    prelude::*,
    stream::{SplitSink, SplitStream},
};
use mahjong::messages::*;
use thespian::*;
use warp::{filters::ws::Message, ws::WebSocket, Filter};

#[tokio::main]
async fn main() {
    // Create the game state actor and spawn it, holding on to its proxy so that the
    // socket tasks can still communicate with it.
    let stage = GameState::new().into_stage();
    let game = stage.proxy();
    tokio::spawn(stage.run());

    let client = warp::path("client")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let game = game.clone();
            ws.on_upgrade(move |socket| {
                async move {
                    // Perform the handshake sequence with the client in order to initiate the session.
                    let (mut client, mut stream) =
                        match ClientConnection::perform_handshake(socket, game).await {
                            Ok(result) => result,

                            // Log the failed connection attempt and then disconnect from the client.
                            Err(err) => {
                                dbg!(&err);
                                return;
                            }
                        };

                    while let Some(message) = stream.next().await {
                        match message {
                            Ok(message) => client
                                .handle_message(message)
                                .await
                                .expect("Failed to send message to client actor"),

                            Err(err) => {
                                dbg!(err);
                                break;
                            }
                        }
                    }

                    todo!("Notify game that client disconnected");
                }
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
struct GameState {
    players: Vec<PlayerState>,
}

impl GameState {
    pub fn new() -> Self {
        Default::default()
    }
}

#[thespian::actor]
impl GameState {
    pub fn create_account(&mut self) -> (AccountId, String) {
        todo!()
    }
}

#[derive(Debug)]
struct PlayerState {
    /// The points balance for the player, currently the only resource in the game.
    points: u64,
}

/// Actor managing an active session with a client.
#[derive(Debug, Actor)]
struct ClientConnection {
    /// The sender half of the socket connection with the client.
    sink: SplitSink<WebSocket, Message>,
}

impl ClientConnection {
    /// Attempts to perform the session handshake with the client, returning a new
    /// `ClientConnection` if it succeeds.
    async fn perform_handshake(
        socket: WebSocket,
        mut game: <GameState as Actor>::Proxy,
    ) -> Result<(ClientConnectionProxy, SplitStream<WebSocket>), HandshakeError> {
        let (sink, mut stream) = socket.split();

        let request = stream
            .next()
            .await
            .ok_or(HandshakeError::ClientDisconnected)??;

        let request = request
            .to_str()
            .map_err(|_| HandshakeError::InvalidRequest)?;
        let request: HandshakeRequest = serde_json::from_str(request)?;

        let server_version =
            Version::parse(env!("CARGO_PKG_VERSION")).expect("Failed to parse server version");
        if server_version != request.client_version {
            todo!("Handle incompatible client version");
        }

        let account = match request.credentials {
            Some(..) => todo!("Support logging into an existing account"),
            None => game.create_account().await?,
        };

        todo!("Send response to the client");
    }
}

#[thespian::actor]
impl ClientConnection {
    fn handle_message(&mut self, message: Message) {
        dbg!(message);
    }

    /// Sends the provided string as a message to the client.
    async fn send_text(&mut self, text: String) {
        self.sink
            .send(Message::text(text))
            .await
            .expect("Failed to send message to client");
    }
}

#[derive(Debug, From)]
enum HandshakeError {
    ClientDisconnected,
    InvalidRequest,
    Socket(warp::Error),
    Json(serde_json::Error),
    Actor(thespian::MessageError),
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
