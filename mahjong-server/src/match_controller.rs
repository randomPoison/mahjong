use crate::client_controller::ClientControllerProxy;
use mahjong::{
    anyhow::*,
    client::{LocalState, LocalTurnState},
    hand::Call,
    match_state::*,
    messages::MatchEvent,
    strum::IntoEnumIterator,
    tile,
};
use rand::{seq::SliceRandom, SeedableRng};
use rand_pcg::*;
use std::collections::HashMap;
use thespian::{Actor, Remote};
use tile::{TileId, Wind};
use tracing::*;

#[derive(Debug, Actor)]
pub struct MatchController {
    rng: Pcg64Mcg,
    state: MatchState,

    /// Mapping of which client controls which player seat. Key is the index of the
    clients: HashMap<Wind, ClientProxy>,
    num_players: u8,

    remote: Remote<Self>,
}

impl MatchController {
    // TODO: Verify that the requested number of player is at least 1 and no more than 4.
    pub fn new(id: MatchId, num_players: u8, remote: Remote<Self>) -> Self {
        let mut rng = Pcg64Mcg::from_entropy();

        // Generate the tileset and shuffle it.
        let mut tiles = tile::TILE_SET.clone();
        tiles.shuffle(&mut rng);

        Self {
            rng,
            state: MatchState::new(id, tiles),
            clients: Default::default(),
            num_players,
            remote,
        }
    }

    fn broadcast(&mut self, event: MatchEvent) {
        trace!("Broadcasting event to all clients: {:?}", event);

        for client in self.clients.values_mut() {
            client
                .send_event(event.clone())
                .expect("Disconnected from client controller");
        }
    }

    #[instrument(skip(self))]
    fn broadcast_draw(&mut self, seat: Wind, tile: TileId) {
        for (&client_seat, client) in &mut self.clients {
            let event = if client_seat == seat {
                MatchEvent::LocalDraw { seat, tile }
            } else {
                MatchEvent::RemoteDraw { seat }
            };

            client
                .send_event(event)
                .expect("Failed to send message to client controller");
        }
    }
}

#[thespian::actor]
impl MatchController {
    #[tracing::instrument(skip(self, controller))]
    pub fn join(&mut self, controller: ClientControllerProxy, seat: Wind) -> Result<LocalState> {
        info!(?seat, "Player joining match");

        if self.clients.contains_key(&seat) {
            bail!("Seat {:?} is already occupied", seat);
        }

        self.clients.insert(seat, ClientProxy::Client(controller));

        // If the expected number of players has joined the match, start the match once we
        // finish processing the current message.
        //
        // NOTE: We send a message to the actor to start the match rather than performing
        // the match start logic here because we want to make sure we send the initial match
        // state to the joining client before we broadcast any state updates.
        if self.clients.len() as u8 == self.num_players {
            info!("All players have joined match, triggering match start");

            self.remote
                .proxy()
                .start_match()
                .expect("Failed to send self a message");
        }

        Ok(self.state.local_state_for_player(seat))
    }

