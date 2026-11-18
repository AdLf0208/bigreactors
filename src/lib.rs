//! Extreme Reactors for Horizon — slice 1, the basic passive reactor.
//!
//! The Controller is the machine: fuel, waste, heat and the energy buffer live in its saved
//! compound. The Solid Access Port is the inventory fuel goes in through, and the Active Forge
//! Energy Power Tap is where FE leaves through a `Capabilities.Energy.BLOCK` buffer.
//!
//! A block entity is out of the world during its own tick, so the tap queues its position in
//! [`TAP_QUEUE`] and moves its FE to its neighbours at `level_tick_post`.
//!
//! Formation is upstream's `MultiblockReactor.isMachineWhole`, restricted to the one fuel
//! assembly this slice simulates: a 3..5 block shell, control rods on one face, and under every
//! control rod one or more fuel rods ending on casing. The rule and every number are in
//! [`runtime`], which has no `mod_api` types and is tested with `cargo test` alone.
pub mod runtime;

use std::sync::Mutex;

use mod_api::*;

use crate::runtime::{BASIC_MAX_EXTRACTION_RATE, Design, Part, Reactor, Shape, check_shape};

/// The six blocks of the slice (`ReactorPartType` in the jar's own registry).
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
/// The access port's one slot (upstream's in/out port; slice 1 has the inlet).
const PORT_SLOTS: usize = 1;

/// A tap position queued for this tick's energy moves. A ticked block entity is out of the
/// world, so its own tick cannot hand FE to a neighbour; `level_tick_post` can.
static TAP_QUEUE: Mutex<Vec<(LevelRef, [i32; 3])>> = Mutex::new(Vec::new());

/// The mod has no settings; the only state is the pending tap queue, which is per world.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Attach a block entity to the three blocks that own one, with the capability each block
    /// exposes: the access port's slot and the tap's FE buffer.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let port = r.block_key(ACCESS_PORT);
        let tap = r.block_key(POWER_TAP);
        r.block_container(
            &port,
            ModContainer {
                slots: PORT_SLOTS,
                faces: std::array::from_fn(|_| (0..PORT_SLOTS).collect()),
                inputs: (0..PORT_SLOTS).collect(),
                outputs: Vec::new(),
                menu: None,
            },
        );
        // Upstream's active tap only gives; every face may drain it.
        r.block_energy(
            &tap,
            ModEnergyBuffer {
                capacity: TAP_CAPACITY,
                max_insert: 0,
                max_extract: TAP_CAPACITY,
                insert_faces: [false; 6],
                extract_faces: [true; 6],
            },
        );
        let controller = r
            .block_entity_type(CONTROLLER)
            .blocks(&[CONTROLLER])
            .build();
        let port_kind = r
            .block_entity_type(ACCESS_PORT)
            .blocks(&[ACCESS_PORT])
            .build();
        let tap_kind = r.block_entity_type(POWER_TAP).blocks(&[POWER_TAP]).build();
        r.block_entity_part(&controller, ControllerPart)
            .expect("a block entity type this mod declares takes its own part");
        r.block_entity_part(&port_kind, PortPart)
            .expect("a block entity type this mod declares takes its own part");
        r.block_entity_part(&tap_kind, TapPart)
            .expect("a block entity type this mod declares takes its own part");
    }

    /// `/bigreactors status <pos>`: a read-only view of a reactor, for players and the harness
    /// (upstream has no command).
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(Command::new("bigreactors status <pos:block_pos>"));
    }

    /// The `/bigreactors` command body: read a reactor's state.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        if event.command.literals.first().map(String::as_str) != Some("bigreactors") {
            return Flow::Continue;
        }
        if event.command.literals.get(1).map(String::as_str) != Some("status") {
            return Flow::Continue;
        }
        let Some(pos) = position_argument(event.command.argument("pos")) else {
            feedback(host, "Usage: /bigreactors status <x> <y> <z>");
            return Flow::Cancel;
        };
        let line = status_text(host, pos);
        feedback(host, &line);
        Flow::Cancel
    }

    /// Right-clicking the Controller switches the reactor on or off; upstream's button for that
    /// is in the controller's GUI, which this slice does not have.
    fn right_click_block(&self, host: HostSide, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let HostSide::Server(host) = host else {
            return Flow::Continue;
        };
        let pos = event.click.pos;
        let Some(level) = in_place_level(host) else {
            return Flow::Continue;
        };
        if block_at(host, level, pos).as_deref() != Some(CONTROLLER) {
            return Flow::Continue;
        }
        let message = match scan(host, level, pos) {
            Ok(_) => {
                let Some(entity) = host.block_entity(level, pos) else {
                    return Flow::Continue;
                };
                let mut reactor = reactor_state(&entity.data(host).unwrap_or_default());
                let active = reactor.toggle();
                let mut patch = Compound::new();
                store_reactor(&mut patch, &reactor);
                if entity.merge_data(host, patch).is_err() {
                    return Flow::Continue;
                }
                if active {
                    "Reactor is on".to_owned()
                } else {
                    "Reactor is off".to_owned()
                }
            }
            Err(reason) => format!("The reactor did not form: {reason}"),
        };
        match host.player(event.click.player) {
            Some(player) => {
                let _ = player.action_bar(host, Text::literal(message));
            }
            None => feedback(host, &message),
        }
        *event.cancellation_result = InteractionResult::Fail;
        Flow::Cancel
    }

    /// A world opened or closed: the queued tap positions belong to that world.
    fn world_lifecycle(&self, _host: &mut dyn ServerHost, _event: &mut WorldLifecycleEvent<'_>) {
        TAP_QUEUE
            .lock()
            .expect("the tap queue lock is held for one clear")
            .clear();
    }

    /// After a level's tick: every queued tap pushes its FE into the blocks beside it. A tap is
    /// out of the world in its own tick, which is why this happens here.
    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        let level = event.level;
        let queued = {
            let mut queue = TAP_QUEUE
                .lock()
                .expect("the tap queue lock is held for one take");
            std::mem::take(&mut *queue)
        };
        if queued.is_empty() {
            return;
        }
        let mut later = Vec::new();
        for (tap_level, pos) in queued {
            if tap_level != level {
                later.push((tap_level, pos));
                continue;
            }
            if block_at(host, level, pos).as_deref() == Some(POWER_TAP) {
                push_out(host, level, pos);
            }
        }
        if !later.is_empty() {
            TAP_QUEUE
                .lock()
                .expect("the tap queue lock is held for one append")
                .extend(later);
        }
    }
}

