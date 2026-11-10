// Game-side glue for the Extreme Reactors slice 1: the basic passive reactor.
//
// Injected into `crate::game` as `bigreactors_state`; it needs the game, so it is
// not part of the standalone library (only `runtime.rs` is).
//
// Shape, as upstream has it: the Controller is the machine (fuel, waste, heat
// and the energy buffer live in its saved compound), the Solid Access Port is
// the inventory fuel goes in through, and the Active Forge Energy Power Tap is
// where FE leaves through a `Capabilities.Energy.BLOCK` buffer (API 142).
// A block entity is out of the world during its own tick, so the tap moves its
// FE to its neighbours at `level_tick_post` (API 23), where every block is back
// in the world.
//
// Formation is upstream's `MultiblockReactor.isMachineWhole`, restricted to the
// one fuel assembly this slice simulates: a 3..5 block shell, control rods on
// one face, and under every control rod one or more fuel rods ending on casing.

use std::sync::Mutex;

use horizon_client::containers::{BlockEntity, ModBlockEntity, ModContainer, ModEnergyBuffer};
use horizon_client::text::Text;
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::bigreactors::{BASIC_MAX_EXTRACTION_RATE, Design, Part, Reactor, Shape, check_shape};
use crate::game::native_server::{NativeCommand, NativeServerPlayer};

const CASING: &str = "bigreactors:basic_reactorcasing";
const CONTROLLER: &str = "bigreactors:basic_reactorcontroller";
const FUEL_ROD: &str = "bigreactors:basic_reactorfuelrod";
const CONTROL_ROD: &str = "bigreactors:basic_reactorcontrolrod";
const ACCESS_PORT: &str = "bigreactors:basic_reactorsolidaccessport";
const POWER_TAP: &str = "bigreactors:basic_reactorpowertapfe_active";
/// The one item the access port turns into fuel (upstream's Yellorium reactant).
const YELLORIUM_INGOT: &str = "bigreactors:yellorium_ingot";
/// The tap's buffer: one tick's extraction rate, upstream's `maxEnergyExtractionRate`.
const TAP_CAPACITY: i32 = BASIC_MAX_EXTRACTION_RATE;

fn nbt_key(name: &str) -> JavaString {
    JavaString::from(name)
}

fn read_int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&nbt_key(key)) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(nbt_key(key), Tag::Int(value));
}

fn read_f64(nbt: &Compound, key: &str) -> Option<f64> {
    match nbt.get(&nbt_key(key)) {
        Some(Tag::Double(bits)) => Some(f64::from_bits(*bits)),
        _ => None,
    }
}

fn write_f64(nbt: &mut Compound, key: &str, value: f64) {
    nbt.insert(nbt_key(key), Tag::Double(value.to_bits()));
}

fn read_bool(nbt: &Compound, key: &str) -> bool {
    matches!(nbt.get(&nbt_key(key)), Some(Tag::Byte(1)))
}

fn write_bool(nbt: &mut Compound, key: &str, value: bool) {
    nbt.insert(nbt_key(key), Tag::Byte(i8::from(value)));
}

fn state_name(
    game: &crate::game::Game<'_>,
    state: horizon_world::registry::BlockStateId,
) -> String {
    game.res.facts().get(state).name.to_string()
}

/// The id of the block at a position, as the status line prints it.
fn block_at(game: &crate::game::Game<'_>, pos: [i32; 3]) -> String {
    state_name(game, game.world.block(pos[0], pos[1], pos[2]))
}

/// `BlockEntity.setChanged`: the chunk is written at the next save.
fn mark_modified(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
}

fn offset(pos: [i32; 3], delta: [i32; 3]) -> [i32; 3] {
    [pos[0] + delta[0], pos[1] + delta[1], pos[2] + delta[2]]
}

/// The part a block id is, if it is one of the reactor's.
fn part_of(name: &str) -> Option<Part> {
    Some(match name {
        CASING => Part::Casing,
        CONTROLLER => Part::Controller,
        FUEL_ROD => Part::FuelRod,
        CONTROL_ROD => Part::ControlRod,
        ACCESS_PORT => Part::AccessPort,
        POWER_TAP => Part::PowerTap,
        _ => return None,
    })
}

