use crate::{
    hand::{Call, HandState},
    match_state::MatchId,
    messages::*,
    tile::{self, TileId, TileInstance, Wind},
};
use anyhow::{anyhow, bail, ensure, Context};
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
                    seat == self.seat,
                    "Seat specified for local draw ({:?}) doesn't match local seat for client ({:?})",
                    seat,
                    self.seat,
                );

                self.draw_local_tile(id)?;
            }

            &MatchEvent::RemoteDraw { seat } => {
                ensure!(
                    seat != self.seat,
                    "Seat specified for remote draw matches local seat ({:?})",
                    self.seat,
                );

                self.draw_remote_tile(seat)?;
            }

            // TODO: It seems suspect that we're ignoring some of the fields here, probably
            // worth reviewing what fields we're ignoring and seeing if there's a better
            // approach to take.
            &MatchEvent::TileDiscarded { seat, tile: id, .. } => {
                self.discard_tile(seat, id)?;
            }

            &MatchEvent::Call(call) => self.decide_call(Some(call))?,

            MatchEvent::Pass => self.decide_call(None)?,

            MatchEvent::MatchEnded => {}
        }
    }

    /// Applies a winning call to the match state.
    #[throws(anyhow::Error)]
    pub fn decide_call(&mut self, call: Option<FinalCall>) {
        cov_mark::hit!(local_state_decide_call);

        let (discard, discarding_player, calls) = match &self.turn_state {
            LocalTurnState::AwaitingCalls {
                discard,
                discarding_player,
                calls,
            } => (discard, discarding_player, calls),

            _ => bail!("Call is not valid for turn state {:?}", self.turn_state),
        };

        if let Some(call) = call {
            // Sanity check that the discarding player and discarded tile specified match the
            // ones tracked locally.
            ensure!(
                *discarding_player == call.called_from,
                "Called from {:?}, but expected to call from {:?}",
                call.called_from,
                discarding_player,
            );
            ensure!(
                *discard == call.discard,
                "Called {:?} from {:?}, but expected to call {:?}",
                call.discard,
                call.called_from,
                discard,
            );

            // If the local player made the winning call, also sanity check the call against the
            // list of valid calls.
            //
            // TODO: It would probably be better to locally track what call the local player
            // actually made in order to properly validate that the final call matches the one
            // that was submitted to the server.
            if call.caller == self.seat {
                ensure!(
                    calls.contains(&call.winning_call),
                    "Winning call {:?} for local player {:?} not in list of valid calls {:?}",
                    call.winning_call,
                    self.seat,
                    calls,
                );
            }

            let discard_instance = {
                let discarding_player = self.players.get_mut(&call.called_from).unwrap();

                ensure!(
                    discarding_player.last_discard() == Some(*discard),
                    "Last discarded tile {:?} for {:?} player does not match expected discard {:?}",
                    discarding_player.last_discard(),
                    call.called_from,
                    discard,
                );

                discarding_player
                    .call_last_discard()
                    .ok_or_else(|| anyhow!("Cannot call from a player with no discards"))?
            };

            // TODO: We have a potential consistency error here. If the attempt to add the
            // called tile to the calling player's hand fails, then we'll return early from the
            // function without updating the turn state but after having already removing the
            // tile tile from the other player's discards. It's not clear if this case can
            // actually be triggered without some other bug also having happened, since we've
            // already confirmed we're in the correct turn state (which should mean that the
            // hand state is also correct).
            self.players
                .get_mut(&call.caller)
                .unwrap()
                .call_tile(discard_instance, call.winning_call)
                .context("Failed to call tile locally")?;

            self.turn_state = LocalTurnState::AwaitingDraw(call.caller.next());
        } else {
            self.turn_state = LocalTurnState::AwaitingDraw(discarding_player.next());
        }
    }

    #[throws(anyhow::Error)]
    pub fn draw_local_tile(&mut self, draw_id: TileId) {
        cov_mark::hit!(local_state_draw_local);

        ensure!(
            self.turn_state == LocalTurnState::AwaitingDraw(self.seat),
            "Local draw is not valid for current turn state {:?}, drawn tile: {:?}",
            self.turn_state,
            draw_id,
        );

        let tile = tile::instance_by_id(draw_id);

        self.players
            .get_mut(&self.seat)
            .unwrap()
            .as_local_mut()
            .unwrap()
            .draw_tile(tile)?;

        self.turn_state = LocalTurnState::AwaitingDiscard(self.seat);
    }

    #[throws(anyhow::Error)]
    pub fn draw_remote_tile(&mut self, seat: Wind) {
        cov_mark::hit!(local_state_draw_remote);

        ensure!(
            self.turn_state == LocalTurnState::AwaitingDraw(seat),
            "Remote draw is not valid for current turn state {:?}, drawing player: {:?}",
            self.turn_state,
            seat,
        );

        self.players
            .get_mut(&seat)
            .unwrap()
            .as_remote_mut()
            .unwrap()
            .draw_tile()?;

        self.turn_state = LocalTurnState::AwaitingDiscard(seat);
    }

    #[throws(anyhow::Error)]
    pub fn discard_tile(&mut self, discarding_player: Wind, discard: TileId) {
        cov_mark::hit!(local_state_discard_tile);

        ensure!(
            self.turn_state == LocalTurnState::AwaitingDiscard(discarding_player),
            "Discard is not valid for turn state {:?}, discarding_player: {:?}, discard: {:?}",
            self.turn_state,
            discarding_player,
            discard,
        );

        self.players
            .get_mut(&discarding_player)
            .unwrap()
            .discard_tile(discard)
            .with_context(|| {
                format!(
                    "Failed to discard tile locally for {:?} player",
                    discarding_player
                )
            })?;

        // Determine if the local player can call the discarded tile.
        let calls = if discarding_player != self.seat {
            self.players[&self.seat]
                .as_local()
                .unwrap()
                .find_possible_calls(tile::by_id(discard), discarding_player.next() == self.seat)
        } else {
            Default::default()
        };

        // Wait for a call to be made.
        //
        // NOTE: We always move to the `AwaitingCalls` state after a player discards, even
        // though we don't know if any players can call the discard. This avoid some
        // complexity in the match logic; We always wait for the server to confirm that
        // either a call was made or all players passed before moving to the next step.
        self.turn_state = LocalTurnState::AwaitingCalls {
            calls,
            discarding_player,
            discard,
        };
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

    // TODO: Remove these duplicate functions once we can directly export the versions
    // that return a `Result`. For now we're stuck returning a bool to indicate if the
    // operation succeeded or not.

    pub fn try_draw_local_tile(&mut self, draw_id: TileId) -> bool {
        self.draw_local_tile(draw_id).is_ok()
    }

    pub fn try_draw_remote_tile(&mut self, seat: Wind) -> bool {
        self.draw_remote_tile(seat).is_ok()
    }

    pub fn try_discard_tile(&mut self, discarding_player: Wind, discard: TileId) -> bool {
        self.discard_tile(discarding_player, discard).is_ok()
    }

    pub fn try_decide_call(&mut self, call: FinalCall) -> bool {
        self.decide_call(Some(call)).is_ok()
    }

    pub fn try_decide_pass(&mut self) -> bool {
        self.decide_call(None).is_ok()
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalHand {
    /// The hand for the player controlled by the client. Contains the full state
    /// information for the hand.
    Local(HandState),

    /// The hand information for a remote player. Only contains the discards for the
    /// player.
    Remote(RemoteHand),
}

impl LocalHand {
    pub fn as_local(&self) -> Option<&HandState> {
        match self {
            LocalHand::Local(hand) => Some(hand),
            _ => None,
        }
    }

    pub fn as_local_mut(&mut self) -> Option<&mut HandState> {
        match self {
            LocalHand::Local(hand) => Some(hand),
            _ => None,
        }
    }

    pub fn as_remote(&self) -> Option<&RemoteHand> {
        match self {
            LocalHand::Remote(hand) => Some(hand),
            _ => None,
        }
    }

    pub fn as_remote_mut(&mut self) -> Option<&mut RemoteHand> {
        match self {
            LocalHand::Remote(hand) => Some(hand),
            _ => None,
        }
    }

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

    pub fn last_discard(&self) -> Option<TileId> {
        match self {
            LocalHand::Local(hand) => hand.discards().last().map(|instance| instance.id),
            LocalHand::Remote(hand) => hand.discards.last().map(|instance| instance.id),
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
    /// Creates a new `RemoteHand` with an valid initial setup for the start of a match.
    ///
    /// The returned hand will have 13 tiles in the hand and no initial draw.
    pub fn new() -> Self {
        Self {
            tiles: 13,
            has_current_draw: false,
            open_chows: Default::default(),
            open_pongs: Default::default(),
            open_kongs: Default::default(),
            closed_kongs: Default::default(),
            discards: Default::default(),
        }
    }

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

#[cfg(test)]
mod tests {
    use crate::{client::*, hand::HandState, match_state::MatchId, tile::Wind, tile::TILE_SET};
    use maplit::hashmap;
    use pretty_assertions::assert_eq;

    // Check that `LocalState::handle_event` is deferring to the more specific event
    // handling functions. This ensures that `LocalState` doesn't behave differently
    // when using `handle_event` vs calling the specific methods directly.
    #[test]
    fn local_state_handle_event_defers_to_other_functions() {
        let mut tile_set = TILE_SET.clone();
        let mut state = LocalState {
            id: MatchId::new(0),
            seat: Wind::East,
            players: hashmap! {
                Wind::East => LocalHand::Local(HandState::new(&mut tile_set)),
                Wind::South => LocalHand::Remote(RemoteHand::new()),
                Wind::West => LocalHand::Remote(RemoteHand::new()),
                Wind::North => LocalHand::Remote(RemoteHand::new()),
            },
            turn_state: LocalTurnState::AwaitingDraw(Wind::East),
        };

        {
            cov_mark::check!(local_state_decide_call);
            let _ = state.handle_event(&MatchEvent::Call(FinalCall {
                called_from: Wind::South,
                discard: TILE_SET[0].id,
                caller: Wind::East,
                winning_call: Call::Kan(TILE_SET[0].tile),
            }));
        }

        {
            cov_mark::check!(local_state_draw_local);
            let _ = state.handle_event(&MatchEvent::LocalDraw {
                seat: Wind::East,
                tile: TILE_SET[0].id,
            });
        }

        {
            cov_mark::check!(local_state_draw_remote);
            let _ = state.handle_event(&MatchEvent::RemoteDraw { seat: Wind::South });
        }

        {
            cov_mark::check!(local_state_discard_tile);
            let _ = state.handle_event(&MatchEvent::TileDiscarded {
                seat: Wind::East,
                tile: TILE_SET[0].id,
                calls: Default::default(),
            });
        }
    }

    // Check that `LocalState` always goes into the `AwaitingCalls` turn state after a
    // player discards a tile, even if the local player can't call the discarded tile.
    #[test]
    fn local_state_awaits_calls_after_discard() {
        let mut tile_set = TILE_SET.clone();
        let mut state = LocalState {
            id: MatchId::new(0),
            seat: Wind::East,
            players: hashmap! {
                Wind::East => LocalHand::Local(HandState::new(&mut tile_set)),
                Wind::South => LocalHand::Remote(RemoteHand::new()),
                Wind::West => LocalHand::Remote(RemoteHand::new()),
                Wind::North => LocalHand::Remote(RemoteHand::new()),
            },
            turn_state: LocalTurnState::AwaitingDraw(Wind::South),
        };

        state.draw_remote_tile(Wind::South).unwrap();
        state.discard_tile(Wind::South, TILE_SET[0].id).unwrap();

        let expected = LocalTurnState::AwaitingCalls {
            calls: vec![],
            discard: TILE_SET[0].id,
            discarding_player: Wind::South,
        };
        assert_eq!(expected, state.turn_state);
    }
}
