// Declare `/bigreactors status`.
fn observe_register(commands: &mut Vec<NativeCommand>) {
    crate::game::bigreactors_state::on_register_commands(commands);
}
