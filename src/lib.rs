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

use std::collections::HashMap;
use std::sync::Mutex;

use mod_api::*;

use crate::runtime::{
    BASIC_MAX_EXTRACTION_RATE, Design, ENERGIZER_MAX_SIDE, Energizer, EnergizerDesign,
    EnergizerPart, EnergizerShape, IoDirection, Part, Reactor, Shape, check_energizer_shape,
    check_shape,
};

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
/// The readout's usage and operator level. It names a container's contents at
/// player-chosen coordinates, so it is a gamemaster tool (level 2), as `/data` is.
const STATUS_USAGE: &str = "bigreactors status <pos:block_pos>";
const STATUS_LEVEL: u8 = 2;

/// The energizer's four blocks (`EnergizerPartType` in the jar's registry).
const ENERGIZER_CASING: &str = "bigreactors:energizercasing";
const ENERGIZER_CELL: &str = "bigreactors:energizercell";
const ENERGIZER_CONTROLLER: &str = "bigreactors:energizercontroller";
const ENERGIZER_PORT: &str = "bigreactors:energizerpowerportfe";

/// A tap position queued for this tick's energy moves. A ticked block entity is out of the
/// world, so its own tick cannot hand FE to a neighbour; `level_tick_post` can.
static TAP_QUEUE: Mutex<Vec<(LevelRef, [i32; 3])>> = Mutex::new(Vec::new());

/// One machine type's proven formations, by controller position.
type Proofs<T> = Mutex<Option<Formations<(LevelRef, [i32; 3]), T>>>;

/// Controller formations, proven once per topology. Every controller of one machine
/// shares the scan, a committed block change drops only the proofs whose box it can
/// touch (see [`change_affects`]), and [`SCAN_BUDGET`] bounds the re-proofs a tick may
/// run. The key carries the `LevelRef` (the world epoch and the level's generation),
/// so a reloaded world can never read an old entry.
static FORMED: Proofs<Formed> = Mutex::new(None);

/// Energizer formations, proven exactly like the reactor's.
static ENERGIZER_FORMED: Proofs<EnergizerFormed> = Mutex::new(None);

/// A formed energizer: its simulated design and where its parts are.
#[derive(Clone)]
struct EnergizerFormed {
    design: EnergizerDesign,
    shape: EnergizerShape,
}

/// Fresh formation scans one level tick may run, across every machine in that level.
/// A scan that starts always finishes (an energizer's 32³ box walk is the most
/// expensive one), so this bounds how many machines are re-proven in a tick, not the
/// cost of one; `level_tick_pre` refills it. A machine the budget cannot reach is
/// reported as not proven for that tick — a pause, never a stale shape.
const SCANS_PER_TICK: u32 = 4;

/// What a machine the tick budget could not re-prove reports.
const UNPROVEN: &str = "the shape is waiting to be re-checked";

/// How far from a controller an unformed basic reactor may still reach: the largest
/// box is 5 on a side (`MAX_SIDE` in [`runtime`], private there), so a group one
/// block short of it can still grow into one.
const REACTOR_REACH: i32 = 4;

/// What a caller expects a fresh scan to cost.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Budget {
    /// A block entity's tick: bounded work, shared with every other machine.
    Tick,
    /// One player action (a click or a command) or a capability call. Outside a tick
    /// it is one scan for one action; the tick's share does not bound it.
    Action,
}

/// The fresh scans this tick has left, and whether a tick is running.
struct ScanBudget {
    left: u32,
    /// Whether a level tick is open. A capability call made while one is running is the
    /// tick's work, whichever machine asked, so it is charged to the same share.
    tick_open: bool,
    /// Fresh scans the open tick has run so far.
    tick_scans: u32,
    /// The busiest tick's [`ScanBudget::tick_scans`] since the last status read. Every
    /// in-tick scan is charged, so this can never pass [`SCANS_PER_TICK`]; the readout
    /// exists so a game run can assert that, because a machine the budget paused and a
    /// machine that is simply broken answer the same to a capability call.
    peak_scans: u32,
}

static SCAN_BUDGET: Mutex<ScanBudget> = Mutex::new(ScanBudget::new(SCANS_PER_TICK));

impl ScanBudget {
    const fn new(left: u32) -> Self {
        Self {
            left,
            tick_open: false,
            tick_scans: 0,
            peak_scans: 0,
        }
    }

    /// Pay for one fresh scan, or refuse it when the tick has spent its share.
    ///
    /// Work that belongs to a tick always pays. A player action or a capability call pays
    /// only while a tick is running: outside one it is one scan for one action, not
    /// per-tick work.
    fn charge(&mut self, intent: Budget) -> bool {
        if intent == Budget::Action && !self.tick_open {
            return true;
        }
        let Some(left) = self.left.checked_sub(1) else {
            return false;
        };
        self.left = left;
        true
    }

    /// Whether one fresh scan may run, counted when it does. The count is separate from
    /// [`Self::charge`] on purpose: it must report the work a tick really did, so that a
    /// scan the budget let through unbilled would still show up.
    fn allow_scan(&mut self, intent: Budget) -> bool {
        if !self.charge(intent) {
            return false;
        }
        if self.tick_open {
            self.tick_scans += 1;
        }
        true
    }

    /// A level tick opened: it starts with the full share.
    fn begin_tick(&mut self) {
        self.left = SCANS_PER_TICK;
        self.tick_open = true;
        self.tick_scans = 0;
    }

    /// The level tick ended, including its `level_tick_post` work: from here a capability
    /// call is a one-off again, and the next tick starts with the full share.
    fn end_tick(&mut self) {
        self.peak_scans = self.peak_scans.max(self.tick_scans);
        self.tick_scans = 0;
        self.tick_open = false;
        self.left = SCANS_PER_TICK;
    }