// ---------------------------------------------------------------------------
// The three block entities: the controller, the access port and the power tap
// ---------------------------------------------------------------------------

/// The machine (`MultiblockReactorControllerEntity`).
struct ControllerPart;
/// The inventory fuel goes in through (`MultiblockReactorSolidAccessPortEntity`).
struct PortPart;
/// The FE face (`MultiblockReactorPowerTapEntity`).
struct TapPart;

#[mod_api::horizon_part]
impl BlockPart for ControllerPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for ControllerPart {
    /// A fresh controller starts empty; its compound holds the reactor (see [`Reactor`]).
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick of the reactor: formation scan, fuel intake, one simulation tick, FE to
    /// the taps.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = in_place_level(host) else {
            return;
        };
        let pos = event.pos;
        let Ok(formed) = scan(host, level, pos) else {
            return;
        };
        let mut reactor = reactor_state(&event.entity.nbt);
        let capacity = formed.design.energy_capacity();
        if reactor.energy > f64::from(capacity) {
            reactor.energy = f64::from(capacity);
        }
        for port in &formed.shape.ports {
            let ingots = take_fuel(host, level, *port);
            if ingots > 0 {
                reactor.insert_ingots(&formed.design, ingots);
            }
        }
        reactor.tick(&formed.design);
        fill_taps(host, level, &formed, &mut reactor);
        store_reactor(&mut event.entity.nbt, &reactor);
    }
}

#[mod_api::horizon_part]
impl BlockPart for PortPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for PortPart {
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// The port itself does nothing on its tick: the controller reaches into its slot.
    fn block_entity_tick(&self, _host: &mut dyn ServerHost, _event: &mut BlockEntityTickEvent<'_>) {
    }
}

#[mod_api::horizon_part]
impl BlockPart for TapPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for TapPart {
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// The tap cannot reach its neighbours during its own tick, so it queues itself.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = in_place_level(host) else {
            return;
        };
        let mut queue = TAP_QUEUE
            .lock()
            .expect("the tap queue lock is held for one push");
        if !queue
            .iter()
            .any(|(queued_level, queued)| *queued_level == level && *queued == event.pos)
        {
            queue.push((level, event.pos));
        }
    }
}

