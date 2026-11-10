// A world opened or closed: drop the queued positions.
fn observe_world_lifecycle(game: &mut crate::game::Game<'_>, phase: WorldPhase) {
    crate::game::bigreactors_state::on_world_lifecycle(game, phase);
}
