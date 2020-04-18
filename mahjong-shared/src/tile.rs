use cs_bindgen::prelude::*;
use derive_more::*;
use serde::*;
use strum::*;

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Serialize, Deserialize)]
pub enum Tile {
    Simple(SimpleTile),
    Wind(Wind),
    Dragon(Dragon),
}

impl Tile {
    pub fn is_honor(self) -> bool {
        match self {
            Tile::Wind(..) | Tile::Dragon(..) => true,
            Tile::Simple(..) => false,
        }
    }

    pub fn as_honor(self) -> Option<HonorTile> {
        match self {
            Tile::Wind(wind) => Some(HonorTile::Wind(wind)),
            Tile::Dragon(dragon) => Some(HonorTile::Dragon(dragon)),
            Tile::Simple(..) => None,
        }
    }
}

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Serialize, Deserialize)]
pub enum Suit {
    Coins,
    Bamboo,
    Characters,
}

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SimpleTile {
    pub number: u8,
    pub suit: Suit,
}

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Serialize, Deserialize)]
pub enum HonorTile {
    Wind(Wind),
    Dragon(Dragon),
}

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Serialize, Deserialize)]
pub enum Wind {
    East,
    South,
    West,
    North,
}

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Serialize, Deserialize)]
pub enum Dragon {
    Red,
    Green,
    White,
}

/// Generates a complete set of Mahjong tiles, including bonus tiles.
// TODO: Make this configurable, such that we can generate tilesets for different
// styles of game without needing a bunch of different functions.
pub fn generate_tileset() -> Vec<Tile> {
    let mut tiles = Vec::with_capacity(144);

    // Add simple tiles for each suit:
    //
    // * Tiles in each suit are numbered 1-9.
    // * There are four copies of each simple tile.
    for suit in Suit::iter() {
        for number in 1..=9 {
            for _ in 0..4 {
                tiles.push(SimpleTile { suit, number }.into());
            }
        }
    }

    // Add honor tiles:
    //
    // * There are dragon and wind honors.
    // * There are four copies of each honor tile.

    for dragon in Dragon::iter() {
        for _ in 0..4 {
            tiles.push(dragon.into());
        }
    }

    for wind in Wind::iter() {
        for _ in 0..4 {
            tiles.push(wind.into());
        }
    }

    tiles
}
