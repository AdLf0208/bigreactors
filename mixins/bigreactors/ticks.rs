// ... and they tick on the server.
fn observe_block_entity_ticks(block: &str, ticks: &mut bool) {
    crate::game::bigreactors_state::on_block_entity_ticks(block, ticks);
}
