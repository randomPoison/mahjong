use crate::client::ClientControllerProxy;
use mahjong::{game::*, strum::IntoEnumIterator, tile};
use rand::{seq::SliceRandom, SeedableRng};
use rand_pcg::*;
use std::collections::HashMap;
use thespian::*;
use tile::Wind;

#[derive(Debug, Actor)]
pub struct MatchController {
    rng: Pcg64Mcg,
    state: MatchState,

    /// Mapping of which client controls which player seat. Key is the index of the
    clients: HashMap<usize, ClientControllerProxy>,
}

impl MatchController {
    pub fn new(id: MatchId) -> Self {
        let mut rng = Pcg64Mcg::from_entropy();

        // Generate the tileset and shuffle it.
        let mut tiles = tile::generate_tileset();
        tiles.shuffle(&mut rng);

        let mut state = MatchState::new(id, tiles);

        // Deal each player their initial 13 tiles.
        for seat in Wind::iter() {
            state.draw_for_player(seat, 13).unwrap();
        }

        // For the east player, have them draw the tile for their first turn.
        state.draw_into_hand(Wind::East).unwrap();

        Self {
            rng,
            state,
            clients: Default::default(),
        }
    }
}

#[thespian::actor]
impl MatchController {
    pub fn state(&self) -> MatchState {
        self.state.clone()
    }
}
