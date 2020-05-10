use crate::client::ClientControllerProxy;
use mahjong::{anyhow::*, match_state::*, messages::MatchEvent, tile};
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
        let mut tiles = tile::TILE_SET.clone();
        tiles.shuffle(&mut rng);

        let mut state = MatchState::new(id, tiles);

        // For the east player, have them draw the tile for their first turn.
        state.draw_for_player(Wind::East).unwrap();

        Self {
            rng,
            state,
            clients: Default::default(),
        }
    }

    fn broadcast(&mut self, event: MatchEvent) {
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
        trace!("Attempting to discard tile");

        // TODO: Provide more robust state transitions such that it's not possible to get
        // this far after the match has ended, e.g. a `MatchControllerState` enum that has
        // different states for whether the match is ongoing or completed.
        if self.state.wall.is_empty() {
            bail!("Match already finished");
        }

        // TODO: Verify that the client submitting the action is actually the one that
        // controls the player.

        self.state.discard_tile(player, tile)?;

        trace!("Successfully discarded tile");

        // Broadcast the discard event to all connected clients.
        self.broadcast(MatchEvent::TileDiscarded { seat: player, tile });

        while !self.state.wall.is_empty() {
            let player = self.state.current_turn;

            // Draw the tile for the next player.
            let draw = self.state.draw_for_player(player)?;
            self.broadcast(MatchEvent::TileDrawn {
                seat: player,
                tile: draw,
            });

            if self.clients.contains_key(&player) {
                trace!(seat = ?player, "Client at current seat, waiting for player action");
                break;
            }

            // Automatically discard the first tile in the player's hand.
            let auto_discard = self.state.player(player).tiles()[0].id;
            info!(
                seat = ?player,
                discard = ?auto_discard,
                "Performing action for computer-controlled player",
            );

            self.state.discard_tile(player, auto_discard)?;
            self.broadcast(MatchEvent::TileDiscarded {
                seat: player,
                tile: auto_discard,
            });
        }

        // If the match is over, broadcast an event notifying all clients of the outcome.
        if self.state.wall.is_empty() {
            self.broadcast(MatchEvent::MatchEnded);
        }

        Ok(())
    }
}