fn part_at(game: &crate::game::Game<'_>, pos: [i32; 3]) -> Option<Part> {
    let state = game.world.block(pos[0], pos[1], pos[2]);
    part_of(game.res.facts().get(state).name.as_ref())
}

/// A formed reactor: its simulated design and where its parts are.
#[derive(Clone, Debug)]
struct Formed {
    design: Design,
    shape: Shape,
}

/// Upstream's `isMachineWhole` over the world: the pure checker walks the blocks
/// this reads, and the design comes from what it found.
fn scan(game: &crate::game::Game<'_>, origin: [i32; 3]) -> Result<Formed, String> {
    let shape = check_shape(|pos| part_at(game, pos), origin).map_err(|error| error.to_string())?;
    let size = [
        (shape.max[0] - shape.min[0] + 1) as u32,
        (shape.max[1] - shape.min[1] + 1) as u32,
        (shape.max[2] - shape.min[2] + 1) as u32,
    ];
    let design = Design::try_new(
        size,
        shape.parts,
        shape.fuel_rods,
        shape.control_rods,
        shape.air_steps,
    )
    .map_err(|error| error.to_string())?;
    Ok(Formed { design, shape })
}

fn reactor_state(nbt: &Compound) -> Reactor {
    let mut reactor = Reactor::new();
    reactor.active = read_bool(nbt, "active");
    reactor.fuel = read_int(nbt, "fuel").max(0);
    reactor.waste = read_int(nbt, "waste").max(0);
    reactor.fuel_heat = read_f64(nbt, "fuel_heat").unwrap_or(0.0);
    reactor.reactor_heat = read_f64(nbt, "reactor_heat").unwrap_or(Reactor::new().reactor_heat);
    let whole = f64::from(read_int(nbt, "energy").max(0));
    reactor.energy = whole + read_f64(nbt, "energy_fraction").unwrap_or(0.0);
    reactor.fertility = read_f64(nbt, "fertility").unwrap_or(1.0);
    reactor.accumulator = read_f64(nbt, "fuel_accumulator").unwrap_or(0.0);
    reactor
}

fn store_reactor(nbt: &mut Compound, reactor: &Reactor) {
    write_bool(nbt, "active", reactor.active);
    write_int(nbt, "fuel", reactor.fuel);
    write_int(nbt, "waste", reactor.waste);
    write_f64(nbt, "fuel_heat", reactor.fuel_heat);
    write_f64(nbt, "reactor_heat", reactor.reactor_heat);
    // The capability's amount is the whole `energy` int; the fraction is ours.
    write_int(nbt, "energy", reactor.energy as i32);
    write_f64(nbt, "energy_fraction", reactor.energy - f64::from(reactor.energy as i32));
    write_f64(nbt, "fertility", reactor.fertility);
    write_f64(nbt, "fuel_accumulator", reactor.accumulator);
}

/// The three blocks that have a block entity (`native_block_entity`, API 15).
#[allow(private_interfaces)]
pub fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if matches!(block, CONTROLLER | ACCESS_PORT | POWER_TAP) {
        *entity = Some((block.to_owned(), Compound::new()));
    }
}

/// ... and tick (`native_block_entity_ticks`, API 15).
#[allow(private_interfaces)]
pub fn on_block_entity_ticks(block: &str, ticks: &mut bool) {
    if matches!(block, CONTROLLER | ACCESS_PORT | POWER_TAP) {
        *ticks = true;
    }
}

/// The Solid Access Port's one slot (`native_block_entity_container`, API 16).
/// Upstream's port has an in/out mode; slice 1 has the inlet.
#[allow(private_interfaces)]
pub fn on_block_entity_container(block: &str, container: &mut Option<ModContainer>) {
    if block != ACCESS_PORT {
        return;
    }
    *container = Some(ModContainer {
        slots: 1,
        faces: std::array::from_fn(|_| vec![0]),
        inputs: vec![0],
        outputs: Vec::new(),
        menu: None,
    });
}

