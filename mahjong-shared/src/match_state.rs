//! Functionality for actually playing a mahjong match.

use crate::{
    hand::{self, Call, Hand},
    messages::*,
    tile::{self, TileId, TileInstance, Wind},
};
use anyhow::bail;
use cs_bindgen::prelude::*;
use derive_more::Display;
use fehler::throws;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug};
use thiserror::Error;

#[cs_bindgen]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchState {
    pub id: MatchId,

    // TODO: Setup a better way of storing the players. Right now getting access to a
    // player always requires an `unwrap`, even though we know ahead of time that
    // there's a player for each wind.
    pub players: HashMap<Wind, Hand>,

    /// The live wall that players will draw from.
    pub wall: Vec<TileInstance>,

    /// The seat wind of the player who is currently taking their turn.
    pub turn_state: TurnState,
}

impl MatchState {
    pub fn new(id: MatchId, mut tiles: Vec<TileInstance>) -> Self {
        Self {
            id,
            players: hashmap! {
                Wind::East => Hand::new(&mut tiles),
                Wind::South => Hand::new(&mut tiles),
                Wind::West => Hand::new(&mut tiles),
                Wind::North => Hand::new(&mut tiles),
            },

            // TODO: Split the dead wall from the live wall and draw out an initial hand.
            wall: tiles,

            turn_state: TurnState::AwaitingDraw(Wind::East),
        }
    }

    pub fn player(&self, seat: Wind) -> &Hand {
        self.players.get(&seat).unwrap()
    }

    /// Draws the next tile from the wall and puts it in a player's draw slot.
    #[throws(anyhow::Error)]
    pub fn draw_for_player(&mut self, seat: Wind) -> TileId {
        // Verify that the draw action is valid.
        if self.turn_state != TurnState::AwaitingDraw(seat) {
            bail!(
                "Attempting to draw tile for {:?} when turn state is {:?}",
                seat,
                self.turn_state,
            );
        }

        let hand = self.players.get_mut(&seat).unwrap();

        let tile = self
            .wall
            .pop()
            .ok_or(InsufficientTiles::new(self.wall.len(), 1))?;
        let id = tile.id;
        hand.draw_tile(tile)?;

        // Update the turn state to wait for the player that just drew to discard.
        self.turn_state = TurnState::AwaitingDiscard(seat);

        id
    }

    #[throws(anyhow::Error)]
    pub fn discard_tile(&mut self, seat: Wind, tile: TileId) {
        // Verify that the discard action is valid.
        if self.turn_state != TurnState::AwaitingDiscard(seat) {
            bail!(
                "Attempting to discard tile for {:?} when current turn is {:?}",
                seat,
                self.turn_state,
            );
        }

        let hand = self.players.get_mut(&seat).unwrap();
        hand.discard_tile(tile)?;

        // Determine if any players are able to call the tile.
        let mut waiting = HashMap::new();
        for (&calling_seat, hand) in self.players.iter().filter(|(&player, _)| player != seat) {
            let calls = hand.find_possible_calls(tile::by_id(tile), seat.next() == calling_seat);
            if !calls.is_empty() {
                waiting.insert(calling_seat, calls);
            }
        }

        // Update the turn state based on whether or not any players can call the discarded
        // tile. If any players can call, we wait for all players to either call or pass.
        // Otherwise, we wait for the next player's draw.
        if !waiting.is_empty() {
            self.turn_state = TurnState::AwaitingCalls {
                discarding_player: seat,
                discard: tile,
                calls: HashMap::new(),
                waiting,
            }
        } else {
            self.turn_state = TurnState::AwaitingDraw(seat.next());
        }
    }

    #[throws(anyhow::Error)]
    pub fn call_tile(&mut self, seat: Wind, call: Option<Call>) {
        let (calls, waiting) = match &mut self.turn_state {
            TurnState::AwaitingCalls { calls, waiting, .. } => (calls, waiting),

            _ => bail!(
                "Attempting to call tile for {:?} when turn state is {:?}",
                seat,
                self.turn_state,
            ),
        };

        let possible_calls = match waiting.get(&seat) {
            Some(possible_calls) => possible_calls,
            None => bail!(
                "Attempting to call for {:?} but that player cannot call currently",
                seat,
            ),
        };

        match call {
            // Verify that the provided call is valid, and if it is remove the player `waiting`
            // and add their call to `calls`.
            Some(call) => {
                if possible_calls.contains(&call) {
                    waiting.remove(&seat);
                    assert!(
                        calls.insert(seat, call).is_none(),
                        "Found previous call for {:?}",
                        seat,
                    );
                } else {
                    bail!(
                        "Attempting to make call {:?} for player {:?} that is not in list of valid \
                        calls: {:?}",
                        call,
                        seat,
                        possible_calls,
                    );
                }
            }

            // Player is passing, so remove them from the waiting list without adding a call for
            // them.
            None => {
                waiting.remove(&seat);
            }
        }

        // NOTE: No state transition here. Once there are no more waiting players, the match
        // runner calls `decide_call` to apply the outcome of the call phase.
    }

