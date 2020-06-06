use crate::{
    hand::{Call, HandState},
    match_state::MatchId,
    messages::*,
    tile::{self, TileId, TileInstance, Wind},
};
use anyhow::{ensure, Context};
use cs_bindgen::prelude::*;
use fehler::throws;
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
///
/// This struct only contains a partial representation of the full match state.
/// Specifically, the local client does not know which tiles are the other players'
/// hands. This struct also does not know which tiles are currently in the wall. As
/// such, the state tracked in this struct is not enough to fully simulate the
/// progression of the game locally.
// TODO: This should go in a `Mahjong.Match` namespace.
#[cs_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalState {
    pub id: MatchId,
    pub seat: Wind,
    pub players: HashMap<Wind, LocalHand>,
    pub turn_state: LocalTurnState,
}

impl LocalState {
    #[throws(anyhow::Error)]
    pub fn handle_event(&mut self, event: &MatchEvent) {
        // Apply the event to the local state.
        match event {
            &MatchEvent::LocalDraw { seat, tile: id } => {
                ensure!(
                    self.turn_state == LocalTurnState::AwaitingDraw(seat),
                    "Draw event does not match current turn",
                );

                ensure!(
                    self.try_draw_local_tile(seat, id),
                    "Failed to draw local tile, seat: {:?}, tile ID: {:?}",
                    seat,
                    id,
                );
            }

            &MatchEvent::RemoteDraw { seat } => {
                ensure!(
                    self.turn_state == LocalTurnState::AwaitingDraw(seat),
                    "Draw event does not match current turn",
                );

                ensure!(
                    self.try_draw_remote_tile(seat),
                    "Failed to draw remote tile, seat: {:?}",
                    seat,
                );
            }

            &MatchEvent::TileDiscarded { seat, tile: id, .. } => {
                ensure!(
                    self.turn_state == LocalTurnState::AwaitingDiscard(seat),
                    "Draw event does not match current turn",
                );

                self.players
                    .get_mut(&seat)
                    .unwrap()
                    .discard_tile(id)
                    .with_context(|| {
                        format!("Failed to discard tile locally for {:?} player", seat)
                    })?;
            }

            &MatchEvent::Call {
                called_from,
                caller,
                winning_call,
                tile: id,
            } => {
                let discard = self
                    .players
                    .get_mut(&called_from)
                    .unwrap()
                    .call_last_discard()
                    .expect("Cannot call from a player with no discards");

                ensure!(
                    id == discard.id,
                    "Last discarded tile {:?} for {:?} player does not match expected discard {:?}",
                    discard,
                    called_from,
                    id,
                );

                self.players
                    .get_mut(&caller)
                    .unwrap()
                    .call_tile(discard, winning_call)
                    .context("Failed to call tile locally")?;
            }

            MatchEvent::MatchEnded => {}
        }
    }
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

    // TODO: Return a `&RemoteHand` once cs-bindgen supports doing so to avoid an
    // unnecessary clone.
    pub fn remote_hand(&self, seat: Wind) -> RemoteHand {
        match &self.players[&seat] {
            LocalHand::Remote(hand) => hand.clone(),

            _ => panic!(
                "Expected seat {:?} to be remote, but was the local hand",
                seat,
            ),
        }
    }

    pub fn turn_state(&self) -> LocalTurnState {
        self.turn_state.clone()
    }

    pub fn player_has_current_draw(&self, seat: Wind) -> bool {
        match &self.players[&seat] {
            LocalHand::Local(hand) => hand.current_draw().is_some(),
            LocalHand::Remote(hand) => hand.has_current_draw,
        }
    }

    /// Creates the request message for sending the discard action to the server.
    pub fn request_discard_tile(&mut self, player: Wind, tile: TileId) -> String {
        let request = ClientRequest::DiscardTile(DiscardTileRequest {
            id: self.id,
            player,
            tile,
        });
        serde_json::to_string(&request).unwrap()
    }

    // TODO: Make `json` a `&str` and return a `Result` here instead of panicking on
    // errors. Both of these are pending support in cs-bindgen.
    pub fn deserialize_and_handle_event(&mut self, json: String) -> MatchEvent {
        let event = serde_json::from_str(&json).unwrap();
        self.handle_event(&event).unwrap();
        event
    }

    pub fn try_draw_local_tile(&mut self, seat: Wind, id: TileId) -> bool {
        let tile = tile::instance_by_id(id);

        assert_eq!(self.seat, seat, "The specified seat for the local player ({:?}) doesn't match the actual local seat ({:?})", seat, self.seat);

        match self.players.get_mut(&seat).unwrap() {
            LocalHand::Local(hand) => hand.draw_tile(tile).is_ok(),
            LocalHand::Remote(_) => false,
        }
    }

    pub fn try_draw_remote_tile(&mut self, seat: Wind) -> bool {
        match self.players.get_mut(&seat).unwrap() {
            LocalHand::Remote(hand) => hand.draw_tile().is_ok(),
            LocalHand::Local(_) => false,
        }
    }

