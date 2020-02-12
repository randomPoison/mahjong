use mahjong::game::*;
use rand::SeedableRng;
use rand_pcg::*;
use thespian::*;

#[derive(Debug, Actor)]
pub struct MatchController {
    state: Match<Pcg64Mcg>,
}

impl MatchController {
    pub fn new(id: MatchId) -> Self {
        let rng = Pcg64Mcg::from_entropy();
        Self {
            state: Match::new(id, rng),
        }
    }
}
