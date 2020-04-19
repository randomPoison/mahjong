use crate::game::MatchState;
use cs_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

pub use semver::Version;

/// Initial handshake request sent by the client after establishing a connection to
/// the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeRequest {
    /// The version of the client making the handshake request.
    ///
    /// If the client version is incompatible with the server, the server will respond
    /// with an error notifying the player that they must update their client before
    /// playing.
    pub client_version: Version,

    /// The ID and token for the account that the client is attempting to log into.
    pub credentials: Option<Credentials>,
}

/// Response to a client's handshake request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub server_version: Version,

    pub new_credentials: Option<Credentials>,
    pub account_data: PlayerState,
}

/// Unique ID for a game account.
#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct AccountId(u64);

impl AccountId {
    /// Creates a new `AccountId` from a numeric ID.
    ///
    /// In general, only the server should create new account IDs. Avoid constructing
    /// new IDs in client code, since there's no guarantee that the ID will be valid.
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    // TODO: Remove this once we can directly pass `AccountId` values to and from C#.
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Unique ID for a client session.
///
/// Each time a new client connects, the server generates an ID for that session.
/// Session IDs are guaranteed to be unique among all active sessions. Once a
/// session ends, the ID may be reused.
#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SessionId(u32);

#[cs_bindgen]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    /// The points balance for the player, currently the only resource in the game.
    pub points: u64,
}

#[cs_bindgen]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    pub id: AccountId,

    // TODO: Use a more structured type for the account token. For now we'll just use a
    // psuedo-random string until we have some actual authentication system in place.
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientRequest {
    StartMatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartMatchResponse {
    pub state: MatchState,
}
