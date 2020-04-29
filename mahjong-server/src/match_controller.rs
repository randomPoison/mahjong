use crate::client::ClientControllerProxy;
use anyhow::*;
use mahjong::{match_state::*, messages::MatchEvent, strum::IntoEnumIterator, tile};
use rand::{seq::SliceRandom, SeedableRng};
use rand_pcg::*;
use std::collections::HashMap;
use thespian::Actor;
use tile::{TileId, Wind};
use tracing::*;

#[derive(Debug, Actor)]
pub struct MatchController {
    rng: Pcg64Mcg,
    state: MatchState,

    /// Mapping of which client controls which player seat. Key is the index of the
    clients: HashMap<Wind, ClientControllerProxy>,
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

    async fn broadcast(&mut self, event: MatchEvent) {
        trace!(
            ?event,
            "Broadcasting event to {} connected client(s)",
            self.clients.len()
        );

        for client in self.clients.values_mut() {
            client
                .send_event(event.clone())
                .expect("Disconnected from client controller");
        }
    }
}

#[thespian::actor]
impl MatchController {
    pub fn id(&self) -> MatchId {
        self.state.id
    }

    pub fn join(&mut self, controller: ClientControllerProxy, seat: Wind) -> Result<MatchState> {
        if self.clients.contains_key(&seat) {
            bail!("Seat is already occupied");
        }

        self.clients.insert(seat, controller);

        Ok(self.state.clone())
    }

    /// Returns the updated match state if the requested discard is valid.
    #[tracing::instrument(skip(self))]
    pub async fn discard_tile(&mut self, player: Wind, tile: TileId) -> Result<()> {
        // TODO: Verify that the client submitting the action is actually the one that
        // controls the player.

        self.state.discard_tile(player, tile)?;

        trace!("Successfully discarded tile");

        // Broadcast the discard event to all connected clients.
        self.broadcast(MatchEvent::TileDiscarded { seat: player, tile })
            .await;

        while !self.state.wall.is_empty() {
            let player = self.state.current_turn;

            // Draw the tile for the next player.
            let draw = self.state.draw_into_hand(player)?;
            self.broadcast(MatchEvent::TileDrawn {
                seat: player,
                tile: draw.id,
            })
            .await;

            if self.clients.contains_key(&player) {
                trace!(seat = ?player, "Client at current seat, waiting for player action");
                break;
            }

            // Automatically discard the first tile in the player's hand.
            let auto_discard = self.state.player(player).hand[0].id;
            info!(
                seat = ?player,
                discard = ?auto_discard,
                "Performing action for computer-controlled player",
            );

            self.state.discard_tile(player, auto_discard)?;
            self.broadcast(MatchEvent::TileDiscarded {
                seat: player,
                tile: auto_discard,
            })
            .await;
        }

        Ok(())
    }
}
