use crate::tile::{TileId, TileInstance};
use fehler::{throw, throws};
use thiserror::Error;
use take_if::TakeIf;

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
#[derive(Debug)]
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
    #[throws(WrongNumberOfTiles)]
    pub fn new<T: Into<Vec<TileInstance>>>(starting_tiles: T) {
        let tiles = starting_tiles.into();
        if tiles.len() != 13 {
            throw!(WrongNumberOfTiles(tiles.len()));
        }

        Hand {
            tiles,
            current_draw: None,
            open_chows: Default::default(),
            open_pongs: Default::default(),
            open_kongs: Default::default(),
            closed_kongs: Default::default(),
            discards: Default::default(),
        }
    }

    #[throws(DrawError)]
    pub fn draw_tile(&mut self, tile: TileInstance) {
        if self.current_draw.is_some() {
            throw!(DrawError(tile));
        }

        self.current_draw = Some(tile);
    }

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