// ---------------------------------------------------------------------------
// Formation and the reactor's saved state
// ---------------------------------------------------------------------------

/// A formed reactor: its simulated design and where its parts are.
struct Formed {
    design: Design,
    shape: Shape,
}

/// Upstream's `isMachineWhole` over the world: the pure checker walks the blocks this reads, and
/// the design comes from what it found.
fn scan(host: &dyn ServerHost, level: LevelRef, origin: [i32; 3]) -> Result<Formed, String> {
    let shape =
        check_shape(|pos| part_at(host, level, pos), origin).map_err(|error| error.to_string())?;
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

fn part_at(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<Part> {
    part_of(&block_at(host, level, pos)?)
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
    write_f64(
        nbt,
        "energy_fraction",
        reactor.energy - f64::from(reactor.energy as i32),
    );
    write_f64(nbt, "fertility", reactor.fertility);
    write_f64(nbt, "fuel_accumulator", reactor.accumulator);
}

// ---------------------------------------------------------------------------
// Fuel, FE and the world reads
// ---------------------------------------------------------------------------

/// `SolidAccessPort` + reactant mapping: one yellorium ingot is 1000 mB. Returns the ingots
/// taken, which the reactor turns into fuel.
fn take_fuel(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> i32 {
    host.transaction(|host, tx| -> Result<i32, ()> {
        let Some(entity) = host.block_entity(level, pos) else {
            return Ok(0);
        };
        let Ok(Some(stack)) = entity.item_slot(host, 0) else {
            return Ok(0);
        };
        if stack.item().id(host) != YELLORIUM_INGOT {
            return Ok(0);
        }
        let ingots = i32::try_from(stack.count()).unwrap_or(i32::MAX);
        // The fuel is only consumed when the slot really lost it; an unchanged slot hands the
        // reactor no fuel rather than making it from a failed write.
        if !matches!(entity.set_item_slot(host, tx, 0, None), Ok(true)) {
            return Ok(0);
        }
        let _ = entity.set_changed(host);
        Ok(ingots)
    })
    .unwrap_or(0)
}

/// Upstream exposes the reactor's buffer on every part: the tap's own buffer takes what fits of
/// the reactor's FE, so the buffer is conserved.
fn fill_taps(host: &mut dyn ServerHost, level: LevelRef, formed: &Formed, reactor: &mut Reactor) {
    for tap in &formed.shape.taps {
        let Some(entity) = host.block_entity(level, *tap) else {
            continue;
        };
        let data = entity.data(host).unwrap_or_default();
        let stored = read_int(&data, "energy").max(0);
        let room = TAP_CAPACITY.saturating_sub(stored);
        let moved = f64::from(room).min(reactor.energy).max(0.0);
        if moved < 1.0 {
            continue;
        }
        let moved = moved as i32;
        let mut patch = Compound::new();
        write_int(&mut patch, "energy", stored + moved);
        if matches!(entity.merge_data(host, patch), Ok(true)) {
            reactor.energy -= f64::from(moved);
        }
    }
}

/// `IPowerPortHandler.extractFromSides` at the extraction rate, into every side whose neighbour
/// takes FE. The move is one transaction: put into the neighbour first, then take the same
/// amount out of the tap, so a refused insert creates no energy.
fn push_out(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    for side in Direction::ALL {
        let [dx, dy, dz] = side.normal();
        let neighbour = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
        let Some(own) = level.capability(host, &ENERGY, pos, Some(side)) else {
            continue;
        };
        let available = own.amount(host);
        if available == 0 {
            return;
        }
        let Some(target) = level.capability(host, &ENERGY, neighbour, Some(side.opposite())) else {
            continue;
        };
        let room = target.capacity(host).saturating_sub(target.amount(host));
        let amount = available.min(room).min(TAP_CAPACITY.unsigned_abs());
        if amount == 0 {
            continue;
        }
        let _ = host.transaction(|host, tx| -> Result<u32, ()> {
            let Some(own) = level.capability(&mut *host, &ENERGY, pos, Some(side)) else {
                return Ok(0);
            };
            let Some(target) =
                level.capability(&mut *host, &ENERGY, neighbour, Some(side.opposite()))
            else {
                return Ok(0);
            };
            let inserted = target.insert(&mut *host, tx, amount);
            if inserted == 0 {
                return Ok(0);
            }
            let extracted = own.extract(&mut *host, tx, inserted);
            if extracted != inserted {
                return Err(());
            }
            Ok(inserted)
        });
    }
}

/// The block at `pos` in `level`, as its id.
fn block_at(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    Some(level.block(host, pos)?.id(host))
}

/// The level the server ticks now; its block entities and capabilities are the ones a tick may
/// read.
fn in_place_level(host: &dyn ServerHost) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|level| level.in_place(host))
}

// ---------------------------------------------------------------------------
// `/bigreactors status`
// ---------------------------------------------------------------------------

/// The block's state, as `/bigreactors status` prints it: one line the harness parses.
fn status_text(host: &mut dyn ServerHost, pos: [i32; 3]) -> String {
    let Some(level) = in_place_level(host) else {
        return format!("No reactor block at {}, {}, {}.", pos[0], pos[1], pos[2]);
    };
    let Some(block) = block_at(host, level, pos) else {
        return format!("No reactor block at {}, {}, {}.", pos[0], pos[1], pos[2]);
    };
    let time = level.game_time(host).unwrap_or(0);
    let Some(entity) = host.block_entity(level, pos) else {
        return format!("No reactor block at {}, {}, {}.", pos[0], pos[1], pos[2]);
    };
    let data = entity.data(host).unwrap_or_default();
    match block.as_str() {
        CONTROLLER => {
            let reactor = reactor_state(&data);
            match scan(host, level, pos) {
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
                    time
                ),
                Err(reason) => format!(
                    "Reactor at {},{},{}: not formed ({reason}), fuel {}, active {}, t={}",
                    pos[0], pos[1], pos[2], reactor.fuel, reactor.active, time
                ),
            }
        }
        POWER_TAP => format!(
            "Power Tap at {},{},{}: {}/{} FE, t={}",
            pos[0],
            pos[1],
            pos[2],
            read_int(&data, "energy"),
            TAP_CAPACITY,
            time
        ),
        ACCESS_PORT => format!(
            "Access Port at {},{},{}: {}, t={}",
            pos[0],
            pos[1],
            pos[2],
            entity.item_slot(host, 0).ok().flatten().map_or_else(
                || "empty".to_owned(),
                |stack| format!("{} x{}", stack.item().id(host), stack.count())
            ),
            time
        ),
        _ => format!("No reactor block at {}, {}, {}.", pos[0], pos[1], pos[2]),
    }
}