/// The tap's FE buffer (`native_block_entity_energy`, API 142): it gives only,
/// which is upstream's `IPowerPort` on an active tap.
#[allow(private_interfaces)]
pub fn on_block_entity_energy(block: &str, energy: &mut Option<ModEnergyBuffer>) {
    if block != POWER_TAP {
        return;
    }
    *energy = Some(ModEnergyBuffer {
        capacity: TAP_CAPACITY,
        max_insert: 0,
        max_extract: TAP_CAPACITY,
        insert_faces: [false; 6],
        extract_faces: [true; 6],
    });
}

/// Positions queued for this tick's FE moves, by dimension.
static QUEUE: Mutex<Vec<(String, [i32; 3])>> = Mutex::new(Vec::new());

fn queue(game: &crate::game::Game<'_>, pos: [i32; 3]) {
    let dimension = game.native_dimension().to_owned();
    let mut queue = QUEUE.lock().unwrap_or_else(|poison| poison.into_inner());
    if !queue.iter().any(|(dim, at)| dim == &dimension && *at == pos) {
        queue.push((dimension, pos));
    }
}

/// One server tick of a Big Reactors block (`native_block_entity_tick`, API 17).
#[allow(private_interfaces)]
pub fn on_block_entity_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
) {
    match entity.id.as_str() {
        CONTROLLER => tick_controller(game, pos, entity),
        ACCESS_PORT => {}
        POWER_TAP => queue(game, pos),
        _ => {}
    }
}

/// The machine: pull fuel in, simulate one tick, hand FE to the taps.
fn tick_controller(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let Ok(formed) = scan(game, pos) else {
        return;
    };
    let mut reactor = reactor_state(&entity.nbt);
    let capacity = formed.design.energy_capacity();
    if reactor.energy > f64::from(capacity) {
        reactor.energy = f64::from(capacity);
    }
    for port in &formed.shape.ports {
        let ingots = game
            .native_with_mod_block_entity(*port, |port| take_fuel(port))
            .unwrap_or(0);
        if ingots > 0 {
            reactor.insert_ingots(&formed.design, ingots);
            mark_modified(game, *port);
        }
    }
    reactor.tick(&formed.design);
    fill_taps(game, &formed, &mut reactor);
    store_reactor(&mut entity.nbt, &reactor);
    mark_modified(game, pos);
}

/// `Solid Access Port` + reactant mapping: one yellorium ingot is 1000 mB.
/// Returns the ingots taken, which the reactor turns into fuel.
fn take_fuel(port: &mut ModBlockEntity) -> i32 {
    let mut taken = 0;
    for slot in &mut port.items {
        let Some(stack) = slot.as_ref() else { continue };
        if stack.item.as_ref() != YELLORIUM_INGOT {
            continue;
        }
        taken += i32::try_from(stack.count).unwrap_or(i32::MAX);
        *slot = None;
    }
    taken
}

/// Upstream exposes the reactor's buffer on every part: the tap's own buffer
/// takes what fits of the reactor's FE, so the buffer is conserved.
fn fill_taps(game: &mut crate::game::Game<'_>, formed: &Formed, reactor: &mut Reactor) {
    for tap in &formed.shape.taps {
        let free = game
            .native_with_mod_block_entity(*tap, |tap| {
                let stored = read_int(&tap.nbt, "energy").max(0);
                let room = (TAP_CAPACITY - stored).max(0);
                (stored, room)
            })
            .unwrap_or((0, 0));
        let moved = (free.1 as f64).min(reactor.energy).max(0.0);
        if moved < 1.0 {
            continue;
        }
        let moved = moved as i32;
        reactor.energy -= f64::from(moved);
        if game
            .native_with_mod_block_entity(*tap, |tap| {
                write_int(&mut tap.nbt, "energy", free.0 + moved);
            })
            .is_some()
        {
            mark_modified(game, *tap);
        }
    }
}

/// After a level's tick: every tap pushes its FE into the blocks beside it
/// (`native_move_energy`, API 142). A tap is out of the world in its own tick.
#[allow(private_interfaces)]
pub fn on_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    let queued = {
        let mut queue = QUEUE.lock().unwrap_or_else(|poison| poison.into_inner());
        std::mem::take(&mut *queue)
    };
    if queued.is_empty() {
        return;
    }
    let mut later = Vec::new();
    for (dim, pos) in queued {
        if dim != dimension {
            later.push((dim, pos));
            continue;
        }
        if block_at(game, pos) == POWER_TAP {
            push_out(game, dimension, pos);
        }
    }
    if !later.is_empty() {
        QUEUE
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .extend(later);
    }
}