    /// A new world: no tick is running, the share is whole and no tick has been counted.
    fn reset(&mut self) {
        *self = Self::new(SCANS_PER_TICK);
    }

    /// The busiest tick since the last read, cleared by it.
    fn take_peak(&mut self) -> u32 {
        std::mem::take(&mut self.peak_scans)
    }
}

/// Whether one fresh scan may run, counted against the tick when one is open.
fn allow_scan(intent: Budget) -> bool {
    SCAN_BUDGET
        .lock()
        .expect("the scan budget lock is held for one scan")
        .allow_scan(intent)
}

/// The busiest level tick's fresh-scan count since the last read, cleared by it. A
/// diagnostic like `t=`: a paused machine and a broken one answer a capability call the
/// same way, so this is how a game run proves the per-tick bound.
fn take_scan_peak() -> u32 {
    SCAN_BUDGET
        .lock()
        .expect("the scan budget lock is held for one read")
        .take_peak()
}

/// A proven formation and the region the proof depends on.
#[derive(Clone)]
struct Proven<T> {
    answer: Result<T, String>,
    /// The level the scan ran in, as `block_changed` names it.
    level: LevelKey,
    /// The block box the answer depends on, inclusive: the machine's own box when it
    /// formed, or the largest box a machine at that controller could occupy.
    min: [i32; 3],
    max: [i32; 3],
}

impl<T> Proven<T> {
    /// Whether a change at `pos` in `level` can make this answer wrong.
    fn depends_on(&self, level: LevelKey, pos: [i32; 3], part_change: bool) -> bool {
        self.level == level && change_affects(self.min, self.max, pos, part_change)
    }
}

/// The proven formations of the current topology, by controller position.
struct Formations<K, T> {
    entries: HashMap<K, Proven<T>>,
}

impl<K, T> Formations<K, T> {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<K: std::hash::Hash + Eq, T: Clone> Formations<K, T> {
    fn get(&self, key: &K) -> Option<&Result<T, String>> {
        self.entries.get(key).map(|proven| &proven.answer)
    }

    /// Record `proven` for every controller of the component it was scanned from.
    fn insert(&mut self, keys: impl IntoIterator<Item = K>, proven: &Proven<T>) {
        for key in keys {
            self.entries.insert(key, proven.clone());
        }
    }