    /// Returns the updated match state if the requested discard is valid.
    #[tracing::instrument(skip(self, player, tile))]
    pub fn discard_tile(&mut self, player: Wind, tile: TileId) -> Result<()> {
        trace!(
            "Attempting to discard tile {:?} for player {:?}",
            tile,
            player,
        );

        // TODO: Provide more robust state transitions such that it's not possible to get
        // this far after the match has ended, e.g. a `MatchControllerState` enum that has
        // different states for whether the match is ongoing or completed.
        if self.state.wall.is_empty() {
            bail!("Match already finished");
        }

        // TODO: Verify that the client submitting the action is actually the one that
        // controls the player.

        let calling_players = self.state.discard_tile(player, tile)?;
        trace!(
            "Successfully discarded tile, calling players: {:?}",
            calling_players,
        );

        // Notify each client of the draw event, including the list of calls that the local
        // player can make.
        for seat in Wind::iter() {
            let calls = calling_players.get(&seat).cloned().unwrap_or_default();
            self.clients
                .get_mut(&seat)
                .unwrap()
                .send_event(MatchEvent::TileDiscarded {
                    seat: player,
                    tile,
                    calls,
                })
                .expect("Failed to send match update to client");
        }

        // If no players can call the discarded tile, immediately move past the call phase
        // and draw for the next player.
        if calling_players.is_empty() {
            self.state.decide_call()?;
            self.broadcast(MatchEvent::Pass);

            let next_player = player.next();
            let draw = self.state.draw_for_player(next_player)?;
            self.broadcast_draw(next_player, draw);
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, player, call))]
    pub fn call_tile(&mut self, player: Wind, call: Option<Call>) -> Result<()> {
        trace!("Player {:?} attempting to make call {:?}", player, call);

        // Register the requested call with the match state. If there are still more players
        // that need to make a call, then return early and wait for the remaining players.
        //
        // NOTE: There's nothing else for us to do in this case since the server doesn't
        // broadcast any calls until the final decision is made, and requesting a call
        // doesn't directly change the turn state.
        if !self.state.request_call(player, call)? {
            trace!("More players need to make a call, not deciding call yet");
            return Ok(());
        }

        // All calling players have registered their intended call, so now we determine
        // which call wins.
        trace!("All players have called, deciding call now");
        match self.state.decide_call()? {
            Some(winning_call) => {
                trace!("Winning call decided: {:?}", winning_call);
                self.broadcast(MatchEvent::Call(winning_call));
            }

            None => {
                trace!("All players passed");
                self.broadcast(MatchEvent::Pass);
            }
        }

        // Handle next step in the match.
        match &self.state.turn_state {
            &TurnState::AwaitingDraw(next_player) => {
                trace!("Drawing for next player {:?} after call", next_player);
                let draw = self.state.draw_for_player(next_player)?;
                self.broadcast_draw(next_player, draw);
            }

            TurnState::MatchEnded { .. } => {
                trace!("Match ended after call");
                self.broadcast(MatchEvent::MatchEnded);
            }

            _ => unimplemented!("Invalid turn state: {:?}", self.state.turn_state),
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn start_match(&mut self) {
        info!("Starting match");

        for seat in Wind::iter() {
            // NOTE: We need to manually split the borrows of `self.state` and `self.remote`
            // here because closures can't capture disjoint fields currently. This can be
            // cleaned up once rustc supports this.
            // See https://github.com/rust-lang/rust/issues/53488
            let state = &self.state;
            let remote = &self.remote;
            self.clients.entry(seat).or_insert_with(|| {
                let dummy = DummyClient {
                    seat,
                    state: state.local_state_for_player(seat),
                    controller: remote.proxy(),
                }
                .spawn();

                ClientProxy::Dummy(dummy)
            });
        }

        // Draw the first tile and broadcast to the connected clients.
        let seat = match &self.state.turn_state {
            &TurnState::AwaitingDraw(seat) => seat,

            _ => panic!(
                "Unexpected turn state at start of match: {:?}",
                self.state.turn_state,
            ),
        };

        let tile = self
            .state
            .draw_for_player(seat)
            .expect("Failed to draw for first player");

        self.broadcast_draw(seat, tile);
    }
}

/// Actor that controls players that aren't controlled by an active client.
#[derive(Debug, Actor)]
struct DummyClient {
    seat: Wind,
    state: LocalState,
    controller: MatchControllerProxy,
}

#[thespian::actor]
impl DummyClient {
    #[instrument(skip(self, event))]
    fn send_event(&mut self, event: MatchEvent) {
        trace!(
            "Dummy client {:?} handling match event: {:?}",
            self.seat,
            event,
        );

        // Apply the event to the local state.
        self.state.handle_event(&event).unwrap();

        // If the player we control can take an action now, make that action now.
        match &self.state.turn_state {
            &LocalTurnState::AwaitingDiscard(seat) => {
                if seat == self.seat {
                    let discard = self.state.local_hand(seat).tiles()[0].id;
                    let _ = self.controller.discard_tile(self.seat, discard).unwrap();
                }
            }

            LocalTurnState::AwaitingCalls { calls, .. } => {
                if !calls.is_empty() {
                    let _ = self
                        .controller
                        .call_tile(self.seat, Some(calls[0]))
                        .unwrap();
                }
            }

            // No actions to be taken for the remaining state.
            LocalTurnState::MatchEnded { .. } => {}
            LocalTurnState::AwaitingDraw(_) => {}
        }
    }
}

/// Abstraction over either a concrete client actor or a dummy client actor.
// TODO: Remove this once thespian has support for actor traits.
// Tracking issue: https://github.com/randomPoison/thespian/issues/15
#[derive(Debug, Clone)]
enum ClientProxy {
    Client(ClientControllerProxy),
    Dummy(DummyClientProxy),
}

impl ClientProxy {
    pub fn send_event(&mut self, event: MatchEvent) -> Result<(), thespian::MessageError> {
        match self {
            ClientProxy::Client(proxy) => proxy.send_event(event),
            ClientProxy::Dummy(proxy) => proxy.send_event(event),
        }
    }
}
