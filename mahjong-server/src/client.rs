use crate::{game::*, GameState};
use anyhow::*;
use derive_more::Display;
use futures::{
    prelude::*,
    stream::{SplitSink, SplitStream},
};
use mahjong::messages::*;
use std::sync::atomic::{AtomicU64, Ordering};
use thespian::*;
use tracing::*;
use warp::{filters::ws::Message as WsMessage, ws::WebSocket};

/// Actor managing an active session with a client.
#[derive(Debug, Actor)]
pub struct ClientController {
    id: ClientId,

    /// The sender half of the socket connection with the client.
    sink: SplitSink<WebSocket, WsMessage>,
    game: <GameState as Actor>::Proxy,
    state: ClientState,
}

impl ClientController {
    /// Attempts to perform the session handshake with the client, returning a new
    /// `ClientConnection` if it succeeds.
    #[tracing::instrument(skip(id, socket, game))]
    pub async fn perform_handshake(
        id: ClientId,
        socket: WebSocket,
        mut game: <GameState as Actor>::Proxy,
    ) -> Result<(<ClientController as Actor>::Proxy, SplitStream<WebSocket>)> {
        let span = info_span!("ClientController::perform_handshake", %id);
        let _span = span.enter();

        info!("Starting client handshake");

        let (mut sink, mut stream) = socket.split();

        // HACK: Send an initial text message to the client after establishing a
        // connection. It looks like there's a bug in WebSocketSharp that means it won't
        // recognize that the connection has been established unit it receives a message,
        // causing the client to hang. This won't be necessary once we move off of web
        // sockets.
        sink.send(WsMessage::text("ping"))
            .await
            .expect("Failed to send initial ping");

        trace!("Sent the client the initial ping, awaiting the handshake request");

        // Wait for the client to send the handshake.
        //
        // TODO: Include a timeout so that we don't wait forever, otherwise this is a vector
        // for DOS attacks.
        let request = stream
            .next()
            .await
            .ok_or(anyhow!("Client disconnected during initial handshake"))?
            .context("Waiting for response to handshake ping")?;

        // Parse the request data.
        let request = request
            .to_str()
            .map_err(|_| anyhow!("Incoming socket message is not a string: {:?}", request))?;
        let request: HandshakeRequest = serde_json::from_str(request)?;

        trace!("Received handshake request from client");

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

        info!("Verified handshake request, completing client connection");

        // Create the response message and send it to the client.
        let response = HandshakeResponse {
            server_version,
            new_credentials: Some(account.credentials),
            account_data: account.data,
        };
        let response =
            serde_json::to_string(&response).expect("Failed to serialize `HandshakeResponse`");
        sink.send(WsMessage::text(response)).await?;

        // Create the actor for the client connection and spawn it.
        let stage = ClientController {
            id,
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

    /// Sends the provided string as a message to the client.
    async fn send_text(&mut self, text: String) -> Result<()> {
        self.sink
            .send(WsMessage::text(text))
            .await
            .context("Failed to send message to client")
    }
}

#[thespian::actor]
impl ClientController {
    pub async fn handle_message(&mut self, message: WsMessage) -> Result<()> {
        let span = trace_span!("ClientController::handle_message", id = %self.id);
        let _span = span.enter();

        let text = match message.to_str() {
            Ok(text) => text,
            Err(_) => bail!("Received non-text message: {:?}", message),
        };

        let request = serde_json::from_str::<ClientRequest>(text)?;
        trace!(?request, "Handling incoming request");

        match request {
            ClientRequest::StartMatch => {
                // TODO: Do an error if the client is already in a match (or would otherwise not be
                // able to start a match).

                trace!("Asking the game controller to start a match...");

                let mut controller = self
                    .game
                    .start_match()
                    .await
                    .expect("Failed to start match");

                trace!("Match started, getting initial state...");

                let state = controller
                    .state()
                    .await
                    .expect("Failed to get initial state from match controller");
                let response = serde_json::to_string(&StartMatchResponse { state })
                    .expect("Failed to serialize `StartMatchResponse`");
                self.send_text(response).await?;

                trace!("Sent initial state to client, transitioning controller to `InMatch`");
                self.state = ClientState::InMatch { controller };
            }

            ClientRequest::DiscardTile(request) => {
                let controller = match &mut self.state {
                    ClientState::InMatch { controller } => controller,
                    _ => bail!("Cannot discard a tile when not in a match"),
                };

                trace!("Forwarding discard request to match controller");

                let match_state = controller
                    .discard_tile(request.player, request.tile)
                    .await??;

                let response = serde_json::to_string(&DiscardTileResponse {
                    success: true,
                    state: match_state,
                })?;
                self.send_text(response).await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
enum ClientState {
    Idle,
    InMatch { controller: MatchControllerProxy },
}

/// Identifier for a connected client session.
///
/// Each connected client session is given an ID when the connection is established.
/// IDs are not guaranteed to be unique over the lifetime of the server application
/// (IDs may be reused after enough sessions are created), but are guaranteed to be
/// unique while the session is active (i.e. no two active sessions will have the
/// same ID).
// TODO: Actually guarantee that IDs are unique. This will require some kind of
// tracking of active sessions IDs to prevent duplicates from being issued.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display(fmt = "{}", _0)]
pub struct ClientId(u64);

pub struct ClientIdGenerator(AtomicU64);

impl ClientIdGenerator {
    pub fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    pub fn next(&self) -> ClientId {
        ClientId(self.0.fetch_add(1, Ordering::SeqCst))
    }
}