    /// Drop the proofs a change at `pos` in `level` can make wrong.
    fn invalidate(&mut self, level: LevelKey, pos: [i32; 3], part_change: bool) {
        self.entries
            .retain(|_, proven| !proven.depends_on(level, pos, part_change));
    }
}

/// Whether a committed block change at `pos` can alter a machine whose proof covers
/// the box `min..=max`. Inside the box it can break or fill the shell or the
/// interior; in the one-block ring only a machine part matters, because that is what
/// can join the connected component and grow the box. Further out cannot touch it.
fn change_affects(min: [i32; 3], max: [i32; 3], pos: [i32; 3], part_change: bool) -> bool {
    let inside = |axis: usize| min[axis] <= pos[axis] && pos[axis] <= max[axis];
    if (0..3).all(inside) {
        return true;
    }
    part_change && (0..3).all(|axis| min[axis] - 1 <= pos[axis] && pos[axis] <= max[axis] + 1)
}

/// A scanned answer with the box it depends on and every controller it answers for.
type Scanned<T> = (T, [i32; 3], [i32; 3], Vec<[i32; 3]>);

/// The reactor formation at this controller, from the cache when the topology is
/// unchanged. A fresh scan is charged to this tick's budget.
fn cached_scan(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Result<Formed, String> {
    cached(&FORMED, host, level, pos, Budget::Tick, REACTOR_REACH, scan)
}

/// The same for a player's click, a command or a capability call: one scan, because
/// it is a single user action rather than per-tick work.
fn read_scan(host: &dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Result<Formed, String> {
    cached(
        &FORMED,
        host,
        level,
        pos,
        Budget::Action,
        REACTOR_REACH,
        scan,
    )
}

/// The energizer formation at this controller, charged to the tick's budget.
fn energizer_cached_scan(
    host: &dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
) -> Result<EnergizerFormed, String> {
    cached(
        &ENERGIZER_FORMED,
        host,
        level,
        pos,
        Budget::Tick,
        ENERGIZER_MAX_SIDE - 1,
        scan_energizer,
    )
}

/// The energizer formation for a click, a command or a capability call.
fn energizer_read_scan(
    host: &dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
) -> Result<EnergizerFormed, String> {
    cached(
        &ENERGIZER_FORMED,
        host,
        level,
        pos,
        Budget::Action,
        ENERGIZER_MAX_SIDE - 1,
        scan_energizer,
    )
}

/// The cached answer for one controller, or a fresh scan that every controller of the
/// same component then shares.
///
/// `unformed_reach` is how far from the controller an unformed group may still reach:
/// a failed scan has no box, so that region is what can make it form.
fn cached<T: Clone>(
    cache: &Proofs<T>,
    host: &dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    budget: Budget,
    unformed_reach: i32,
    scan: impl FnOnce(&dyn ServerHost, LevelRef, [i32; 3]) -> Result<Scanned<T>, String>,
) -> Result<T, String> {
    let key = (level, pos);
    {
        let guard = cache
            .lock()
            .expect("the formation cache lock is held for one read");
        if let Some(hit) = guard.as_ref().and_then(|cache| cache.get(&key)) {
            return hit.clone();
        }
    }
    if !allow_scan(budget) {
        return Err(UNPROVEN.to_owned());
    }
    let (answer, min, max, controllers) = match scan(host, level, pos) {
        Ok((answer, min, max, controllers)) => (Ok(answer), min, max, controllers),
        Err(reason) => {
            let low = [
                pos[0] - unformed_reach,
                pos[1] - unformed_reach,
                pos[2] - unformed_reach,
            ];
            let high = [
                pos[0] + unformed_reach,
                pos[1] + unformed_reach,
                pos[2] + unformed_reach,
            ];
            (Err(reason), low, high, vec![pos])
        }
    };
    if let Some(level_key) = level
        .dimension()
        .id(host)
        .and_then(|id| LevelKey::parse(&id).ok())
    {
        let proven = Proven {
            answer: answer.clone(),
            level: level_key,
            min,
            max,
        };
        cache
            .lock()
            .expect("the formation cache lock is held for one write")
            .get_or_insert_with(Formations::new)
            .insert(
                controllers.iter().map(|controller| (level, *controller)),
                &proven,
            );
    }
    // A level the game cannot name cannot be invalidated precisely; it is not cached,
    // so the next tick asks again rather than trusting a proof nothing drops.
    answer
}

/// Drop the proofs one level's committed block change can make wrong.
fn drop_proofs<T: Clone>(cache: &Proofs<T>, level: LevelKey, pos: [i32; 3], part_change: bool) {
    let mut guard = cache
        .lock()
        .expect("the formation cache lock is held for one invalidation");
    if let Some(cache) = guard.as_mut() {
        cache.invalidate(level, pos, part_change);
    }
}

/// Whether a block state is one of this mod's machine parts.
fn machine_part(level: &dyn LevelHost, state: BlockState) -> bool {
    level
        .state_block(state)
        .is_some_and(|name| part_of(name).is_some() || energizer_part_of(name).is_some())
}

/// Upstream's `isMachineWhole` for the energizer over the world: the pure checker walks the
/// blocks this reads, and the design comes from what it found. Every controller of the box
/// is named, so one scan answers all of them.
fn scan_energizer(
    host: &dyn ServerHost,
    level: LevelRef,
    origin: [i32; 3],
) -> Result<Scanned<EnergizerFormed>, String> {
    let shape = check_energizer_shape(|pos| energizer_part_at(host, level, pos), origin)
        .map_err(|error| error.to_string())?;
    let size = [
        (shape.max[0] - shape.min[0] + 1) as u32,
        (shape.max[1] - shape.min[1] + 1) as u32,
        (shape.max[2] - shape.min[2] + 1) as u32,
    ];
    let design = EnergizerDesign::try_new(size, shape.parts, shape.cells)
        .map_err(|error| error.to_string())?;
    let (min, max) = (shape.min, shape.max);
    let controllers = shape.controllers.clone();
    Ok((EnergizerFormed { design, shape }, min, max, controllers))
}

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
        // The energizer: the controller owns the buffer, and the power port answers the energy
        // capability with that buffer, as upstream's passive port does. A custom handler rather
        // than `block_energy` because the amount is a u64 in the controller, the direction is
        // per-port, and an unformed port must expose nothing.
        let energizer_port = r.block_key(ENERGIZER_PORT);
        let energizer_controller = r
            .block_entity_type(ENERGIZER_CONTROLLER)
            .blocks(&[ENERGIZER_CONTROLLER])
            .build();
        let energizer_port_kind = r
            .block_entity_type(ENERGIZER_PORT)
            .blocks(&[ENERGIZER_PORT])
            .build();
        r.block_entity_part(&energizer_controller, EnergizerControllerPart)
            .expect("a block entity type this mod declares takes its own part");
        r.block_entity_part(&energizer_port_kind, EnergizerPortPart)
            .expect("a block entity type this mod declares takes its own part");
        r.block_capability(&ENERGY, &energizer_port, |_host, at, _state, entity| {
            entity.map(|_| Box::new(PortEnergy { at }) as Box<dyn EnergyHandler>)
        });
        // The controller also answers the energy capability as a receive face, as upstream
        // exposes the reactor's buffer on every part: another machine's port can push FE in.
        let controller_block = r.block_key(CONTROLLER);
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
        r.block_capability(&ENERGY, &controller_block, |_host, at, _state, entity| {
            entity.map(|_| Box::new(ReactorInput { at }) as Box<dyn EnergyHandler>)
        });
    }

    /// `/bigreactors status <pos>`: a read-only view of a reactor, for players and the harness
    /// (upstream has no command).
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(Command::op(STATUS_USAGE, STATUS_LEVEL));
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

    /// A committed block change drops only the formations it can touch: one inside a
    /// machine's box can break the shell or fill the interior, and a machine part in the
    /// one-block ring can join the component and grow the box. An unrelated break leaves
    /// every proof standing, so the controllers do not all scan again on the next tick.
    fn block_changed(&self, level: &dyn LevelHost, event: &mut BlockChangedEvent) {
        let part_change = machine_part(level, event.old) || machine_part(level, event.new);
        let pos = [event.pos.x, event.pos.y, event.pos.z];
        let key = event.dimension.key();
        drop_proofs(&FORMED, key, pos, part_change);
        drop_proofs(&ENERGIZER_FORMED, key, pos, part_change);
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
        let Some(block) = block_at(host, level, pos) else {
            return Flow::Continue;
        };
        let message = match block.as_str() {
            CONTROLLER => match read_scan(host, level, pos) {
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
            },
            ENERGIZER_CONTROLLER => match energizer_read_scan(host, level, pos) {
                Ok(_) => {
                    let Some(entity) = host.block_entity(level, pos) else {
                        return Flow::Continue;
                    };
                    let mut energizer = energizer_state(&entity.data(host).unwrap_or_default());
                    let active = energizer.toggle();
                    let mut patch = Compound::new();
                    store_energizer(&mut patch, &energizer);
                    if entity.merge_data(host, patch).is_err() {
                        return Flow::Continue;
                    }
                    if active {
                        "Energizer is on".to_owned()
                    } else {
                        "Energizer is off".to_owned()
                    }
                }
                Err(reason) => format!("The energizer did not form: {reason}"),
            },
            ENERGIZER_PORT => {
                let Some(entity) = host.block_entity(level, pos) else {
                    return Flow::Continue;
                };
                let direction = read_direction(&entity.data(host).unwrap_or_default()).toggled();
                let mut patch = Compound::new();
                write_direction(&mut patch, direction);
                if entity.merge_data(host, patch).is_err() {
                    return Flow::Continue;
                }
                format!("Power port is {direction}")
            }
            _ => return Flow::Continue,
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

    /// A world opened or closed: the queued tap positions belong to that world, and so
    /// does every proven formation (its key names a level of the old one).
    fn world_lifecycle(&self, _host: &mut dyn ServerHost, _event: &mut WorldLifecycleEvent<'_>) {
        TAP_QUEUE
            .lock()
            .expect("the tap queue lock is held for one clear")
            .clear();
        FORMED
            .lock()
            .expect("the formation cache lock is held for one clear")
            .take();
        ENERGIZER_FORMED
            .lock()
            .expect("the formation cache lock is held for one clear")
            .take();
        SCAN_BUDGET
            .lock()
            .expect("the scan budget lock is held for one reset")
            .reset();
    }

    /// Before a level's tick: a new tick starts with the full scan share.
    fn level_tick_pre(&self, host: HostSide, _event: &mut LevelTickPreEvent) {
        let HostSide::Server(_host) = host else {
            return;
        };
        SCAN_BUDGET
            .lock()
            .expect("the scan budget lock is held for one refill")
            .begin_tick();
    }

    /// After a level's tick: every queued tap pushes its FE into the blocks beside it. A tap is
    /// out of the world in its own tick, which is why this happens here. This is still the
    /// tick's work (a push asks the neighbour for its `capacity`), so the share stays open
    /// until the pushes are done.
    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        let level = event.level;
        push_queued_taps(host, level);
        SCAN_BUDGET
            .lock()
            .expect("the scan budget lock is held for one tick close")
            .end_tick();
    }
}

/// Push every queued tap of `level` into its neighbours, and keep the taps of other
/// levels for their own `level_tick_post`.
fn push_queued_taps(host: &mut dyn ServerHost, level: LevelRef) {
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
        let Ok(formed) = cached_scan(host, level, pos) else {
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
// The energizer: the controller and its power port
// ---------------------------------------------------------------------------

/// The energizer's machine (`EnergizerControllerEntity`).
struct EnergizerControllerPart;
/// The energizer's FE face (`EnergizerPowerPortEntity`).
struct EnergizerPortPart;

#[mod_api::horizon_part]
impl BlockPart for EnergizerControllerPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for EnergizerControllerPart {
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

    /// One server tick: formation scan, capacity, the ports' controller reference, and the
    /// output push.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        energizer_tick(host, event);
    }
}

#[mod_api::horizon_part]
impl BlockPart for EnergizerPortPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for EnergizerPortPart {
    /// The port only holds its direction and the controller it belongs to; the controller
    /// reads and writes the machine's buffer through the capability.
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }
}

/// One server tick of a formed energizer.
fn energizer_tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    let Some(level) = in_place_level(host) else {
        return;
    };
    let pos = event.pos;
    let Ok(formed) = energizer_cached_scan(host, level, pos) else {
        // A port must stop answering the capability the moment its machine is no longer whole.
        write_bool(&mut event.entity.nbt, "formed", false);
        return;
    };
    let capacity = formed.design.capacity();
    let mut energizer = energizer_state(&event.entity.nbt);
    if energizer.energy > capacity {
        energizer.energy = capacity;
    }
    for port in &formed.shape.ports {
        announce_port(host, level, *port, pos);
        push_from_port(host, level, *port, &mut energizer);
    }
    store_energizer(&mut event.entity.nbt, &energizer);
    write_bool(&mut event.entity.nbt, "formed", true);
    write_u64(&mut event.entity.nbt, "capacity", capacity);
}

/// Tell a port which controller it belongs to, so its capability can reach the buffer.
fn announce_port(host: &mut dyn ServerHost, level: LevelRef, port: [i32; 3], controller: [i32; 3]) {
    let Some(entity) = host.block_entity(level, port) else {
        return;
    };
    let data = entity.data(host).unwrap_or_default();
    if read_int(&data, "cx") == controller[0]
        && read_int(&data, "cy") == controller[1]
        && read_int(&data, "cz") == controller[2]
    {
        return;
    }
    let mut patch = Compound::new();
    write_int(&mut patch, "cx", controller[0]);
    write_int(&mut patch, "cy", controller[1]);
    write_int(&mut patch, "cz", controller[2]);
    let _ = entity.merge_data(host, patch);
}

/// An output port pushes the machine's FE into the first neighbour that takes it. Upstream
/// 2.4.9's passive port only lets a neighbour pull; this is the same energy in the same
/// direction, and the push 2.4.27 later ships as its active port (see `notes.md`).
fn push_from_port(
    host: &mut dyn ServerHost,
    level: LevelRef,
    port: [i32; 3],
    energizer: &mut Energizer,
) {
    if !energizer.active || energizer.energy == 0 {
        return;
    }
    let Some(entity) = host.block_entity(level, port) else {
        return;
    };
    let data = entity.data(host).unwrap_or_default();
    if read_direction(&data) != IoDirection::Output {
        return;
    }
    for side in Direction::ALL {
        let available = u32::try_from(energizer.energy).unwrap_or(u32::MAX);
        if available == 0 {
            return;
        }
        let [dx, dy, dz] = side.normal();
        let neighbour = [port[0] + dx, port[1] + dy, port[2] + dz];
        // Never push into this machine's own parts: that would only move the buffer into
        // itself (and the controller's own write would race the insert).
        if energizer_part_at(host, level, neighbour).is_some() {
            continue;
        }
        let Some(target) = level.capability(host, &ENERGY, neighbour, Some(side.opposite())) else {
            continue;
        };
        let room = target.capacity(host).saturating_sub(target.amount(host));
        let amount = available.min(room);
        if amount == 0 {
            continue;
        }
        // One transaction: the neighbour's insert is the only write; a refused insert moves
        // nothing out of the machine.
        let moved = host
            .transaction(|host, tx| -> Result<u32, ()> {
                let Some(target) =
                    level.capability(host, &ENERGY, neighbour, Some(side.opposite()))
                else {
                    return Ok(0);
                };
                Ok(target.insert(host, tx, amount))
            })
            .unwrap_or(0);
        if moved > 0 {
            energizer.energy = energizer.energy.saturating_sub(u64::from(moved));
            return;
        }
    }
}

/// The energizer's energy face: a view of the controller's buffer, as upstream's passive
/// power port is. `iodir` gates it (input accepts, output gives) and `formed` + `active` gate
/// it too, so an unformed or switched-off machine refuses both ways.
struct PortEnergy {
    at: BlockSide,
}

impl PortEnergy {
    fn direction(&self, host: &mut dyn ServerHost) -> IoDirection {
        host.block_entity(self.at.level, self.at.pos)
            .and_then(|entity| entity.data(host))
            .map_or(IoDirection::Input, |data| read_direction(&data))
    }
}

impl EnergyHandler for PortEnergy {
    fn amount(&self, host: &mut dyn ServerHost) -> u32 {
        let Some((_, data)) = port_controller(host, self.at.level, self.at.pos) else {
            return 0;
        };
        u32::try_from(energizer_state(&data).energy).unwrap_or(u32::MAX)
    }

