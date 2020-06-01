use crate::{
    hand::{Call, HandState},
    match_state::MatchId,
    messages::*,
    tile::{TileId, Wind},
};
use cs_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// The local state that a client has access to when playing in an online match.
// TODO: This should go in a `Mahjong.Match` namespace.
#[cs_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalState {
    pub id: MatchId,
    pub seat: Wind,
    pub players: HashMap<Wind, LocalHand>,
    pub turn_state: LocalTurnState,
}

#[cs_bindgen]
impl LocalState {
    pub fn id(&self) -> MatchId {
        self.id
    }

    pub fn seat(&self) -> Wind {
        self.seat
    }

    // HACK: Expose separate getters for remote and local hands because we can't
    // directly expose `LocalHand`. This is because `LocalHand` is a value type which
    // contains a handle type `HandState`, and right now cs-bindgen doesn't support
    // handle types within value types. Once the necessary functionality is added
    // upstream we should be able to directly return a `&LocalHand` directly.
    //
    // See https://github.com/randomPoison/cs-bindgen/issues/59.

    pub fn local_hand(&self, seat: Wind) -> HandState {
        match &self.players[&seat] {
            LocalHand::Local(hand) => hand.clone(),

            _ => panic!(
                "Expected seat {:?} to be the local hand, but was a remote hand",
                seat,
            ),
        }
    }

    pub fn remote_discards(&self, seat: Wind) -> Vec<TileId> {
        match &self.players[&seat] {
            LocalHand::Remote { discards, .. } => discards.clone(),

            _ => panic!(
                "Expected seat {:?} to be remote, but was the local hand",
                seat,
            ),
        }
    }

    pub fn player_has_current_draw(&self, seat: Wind) -> bool {
        match &self.players[&seat] {
            LocalHand::Remote { has_draw, .. } => *has_draw,
            LocalHand::Local(state) => state.current_draw().is_some(),
        }
    }

    pub fn turn_state(&self) -> LocalTurnState {
        self.turn_state.clone()
    }
}

/// The turn information for `LocalState`.
///
/// Mirrors `TurnState` but doesn't expose state information about players other
/// than the local one. Notably, the `AwaitingCalls` state doesn't include the list
/// of players that can call the discarded tile.
#[cs_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocalTurnState {
    AwaitingDraw(Wind),

    AwaitingDiscard(Wind),

    AwaitingCalls {
        discarding_player: Wind,
        discard: TileId,
        calls: Vec<Call>,
    },

    MatchEnded {
        winner: Wind,
    },
}

// TODO: This should go in a `Mahjong.Match` namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocalHand {
    /// The hand for the player controlled by the client. Contains the full state
    /// information for the hand.
    Local(HandState),

    /// The hand information for a remote player. Only contains the discards for the
    /// player.
    Remote {
        has_draw: bool,
        discards: Vec<TileId>,
    },
}