fn position_argument(value: Option<&CommandArgumentValue>) -> Option<[i32; 3]> {
    match value? {
        CommandArgumentValue::BlockPos(pos) => Some(*pos),
        _ => None,
    }
}

fn feedback(host: &mut dyn ServerHost, line: &str) {
    let _ = host.commands().feedback(Text::literal(line));
}

// ---------------------------------------------------------------------------
// Saved-compound reads
// ---------------------------------------------------------------------------

fn read_int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

fn read_f64(nbt: &Compound, key: &str) -> Option<f64> {
    match nbt.get(&key.into()) {
        Some(Tag::Double(bits)) => Some(f64::from_bits(*bits)),
        _ => None,
    }
}

fn write_f64(nbt: &mut Compound, key: &str, value: f64) {
    nbt.insert(key.into(), Tag::Double(value.to_bits()));
}

fn read_bool(nbt: &Compound, key: &str) -> bool {
    matches!(nbt.get(&key.into()), Some(Tag::Byte(1)))
}

fn write_bool(nbt: &mut Compound, key: &str, value: bool) {
    nbt.insert(key.into(), Tag::Byte(i8::from(value)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_six_reactor_blocks_are_the_six_parts() {
        assert_eq!(part_of(CASING), Some(Part::Casing));
        assert_eq!(part_of(CONTROLLER), Some(Part::Controller));
        assert_eq!(part_of(FUEL_ROD), Some(Part::FuelRod));
        assert_eq!(part_of(CONTROL_ROD), Some(Part::ControlRod));
        assert_eq!(part_of(ACCESS_PORT), Some(Part::AccessPort));
        assert_eq!(part_of(POWER_TAP), Some(Part::PowerTap));
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
}
