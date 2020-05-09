use crate::tile::{self, TileId, TileInstance};
use cs_bindgen::prelude::*;
use fehler::{throw, throws};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use take_if::TakeIf;
use thiserror::Error;

/// Representation of a player's hand during a match.
///
/// `Hand` tracks the full state of a player's hand, and enforces that the hand is
/// always in a valid state during a match. As such, it's not necessary to validate
/// the hand's state before working with it. Specifically, the following will always
/// be true:
///
/// * The number of tiles in the player's hand will always be at least 1, and will
///   be at most 13.
/// * If the number of tiles in the player's hand is less than 13, the player will
///   have at least one open chow, pong, kong, or a closed kong.
/// * The player will have 0 or 1 currently-drawn tile, and must discard a tile
///   before they may draw another.
///
/// `Hand` does not attempt to check for overall validity of the game state, i.e. it
/// will not generally attempt to detect duplicate instances of the same tile.
#[cs_bindgen]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hand {
    // "Active" tiles in the player's hand, i.e. ones that can still be discarded.
    tiles: Vec<TileInstance>,
    current_draw: Option<TileInstance>,

    // "Inactive" tiles, i.e. ones that are in open melds (or a closed kong) and cannot
    // be discarded.
    open_chows: Vec<[TileInstance; 3]>,
    open_pongs: Vec<[TileInstance; 3]>,
    open_kongs: Vec<[TileInstance; 4]>,
    closed_kongs: Vec<[TileInstance; 4]>,

    // The player's discard pile.
    discards: Vec<TileInstance>,
}

impl Hand {
    /// Creates a new hand by drawing the starting tiles from `draw_from`.
    ///
    /// Draws 13 tiles from the end of `draw_from` to populate the starting hand.
    ///
    /// # Panics
    ///
    /// Panics if `draw_from` has fewer than 13 elements.
    pub fn new(draw_from: &mut Vec<TileInstance>) -> Self {
        assert!(
            draw_from.len() >= 13,
            "Not enough tiles for initial hand, only {} left",
            draw_from.len()
        );

        Hand {
            tiles: draw_from.split_off(draw_from.len() - 13),
            current_draw: None,
            open_chows: Default::default(),
            open_pongs: Default::default(),
            open_kongs: Default::default(),
            closed_kongs: Default::default(),
            discards: Default::default(),
        }
    }

    /// Sets `tile` as the hand's current draw.
    ///
    /// # Errors
    ///
    /// Returns an error if the hand already has a current draw.
    #[throws(DrawError)]
    pub fn draw_tile(&mut self, tile: TileInstance) {
        if self.current_draw.is_some() {
            throw!(DrawError(tile));
        }

        self.current_draw = Some(tile);
    }

    /// Discards the specified tile from the hand.
    ///
    /// The tile specified by `id` is moved into the discard pile. If `id` doesn't
    /// refer to the current draw, the current draw will be moved into the main hand.
    ///
    /// # Errors
    ///
    /// Returns an error if the tile specified by `id` is not in the player's hand.
    /// This includes if the tile is in an open meld, since such tiles can't be
    /// discarded.
    #[throws(DiscardError)]
    pub fn discard_tile(&mut self, id: TileId) {
        if self.current_draw.is_none() {
            throw!(DiscardError::NoDraw);
        }

        // First attempt to remove the tile from the player's hand, otherwise attempt to
        // discard the current draw. If the specified tile is neither in the player's
        // hand nor is the current draw, return an error.
        let tile = self
            .tiles
            .iter()
            .position(|tile| tile.id == id)
            .map(|index| self.tiles.remove(index))
            .or_else(|| self.current_draw.take_if(|draw| draw.id == id))
            .ok_or(DiscardError::NotInHand)?;

        self.discards.push(tile);

        // If the player didn't discard their current draw, move the draw into their hand.
        if let Some(draw) = self.current_draw.take() {
            self.tiles.push(draw);
        }
    }

