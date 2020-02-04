pub mod messages;
pub mod tile;

use crate::messages::{HandshakeRequest, HandshakeResponse, Version};
use cs_bindgen::prelude::*;

cs_bindgen::generate_static_bindings!();

#[cs_bindgen]
pub fn create_handshake_request() -> String {
    let client_version =
        Version::parse(env!("CARGO_PKG_VERSION")).expect("Failed to parse client version");
    let request = HandshakeRequest {
        client_version,
        credentials: None,
    };

    serde_json::to_string(&request).expect("Failed to serialize `HandshakeRequest`")
}

#[cs_bindgen]
pub fn handle_handshake_response(json: String) -> u64 {
    let message = serde_json::from_str::<HandshakeResponse>(&json)
        .expect("Failed to deserialize `HandshakeResponse`");

    // Return the player's point total.
    message.account_data.points
}
