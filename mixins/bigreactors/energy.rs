// The power tap's FE buffer.
fn observe_block_entity_energy(
    block: &str,
    energy: &mut Option<horizon_client::containers::ModEnergyBuffer>,
) {
    crate::game::bigreactors_state::on_block_entity_energy(block, energy);
}