/// `IPowerPortHandler.extractFromSides` at the extraction rate, into every side
/// whose neighbour takes FE.
#[allow(private_interfaces)]
fn push_out(game: &mut crate::game::Game<'_>, dimension: &str, pos: [i32; 3]) {
    for side in Direction::ALL {
        let normal = side.normal();
        let neighbour = offset(pos, [normal[0], normal[1], normal[2]]);
        let Some(own) = game.native_energy(dimension, pos, side) else {
            continue;
        };
        if own.amount <= 0 {
            return;
        }
        let Some(target) = game.native_energy(dimension, neighbour, side.opposite()) else {
            continue;
        };
        if !target.can_insert || target.max_insert <= 0 {
            continue;
        }
        let amount = own.amount.min(target.max_insert).min(TAP_CAPACITY);
        if amount > 0 {
            game.native_move_energy(
                dimension,
                pos,
                side,
                neighbour,
                side.opposite(),
                amount,
            );
        }
    }
}

/// Right-clicking the Controller switches the reactor on or off; upstream's
/// button for that is in the controller's GUI, which this slice does not have.
#[allow(private_interfaces)]
pub fn on_right_click_block(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    _face: [i32; 3],
    canceled: &mut bool,
) {
    if block_at(game, pos) != CONTROLLER {
        return;
    }
    *canceled = true;
    let message = match scan(game, pos) {
        Ok(formed) => match game.native_with_mod_block_entity(pos, |entity| {
            let mut reactor = reactor_state(&entity.nbt);
            let active = reactor.toggle();
            store_reactor(&mut entity.nbt, &reactor);
            (active, formed.design.fuel_capacity())
        }) {
            Some((true, _)) => "Reactor is on".to_owned(),
            Some((false, _)) => "Reactor is off".to_owned(),
            None => "No reactor controller there.".to_owned(),
        },
        Err(reason) => format!("The reactor did not form: {reason}"),
    };
    if let Some(player) = game.native_acting_player() {
        game.native_action_bar(player.uuid, Text::literal(message));
    } else {
        game.native_feedback(Text::literal(message));
    }
}

/// `/bigreactors status <x> <y> <z>` (`register_commands`, API 8): a read-only
/// view of a reactor, for players and for the harness (upstream has no command).
#[allow(private_interfaces)]
pub fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("bigreactors status <pos:block_pos>"));
}

fn status_text(game: &crate::game::Game<'_>, pos: [i32; 3]) -> String {
    let key = (pos[0], pos[1], pos[2]);
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get(&key) else {
        return format!("No reactor block at {}, {}, {}.", pos[0], pos[1], pos[2]);
    };
    match entity.id.as_str() {
        CONTROLLER => {
            let reactor = reactor_state(&entity.nbt);
            match scan(game, pos) {
                Ok(formed) => format!(
                    "Reactor at {},{},{}: formed {}x{}x{}, {} fuel rods, {} control rods, {}/{} FE, fuel {}, waste {}, heat {:.1}, fuel heat {:.1}, active {}, t={}",
                    pos[0],
                    pos[1],
                    pos[2],
                    formed.shape.max[0] - formed.shape.min[0] + 1,
                    formed.shape.max[1] - formed.shape.min[1] + 1,
                    formed.shape.max[2] - formed.shape.min[2] + 1,
                    formed.design.fuel_rods,
                    formed.design.control_rods,
                    reactor.energy as i32,
                    formed.design.energy_capacity(),
                    reactor.fuel,
                    reactor.waste,
                    reactor.reactor_heat,
                    reactor.fuel_heat,
                    reactor.active,
                    game.world.game_time
                ),
                Err(reason) => format!(
                    "Reactor at {},{},{}: not formed ({reason}), fuel {}, active {}, t={}",
                    pos[0], pos[1], pos[2], reactor.fuel, reactor.active, game.world.game_time
                ),
            }
        }
        POWER_TAP => format!(
            "Power Tap at {},{},{}: {}/{} FE, t={}",
            pos[0],
            pos[1],
            pos[2],
            read_int(&entity.nbt, "energy"),
            TAP_CAPACITY,
            game.world.game_time
        ),
        ACCESS_PORT => format!(
            "Access Port at {},{},{}: {}, t={}",
            pos[0],
            pos[1],
            pos[2],
            entity
                .items
                .first()
                .and_then(Option::as_ref)
                .map_or_else(|| "empty".to_owned(), |stack| format!("{} x{}", stack.item, stack.count)),
            game.world.game_time
        ),
        _ => format!("No reactor block at {}, {}, {}.", pos[0], pos[1], pos[2]),
    }
}