    fn capacity(&self, host: &mut dyn ServerHost) -> u32 {
        let Some((_, data)) = port_controller(host, self.at.level, self.at.pos) else {
            return 0;
        };
        u32::try_from(read_u64(&data, "capacity")).unwrap_or(u32::MAX)
    }

    fn insert(&self, host: &mut dyn ServerHost, _tx: &Transaction, amount: u32) -> u32 {
        if amount == 0 || self.direction(host) != IoDirection::Input {
            return 0;
        }
        let Some((controller, data)) = port_controller(host, self.at.level, self.at.pos) else {
            return 0;
        };
        let mut energizer = energizer_state(&data);
        let taken = energizer.insert_within(read_u64(&data, "capacity"), u64::from(amount));
        if taken == 0 {
            return 0;
        }
        let mut patch = Compound::new();
        write_u64(&mut patch, "energy", energizer.energy);
        if controller.merge_data(host, patch).is_err() {
            return 0;
        }
        u32::try_from(taken).unwrap_or(u32::MAX)
    }

    fn extract(&self, host: &mut dyn ServerHost, _tx: &Transaction, amount: u32) -> u32 {
        if amount == 0 || self.direction(host) != IoDirection::Output {
            return 0;
        }
        let Some((controller, data)) = port_controller(host, self.at.level, self.at.pos) else {
            return 0;
        };
        let mut energizer = energizer_state(&data);
        let given = energizer.extract_within(u64::from(amount));
        if given == 0 {
            return 0;
        }
        let mut patch = Compound::new();
        write_u64(&mut patch, "energy", energizer.energy);
        if controller.merge_data(host, patch).is_err() {
            return 0;
        }
        u32::try_from(given).unwrap_or(u32::MAX)
    }
}

/// The reactor's controller as a receive face: another machine's port can push FE into the
/// reactor's buffer, as upstream exposes the buffer on every part. Extraction stays on the
/// active tap, so the machine's FE face is unchanged.
struct ReactorInput {
    at: BlockSide,
}

impl ReactorInput {
    fn state(&self, host: &mut dyn ServerHost) -> Option<(BlockEntityRef, Compound)> {
        let entity = host.block_entity(self.at.level, self.at.pos)?;
        let data = entity.data(host)?;
        Some((entity, data))
    }
}

impl EnergyHandler for ReactorInput {
    fn amount(&self, host: &mut dyn ServerHost) -> u32 {
        let Some((_, data)) = self.state(host) else {
            return 0;
        };
        u32::try_from(reactor_state(&data).energy.max(0.0).round() as u64).unwrap_or(u32::MAX)
    }

