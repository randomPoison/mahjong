use crate::tile::{self, Tile, TileId, TileInstance};
use anyhow::{anyhow, bail};
use cs_bindgen::prelude::*;
use fehler::{throw, throws};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashSet};
use take_if::TakeIf;
use thiserror::Error;
use tile::Wind;
use vec_drain_where::VecDrainWhereExt;

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
pub struct HandState {
    // "Active" tiles in the player's hand, i.e. ones that can still be discarded.
    tiles: Vec<TileInstance>,
    current_draw: Option<TileInstance>,

    // "Inactive" tiles, i.e. ones that are in open melds (or a closed kong) and cannot
    // be discarded.
    //
    // TODO: For each open meld, track which tile was called and which player it was
    // called from. This is necessary for visualizing open melds correctly.
    open_chows: Vec<[TileInstance; 3]>,
    open_pongs: Vec<[TileInstance; 3]>,
    open_kongs: Vec<[TileInstance; 4]>,
    closed_kongs: Vec<[TileInstance; 4]>,

    // The player's discard pile.
    discards: Vec<TileInstance>,
}

impl HandState {
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

        HandState {
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
    pub fn find_possible_calls(&self, discard: Tile, can_call_chii: bool) -> Vec<Call> {
        let mut calls = Vec::new();

        if can_call_chii {
            // We gather the found calls in an intermediate hash set in order so that if there's
            // multiple ways the form the same call we only return one instance.
            let mut chii_calls = HashSet::new();

            // Iterate over all combinations of 2 tiles from the hand and check to see if those
            // tiles can form chow with the discarded tile.
            for (first, second) in self.tiles.iter().tuple_combinations() {
                if tile::is_chow(discard, first.tile, second.tile) {
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
            .filter(|instance| instance.tile == discard)
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

    /// Apply the selected call to the hand.
    ///
    /// # Errors
    ///
    /// Validates that the specified call is actually valid. No state is modified is
    /// modified in the case that the call is invalid.
    // TODO: The error handling here isn't quite right. We take care not to modify the
    // hand's state in the case that the call is invalid, however we're not actually
    // returning the discarded tile so even if the call proves to be invalid the
    // discarding player's hand is likely still in an invalid state. Depending on how we
    // want to do the error handling logic on both the client and the server, we'll
    // either want to return the discarded tile so that it can be returned to the
    // discarding player or panic in order to indicate that we shouldn't attempt to
    // recover in the case of an error.
    #[throws(anyhow::Error)]
    pub fn call_tile(&mut self, discard: TileInstance, call: Call) {
        match call {
            Call::Ron => {
                self.tiles.push(discard);

                // TODO: Validate that the hand is a valid mahjong.
            }

            Call::Kan => {
                // Verify the other 3 instances of `discard` are in the player's hand before
                // modifying any state.
                let matching_tiles = self
                    .tiles
                    .iter()
                    .filter(|instance| instance.tile == discard.tile)
                    .count();
                if matching_tiles != 3 {
                    bail!(
                        r#"Not enough tiles matching {:?} in hand for "kan" call (expected 3, found {})"#,
                        discard,
                        matching_tiles,
                    );
                }

                // Remove the other 3 tiles from the player's hand and add them to an open kong.
                let kong_tiles: Vec<_> = self
                    .tiles
                    .e_drain_where(|instance| instance.tile == discard.tile)
                    .collect();
                self.open_kongs
                    .push([discard, kong_tiles[0], kong_tiles[1], kong_tiles[2]]);
            }

            Call::Pon => {
                // Verify that at least 2 more instances of `discard` are in the player's hand
                // before modifying any state.
                let matching_tiles = self
                    .tiles
                    .iter()
                    .filter(|instance| instance.tile == discard.tile)
                    .count();
                if matching_tiles >= 2 {
                    bail!(
                        r#"Not enough tiles matching {:?} in hand for "pon" call (expected at least 2, found {})"#,
                        discard,
                        matching_tiles,
                    );
                }

                // Remove the first 2 matching tiles from the players hand, leaving the fourth in
                // the hand if present.
                let pong_tiles: Vec<_> = self
                    .tiles
                    .e_drain_where(|instance| instance.tile == discard.tile)
                    .take(2)
                    .collect();
                self.open_pongs
                    .push([discard, pong_tiles[0], pong_tiles[1]]);
            }

            Call::Chii(id_a, id_b) => {
                // Verify that both specified tiles are in the player's hand and that they form a
                // valid chow before modifying any state.
                let tile_a = self
                    .tiles
                    .iter()
                    .find(|instance| instance.id == id_a)
                    .ok_or_else(|| anyhow!(r#"Missing tile {:?} for "chii" call"#, id_a))?;

                let tile_b = self
                    .tiles()
                    .iter()
                    .find(|instance| instance.id == id_b)
                    .ok_or_else(|| anyhow!(r#"Missing tile {:?} for "chii" call"#, id_b))?;

                anyhow::ensure!(
                    tile::is_chow(discard.tile, tile_a.tile, tile_b.tile),
                    r#"Tiles specified in "chii" call do not form a valid sequence, discard = {:?}, {:?}, {:?}"#
                );

                // Remove both tiles from the players hand and move them into an open chow.
                //
                // NOTE: The unwraps belows will not panic because we have already confirmed that
                // both tiles are in the player's hand.
                let index = self
                    .tiles
                    .iter()
                    .position(|instance| instance.id == id_a)
                    .unwrap();
                let tile_a = self.tiles.remove(index);

                let index = self
                    .tiles
                    .iter()
                    .position(|instance| instance.id == id_b)
                    .unwrap();
                let tile_b = self.tiles.remove(index);

                self.open_chows.push([discard, tile_a, tile_b]);
            }
        }
    }

    /// Calls the last discarded tile from the player's discards.
    pub fn call_last_discard(&mut self) -> Option<TileInstance> {
        self.discards.pop()
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
///
/// # Ordering
///
/// Calls are ordered by precedence value when multiple players make a call, such
/// that kan has higher priority than pon, and pon has higher priority than chii.
/// All chii calls have the same priority regardless of the sequence being made.
#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// A call for a player's winning tile. Can be a "chii", a "pon", or one of the
    Ron,
}

/// Compares two calls given the full context for the call.
///
/// In order to full determine precedence between two calls in all cases, we need
/// the seat for both callers and the seat of the discarding player (in order to
/// evaluate the head bump rule if two or more players call "ron" on the same
/// discard).
///
/// # Panics
///
/// This function panics if the pair of calls would not be valid during a match.
/// Specifically:
///
/// * If both calls are "kan", since there can only be one four-of-a-kind formed
///   from a given discard.
/// * If both calls are "pon", since there can only be one three-of-a-kind formed
///   from a given discard.
/// * If both calls are "chii", since only one player may call "chii" for a given
///   discard.
pub fn compare_calls(
    left_seat: Wind,
    left_call: Call,
    right_seat: Wind,
    right_call: Call,
    discarding_seat: Wind,
) -> Ordering {
    match (left_call, right_call) {
        // If both players called "ron", the head bump rule says that the winner is the
        // player closest in turn order to the discarding player.
        (Call::Ron, Call::Ron) => {
            let left_distance = discarding_seat.distance_to(left_seat);
            let right_distance = discarding_seat.distance_to(right_seat);
            right_distance.cmp(&left_distance)
        }

        (Call::Ron, _) => Ordering::Greater,

        (Call::Kan, Call::Ron) => Ordering::Less,
        (Call::Kan, Call::Kan) => panic!(r#"More than one "kan" call for discard"#),
        (Call::Kan, _) => Ordering::Greater,

        (Call::Pon, Call::Ron) => Ordering::Less,
        (Call::Pon, Call::Kan) => Ordering::Less,
        (Call::Pon, Call::Pon) => panic!(r#"More than one "pon" call for discard"#),
        (Call::Pon, Call::Chii(..)) => Ordering::Greater,

        (Call::Chii(..), Call::Chii(..)) => panic!(r#"More than one "chii" call for discard"#),
        (Call::Chii(..), _) => Ordering::Less,
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

#[cfg(test)]
mod tests {
    use super::{compare_calls, Call::*};
    use crate::tile::Wind::*;
    use crate::tile::TILE_SET;
    use std::cmp::Ordering;

    #[test]
    fn call_precedence() {
        // Grab the ID of the first tile in `TILE_SET` to use as a dummy ID when checking
        // chii calls. The tiles selected in the call shouldn't impact ordering, so it
        // doesn't matter that it's not technically valid to have two of the same ID in the
        // call.
        let id = TILE_SET[0].id;

        // "Ron" has highest precedence.
        assert_eq!(
            compare_calls(East, Ron, West, Kan, South),
            Ordering::Greater
        );
        assert_eq!(
            compare_calls(East, Ron, West, Pon, South),
            Ordering::Greater
        );
        assert_eq!(
            compare_calls(East, Ron, West, Chii(id, id), South),
            Ordering::Greater
        );

        // If both calls are "ron", the closest to the discarding player has precedence.
        assert_eq!(
            compare_calls(East, Ron, West, Ron, North),
            Ordering::Greater
        );
        assert_eq!(compare_calls(East, Ron, West, Ron, South), Ordering::Less);

        // "Kan" has next highest.
        assert_eq!(
            compare_calls(East, Kan, West, Pon, South),
            Ordering::Greater
        );
        assert_eq!(
            compare_calls(East, Kan, West, Chii(id, id), South),
            Ordering::Greater
        );

        // "Pon" only has precedence over "chii".
        assert_eq!(
            compare_calls(East, Pon, West, Chii(id, id), South),
            Ordering::Greater
        );
    }
}
