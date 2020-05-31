use crate::{
    hand::HandState,
    messages::*,
    tile::{TileId, Wind},
};
use cs_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::*;

/// Main game state tracking for the client.
#[cs_bindgen]
#[derive(Debug, Clone, Default)]
pub struct ClientState {
    credentials: Option<Credentials>,
    state: Option<AccountState>,
}

#[cs_bindgen]
impl ClientState {
    pub fn new() -> ClientState {
        Default::default()
    }

    pub fn set_credentials(&mut self, id: u64, token: String) {
        let id = AccountId::new(id);
        self.credentials = Some(Credentials { id, token });
    }

    pub fn create_handshake_request(&self) -> String {
        let client_version =
            Version::parse(env!("CARGO_PKG_VERSION")).expect("Failed to parse client version");

        let request = HandshakeRequest {
            client_version,
            credentials: self.credentials.clone(),
        };

        serde_json::to_string(&request).expect("Failed to serialize `HandshakeRequest`")
    }

    /// Deserializes and handles the handshake response received from the server.
    ///
    /// Returns `true` if the handshake response was able to be processed and the server
    /// accepted the handshake request, returns `false` if the server rejected the
    /// request or an error otherwise occurred during the process.
    pub fn handle_handshake_response(&mut self, json: String) -> bool {
        match serde_json::from_str::<HandshakeResponse>(&json) {
            Ok(message) => {
                if let Some(new_credentials) = message.new_credentials {
                    info!(
                        "Overwriting existing credentials, new: {:?}, prev: {:?}",
                        new_credentials, self.credentials,
                    );

                    self.credentials = Some(new_credentials);
                }

                self.state = Some(message.account_data);
                true
            }

            Err(_) => false,
        }
    }

    pub fn create_start_match_request(&self) -> String {
        let request = ClientRequest::StartMatch;
        serde_json::to_string(&request).expect("Failed to serialize request")
    }

    pub fn handle_start_match_response(&self, response: String) -> LocalState {
        let response = serde_json::from_str::<StartMatchResponse>(&response)
            .expect("Failed to deserialize `StartMatchResponse`");

        response.state
    }

    pub fn account_id(&self) -> AccountId {
        self.credentials.as_ref().unwrap().id
    }

    pub fn points(&self) -> u64 {
        self.state.as_ref().unwrap().points
    }
}

// TODO: This should go in a `Mahjong.Match` namespace.
#[cs_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalState {
    pub seat: Wind,
    pub players: [LocalHand; 4],
}

// TODO: This should go in a `Mahjong.Match` namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocalHand {
    Local(HandState),
    Remote { discards: Vec<TileId> },
}