    /// Determines what possible calls can be made for another player's discarded tile.
    ///
    /// `can_call_chii` indicates if "chii" calls are valid for the current discard, i.e.
    /// if the player discarding the tile is immediately to the left of the this player.
    /// If it is `false`, only `Pon` and `Kan` calls are returned.
    ///
    /// At most one `Pon` call and `Kan` call will be returned. If `can_call_chii` is
    /// `true`, then multiple `Chii` calls may be returned. If there are multiple ways
    /// to form the same `Chii` call, only one instance is returned, i.e. there's no
    /// need to "de-duplicate" the returned calls.
    pub fn find_possible_calls(&self, discard: &TileInstance, can_call_chii: bool) -> Vec<Call> {
        let mut calls = Vec::new();

        if can_call_chii {
            // We gather the found calls in an intermediate hash set in order so that if there's
            // multiple ways the form the same call we only return one instance.
            let mut chii_calls = HashSet::new();

            // Iterate over all combinations of 2 tiles from the hand and check to see if those
            // tiles can form chow with the discarded tile.
            for (first, second) in self.tiles.iter().tuple_combinations() {
                if tile::is_chow(discard.tile, first.tile, second.tile) {
                    chii_calls.insert(TilePair(first, second));
                }
            }

            // Convert the set of tile pairs into the list of `Chii` calls.
            calls.extend(
                chii_calls
                    .into_iter()
                    .map(|TilePair(first, second)| Call::Chii(first.id, second.id)),
            );
        }

        // Count how many copies of the discarded tile are in the player's hand to determine
        // if we can call pon or kan.
        let matching_tiles_in_hand = self
            .tiles
            .iter()
            .filter(|tile| tile.tile == discard.tile)
            .count();

        assert!(
            matching_tiles_in_hand <= 3,
            "Too many instances of {:?}! {} in players hand, plus the discard",
            discard,
            matching_tiles_in_hand,
        );

        // If there are at least 2 other instances of the discarded tile, we can call "pon".
        if matching_tiles_in_hand >= 2 {
            calls.push(Call::Pon);
        }

        // If the three other instances of the discarded tile are in the player's hand, we
        // can call "kan".
        if matching_tiles_in_hand == 3 {
            calls.push(Call::Kan);
        }

        calls
    }

    pub fn tiles(&self) -> &[TileInstance] {
        &self.tiles
    }

    pub fn current_draw(&self) -> Option<&TileInstance> {
        self.current_draw.as_ref()
    }

    pub fn open_chows(&self) -> &[[TileInstance; 3]] {
        &self.open_chows
    }

    pub fn open_pongs(&self) -> &[[TileInstance; 3]] {
        &self.open_pongs
    }

    pub fn open_kongs(&self) -> &[[TileInstance; 4]] {
        &self.open_kongs
    }

    pub fn closed_kongs(&self) -> &[[TileInstance; 4]] {
        &self.closed_kongs
    }

    pub fn discards(&self) -> &[TileInstance] {
        &self.discards
    }
}

/// A possible call when another player discards a tile.
#[cs_bindgen]
#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub enum Call {
    /// A "chii" call, making a chow meld (i.e. three tiles in a sequence).
    ///
    /// Specifies the two tiles from the hand that complete the sequence. There may be
    /// multiple possible chii calls for a given discard.
    Chii(TileId, TileId),

    /// A "pon" call, making a pong meld (i.e. three of a kind).
    ///
    /// No tiles from the hand are specified, since even if there are technically
    /// multiple ways to make the call there's no meaningful distinction between them.
    Pon,

    /// A "kan" call, making a kong meld (i.e. four of a kind).
    ///
    /// Only represents a call to form an open kong, since closed kongs are made from
    /// draws instead of discards. No tiles from the hand are specified, since there
    /// is only ever at most one way to make a kong.
    Kan,
}

impl PartialEq for Call {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Call::Pon, Call::Pon) => true,
            (Call::Kan, Call::Kan) => true,

            // Equality for chii calls is independent of the order of the tiles specified.
            (Call::Chii(self_0, self_1), Call::Chii(other_0, other_1)) => {
                (self_0 == other_1 && self_1 == other_1) || (self_0 == other_1 && self_1 == other_0)
            }

            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[error("The wrong number of tiles were provided for a player's starting hand, expected 13 but received {}", _0)]
pub struct WrongNumberOfTiles(usize);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[error("The player already has a drawn tile, and must discard before they can draw again")]
pub struct DrawError(TileInstance);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
pub enum DiscardError {
    #[error("The player doesn't have a current draw")]
    NoDraw,

    #[error("Tile is not in the player's hand, or is in an open meld and so cannot be discarded")]
    NotInHand,
}

/// Helper for de-duplicating chii calls.
///
/// Implements a custom equality comparison that ignores the tile IDs and ignores
/// the order that the tiles are specified in. This allows us to only return a
/// single "instance" of a given call when finding the possible calls for a discard.
#[derive(Debug, Clone, Copy, Eq, Hash)]
struct TilePair<'a>(&'a TileInstance, &'a TileInstance);

impl PartialEq for TilePair<'_> {
    fn eq(&self, other: &Self) -> bool {
        (self.0.tile == other.0.tile && self.1.tile == other.1.tile)
            || (self.0.tile == other.1.tile && self.1.tile == other.0.tile)
    }
}
