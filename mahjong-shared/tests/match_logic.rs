//! Integration tests verifying that `MatchState` correctly updates its internal
//! state as actions are performed in the game.

use mahjong::{
    match_state::{MatchId, MatchState, TurnState},
    tile::{self, Wind},
};

// Test that the match state stays consistent when players discard tiles from their
// hands (i.e. not discarding the tile they just drew).
#[test]
fn discard_from_hand() {
    let mut state = MatchState::new(MatchId::new(0), tile::TILE_SET.clone());

    let mut current_player = Wind::East;
    while !state.wall.is_empty() {
        assert_eq!(TurnState::AwaitingDraw(current_player), state.turn_state);

        state.draw_for_player(current_player).unwrap();
        state
            .discard_tile(current_player, state.player(current_player).tiles()[0].id)
            .unwrap();

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
                state.call_tile(seat, None).unwrap();
            }

            state.decide_call().unwrap();
        }

        current_player = current_player.next();
    }
}
