//! Integration tests verifying that `MatchState` correctly updates its internal
//! state as actions are performed in the game.

use mahjong::{
    client::{LocalState, LocalTurnState},
    match_state::{MatchId, MatchState, TurnState},
    strum::IntoEnumIterator,
    tile::{self, Wind},
};
use maplit::hashmap;
use pretty_assertions::assert_eq;
use std::collections::HashMap;

// Test that the match state stays consistent when players discard tiles from their
// hands (i.e. not discarding the tile they just drew).
#[test]
fn discard_from_hand() {
    let mut state = MatchState::new(MatchId::new(0), tile::TILE_SET.clone());

    // Maintain a local state for each of the seats. This allows us to also verify that
    // the local state tracking remains consistent throughout the progression of a match.
    let mut local_states = hashmap! {
        Wind::East => state.local_state_for_player(Wind::East),
        Wind::South => state.local_state_for_player(Wind::South),
        Wind::West => state.local_state_for_player(Wind::West),
        Wind::North => state.local_state_for_player(Wind::North),
    };

    let mut current_player = Wind::East;
    while !state.wall.is_empty() {
        assert_eq!(TurnState::AwaitingDraw(current_player), state.turn_state);

        // Draw the tile for the current player.
        let draw = state.draw_for_player(current_player).unwrap();
        for seat in Wind::iter() {
            let local_state = local_states.get_mut(&seat).unwrap();

            if seat == current_player {
                local_state.draw_local_tile(draw).unwrap();
            } else {
                local_state.draw_remote_tile(current_player).unwrap();
            }
        }
        assert_state_sync(&state, &local_states);

        // Discard a tile for the current player.
        let discard = state.player(current_player).tiles()[0].id;
        state.discard_tile(current_player, discard).unwrap();
        for seat in Wind::iter() {
            let local_state = local_states.get_mut(&seat).unwrap();
            local_state.discard_tile(current_player, discard).unwrap();
        }
        assert_state_sync(&state, &local_states);

        // Verify that the player's hand is in the correct state after discarding.
        let player = state.player(current_player);
        assert_eq!(
            13,
            player.tiles().len(),
            "Player has wrong number of tiles in hand"
        );
        assert!(
            player.current_draw().is_none(),
            "Player still has a current draw after discarding"
        );

        // If any players can call the discarded tile, have them pass. We specifically
        // DON'T want any of them to actually make a call because that would potentially
        // change the turn order which isn't something we care about testing here.
        if let TurnState::AwaitingCalls { waiting, .. } = &state.turn_state {
            // NOTE: Clone `waiting` so that we aren't still borrowing `state` when we do
            // `state.call_tile()`.
            let waiting = waiting.clone();
            for &seat in waiting.keys() {
                state.request_call(seat, None).unwrap();
                local_states
                    .get_mut(&seat)
                    .unwrap()
                    .decide_call(None)
                    .unwrap();
            }

            assert_eq!(None, state.decide_call().unwrap());
        }
        assert_state_sync(&state, &local_states);

        current_player = current_player.next();
    }
}

/// Verifies that the local state for each client is in sync with the main state.
fn assert_state_sync(state: &MatchState, local_states: &HashMap<Wind, LocalState>) {
    for client_seat in Wind::iter() {
        let local_state = &local_states[&client_seat];

        println!();
        println!("Validating state for {:?} client", client_seat);

        // Check the local version of each player's hand against their main state to make
        // sure they match.
        for hand_seat in Wind::iter() {
            let hand = &state.players[&hand_seat];

            assert_eq!(
                &hand.to_local(hand_seat == client_seat),
                &local_state.players[&hand_seat],
                "Local hand state for {hand_seat:?} seat on {client_seat:?} client does not match the main state",
                hand_seat = hand_seat,
                client_seat = client_seat,
            );
        }

        // Compare the local turn state to the main turn state.
        dbg!(&state.turn_state);
        dbg!(&local_state.turn_state);

        match (&state.turn_state, &local_state.turn_state) {
            (TurnState::AwaitingDraw(seat), LocalTurnState::AwaitingDraw(local_seat)) => {
                assert_eq!(seat, local_seat);
            }

            (TurnState::AwaitingDiscard(seat), LocalTurnState::AwaitingDiscard(local_seat)) => {
                assert_eq!(seat, local_seat);
            }

            (
                TurnState::AwaitingCalls {
                    waiting,
                    discard,
                    discarding_player,
                    ..
                },
                LocalTurnState::AwaitingCalls {
                    calls: local_calls,
                    discard: local_discard,
                    discarding_player: local_discarding_player,
                },
            ) => {
                // Make sorted copies of the list of calls so that we can compare them directly.
                let mut expected_calls = waiting[&client_seat].clone();
                let mut actual_calls = local_calls.clone();
                expected_calls.sort();
                actual_calls.sort();

                assert_eq!(expected_calls, actual_calls);
                assert_eq!(discard, local_discard);
                assert_eq!(discarding_player, local_discarding_player);
            }

            // If the main state is currently awaiting calls, it's okay for the local state to
            // be awaiting the next discard as long as:
            //
            // * The local player cannot make a call, since in that case they won't know that
            //   other players can possibly make a call.
            // * The player the client expects to make the next draw is correct given which
            //   player last discarded.
            (
                TurnState::AwaitingCalls {
                    waiting,
                    discarding_player,
                    calls,
                    ..
                },
                &LocalTurnState::AwaitingDraw(next_player),
            ) => {
                assert!(!waiting.contains_key(&client_seat));
                assert!(!calls.contains_key(&client_seat));
                assert_eq!(
                    discarding_player.next(),
                    next_player,
                    "Incorrect next player for the previous discard, discarding player was {:?} so \
                    next should be {:?}, but local state is expecting {:?}",
                    discarding_player,
                    discarding_player.next(),
                    next_player,
                );
            }

            (
                TurnState::MatchEnded { winner },
                LocalTurnState::MatchEnded {
                    winner: local_winner,
                },
            ) => assert_eq!(winner, local_winner),

            _ => panic!("Local turn state is not a valid match for the current state"),
        }
    }
}