/// The `/bigreactors` command body (`perform_command`, API 8).
#[allow(private_interfaces)]
pub fn on_perform_command(
    game: &mut crate::game::Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    let ["bigreactors", "status", x, y, z] = words.as_slice() else {
        return;
    };
    *ran = true;
    let parsed = (|| {
        Some([
            x.parse::<i32>().ok()?,
            y.parse::<i32>().ok()?,
            z.parse::<i32>().ok()?,
        ])
    })();
    let Some(pos) = parsed else {
        game.native_feedback(Text::literal("Usage: /bigreactors status <x> <y> <z>"));
        return;
    };
    game.native_feedback(Text::literal(status_text(game, pos)));
}

/// A world opened or closed (`world_lifecycle`, v1): the queue is per world.
#[allow(private_interfaces)]
pub fn on_world_lifecycle(
    _game: &mut crate::game::Game<'_>,
    _phase: crate::game::native_lifecycle::WorldPhase,
) {
    QUEUE
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::bigreactors::FUEL_PER_INGOT;
    use horizon_client::items::ItemStack;

    #[test]
    fn the_six_reactor_blocks_are_the_six_parts() {
        assert_eq!(part_of(CASING), Some(Part::Casing));
        assert_eq!(part_of(POWER_TAP), Some(Part::PowerTap));
        assert_eq!(part_of(ACCESS_PORT), Some(Part::AccessPort));
        assert_eq!(part_of("minecraft:iron_block"), None);
        assert_eq!(
            part_of("bigreactors:basic_reactorglass"),
            None,
            "the glass is not carried in slice 1"
        );
    }

    #[test]
    fn a_stored_reactor_reads_back() {
        let mut nbt = Compound::new();
        let mut reactor = Reactor::new();
        reactor.active = true;
        reactor.fuel = 2_500;
        reactor.energy = 1_234.5;
        reactor.fuel_heat = 88.25;
        reactor.reactor_heat = 61.5;
        store_reactor(&mut nbt, &reactor);
        let back = reactor_state(&nbt);
        assert_eq!(back, reactor);
        assert_eq!(read_int(&nbt, "energy"), 1_234);
        assert!((read_f64(&nbt, "energy_fraction").unwrap_or(0.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn an_empty_compound_is_a_cold_off_reactor() {
        let reactor = reactor_state(&Compound::new());
        assert!(!reactor.active);
        assert_eq!(reactor.fuel, 0);
        assert!((reactor.reactor_heat - Reactor::new().reactor_heat).abs() < f64::EPSILON);
        assert!((reactor.fertility - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn the_port_gives_up_its_yellorium() {
        let mut port = ModBlockEntity::new(Default::default());
        port.items = vec![Some(ItemStack::new(YELLORIUM_INGOT, 3))];
        assert_eq!(take_fuel(&mut port), 3);
        assert_eq!(3 * FUEL_PER_INGOT, 3_000, "one ingot is 1000 mB");
        assert!(port.items[0].is_none(), "the port is emptied");
        assert_eq!(take_fuel(&mut port), 0);
    }

    #[test]
    fn another_item_is_not_fuel() {
        let mut port = ModBlockEntity::new(Default::default());
        port.items = vec![Some(ItemStack::new("minecraft:iron_ingot", 5))];
        assert_eq!(take_fuel(&mut port), 0);
        assert!(port.items[0].is_some(), "the iron stays");
    }
}
