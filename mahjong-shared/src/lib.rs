pub mod messages;
pub mod tile;

use crate::messages::{HandshakeRequest, Version};
use cs_bindgen::prelude::*;

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
