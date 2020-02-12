pub mod game;
pub mod messages;
pub mod tile;

use crate::messages::{Credentials, HandshakeRequest, HandshakeResponse, PlayerState, Version};
use cs_bindgen::prelude::*;
use messages::AccountId;

cs_bindgen::generate_static_bindings!();

#[cs_bindgen]
#[derive(Debug, Clone, Default)]
pub struct ClientState {
    credentials: Option<Credentials>,
    state: Option<PlayerState>,
}

#[cs_bindgen]
impl ClientState {
    pub fn new() -> Self {
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
        dbg!(&json);
        match serde_json::from_str::<HandshakeResponse>(&json) {
            Ok(message) => {
                if let Some(new_credentials) = message.new_credentials {
                    println!(
                        "Overwriting existing credentials, new: {:?}, prev: {:?}",
                        new_credentials, self.credentials,
                    );

                    self.credentials = Some(new_credentials);
                }

                self.state = Some(message.account_data);
                true
            }

            Err(err) => {
                dbg!(err);
                false
            }
        }
    }

    pub fn account_id(&self) -> u64 {
        self.credentials.as_ref().unwrap().id.raw()
    }

    pub fn points(&self) -> u64 {
        self.state.as_ref().unwrap().points
    }
}
