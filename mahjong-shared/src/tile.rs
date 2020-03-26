use cs_bindgen::prelude::*;
use derive_more::*;
use serde::*;
use strum::*;

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From, Serialize, Deserialize)]
pub enum Tile {
    Simple(SimpleTile),
    Bonus(BonusTile),
    Honor(HonorTile),
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
pub enum BonusTile {
    Flower(Flower),
    Season(Season),
}

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Serialize, Deserialize)]
pub enum Flower {
    PlumBlossom,
    Orchid,
    Chrysanthemum,
    Bamboo,
}

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
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
            tiles.push(HonorTile::Dragon(dragon).into());
        }
    }

    for wind in Wind::iter() {
        for _ in 0..4 {
            tiles.push(HonorTile::Wind(wind).into());
        }
    }

    // Add the bonus tiles:
    //
    // * There are flower and wind bonus tiles.
    // * There is only one of each bonus tile.

    for flower in Flower::iter() {
        tiles.push(BonusTile::Flower(flower).into());
    }

    for season in Season::iter() {
        tiles.push(BonusTile::Season(season).into());
    }

    tiles
}
