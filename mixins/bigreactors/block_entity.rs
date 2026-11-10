// The controller, the access port and the power tap have a block entity.
fn observe_block_entity(block: &str, entity: &mut Option<(String, horizon_world::nbt::Compound)>) {
    crate::game::bigreactors_state::on_block_entity(block, entity);
}
