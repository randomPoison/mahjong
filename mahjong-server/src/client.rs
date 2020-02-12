use crate::{game::*, GameState};
use derive_more::From;
use futures::{
    prelude::*,
    stream::{SplitSink, SplitStream},
};
use mahjong::messages::*;
use snafu::*;
use thespian::*;
use warp::{filters::ws::Message as WsMessage, ws::WebSocket};

/// Actor managing an active session with a client.
#[derive(Debug, Actor)]
pub struct ClientController {
    /// The sender half of the socket connection with the client.
    sink: SplitSink<WebSocket, WsMessage>,
    game: <GameState as Actor>::Proxy,
    state: ClientState,
}

impl ClientController {
    /// Attempts to perform the session handshake with the client, returning a new
    /// `ClientConnection` if it succeeds.
    pub async fn perform_handshake(
        socket: WebSocket,
        mut game: <GameState as Actor>::Proxy,
    ) -> Result<(<ClientController as Actor>::Proxy, SplitStream<WebSocket>), HandshakeError> {
        let (mut sink, mut stream) = socket.split();

        // HACK: Send an initial text message to the client after establishing a
        // connection. It looks like there's a bug in WebSocketSharp that means it won't
        // recognize that the connection has been established unit it receives a message,
        // causing the client to hang. This won't be necessary once we move off of web
        // sockets.
        sink.send(WsMessage::text("ping"))
            .await
            .expect("Failed to send initial ping");

        // Wait for the client to send the handshake.
        //
        // TODO: Include a timeout so that we don't wait forever, otherwise this is a vector
        // for DOS attacks.
        let request = stream
            .next()
            .await
            .ok_or(HandshakeError::ClientDisconnected)??;

        // Parse the request data.
        let request = request
            .to_str()
            .map_err(|_| HandshakeError::InvalidRequest)?;
        let request: HandshakeRequest = serde_json::from_str(request)?;

        // Verify that the client is compatible with the current server version. For now
        // we only check that the client version matches the server version, which is
        // enough for development purposes. Once we're in production we may want a more
        // permissive strategy that allows us to push server updates without invalidating
        // existing clients.
        let server_version =
            Version::parse(env!("CARGO_PKG_VERSION")).expect("Failed to parse server version");
        if server_version != request.client_version {
            todo!("Handle incompatible client version");
        }

        // Get account information from the server, creating a new account if the client
        // did not provide credentials for an existing account.
        let account = match request.credentials {
            Some(..) => todo!("Support logging into an existing account"),
            None => game.create_account().await?,
        };

        // Create the response message and send it to the client.
        let response = HandshakeResponse {
            server_version,
            new_credentials: Some(account.credentials),
            account_data: account.data,
        };
        let response =
            serde_json::to_string(&response).expect("Failed to serialize `HandshakeResponse`");
        dbg!(&response);
        sink.send(WsMessage::text(response)).await?;

        // Create the actor for the client connection and spawn it.
        let stage = ClientController {
            sink,
            game,
            state: ClientState::Idle,
        }
        .into_stage();
        let client = stage.proxy();
        tokio::spawn(stage.run());

        // TODO: Track the active session in the central game state.

        Ok((client, stream))
    }
}

#[thespian::actor]
impl ClientController {
    pub async fn handle_message(&mut self, message: WsMessage) -> Result<(), MessageError> {
        let text = match message.to_str() {
            Ok(text) => text,
            Err(_) => return Err(MessageError::NonText { message }),
        };

        let request = serde_json::from_str::<ClientRequest>(text)?;

        match request {
            ClientRequest::StartMatch => {
                // TODO: Do an error if the client is already in a match (or would otherwise not be
                // able to start a match).

                let controller = self
                    .game
                    .start_match()
                    .await
                    .expect("Failed to start match");

                self.state = ClientState::InMatch { controller };
            }
        }

        Ok(())
    }

    /// Sends the provided string as a message to the client.
    async fn send_text(&mut self, text: String) {
        self.sink
            .send(WsMessage::text(text))
            .await
            .expect("Failed to send message to client");
    }
}

#[derive(Debug, Clone)]
enum ClientState {
    Idle,
    InMatch { controller: MatchControllerProxy },
}

#[derive(Debug, From)]
pub enum HandshakeError {
    ClientDisconnected,
    InvalidRequest,
    Socket(warp::Error),
    Json(serde_json::Error),
    Actor(thespian::MessageError),
}

#[derive(Debug, Snafu)]
pub enum MessageError {
    NonText {
        message: WsMessage,
    },

    #[snafu(context(false))]
    BadJson {
        source: serde_json::Error,
    },
}
