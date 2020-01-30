use futures::prelude::*;
use mahjong::Tile;
use thespian::*;
use warp::{filters::ws::Message, Filter};

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
            ws.on_upgrade(move |mut socket| {
                async move {
                    let player_tiles = game
                        .draw_tiles(13)
                        .await
                        .expect("Error when communicating with game actor while drawing tiles")
                        .expect("Not enough tiles left in deck to draw");

                    let message = serde_json::to_string(&player_tiles)
                        .expect("Failed to serialize list of tiles");
                    socket
                        .send(Message::text(message))
                        .await
                        .expect("Failed to send initial hand to client");

                    // TODO: Process incoming messages from the client.
                }
            })
        });

    let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));

    let routes = index.or(client);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

#[derive(Debug, Clone, Actor)]
struct GameState {
    tiles: Vec<Tile>,
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
}

impl GameState {
    pub fn new(tiles: Vec<Tile>) -> Self {
        Self { tiles }
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
