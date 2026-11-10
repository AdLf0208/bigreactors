// After the level's tick: the power taps push their FE out.
fn observe_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::bigreactors_state::on_level_tick_post(game, dimension);
}