    fn capacity(&self, host: &mut dyn ServerHost) -> u32 {
        let Ok(formed) = read_scan(host, self.at.level, self.at.pos) else {
            return 0;
        };
        u32::try_from(formed.design.energy_capacity()).unwrap_or(u32::MAX)
    }

    fn insert(&self, host: &mut dyn ServerHost, _tx: &Transaction, amount: u32) -> u32 {
        if amount == 0 {
            return 0;
        }
        let Some((entity, data)) = self.state(host) else {
            return 0;
        };
        let mut reactor = reactor_state(&data);
        if !reactor.active {
            return 0;
        }
        let Ok(formed) = read_scan(host, self.at.level, self.at.pos) else {
            return 0;
        };
        let room = f64::from(formed.design.energy_capacity()) - reactor.energy;
        let taken = f64::from(amount).min(room).max(0.0);
        if taken < 1.0 {
            return 0;
        }
        reactor.energy += taken;
        let mut patch = Compound::new();
        store_reactor(&mut patch, &reactor);
        if entity.merge_data(host, patch).is_err() {
            return 0;
        }
        taken as u32
    }

    fn extract(&self, _host: &mut dyn ServerHost, _tx: &Transaction, _amount: u32) -> u32 {
        0
    }
}

/// The controller a formed energizer's port belongs to, with its saved compound.
fn port_controller(
    host: &dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
) -> Option<(BlockEntityRef, Compound)> {
    let port = host.block_entity(level, pos)?;
    let data = port.data(host)?;
    let controller = [
        read_int(&data, "cx"),
        read_int(&data, "cy"),
        read_int(&data, "cz"),
    ];
    if block_at(host, level, controller).as_deref() != Some(ENERGIZER_CONTROLLER) {
        return None;
    }
    let entity = host.block_entity(level, controller)?;
    let data = entity.data(host)?;
    if !read_bool(&data, "formed") {
        return None;
    }
    Some((entity, data))
}

// ---------------------------------------------------------------------------
// Formation and the reactor's saved state
// ---------------------------------------------------------------------------

/// A formed reactor: its simulated design and where its parts are.
#[derive(Clone)]
struct Formed {
    design: Design,
    shape: Shape,
}

/// Upstream's `isMachineWhole` over the world: the pure checker walks the blocks this reads, and
/// the design comes from what it found. Every controller of the box is named, so one scan
/// answers all of them.
fn scan(
    host: &dyn ServerHost,
    level: LevelRef,
    origin: [i32; 3],
) -> Result<Scanned<Formed>, String> {
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
    let (min, max) = (shape.min, shape.max);
    let controllers = controllers_in(host, level, min, max);
    Ok((Formed { design, shape }, min, max, controllers))
}

/// Every controller of a formed reactor. The scan's box is at most 5³, so this walk is a
/// handful of reads, and it is what lets one scan answer a machine's second controller.
fn controllers_in(
    host: &dyn ServerHost,
    level: LevelRef,
    min: [i32; 3],
    max: [i32; 3],
) -> Vec<[i32; 3]> {
    let mut controllers = Vec::new();
    for x in min[0]..=max[0] {
        for y in min[1]..=max[1] {
            for z in min[2]..=max[2] {
                if part_at(host, level, [x, y, z]) == Some(Part::Controller) {
                    controllers.push([x, y, z]);
                }
            }
        }
    }
    controllers
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

/// The energizer part a block id is, if it is one of the energizer's.
fn energizer_part_of(name: &str) -> Option<EnergizerPart> {
    Some(match name {
        ENERGIZER_CASING => EnergizerPart::Casing,
        ENERGIZER_CELL => EnergizerPart::Cell,
        ENERGIZER_CONTROLLER => EnergizerPart::Controller,
        ENERGIZER_PORT => EnergizerPart::PowerPort,
        _ => return None,
    })
}

fn energizer_part_at(
    host: &dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
) -> Option<EnergizerPart> {
    energizer_part_of(&block_at(host, level, pos)?)
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

fn energizer_state(nbt: &Compound) -> Energizer {
    let mut energizer = Energizer::new();
    energizer.active = read_bool(nbt, "active");
    energizer.energy = read_u64(nbt, "energy");
    energizer
}

fn store_energizer(nbt: &mut Compound, energizer: &Energizer) {
    write_bool(nbt, "active", energizer.active);
    write_u64(nbt, "energy", energizer.energy);
}

fn read_direction(nbt: &Compound) -> IoDirection {
    match nbt.get(&"iodir".into()) {
        Some(Tag::Byte(1)) => IoDirection::Output,
        _ => IoDirection::Input,
    }
}

fn write_direction(nbt: &mut Compound, direction: IoDirection) {
    nbt.insert(
        "iodir".into(),
        Tag::Byte(i8::from(direction == IoDirection::Output)),
    );
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
        // Never push into this reactor's own parts: the controller is a receive face too, and
        // a tap beside it would otherwise hand its buffer straight back.
        if part_at(host, level, neighbour).is_some() {
            continue;
        }
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
    let entity = host.block_entity(level, pos);
    let data = entity
        .and_then(|entity| entity.data(host))
        .unwrap_or_default();
    // The busiest level tick's fresh-scan count since the last read (see
    // [`take_scan_peak`]): a harness field, read exactly once per status command.
    let scans = take_scan_peak();
    match block.as_str() {
        CONTROLLER => {
            let reactor = reactor_state(&data);
            match read_scan(host, level, pos) {
                Ok(formed) => format!(
                    "Reactor at {},{},{}: formed {}x{}x{}, {} fuel rods, {} control rods, {}/{} FE, fuel {}, waste {}, heat {:.1}, fuel heat {:.1}, active {}, t={}, scans={}",
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
                    time,
                    scans
                ),
                Err(reason) => format!(
                    "Reactor at {},{},{}: not formed ({reason}), fuel {}, active {}, t={}, scans={}",
                    pos[0], pos[1], pos[2], reactor.fuel, reactor.active, time, scans
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
            entity
                .and_then(|entity| entity.item_slot(host, 0).ok().flatten())
                .map_or_else(
                    || "empty".to_owned(),
                    |stack| format!("{} x{}", stack.item().id(host), stack.count())
                ),
            time
        ),
        ENERGIZER_CONTROLLER => {
            let energizer = energizer_state(&data);
            let capacity = read_u64(&data, "capacity");
            match energizer_read_scan(host, level, pos) {
                Ok(formed) => format!(
                    "Energizer at {},{},{}: formed {}x{}x{}, {} cells, {}/{} FE, active {}, t={}, scans={}",
                    pos[0],
                    pos[1],
                    pos[2],
                    formed.shape.max[0] - formed.shape.min[0] + 1,
                    formed.shape.max[1] - formed.shape.min[1] + 1,
                    formed.shape.max[2] - formed.shape.min[2] + 1,
                    formed.design.cells,
                    energizer.energy,
                    capacity,
                    energizer.active,
                    time,
                    scans
                ),
                Err(reason) => format!(
                    "Energizer at {},{},{}: not formed ({reason}), active {}, t={}, scans={}",
                    pos[0], pos[1], pos[2], energizer.active, time, scans
                ),
            }
        }
        ENERGIZER_PORT => {
            let direction = read_direction(&data);
            match port_controller(host, level, pos) {
                Some((_, controller)) => format!(
                    "Energizer Power Port at {},{},{}: {}, {}/{} FE, t={}",
                    pos[0],
                    pos[1],
                    pos[2],
                    direction,
                    energizer_state(&controller).energy,
                    read_u64(&controller, "capacity"),
                    time
                ),
                None => format!(
                    "Energizer Power Port at {},{},{}: not formed, {}, t={}",
                    pos[0], pos[1], pos[2], direction, time
                ),
            }
        }
        ENERGIZER_CELL | ENERGIZER_CASING => match energizer_read_scan(host, level, pos) {
            Ok(formed) => format!(
                "Energizer part at {},{},{}: formed {}x{}x{}, {} cells, t={}",
                pos[0],
                pos[1],
                pos[2],
                formed.shape.max[0] - formed.shape.min[0] + 1,
                formed.shape.max[1] - formed.shape.min[1] + 1,
                formed.shape.max[2] - formed.shape.min[2] + 1,
                formed.design.cells,
                time
            ),
            Err(reason) => format!(
                "Energizer part at {},{},{}: not formed ({reason}), t={}",
                pos[0], pos[1], pos[2], time
            ),
        },
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

/// An unsigned 64-bit amount, as upstream's `WideAmount` integer part, kept in a `Long`.
fn read_u64(nbt: &Compound, key: &str) -> u64 {
    match nbt.get(&key.into()) {
        Some(Tag::Long(value)) => *value as u64,
        _ => 0,
    }
}

fn write_u64(nbt: &mut Compound, key: &str, value: u64) {
    nbt.insert(key.into(), Tag::Long(value as i64));
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
    fn the_status_readout_is_operator_only() {
        // The readout names a loaded access port's contents at coordinates a player
        // chooses: level 0 would let any player read another player's machine.
        assert_eq!(STATUS_LEVEL, 2);
        let command = Command::op(STATUS_USAGE, STATUS_LEVEL);
        assert_eq!(command.level, 2);
        assert!(command.usage.starts_with("bigreactors status "));
    }

    #[test]
    fn a_formation_is_answered_from_the_cache_until_it_is_cleared() {
        // A formed machine's proof survives a change that cannot touch it; a change
        // inside its box drops it, so the next tick re-scans.
        let mut cache: Formations<u8, &'static str> = Formations::new();
        cache.insert([1], &proven_formation(Ok("formed")));
        assert_eq!(cache.get(&1).cloned(), Some(Ok("formed")));
        cache.invalidate(LevelKey::OVERWORLD, [11, 21, 31], false);
        assert_eq!(cache.get(&1).cloned(), None);
    }

    /// A proof of a machine whose box is `[10,20,30]..=[12,22,32]`.
    fn proven_formation(answer: Result<&'static str, String>) -> Proven<&'static str> {
        Proven {
            answer,
            level: LevelKey::OVERWORLD,
            min: [10, 20, 30],
            max: [12, 22, 32],
        }
    }

    #[test]
    fn an_unrelated_block_change_does_not_force_a_rescan() {
        // The tick cost this cache exists to avoid: a break four blocks north of the
        // shell cannot change the machine, so its proof must stand and no controller
        // scans on the next tick.
        let mut cache: Formations<u8, &'static str> = Formations::new();
        cache.insert([1], &proven_formation(Ok("formed")));
        cache.invalidate(LevelKey::OVERWORLD, [10, 20, 37], false);
        assert_eq!(
            cache.get(&1).cloned(),
            Some(Ok("formed")),
            "an unrelated break dropped the proof and would force a rescan"
        );
    }

    #[test]
    fn a_part_in_the_ring_is_re_proven() {
        // A casing placed beside the shell joins the connected component and can grow
        // the box past the limit, so it must not be answered from the old proof.
        let mut cache: Formations<u8, &'static str> = Formations::new();
        cache.insert([1], &proven_formation(Ok("formed")));
        cache.invalidate(LevelKey::OVERWORLD, [13, 21, 31], true);
        assert_eq!(
            cache.get(&1).cloned(),
            None,
            "a part in the ring was ignored"
        );
        // A non-part in the same ring cannot join the component.
        cache.insert([1], &proven_formation(Ok("formed")));
        cache.invalidate(LevelKey::OVERWORLD, [13, 21, 31], false);
        assert_eq!(cache.get(&1).cloned(), Some(Ok("formed")));
    }

    #[test]
    fn a_change_in_another_level_leaves_the_proof_alone() {
        let mut cache: Formations<u8, &'static str> = Formations::new();
        cache.insert([1], &proven_formation(Ok("formed")));
        cache.invalidate(LevelKey::NETHER, [11, 21, 31], false);
        assert_eq!(
            cache.get(&1).cloned(),
            Some(Ok("formed")),
            "a Nether write dropped an Overworld proof"
        );
    }

    #[test]
    fn one_scan_answers_every_controller_of_a_component() {
        let mut cache: Formations<u8, &'static str> = Formations::new();
        cache.insert([1, 2], &proven_formation(Ok("formed")));
        assert_eq!(cache.get(&1).cloned(), Some(Ok("formed")));
        assert_eq!(
            cache.get(&2).cloned(),
            Some(Ok("formed")),
            "the machine's second controller scanned again"
        );
    }

    #[test]
    fn the_tick_budget_bounds_fresh_scans() {
        let mut budget = ScanBudget::new(2);
        assert!(budget.charge(Budget::Tick), "the first scan");
        assert!(budget.charge(Budget::Tick), "the second");
        assert!(
            !budget.charge(Budget::Tick),
            "the third waits for the next tick"
        );
        budget.begin_tick();
        assert!(budget.charge(Budget::Tick), "a new tick pays again");
    }

    /// The reviewer's bypass (0.2.4): an energizer tick asks each adjacent machine for its
    /// `capacity`, and every answer is a fresh formation scan. Inside a tick those scans
    /// are the tick's own work and must come out of the same share, or the cap bounds one
    /// scan and not the tick.
    #[test]
    fn a_capability_scan_inside_a_tick_is_charged_to_the_tick() {
        let mut budget = ScanBudget::new(SCANS_PER_TICK);
        budget.begin_tick();
        let machines = SCANS_PER_TICK + 2;
        let mut scanned = 0;
        let mut paused = 0;
        for _ in 0..machines {
            if budget.allow_scan(Budget::Action) {
                scanned += 1;
            } else {
                paused += 1;
            }
        }
        assert_eq!(
            scanned, SCANS_PER_TICK as usize,
            "a capability scan inside a tick must come out of the tick's share"
        );
        assert_eq!(
            paused, 2,
            "the machines past the share must pause, not scan"
        );
        budget.end_tick();
        assert_eq!(
            budget.take_peak(),
            SCANS_PER_TICK,
            "the readout must report the tick's fresh scans, which can never pass the share"
        );
        budget.begin_tick();
        assert!(
            budget.allow_scan(Budget::Action),
            "a machine paused by the budget recovers on a later tick"
        );
    }

    /// The readout the game leg asserts on: every fresh scan a tick runs is counted, and a
    /// read clears the high-water mark. A scan outside a tick is not tick work.
    #[test]
    fn the_readout_reports_the_busiest_tick_and_clears_it() {
        let mut budget = ScanBudget::new(SCANS_PER_TICK);
        budget.begin_tick();
        assert!(budget.allow_scan(Budget::Tick));
        assert!(budget.allow_scan(Budget::Action));
        budget.end_tick();
        assert_eq!(
            budget.take_peak(),
            2,
            "the readout must count every fresh scan of the tick"
        );
        assert_eq!(budget.take_peak(), 0, "a read clears the high-water mark");
        assert!(
            budget.allow_scan(Budget::Action),
            "a capability call outside a tick is still one free scan"
        );
        budget.end_tick();
        assert_eq!(
            budget.take_peak(),
            0,
            "a scan outside a tick is not the tick's work"
        );
    }

    /// The control: the same capability call outside a tick is one scan for one action, so
    /// it is never refused. If the fix only worked because every scan became budgeted,
    /// this fails.
    #[test]
    fn a_capability_scan_outside_a_tick_is_one_free_scan() {
        let mut budget = ScanBudget::new(SCANS_PER_TICK);
        for machine in 0..(SCANS_PER_TICK * 2) {
            assert!(
                budget.charge(Budget::Action),
                "capability call {machine} outside a tick was charged to a tick"
            );
        }
    }

    #[test]
    fn the_ring_only_matters_when_a_machine_part_moves() {
        let (min, max) = ([10, 20, 30], [12, 22, 32]);
        assert!(
            change_affects(min, max, [11, 21, 31], false),
            "a change inside the box always matters"
        );
        assert!(
            change_affects(min, max, [12, 22, 32], false),
            "a change on the shell matters"
        );
        assert!(
            change_affects(min, max, [13, 21, 31], true),
            "a part in the ring can join the component"
        );
        assert!(
            !change_affects(min, max, [13, 21, 31], false),
            "a non-part in the ring cannot"
        );
        assert!(
            !change_affects(min, max, [14, 21, 31], true),
            "a part two blocks out cannot reach the box"
        );
        assert!(
            !change_affects(min, max, [10, 20, 37], false),
            "a break four blocks away cannot"
        );
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
    fn the_energizer_blocks_are_the_four_parts() {
        assert_eq!(
            energizer_part_of(ENERGIZER_CASING),
            Some(EnergizerPart::Casing)
        );
        assert_eq!(energizer_part_of(ENERGIZER_CELL), Some(EnergizerPart::Cell));
        assert_eq!(
            energizer_part_of(ENERGIZER_CONTROLLER),
            Some(EnergizerPart::Controller)
        );
        assert_eq!(
            energizer_part_of(ENERGIZER_PORT),
            Some(EnergizerPart::PowerPort)
        );
        assert_eq!(energizer_part_of("minecraft:iron_block"), None);
        assert_eq!(
            energizer_part_of("bigreactors:basic_reactorcasing"),
            None,
            "the reactor's casing is not an energizer part"
        );
    }

    #[test]
    fn a_stored_energizer_reads_back() {
        let mut nbt = Compound::new();
        let mut energizer = Energizer::new();
        energizer.active = true;
        energizer.energy = 683_212_743_470_724;
        store_energizer(&mut nbt, &energizer);
        assert_eq!(energizer_state(&nbt), energizer);
        assert_eq!(read_u64(&nbt, "energy"), 683_212_743_470_724);
    }

    #[test]
    fn a_power_port_direction_reads_back() {
        let mut nbt = Compound::new();
        assert_eq!(read_direction(&nbt), IoDirection::Input);
        write_direction(&mut nbt, IoDirection::Output);
        assert_eq!(read_direction(&nbt), IoDirection::Output);
        write_direction(&mut nbt, IoDirection::Input);
        assert_eq!(read_direction(&nbt), IoDirection::Input);
    }

    #[test]
    fn an_empty_compound_is_an_off_empty_energizer() {
        let energizer = energizer_state(&Compound::new());
        assert!(!energizer.active);
        assert_eq!(energizer.energy, 0);
    }
}
