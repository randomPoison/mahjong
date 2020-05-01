use cs_bindgen::prelude::*;
use derive_more::*;
use serde::*;
use strum::*;

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Serialize, Deserialize)]
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter, Serialize, Deserialize,
)]
pub enum Suit {
    Coins,
    Bamboo,
    Characters,
}

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SimpleTile {
    pub number: u8,
    pub suit: Suit,
}

#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Serialize, Deserialize)]
pub enum HonorTile {
    Wind(Wind),
    Dragon(Dragon),
}

#[cs_bindgen]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter, Serialize, Deserialize,
)]
pub enum Wind {
    East,
    South,
    West,
    North,
}

impl Wind {
    /// Returns the next wind in the cycle order for winds.
    ///
    /// Winds follow the order:
    ///
    /// ```text
    /// East -> South -> West -> North -> East
    /// ```
    ///
    /// Where North cycles back around to East. This is used for determining the dora
    /// from the dora indicator, and for determining turn order based on seat winds.
    ///
    /// # Examples
    ///
    /// ```
    /// use mahjong::tile::Wind;
    ///
    /// let mut wind = Wind::East;
    ///
    /// wind = wind.next();
    /// assert_eq!(Wind::South, wind);
    ///
    /// wind = wind.next();
    /// assert_eq!(Wind::West, wind);
    ///
    /// wind = wind.next();
    /// assert_eq!(Wind::North, wind);
    ///
    /// wind = wind.next();
    /// assert_eq!(Wind::East, wind);
    /// ```
    pub fn next(self) -> Self {
        match self {
            Wind::East => Wind::South,
            Wind::South => Wind::West,
            Wind::West => Wind::North,
            Wind::North => Wind::East,
        }
    }
}

#[cs_bindgen]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter, Serialize, Deserialize,
)]
pub enum Dragon {
    White,
    Green,
    Red,
}

impl Dragon {
    /// Returns the next dragon in the cycle order for dragons.
    ///
    /// Dragons follow the order:
    ///
    /// ```text
    /// White -> Green -> Red -> White
    /// ```
    ///
    /// Where Red cycles back around to Red. This is used to determine the dora based
    /// on the dora indicator.
    ///
    /// # Examples
    ///
    /// ```
    /// use mahjong::tile::Dragon;
    ///
    /// let mut dragon = Dragon::White;
    ///
    /// dragon = dragon.next();
    /// assert_eq!(Dragon::Green, dragon);
    ///
    /// dragon = dragon.next();
    /// assert_eq!(Dragon::Red, dragon);
    ///
    /// dragon = dragon.next();
    /// assert_eq!(Dragon::White, dragon);
    /// ```
    pub fn next(self) -> Self {
        match self {
            Dragon::White => Dragon::Green,
            Dragon::Green => Dragon::Red,
            Dragon::Red => Dragon::White,
        }
    }
}

/// Unique identifier for a tile within a match.
///
/// Since there are 4 copies of each tile in a standard Mahjong set, we need a way
/// to uniquely identify each tile instance separately. This type, combined with
/// [`TileInstance`], provides a way to unambiguously refer to a specific tile
/// during a match.
///
/// Tile IDs are generated once at the start of the match by the server and should
/// not change for the duration of the match. Client code should avoid creating new
/// `TileId` values.
///
/// [`TileInstance`]: struct.TileInstance.html
#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TileId(u8);

#[derive(Debug, Clone, Default)]
struct TileIdGenerator(u8);

impl TileIdGenerator {
    fn next(&mut self) -> TileId {
        let id = TileId(self.0);
        self.0 += 1;
        id
    }
}

/// An instance of a tile within a player's hand during a match.
///
/// Combines a [`TileId`] with a [`Tile`] value in order to differentiate between
/// the four copies of each tile in a mahjong set.
///
/// [`TileId`]: struct.TileId.html
/// [`Tile`]: struct.Tile.html
// TODO: Make this class not `Copy` once cs-bindgen has a different way to specify
// that a type should be marshaled by value. Since tile instances are meant to be
// unique, we don't want it to be easy to accidentally create a copy of a tile. We
// should try to always "move" the tile as a logical object in order to reduce the
// risk of bugs coming from accidentally duplicating tiles. We might even want to
// remove the `Clone` impl, since we could still use `new` to create a new instance
// if we *really* needed to.
#[cs_bindgen]
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileInstance {
    pub id: TileId,
    pub tile: Tile,
}

impl TileInstance {
    pub fn new<T: Into<Tile>>(tile: T, id: TileId) -> Self {
        Self {
            id,
            tile: tile.into(),
        }
    }
}

/// Generates a complete set of Mahjong tiles, including bonus tiles.
///
/// The tiles are not shuffled, and will always be in the same initial order.
pub fn generate_tileset() -> Vec<TileInstance> {
    let mut tiles = Vec::with_capacity(144);
    let mut id_generator = TileIdGenerator::default();

    // Add simple tiles for each suit:
    //
    // * Tiles in each suit are numbered 1-9.
    // * There are four copies of each simple tile.
    for suit in Suit::iter() {
        for number in 1..=9 {
            for _ in 0..4 {
                tiles.push(TileInstance::new(
                    SimpleTile { suit, number },
                    id_generator.next(),
                ));
            }
        }
    }

    // Add honor tiles:
    //
    // * There are dragon and wind honors.
    // * There are four copies of each honor tile.

    for dragon in Dragon::iter() {
        for _ in 0..4 {
            tiles.push(TileInstance::new(dragon, id_generator.next()));
        }
    }

    for wind in Wind::iter() {
        for _ in 0..4 {
            tiles.push(TileInstance::new(wind, id_generator.next()));
        }
    }

    tiles
}
