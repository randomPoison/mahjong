use crate::client::ClientControllerProxy;
use mahjong::{game::*, tile};
use rand::{seq::SliceRandom, SeedableRng};
use rand_pcg::*;
use std::collections::HashMap;
use thespian::*;

#[derive(Debug, Actor)]
pub struct MatchController {
    rng: Pcg64Mcg,
    state: Match,

    /// Mapping of which client controls which player seat. Key is the index of the
    clients: HashMap<usize, ClientControllerProxy>,
}

impl MatchController {
    pub fn new(id: MatchId) -> Self {
        let mut rng = Pcg64Mcg::from_entropy();

        // Generate the tileset and shuffle it.
        let mut tiles = tile::generate_tileset();
        tiles.shuffle(&mut rng);

        Self {
            rng,
            state: Match::new(id, tiles),
            clients: Default::default(),
        }
    }
}

#[thespian::actor]
impl MatchController {
    pub fn state(&self) -> Match {
        self.state.clone()
    }
}
