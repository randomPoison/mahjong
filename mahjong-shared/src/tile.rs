use cs_bindgen::prelude::*;
use derive_more::*;
use lazy_static::lazy_static;
use num_traits::{ops::wrapping::WrappingAdd, One, PrimInt};
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

impl SimpleTile {
    pub const fn new(suit: Suit, number: u8) -> Self {
        Self { suit, number }
    }
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

    /// Determines the turn distance to `other`.
    ///
    /// Distance is determined by turn order, e.g. the distance from `East` to `South`
    /// is 1, and from `East` to `North` is 3.
    ///
    /// ```
    /// use mahjong::tile::Wind::*;
    ///
    /// assert_eq!(East.distance_to(North), 3);
    /// assert_eq!(North.distance_to(East), 1);
    /// ```
    pub fn distance_to(mut self, other: Self) -> u8 {
        let mut count = 0;
        while self != other {
            self = self.next();
            count += 1;
        }
        count
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
/// A given tile ID always maps to the same tile value, as specified by [`TILE_SET`].
/// You can use [`by_id`] to lookup the [`Tile`] value for a `TileId`.
///
/// [`TileInstance`]: struct.TileInstance.html
/// [`Tile`]: struct.Tile.html
/// [`by_id`]: fn.by_id.html
#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TileId(u8);

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
// if we *really* needed to. This will likely also require support for returning
// values by reference, since we wouldn't be able to return a copy when passing
// values to Rust.
#[cs_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

lazy_static! {
    /// The full set of tile instances for a Riichi Mahjong match.
    pub static ref TILE_SET: Vec<TileInstance> =  {
        /// Helper struct for generating the tile IDs.
        #[derive(Default)]
        struct TileIdGenerator(u8);

        impl TileIdGenerator {
            fn next(&mut self) -> TileId {
                let id = TileId(self.0);
                self.0 += 1;
                id
            }
        }

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
    };
}

/// Returns the tile value associated with the specified ID.
///
/// Since each [`TileId`] has a unique mapping to a [`Tile`] value, we can lookup
/// the tile associated with a given ID. This allows us to pass around [`TileId`]
/// values while still being able to reason about the tile they refer to when
/// necessary.
///
/// [`TileId`]: struct.TileId.html
/// [`Tile`]: struct.Tile.html
#[cs_bindgen]
pub fn by_id(id: TileId) -> Tile {
    TILE_SET
        .iter()
        .find(|instance| instance.id == id)
        .map(|instance| instance.tile)
        .unwrap_or_else(|| panic!("Unknown tile ID: {:?}", id))
}

/// Determines if the given tiles form a chow, i.e. a sequence in the same suit.
///
/// All three tiles must be simple tiles of the same suit (i.e. no dragons or
/// winds), and their numeric values must form a numeric sequence. Returns `true` if
/// any permutation of the tiles is a valid sequence.
pub fn is_chow<T, U, V>(first: T, second: U, third: V) -> bool
where
    T: Into<Tile>,
    U: Into<Tile>,
    V: Into<Tile>,
{
    // Determine if all three tiles are simple tiles. Wind/Dragon tiles cannot form a
    // chow, so if any of the tiles is not a simple then we return `false.`

    let first = match first.into() {
        Tile::Simple(tile) => tile,
        _ => return false,
    };

    let second = match second.into() {
        Tile::Simple(tile) => tile,
        _ => return false,
    };

    let third = match third.into() {
        Tile::Simple(tile) => tile,
        _ => return false,
    };

    // Determine if all three tiles have the same suit.
    if first.suit != second.suit || first.suit != third.suit {
        return false;
    }

    // Check the six possible orderings for the tiles. If any of them forms a sequence
    // then it is a valid chow.
    let (first, second, third) = (first.number, second.number, third.number);
    is_sequence(&[first, second, third])
        || is_sequence(&[first, third, second])
        || is_sequence(&[second, first, third])
        || is_sequence(&[second, third, first])
        || is_sequence(&[third, first, second])
        || is_sequence(&[third, second, first])
}

/// Checks if a slice of integers is a consecutive sequence.
///
/// Returns `true` if all elements in `values` form a consecutive sequence in
/// ascending order. Specifically, each element must be exactly one greater than the
/// preceding element. This does not include wrapping, i.e. `[T::MAX, T::MIN]` is
/// not considered a valid sequence.
///
/// Returns `true` if `values` is empty or only has one element.
fn is_sequence<T>(values: &[T]) -> bool
where
    T: PrimInt + One + WrappingAdd,
{
    if values.is_empty() {
        return true;
    }

    let mut last = values[0];
    for &next in &values[1..] {
        // Check for overflow when adding 1 to the last value. If the value overflowed while
        // there are still more elements then `values` cannot be a valid sequence.
        let expected_next = last.wrapping_add(&T::one());
        if expected_next < last {
            return false;
        }

        if next != expected_next {
            return false;
        }

        last = next;
    }

    true
}

#[cfg(test)]
mod is_chow_tests {
    use super::*;
    use itertools::Itertools;

    // Tests for `is_chow`.

    #[test]
    fn rejects_honors() {
        assert!(!is_chow(
            Dragon::White,
            SimpleTile {
                suit: Suit::Coins,
                number: 1,
            },
            SimpleTile {
                suit: Suit::Coins,
                number: 2,
            },
        ));

        assert!(!is_chow(
            SimpleTile {
                suit: Suit::Coins,
                number: 1,
            },
            SimpleTile {
                suit: Suit::Coins,
                number: 2,
            },
            Dragon::White,
        ));

        assert!(!is_chow(
            Wind::East,
            SimpleTile {
                suit: Suit::Coins,
                number: 1,
            },
            SimpleTile {
                suit: Suit::Coins,
                number: 2,
            },
        ));

        assert!(!is_chow(
            SimpleTile {
                suit: Suit::Coins,
                number: 1,
            },
            SimpleTile {
                suit: Suit::Coins,
                number: 2,
            },
            Wind::East,
        ));
    }

    #[test]
    fn rejects_mismatched_suits() {
        assert!(!is_chow(
            SimpleTile {
                suit: Suit::Coins,
                number: 1,
            },
            SimpleTile {
                suit: Suit::Coins,
                number: 2,
            },
            SimpleTile {
                suit: Suit::Bamboo,
                number: 3,
            },
        ));

        assert!(!is_chow(
            SimpleTile {
                suit: Suit::Bamboo,
                number: 1,
            },
            SimpleTile {
                suit: Suit::Coins,
                number: 2,
            },
            SimpleTile {
                suit: Suit::Coins,
                number: 3,
            },
        ));

        assert!(!is_chow(
            SimpleTile {
                suit: Suit::Coins,
                number: 1,
            },
            SimpleTile {
                suit: Suit::Bamboo,
                number: 2,
            },
            SimpleTile {
                suit: Suit::Coins,
                number: 3,
            },
        ));
    }

    #[test]
    fn all_permutations() {
        let tiles = [
            SimpleTile {
                suit: Suit::Coins,
                number: 1,
            },
            SimpleTile {
                suit: Suit::Coins,
                number: 2,
            },
            SimpleTile {
                suit: Suit::Coins,
                number: 3,
            },
        ];

        for permutation in tiles.iter().permutations(3) {
            assert!(is_chow(*permutation[0], *permutation[1], *permutation[2]));
        }
    }
}

#[cfg(tests)]
mod is_sequence_tests {
    use super::is_sequence;

    #[test]
    fn empty_sequence() {
        assert!(is_sequence::<i32>(&[]));
    }

    #[test]
    fn single_sequence() {
        assert!(is_sequence(&[0]));
        assert!(is_sequence(&[i32::MIN]));
        assert!(is_sequence(&[i32::MAX]));
    }

    #[test]
    fn detects_sequences() {
        // Positive sequences.
        assert!(is_sequence(&[0, 1, 2]));
        assert!(is_sequence(&[0, 1, 2, 3, 4]));
        assert!(is_sequence(&[u32::MIN, u32::MIN + 1, u32::MIN + 2]));

        // Short sequences.
        assert!(is_sequence(&[0, 1]));
        assert!(is_sequence(&[1234, 1235]));

        // Negative sequences.
        assert!(is_sequence(&[-3, -2, -1, 0, 1, 2, 3]));
        assert!(is_sequence(&[u32::MAX - 2, u32::MAX - 1, u32::MAX]));
    }

    #[test]
    fn rejects_non_sequences() {
        assert!(!is_sequence(&[1, 2, 0]));
        assert!(!is_sequence(&[0, 1, 3, 4]));
    }

    #[test]
    fn rejects_descending_sequence() {
        assert!(!is_sequence(&[3, 2, 1]));
        assert!(!is_sequence(&[-1, -2, -3]));
    }

    #[test]
    fn wrapping_sequence() {
        assert!(!is_sequence(&[u32::MAX, u32::MIN]));
        assert!(!is_sequence(&[
            u32::MAX - 2,
            u32::MAX - 1,
            u32::MAX,
            u32::MIN,
            u32::MIN + 1,
            u32::MIN + 2,
        ]));
    }
}