    pub fn try_discard_tile(&mut self, seat: Wind, tile: TileId) -> bool {
        self.players
            .get_mut(&seat)
            .unwrap()
            .discard_tile(tile)
            .is_ok()
    }
}

/// The turn information for `LocalState`.
///
/// Mirrors `TurnState` but doesn't expose state information about players other
/// than the local one. Notably, the `AwaitingCalls` state doesn't include the list
/// of players that can call the discarded tile.
#[cs_bindgen]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    Remote(RemoteHand),
}

impl LocalHand {
    #[throws(anyhow::Error)]
    pub fn draw_tile(&mut self, id: TileId) {
        match self {
            LocalHand::Local(hand) => {
                // NOTE: We need to reconstruct the tile instance locally because only the tile
                // ID is sent over the network. We generally don't want to be constructing new
                // tile instances, but in this case it's valid since this is the point where we
                // receive the tile information locally.
                let instance = TileInstance::new(tile::by_id(id), id);

                hand.draw_tile(instance)?;
            }

            LocalHand::Remote(hand) => {
                ensure!(
                    !hand.has_current_draw,
                    "Hand already has a current draw, so draw is not valid",
                );

                hand.has_current_draw = true;
            }
        }
    }

    #[throws(anyhow::Error)]
    pub fn discard_tile(&mut self, id: TileId) {
        match self {
            LocalHand::Local(hand) => {
                hand.discard_tile(id)?;
            }

            LocalHand::Remote(hand) => {
                ensure!(
                    hand.has_current_draw,
                    "Hand does not have a current draw, so discard is not valid",
                );

                // NOTE: We need to reconstruct the tile instance locally because only the tile
                // ID is sent over the network. We generally don't want to be constructing new
                // tile instances, but in this case it's valid since this is the point where we
                // receive the tile information locally.
                let instance = TileInstance::new(tile::by_id(id), id);

                hand.has_current_draw = false;
                hand.discards.push(instance);
            }
        }
    }

    /// Calls the last discarded tile from the player's discards.
    pub fn call_last_discard(&mut self) -> Option<TileInstance> {
        match self {
            LocalHand::Local(hand) => hand.call_last_discard(),
            LocalHand::Remote(hand) => hand.discards.pop(),
        }
    }

    #[throws(anyhow::Error)]
    pub fn call_tile(&mut self, discard: TileInstance, call: Call) {
        match self {
            LocalHand::Local(hand) => hand.call_tile(discard, call)?,
            LocalHand::Remote(hand) => hand.call_tile(discard, call)?,
        }
    }
}

#[cs_bindgen]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteHand {
    // NOTE: This struct doesn't expose any information about any hidden tiles in the
    // player's hand. The only information is that which would be visible to another
    // player, i.e. the number of hidden tiles and whether or not they have a current
    // draw.
    pub tiles: u8,
    pub has_current_draw: bool,

    // "Inactive" tiles, i.e. ones that are in open melds (or a closed kong) and cannot
    // be discarded.
    //
    // TODO: For each open meld, track which tile was called and which player it was
    // called from. This is necessary for visualizing open melds correctly.
    pub open_chows: Vec<[TileInstance; 3]>,
    pub open_pongs: Vec<[TileInstance; 3]>,
    pub open_kongs: Vec<[TileInstance; 4]>,
    pub closed_kongs: Vec<[TileInstance; 4]>,

    // The player's discard pile.
    pub discards: Vec<TileInstance>,
}

impl RemoteHand {
    #[throws(anyhow::Error)]
    pub fn draw_tile(&mut self) {
        ensure!(
            !self.has_current_draw,
            "Cannot draw when hand already has a current draw",
        );

        self.has_current_draw = true;
    }

    #[throws(anyhow::Error)]
    pub fn call_tile(&mut self, discard: TileInstance, call: Call) {
        match call {
            Call::Chii(id_a, id_b) => {
                let tile_a = tile::instance_by_id(id_a);
                let tile_b = tile::instance_by_id(id_b);

                ensure!(
                    tile::is_chow(discard.tile, tile_a.tile, tile_b.tile),
                    r#"Tiles specified in "chii" call do not form valid chow"#,
                );

                self.tiles -= 2;

                self.open_chows.push([discard, tile_a, tile_b]);
            }

            Call::Pon(id_a, id_b) => {
                let tile_a = tile::instance_by_id(id_a);
                let tile_b = tile::instance_by_id(id_b);

                ensure!(
                    discard.tile == tile_a.tile && discard.tile == tile_b.tile,
                    r#"Tiles specified in "pon" call do not form a valid pong"#,
                );

                self.tiles -= 2;

                self.open_pongs.push([discard, tile_a, tile_b]);
            }

            Call::Kan(tile) => {
                let tiles = tile::all_instances_of(tile);

                self.tiles -= 3;

                ensure!(
                    tiles[0].tile == discard.tile,
                    r#"Tile specified in "kan" call does not match the discarded tile"#,
                );

                self.open_kongs.push(tiles);
            }

            Call::Ron => todo!("Handle calling a ron"),
        }
    }
}
