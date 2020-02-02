use futures::{prelude::*, stream::SplitSink};
use mahjong::Tile;
use thespian::*;
use warp::{filters::ws::Message, ws::WebSocket, Filter};

#[tokio::main]
async fn main() {
    // Create the game state actor and spawn it, holding on to its proxy so that the
    // socket tasks can still communicate with it.
    let stage = GameState::new(mahjong::generate_tileset()).into_stage();
    let game = stage.proxy();
    tokio::spawn(stage.run());

    let client = warp::path("client")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let mut game = game.clone();
            ws.on_upgrade(move |socket| {
                async move {
                    let (sink, mut stream) = socket.split();

                    game.client_connected(ClientConnection { sink })
                        .await
                        .expect("Failed to notify game that a client connected");

                    while let Some(message) = stream.next().await {
                        let _ = dbg!(message);
                    }
                }
            })
        });

    let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));

    let routes = index.or(client);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

#[derive(Debug, Actor)]
struct GameState {
    tiles: Vec<Tile>,
    players: Vec<PlayerState>,
}

#[thespian::actor]
impl GameState {
    /// Removes the first `count` tiles from the deck and returns them.
    ///
    /// If there are fewer than `count` tiles left in the deck, returns an error with the number of remaining tiles.
    pub fn draw_tiles(&mut self, count: usize) -> Result<Vec<Tile>, usize> {
        if self.tiles.len() < count {
            return Err(self.tiles.len());
        }

        Ok(self.tiles.split_off(self.tiles.len() - count))
    }

    pub async fn client_connected(&mut self, mut client: ClientConnection) {
        let hand = self
            .draw_tiles(13)
            .expect("Not enough tiles left in deck to draw");

        let message = serde_json::to_string(&hand).expect("Failed to serialize list of tiles");
        client.send_text(message).await;

        self.players.push(PlayerState {
            hand,
            client: Some(client),
        });
    }
}

impl GameState {
    pub fn new(tiles: Vec<Tile>) -> Self {
        Self {
            tiles,
            players: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct PlayerState {
    hand: Vec<Tile>,

    /// The client that's controlling this player. Will be `None` if there is no
    /// connected client, e.g. if the client disconnected during the game.
    client: Option<ClientConnection>,
}

/// Wrapper around the sender half of the client socket providing a clean api for
/// sending messages to the client.
#[derive(Debug)]
struct ClientConnection {
    sink: SplitSink<WebSocket, Message>,
}

impl ClientConnection {
    pub async fn send_text(&mut self, text: String) {
        self.sink
            .send(Message::text(text))
            .await
            .expect("Failed to send message to client");
    }
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