    #[throws(anyhow::Error)]
    pub fn decide_call(&mut self) -> Option<(Wind, Call)> {
        let (calls, waiting, &mut discard_id, &mut discarding_player) = match &mut self.turn_state {
            TurnState::AwaitingCalls {
                calls,
                waiting,
                discard,
                discarding_player,
            } => (calls, waiting, discard, discarding_player),

            _ => bail!(
                "Attempting to decide call when turn state is {:?}",
                self.turn_state,
            ),
        };

        if !waiting.is_empty() {
            bail!(
                "Attempting to decide call when players still need to call: {:?}",
                waiting,
            );
        }

        if calls.is_empty() {}

        let max = calls
            .iter()
            .max_by(|(&left_seat, &left_call), (&right_seat, &right_call)| {
                hand::compare_calls(
                    left_seat,
                    left_call,
                    right_seat,
                    right_call,
                    discarding_player,
                )
            });

        if let Some((&seat, &call)) = max {
            // Remove the called tile from the discarding player's discards.
            let discarding_hand = self.players.get_mut(&discarding_player).unwrap();
            let discard = discarding_hand
                .call_last_discard()
                .expect("Discarding player has no discarded tiles");
            assert_eq!(
                discard.id, discard_id,
                "Last discarded tile does not match saved ID",
            );

            // Add the tile to the calling player's hand, making the appropriate meld.
            let calling_hand = self.players.get_mut(&seat).unwrap();
            calling_hand.call_tile(discard, call);

            // Set the turn order to the next player after the calling player.
            self.turn_state = TurnState::AwaitingDraw(seat.next());

            // Return the winning call, I guess?
            Some((seat, call))
        } else {
            // If all players passed, move to the next player's draw phase and return `None`.
            self.turn_state = TurnState::AwaitingDraw(discarding_player.next());
            None
        }
    }
}

#[cs_bindgen]
impl MatchState {
    // TODO: Remove the manual getter definitions once cs-bindgen supports exposing
    // public fields on handle types as properties.

    pub fn id(&self) -> MatchId {
        self.id
    }

    // TODO: Make the return type `&[Tile]` once cs-bindgen supports returning slices.
    pub fn player_hand(&self, seat: Wind) -> Vec<TileInstance> {
        self.players.get(&seat).unwrap().tiles().into()
    }

    // TODO: Combine `player_has_current_draw` and `get_current_draw` into a single
    // function that returns an `Option<Tile>` once cs-bindgen supports `Option`.

    pub fn player_has_current_draw(&self, seat: Wind) -> bool {
        self.players.get(&seat).unwrap().current_draw().is_some()
    }

    pub fn current_draw(&self, seat: Wind) -> TileInstance {
        self.players
            .get(&seat)
            .unwrap()
            .current_draw()
            .unwrap()
            .clone()
    }

    // TODO: Remove this function once we can export `discard_tile` directly.
    pub fn try_discard_tile(&mut self, seat: Wind, tile: TileId) -> bool {
        self.discard_tile(seat, tile).is_ok()
    }

    // TODO: Remove this function once we can export `draw_tile` directly.
    pub fn try_draw_tile(&mut self, seat: Wind) -> bool {
        self.draw_for_player(seat).is_ok()
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

    // TODO: Return a `Result` here
    pub fn handle_event(&mut self, json: String) -> MatchEvent {
        let event = serde_json::from_str(&json).unwrap();

        // Apply the event to the local state.
        match &event {
            &MatchEvent::TileDiscarded { seat, tile } => {
                assert_eq!(
                    self.turn_state,
                    TurnState::AwaitingDiscard(seat),
                    "Draw event does not match current turn",
                );

                self.discard_tile(seat, tile)
                    .expect("Failed to discard locally");
            }

            &MatchEvent::TileDrawn { seat, tile } => {
                assert_eq!(
                    self.turn_state,
                    TurnState::AwaitingDraw(seat),
                    "Draw event does not match current turn",
                );

                let draw = self.draw_for_player(seat).expect("Unable to draw locally");
                assert_eq!(draw, tile, "Local draw does not match draw event");
            }

            MatchEvent::MatchEnded => {}
        }

        // Forward the event to the host environment
        event
    }
}

/// Unique identifier for an active match.
///
/// Values are generated by the server, and should not be created by the client.
#[cs_bindgen]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[display("{}", _0)]
#[repr(transparent)]
pub struct MatchId(u32);

impl MatchId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn raw(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Error)]
#[error("Not enough tiles in wall for draw: {needed} tiles requested, but only {remaining} left")]
pub struct InsufficientTiles {
    pub remaining: usize,
    pub needed: usize,
}

impl InsufficientTiles {
    pub fn new(remaining: usize, needed: usize) -> Self {
        Self { remaining, needed }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Error)]
pub enum InvalidDiscard {
    #[error("The tile being discarded is not in the player's hand")]
    TileNotInHand,

    #[error(
        "Player at {expected:?} attempted to discard, but it was the {actual:?} player's turn"
    )]
    IncorrectTurn {
        /// The player that attempted to play.
        expected: Wind,

        /// The player who's turn was active.
        actual: Wind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnState {
    AwaitingDraw(Wind),

    AwaitingDiscard(Wind),

    AwaitingCalls {
        discarding_player: Wind,
        discard: TileId,

        /// Calls made so far by players who can call the discarded tile.
        calls: HashMap<Wind, Call>,

        /// Remaining player that need to either call or pass.
        ///
        /// Key is the seat of the player that can call, value is the list of valid calls
        /// for the player.
        waiting: HashMap<Wind, Vec<Call>>,
    },
}
