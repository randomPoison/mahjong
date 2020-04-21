//! Functionality for actually playing a mahjong match.

use crate::{messages::*, tile::*};
use cs_bindgen::prelude::*;
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
    pub players: HashMap<Wind, Player>,

    /// The live wall that players will draw from.
    pub wall: Vec<TileInstance>,

    /// The seat wind of the player who is currently taking their turn.
    pub current_turn: Wind,
}

impl MatchState {
    pub fn new(id: MatchId, tiles: Vec<TileInstance>) -> Self {
        Self {
            id,
            players: hashmap! {
                Wind::East => Player::new(),
                Wind::South => Player::new(),
                Wind::West => Player::new(),
                Wind::North => Player::new(),
            },

            // TODO: Split the dead wall from the live wall and draw out an initial hand.
            wall: tiles,

            current_turn: Wind::East,
        }
    }

    pub fn player(&self, seat: Wind) -> &Player {
        self.players.get(&seat).unwrap()
    }

    /// Draws `count` tiles from the wall directly into a player's hand.
    ///
    /// If there are fewer than `count` tiles left in the wall, no tiles are drawn.
    #[throws(InsufficientTiles)]
    pub fn draw_for_player(&mut self, seat: Wind, count: usize) {
        let player = self.players.get_mut(&seat).unwrap();

        // Check if there are enough tiles for the draw before actually drawing any, that
        // way we don't have a partially-completed draw if there aren't enough tiles left.
        if self.wall.len() < count {
            throw!(InsufficientTiles::new(self.wall.len(), count));
        }

        for _ in 0..count {
            player.hand.push(self.wall.pop().unwrap());
        }
    }

    /// Draws the next tile from the wall and puts it in a player's draw slot.
    #[throws(InsufficientTiles)]
    pub fn draw_into_hand(&mut self, seat: Wind) {
        let player = self.players.get_mut(&seat).unwrap();

        let tile = self
            .wall
            .pop()
            .ok_or(InsufficientTiles::new(self.wall.len(), 1))?;

        player.current_draw = Some(tile);
    }

    #[throws(InvalidDiscard)]
    pub fn discard_tile(&mut self, seat: Wind, tile: TileId) {
        if seat != self.current_turn {
            throw!(InvalidDiscard::IncorrectTurn {
                expected: seat,
                actual: self.current_turn,
            });
        }

        let player = self.players.get_mut(&seat).unwrap();
        let tile = player
            .remove_from_hand(tile)
            .or_else(|| {
                if player
                    .current_draw
                    .map(|draw| draw.id == tile)
                    .unwrap_or(false)
                {
                    player.current_draw.take()
                } else {
                    None
                }
            })
            .ok_or(InvalidDiscard::TileNotInHand)?;
        player.discards.push(tile);
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
        self.players.get(&seat).unwrap().hand.clone()
    }

    // TODO: Combine `player_has_current_draw` and `get_current_draw` into a single
    // function that returns an `Option<Tile>` once cs-bindgen supports `Option`.

    pub fn player_has_current_draw(&self, seat: Wind) -> bool {
        self.players.get(&seat).unwrap().current_draw.is_some()
    }

    pub fn current_draw(&self, seat: Wind) -> TileInstance {
        self.players.get(&seat).unwrap().current_draw.unwrap()
    }

    // TODO: Remove this function once we can export `discard_tile` directly.
    pub fn try_discard_tile(&mut self, seat: Wind, tile: TileId) -> bool {
        self.discard_tile(seat, tile).is_ok()
    }

    /// Creates the request message for sending the discard action to the server.
    pub fn request_discard_tile(&mut self, player: Wind, tile: TileId) -> String {
        let request = DiscardTileRequest {
            id: self.id,
            player,
            tile,
        };
        serde_json::to_string(&request).unwrap()
    }

    pub fn handle_discard_tile_response(&mut self, response: String) -> bool {
        let response = serde_json::from_str::<DiscardTileResponse>(&response).unwrap();

        // Error out if the server rejected the action.
        if !response.success {
            return false;
        }

        // Validate that the returned server state matches the local server state.
        //
        // TODO: We'll need more robust tools for reconciling client state with server state
        // as changes come in. This will be especially important as we setup the server to
        // send data deltas rather than full data sets.
        self == &response.state
    }
}

/// Unique identifier for an active match.
///
/// Values are generated by the server, and should not be created by the client.
#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Player state within a match.
#[cs_bindgen]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    /// The client session controlling this player in the match, if any.
    ///
    /// If no client controls this player, then the player is CPU-controlled by default.
    pub controller: Option<SessionId>,

    /// The player's current hand.
    // TODO: We probably want this to be a `HashMap<TileId, TileInstance>` instead of
    // `Vec`. We should change that once we can export maps.
    pub hand: Vec<TileInstance>,

    /// The player's current draw, if any.
    pub current_draw: Option<TileInstance>,

    /// The player's discard pile.
    // TODO: This should also probably be `HashMap<TileId, TileInstance>`.
    pub discards: Vec<TileInstance>,
}

impl Player {
    /// Creates a new player with all default (i.e. empty) values.
    pub fn new() -> Self {
        Default::default()
    }

    /// Removes the specified tile from the player's hand, if present.
    pub fn remove_from_hand(&mut self, id: TileId) -> Option<TileInstance> {
        self.hand
            .iter()
            .position(|tile| tile.id == id)
            .map(|index| self.hand.remove(index))
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
