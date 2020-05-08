//! Functionality for actually playing a mahjong match.

use crate::{hand::Hand, messages::*, tile::*};
use cs_bindgen::prelude::*;
use derive_more::Display;
use fehler::{throw, throws};
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
    pub current_turn: Wind,
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

            current_turn: Wind::East,
        }
    }

    pub fn player(&self, seat: Wind) -> &Hand {
        self.players.get(&seat).unwrap()
    }

    /// Draws the next tile from the wall and puts it in a player's draw slot.
    #[throws(anyhow::Error)]
    pub fn draw_for_player(&mut self, seat: Wind) -> TileId {
        let hand = self.players.get_mut(&seat).unwrap();

        let tile = self
            .wall
            .pop()
            .ok_or(InsufficientTiles::new(self.wall.len(), 1))?;
        let id = tile.id;
        hand.draw_tile(tile)?;

        id
    }

    #[throws(anyhow::Error)]
    pub fn discard_tile(&mut self, seat: Wind, tile: TileId) {
        if seat != self.current_turn {
            throw!(InvalidDiscard::IncorrectTurn {
                expected: seat,
                actual: self.current_turn,
            });
        }

        let hand = self.players.get_mut(&seat).unwrap();
        hand.discard_tile(tile)?;

        // Update to the next player's turn, cycling through the seats in wind order.
        self.current_turn = self.current_turn.next();
    }
}

#[cs_bindgen]
impl MatchState {
    // TODO: Remove the manual getter definitions once cs-bindgen supports exposing
    // public fields on handle types as properties.

    pub fn id(&self) -> MatchId {
        self.id
    }

    pub fn current_turn(&self) -> Wind {
        self.current_turn
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

    pub fn handle_event(&mut self, json: String) -> MatchEvent {
        let event = serde_json::from_str(&json).unwrap();

        // Apply the event to the local state.
        match &event {
            &MatchEvent::TileDiscarded { seat, tile } => {
                assert_eq!(
                    self.current_turn, seat,
                    "Draw event does not match current turn"
                );

                self.discard_tile(seat, tile)
                    .expect("Failed to discard locally");
            }

            &MatchEvent::TileDrawn { seat, tile } => {
                assert_eq!(
                    self.current_turn, seat,
                    "Draw event does not match current turn"
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
