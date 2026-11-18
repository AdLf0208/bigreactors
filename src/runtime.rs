// Pure model of a basic Extreme Reactors reactor (passive cooling).
//
// The numbers are the upstream mod's own: `FINDINGS.md` records the file each
// one was read from, and the reference trajectory the tests pin was produced by
// a second, independent transcription of those formulas.

/// FE produced per unit of radiation (upstream `EnergyConversion`).
pub const ENERGY_PER_RADIATION_UNIT: f64 = 10.0;
/// FE per °C per block of volume (upstream `EnergyConversion`).
pub const ENERGY_PER_CENTIGRADE_PER_UNIT_VOLUME: f64 = 10.0;
/// A cold reactor sits at ambient (upstream `IHeatEntity.AMBIENT_HEAT`).
pub const AMBIENT_HEAT: f64 = 20.0;
/// Upstream `Moderator.AIR`: absorption, heat efficiency, moderation, conductivity.
pub const MODERATOR_AIR: Moderator = Moderator {
    absorption: 0.1,
    heat_efficiency: 0.25,
    moderation: 1.1,
    conductivity: 0.05,
};
/// Upstream `IHeatEntity.CONDUCTIVITY_IRON`, the reactor→coolant coefficient.
pub const CONDUCTIVITY_IRON: f64 = 0.6;
/// Upstream `REACTOR_HEAT_LOSS_CONDUCTIVITY`, per external surface block.
pub const REACTOR_HEAT_LOSS_CONDUCTIVITY: f64 = 0.001;
/// Passive cooling transfers this share of the heat differential per tick.
pub const PASSIVE_COOLING_TRANSFER_EFFICIENCY: f64 = 0.2;
/// ... and turns it into power at this efficiency.
pub const PASSIVE_COOLING_POWER_EFFICIENCY: f64 = 0.5;
/// The basic variant's energy generation efficiency.
pub const BASIC_GENERATION_EFFICIENCY: f64 = 0.8;
/// The basic variant's buffer per part.
pub const BASIC_PART_ENERGY_CAPACITY: i32 = 10_000;
/// The basic variant's maximum extraction rate (the power tap's throughput).
pub const BASIC_MAX_EXTRACTION_RATE: i32 = 50_000;
/// Fuel per fuel rod, in mB (4 ingots).
pub const FUEL_CAPACITY_PER_FUEL_ROD: i32 = 4_000;
/// One fuel item is this many mB (`STANDARD_SOLID_REACTANT_AMOUNT`).
pub const FUEL_PER_INGOT: i32 = 1_000;
/// Yellorium's `FuelProperties.DEFAULT`.
pub const FISSION_EVENTS_PER_FUEL_UNIT: f64 = 0.01;
pub const FUEL_UNITS_PER_FISSION_EVENT: f64 = 0.0007;
/// Yellorium's reaction reactivity (`Reaction.STANDARD_REACTIVITY`).
pub const STANDARD_REACTIVITY: f64 = 1.05;
/// Blocks a radiation packet crosses before it stops (`RadiationPacket` ttl).
pub const IRRADIATION_TTL: u8 = 4;

/// One moderator cell's properties.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moderator {
    pub absorption: f64,
    pub heat_efficiency: f64,
    pub moderation: f64,
    pub conductivity: f64,
}

/// Why a reactor shape cannot be simulated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesignError {
    /// Upstream `multiblock.validation.reactor.too_few_rods`.
    NoControlRods,
    /// A reactor with no fuel rods can only ever make nothing.
    NoFuelRods,
}

impl std::fmt::Display for DesignError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DesignError::NoControlRods => formatter.write_str("no control rods"),
            DesignError::NoFuelRods => formatter.write_str("no fuel rods"),
        }
    }
}

impl std::error::Error for DesignError {}

/// A formed reactor's geometry: what the simulation needs, nothing more.
#[derive(Clone, Debug, PartialEq)]
pub struct Design {
    /// Every part in the multiblock, the shell included (upstream `getPartsCount`).
    pub parts: u32,
    pub fuel_rods: u32,
    pub control_rods: u32,
    /// Blocks strictly inside the shell.
    pub interior_volume: u32,
    /// Moderation cells the radiation crosses, per horizontal direction.
    pub air_steps: [u8; 4],
    fuel_to_reactor: f64,
    reactor_to_coolant: f64,
    heat_loss: f64,
}

impl Design {
    /// A box of `size` (outer lengths) holding `fuel_rods` rods, each with a
    /// control rod above it, and the moderator reach the scan measured.
    ///
    /// # Errors
    /// `NoControlRods` / `NoFuelRods` when the shape cannot react at all.
    pub fn try_new(
        size: [u32; 3],
        parts: u32,
        fuel_rods: u32,
        control_rods: u32,
        air_steps: [u8; 4],
    ) -> Result<Self, DesignError> {
        if control_rods == 0 {
            return Err(DesignError::NoControlRods);
        }
        if fuel_rods == 0 {
            return Err(DesignError::NoFuelRods);
        }
        let inner = |length: u32| length.saturating_sub(2);
        let (x, y, z) = (inner(size[0]), inner(size[1]), inner(size[2]));
        if x == 0 || y == 0 || z == 0 || parts == 0 {
            return Err(DesignError::NoFuelRods);
        }
        let fuel_to_reactor = f64::from(fuel_rods) * f64::from(4u8) * MODERATOR_AIR.conductivity;
        let reactor_to_coolant = CONDUCTIVITY_IRON * f64::from(2 * (x * y + x * z + y * z));
        let heat_loss = REACTOR_HEAT_LOSS_CONDUCTIVITY
            * f64::from(2 * (size[0] * size[1] + size[0] * size[2] + size[1] * size[2]));
        Ok(Self {
            parts,
            fuel_rods,
            control_rods,
            interior_volume: x * y * z,
            air_steps,
            fuel_to_reactor,
            reactor_to_coolant,
            heat_loss,
        })
    }

    /// Upstream: `partEnergyCapacity × partsCount × (powerMultiplier × efficiency)`.
    #[must_use]
    pub fn energy_capacity(&self) -> i32 {
        let raw = f64::from(BASIC_PART_ENERGY_CAPACITY)
            * f64::from(self.parts)
            * BASIC_GENERATION_EFFICIENCY;
        raw as i32
    }

    /// Upstream: 4 ingots per fuel rod.
    #[must_use]
    pub fn fuel_capacity(&self) -> i32 {
        FUEL_CAPACITY_PER_FUEL_ROD * self.fuel_rods as i32
    }
}

/// What one tick produced.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tick {
    /// FE added to the buffer this tick (before the capacity clamp).
    pub energy_generated: f64,
    /// Fuel converted to waste this tick, in mB.
    pub fuel_used: f64,
}

/// A reactor's saved state: what upstream keeps in the controller's NBT.
#[derive(Clone, Debug, PartialEq)]
pub struct Reactor {
    pub active: bool,
    pub fuel: i32,
    pub waste: i32,
    pub fuel_heat: f64,
    pub reactor_heat: f64,
    /// Stored FE; upstream's buffer holds the fraction too.
    pub energy: f64,
    /// Upstream's `_fertility` (`ReactorLogic`), saved with the machine.
    pub fertility: f64,
    /// Fuel fraction not yet turned into waste (upstream's `_radiationFuelUsage`).
    pub accumulator: f64,
}

impl Default for Reactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Reactor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            active: false,
            fuel: 0,
            waste: 0,
            fuel_heat: 0.0,
            reactor_heat: AMBIENT_HEAT,
            energy: 0.0,
            fertility: 1.0,
            accumulator: 0.0,
        }
    }

    /// Adds fuel, bounded by the tank and by what the buffer holds.
    ///
    /// Returns the mB accepted.
    pub fn insert_fuel(&mut self, design: &Design, millibuckets: i32) -> i32 {
        if millibuckets <= 0 {
            return 0;
        }
        let room = (design.fuel_capacity() - self.fuel).max(0);
        let accepted = millibuckets.min(room);
        self.fuel += accepted;
        accepted
    }

    /// Upstream's Solid Access Port: one ingot is 1000 mB.
    pub fn insert_ingots(&mut self, design: &Design, ingots: i32) -> i32 {
        let asked = ingots.max(0) * FUEL_PER_INGOT;
        self.insert_fuel(design, asked) / FUEL_PER_INGOT
    }

    /// Switches the reactor on or off; returns the new state.
    pub fn toggle(&mut self) -> bool {
        self.active = !self.active;
        self.active
    }

    /// One server tick of the reactor, exactly upstream's `ReactorLogic.update`
    /// order: irradiate, decay, refuel, fuel→reactor heat, reactor→coolant,
    /// passive loss.
    pub fn tick(&mut self, design: &Design) -> Tick {
        let mut out = Tick::default();
        if self.active {
            self.irradiate(design, &mut out);
        }
        self.fertility = (self.fertility - (self.fertility / 20.0).max(0.1)).max(0.0);
        if self.active && self.accumulator >= 1.0 {
            let wanted = self.accumulator as i32;
            let converted = wanted.min(self.fuel);
            if converted > 0 {
                self.fuel -= converted;
                self.waste += converted;
                self.accumulator = (self.accumulator - f64::from(converted)).max(0.0);
            } else {
                self.accumulator = 1.0_f64.min(self.accumulator);
            }
            out.fuel_used = f64::from(converted);
        }
        self.transfer_fuel_heat(design);
        self.generate(design, &mut out);
        self.passive_loss(design);
        out
    }

    /// Upstream `performIrradiationFrom` for one fuel rod in a vertical pile.
    fn irradiate(&mut self, design: &Design, out: &mut Tick) {
        if self.fuel <= 0 {
            return;
        }
        let radiation_penalty = (-15.0 * (-0.0025 * self.fuel_heat).exp()).exp();
        let base_fuel = f64::from(self.fuel) + f64::from(self.waste / 100);
        let raw_intensity = base_fuel * FISSION_EVENTS_PER_FUEL_UNIT;
        let mut scaled = raw_intensity.powf(STANDARD_REACTIVITY);
        let rods = f64::from(design.control_rods);
        scaled = (scaled / rods).powf(STANDARD_REACTIVITY) * rods;
        let effective = scaled * (1.0 + (-0.95 * (-10.0 * (-0.0012 * self.fuel_heat).exp()).exp()));
        let hardness = 0.2 + 0.8 * radiation_penalty;
        let fertility_modifier = if self.fertility <= 1.0 {
            1.0
        } else {
            self.fertility.log10() + 1.0
        };
        out.fuel_used = 0.0;
        self.accumulator += FUEL_UNITS_PER_FISSION_EVENT * raw_intensity / fertility_modifier;

        // The rod's own share, then the moderator cells along the plane.
        self.fuel_heat += ENERGY_PER_RADIATION_UNIT * effective
            / (f64::from(design.fuel_rods) * ENERGY_PER_CENTIGRADE_PER_UNIT_VOLUME);
        let mut environment = 0.0;
        for steps in design.air_steps {
            let mut intensity = effective * 0.25;
            let mut hard = hardness;
            for _ in 0..steps.min(4) {
                let absorbed = intensity * MODERATOR_AIR.absorption * (1.0 - hard);
                intensity = (intensity - absorbed).max(0.0);
                hard /= MODERATOR_AIR.moderation;
                environment += MODERATOR_AIR.heat_efficiency * absorbed * ENERGY_PER_RADIATION_UNIT;
            }
        }
        self.reactor_heat += environment
            / (f64::from(design.interior_volume) * ENERGY_PER_CENTIGRADE_PER_UNIT_VOLUME);
    }

    /// Upstream `transferHeatBetweenFuelAndReactor`.
    fn transfer_fuel_heat(&mut self, design: &Design) {
        let difference = self.fuel_heat - self.reactor_heat;
        if difference <= 0.01 {
            return;
        }
        let energy = difference * design.fuel_to_reactor;
        let fuel_volume = f64::from(design.fuel_rods) * ENERGY_PER_CENTIGRADE_PER_UNIT_VOLUME;
        let reactor_volume =
            f64::from(design.interior_volume) * ENERGY_PER_CENTIGRADE_PER_UNIT_VOLUME;
        self.fuel_heat = (self.fuel_heat * fuel_volume - energy) / fuel_volume;
        self.reactor_heat = (self.reactor_heat * reactor_volume + energy) / reactor_volume;
    }

    /// Upstream `transferHeatBetweenReactorAndCoolant`, passive mode.
    fn generate(&mut self, design: &Design, out: &mut Tick) {
        let difference = self.reactor_heat - AMBIENT_HEAT;
        if difference <= 0.01 {
            return;
        }
        let energy = difference * design.reactor_to_coolant * PASSIVE_COOLING_TRANSFER_EFFICIENCY;
        let generated = energy * PASSIVE_COOLING_POWER_EFFICIENCY * BASIC_GENERATION_EFFICIENCY;
        let capacity = f64::from(design.energy_capacity());
        self.energy = (self.energy + generated).min(capacity);
        out.energy_generated = generated;
        let reactor_volume =
            f64::from(design.interior_volume) * ENERGY_PER_CENTIGRADE_PER_UNIT_VOLUME;
        self.reactor_heat = (self.reactor_heat * reactor_volume - energy) / reactor_volume;
    }

    /// Upstream `performPassiveHeatLoss`.
    fn passive_loss(&mut self, design: &Design) {
        let difference = self.reactor_heat - AMBIENT_HEAT;
        if difference <= 0.000_001 {
            return;
        }
        let lost = 1.0_f64.max(difference * design.heat_loss);
        let reactor_volume =
            f64::from(design.interior_volume) * ENERGY_PER_CENTIGRADE_PER_UNIT_VOLUME;
        let energy = (self.reactor_heat * reactor_volume - lost).max(0.0);
        self.reactor_heat = energy / reactor_volume;
    }
}

/// What one cell of a reactor holds, as the glue maps block ids to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Casing,
    Controller,
    FuelRod,
    ControlRod,
    AccessPort,
    PowerTap,
}

/// A formed reactor's block layout, straight from the world scan.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub min: [i32; 3],
    pub max: [i32; 3],
    /// Every connected part, the shell and the rods (upstream's `getPartsCount`).
    pub parts: u32,
    pub fuel_rods: u32,
    pub control_rods: u32,
    pub ports: Vec<[i32; 3]>,
    pub taps: Vec<[i32; 3]>,
    /// Moderation cells between the rods and the outer faces, up to the
    /// irradiation reach, tightest rod first.
    pub air_steps: [u8; 4],
}

/// Why a shape is not a reactor. The text is what a player is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeError {
    TooFewParts,
    NotABox,
    HoleInTheShell,
    InsideIsNotEmpty,
    NoController,
    NoControlRods,
    NoFuelRods,
    ControlRodNotOnAFace,
    ControlRodsOnDifferentFaces,
    AssemblyDoesNotEndOnCasing,
    RodWithoutAssembly,
    LooseFuelRod,
}

impl std::fmt::Display for ShapeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            ShapeError::TooFewParts => "too few parts",
            ShapeError::NotABox => "a basic reactor must be 3 to 5 blocks on each side",
            ShapeError::HoleInTheShell => "the shell has a hole in it",
            ShapeError::InsideIsNotEmpty => "the inside must be air, fuel rods or control rods",
            ShapeError::NoController => "a reactor needs a controller",
            ShapeError::NoControlRods => "a reactor needs at least one control rod",
            ShapeError::NoFuelRods => "a reactor needs at least one fuel rod",
            ShapeError::ControlRodNotOnAFace => "a control rod must sit on an outer face",
            ShapeError::ControlRodsOnDifferentFaces => "all control rods must be on the same face",
            ShapeError::AssemblyDoesNotEndOnCasing => "a fuel rod assembly must end on a casing",
            ShapeError::RodWithoutAssembly => "a control rod has no fuel rod under it",
            ShapeError::LooseFuelRod => "every fuel rod must sit under a control rod",
        })
    }
}

impl std::error::Error for ShapeError {}

/// Cells a flood fill may walk before it calls a shape absurd.
const MAX_SHAPE_CELLS: usize = 1024;
/// The most a reactor's side may be (basic variant, `ReactorVariant.Basic`).
const MAX_SIDE: i32 = 5;
/// The least (`minParts`, a 3×3×3 box).
const MIN_SIDE: i32 = 3;

/// Upstream's `isMachineWhole` over a block reader: a flood fill from the
/// controller, then one box, a closed shell, control rods on one face and a
/// fuel-rod column under every one of them.
///
/// # Errors
/// The first check that fails, as `ShapeError`.
pub fn check_shape(
    at: impl Fn([i32; 3]) -> Option<Part>,
    origin: [i32; 3],
) -> Result<Shape, ShapeError> {
    let mut cells: Vec<[i32; 3]> = vec![origin];
    let mut seen: std::collections::HashSet<[i32; 3]> = std::collections::HashSet::from([origin]);
    let mut index = 0;
    while index < cells.len() {
        let pos = cells[index];
        index += 1;
        for side in 0..6 {
            let next = step(pos, side);
            if seen.contains(&next) || at(next).is_none() {
                continue;
            }
            seen.insert(next);
            cells.push(next);
            if cells.len() > MAX_SHAPE_CELLS {
                return Err(ShapeError::TooFewParts);
            }
        }
    }
    if cells.len() < 7 {
        return Err(ShapeError::TooFewParts);
    }
    let mut min = origin;
    let mut max = origin;
    for pos in &cells {
        for axis in 0..3 {
            min[axis] = min[axis].min(pos[axis]);
            max[axis] = max[axis].max(pos[axis]);
        }
    }
    let size = [
        max[0] - min[0] + 1,
        max[1] - min[1] + 1,
        max[2] - min[2] + 1,
    ];
    if size
        .iter()
        .any(|length| !(MIN_SIDE..=MAX_SIDE).contains(length))
    {
        return Err(ShapeError::NotABox);
    }

    let mut control_rods = Vec::new();
    let mut fuel_rods = Vec::new();
    let mut ports = Vec::new();
    let mut taps = Vec::new();
    let mut controllers = 0u32;
    for x in min[0]..=max[0] {
        for y in min[1]..=max[1] {
            for z in min[2]..=max[2] {
                let pos = [x, y, z];
                // Strictly inside on all three axes: air or a rod. Everything
                // else (faces, edges, corners) is the shell and must be a part.
                let inside = x > min[0]
                    && x < max[0]
                    && y > min[1]
                    && y < max[1]
                    && z > min[2]
                    && z < max[2];
                if inside {
                    match at(pos) {
                        None | Some(Part::FuelRod) | Some(Part::ControlRod) => {}
                        Some(_) => return Err(ShapeError::InsideIsNotEmpty),
                    }
                } else if at(pos).is_none() {
                    return Err(ShapeError::HoleInTheShell);
                }
                match at(pos) {
                    Some(Part::FuelRod) => fuel_rods.push(pos),
                    Some(Part::ControlRod) => control_rods.push(pos),
                    Some(Part::AccessPort) => ports.push(pos),
                    Some(Part::PowerTap) => taps.push(pos),
                    Some(Part::Controller) => controllers += 1,
                    _ => {}
                }
            }
        }
    }
    if controllers == 0 {
        return Err(ShapeError::NoController);
    }
    if control_rods.is_empty() {
        return Err(ShapeError::NoControlRods);
    }
    if fuel_rods.is_empty() {
        return Err(ShapeError::NoFuelRods);
    }
    let faces = control_rods
        .iter()
        .map(|pos| shell_face(*pos, min, max).ok_or(ShapeError::ControlRodNotOnAFace))
        .collect::<Result<Vec<_>, ShapeError>>()?;
    let first = faces.first().copied().ok_or(ShapeError::NoControlRods)?;
    if faces.iter().any(|face| *face != first) {
        return Err(ShapeError::ControlRodNotOnAFace);
    }
    if faces.iter().any(|face| *face != first) {
        return Err(ShapeError::ControlRodsOnDifferentFaces);
    }
    let inward = inward_step(first);
    let mut rods_in_assemblies = 0u32;
    for rod in &control_rods {
        let mut cursor = *rod;
        let mut found = 0u32;
        loop {
            cursor = step(cursor, inward);
            match at(cursor) {
                Some(Part::FuelRod) => found += 1,
                Some(Part::Casing) => break,
                _ => return Err(ShapeError::AssemblyDoesNotEndOnCasing),
            }
        }
        if found == 0 {
            return Err(ShapeError::RodWithoutAssembly);
        }
        rods_in_assemblies += found;
    }
    if rods_in_assemblies != fuel_rods.len() as u32 {
        return Err(ShapeError::LooseFuelRod);
    }

    let mut air_steps = [IRRADIATION_TTL; 4];
    for rod in &fuel_rods {
        let reach = [
            (max[0] - rod[0]).clamp(0, i32::from(IRRADIATION_TTL)),
            (rod[0] - min[0]).clamp(0, i32::from(IRRADIATION_TTL)),
            (max[2] - rod[2]).clamp(0, i32::from(IRRADIATION_TTL)),
            (rod[2] - min[2]).clamp(0, i32::from(IRRADIATION_TTL)),
        ];
        for (step, value) in air_steps.iter_mut().zip(reach) {
            *step = (*step).min(value as u8);
        }
    }
    Ok(Shape {
        min,
        max,
        parts: cells.len() as u32,
        fuel_rods: fuel_rods.len() as u32,
        control_rods: control_rods.len() as u32,
        ports,
        taps,
        air_steps,
    })
}

/// `Direction` 0..6 in `horizon_core`'s order: down, up, north, south, west, east.
fn step(pos: [i32; 3], side: usize) -> [i32; 3] {
    let delta = [
        [0, -1, 0],
        [0, 1, 0],
        [0, 0, -1],
        [0, 0, 1],
        [-1, 0, 0],
        [1, 0, 0],
    ][side];
    [pos[0] + delta[0], pos[1] + delta[1], pos[2] + delta[2]]
}

/// The opposite of a `step` index.
fn inward_step(side: usize) -> usize {
    [1, 0, 3, 2, 5, 4][side]
}

/// The face of the box a shell position sits on, in `step` order, if exactly one.
fn shell_face(pos: [i32; 3], min: [i32; 3], max: [i32; 3]) -> Option<usize> {
    let mut faces = Vec::new();
    if pos[1] == min[1] {
        faces.push(0);
    }
    if pos[1] == max[1] {
        faces.push(1);
    }
    if pos[2] == min[2] {
        faces.push(2);
    }
    if pos[2] == max[2] {
        faces.push(3);
    }
    if pos[0] == min[0] {
        faces.push(4);
    }
    if pos[0] == max[0] {
        faces.push(5);
    }
    if faces.len() == 1 { faces.pop() } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The book's first reactor: 3×3×3, one fuel rod in the 1×1×1 interior,
    /// one control rod on the top face.
    fn first_reactor() -> Design {
        Design::try_new([3, 3, 3], 27, 1, 1, [1, 1, 1, 1]).expect("the book's first reactor forms")
    }

    fn run(reactor: &mut Reactor, design: &Design, ticks: u32) {
        for _ in 0..ticks {
            reactor.tick(design);
        }
    }

    type Cells = std::collections::HashMap<[i32; 3], Part>;

    /// The book's first reactor at (0..2)³: casing shell, controller on the west
    /// face, access port east, power tap north, one fuel rod inside, one control
    /// rod on the top face above it.
    fn book_reactor() -> Cells {
        let mut cells = Cells::new();
        for x in 0..=2 {
            for y in 0..=2 {
                for z in 0..=2 {
                    let shell = x == 0 || x == 2 || y == 0 || y == 2 || z == 0 || z == 2;
                    cells.insert([x, y, z], if shell { Part::Casing } else { Part::FuelRod });
                }
            }
        }
        cells.insert([0, 1, 1], Part::Controller);
        cells.insert([2, 1, 1], Part::AccessPort);
        cells.insert([1, 1, 0], Part::PowerTap);
        cells.insert([1, 2, 1], Part::ControlRod);
        cells
    }

    fn shape_of(cells: &Cells) -> Result<Shape, ShapeError> {
        let cells = cells.clone();
        check_shape(move |pos| cells.get(&pos).copied(), [0, 1, 1])
    }

    /// 5×5×5 with a 3×3×3 air interior: casing shell, a three-rod column under
    /// the top-face control rod, the controller on the west face.
    fn big_reactor() -> Cells {
        let mut cells = Cells::new();
        for x in 0..=4 {
            for y in 0..=4 {
                for z in 0..=4 {
                    let shell = x == 0 || x == 4 || y == 0 || y == 4 || z == 0 || z == 4;
                    if shell {
                        cells.insert([x, y, z], Part::Casing);
                    }
                }
            }
        }
        cells.insert([0, 2, 2], Part::Controller);
        for y in 1..=3 {
            cells.insert([2, y, 2], Part::FuelRod);
        }
        cells.insert([2, 4, 2], Part::ControlRod);
        cells
    }

    #[test]
    fn the_book_reactor_forms() {
        let cells = book_reactor();
        let shape = shape_of(&cells).expect("the book's first reactor forms");
        assert_eq!(shape.parts, 27, "26 shell blocks and the fuel rod");
        assert_eq!((shape.fuel_rods, shape.control_rods), (1, 1));
        assert_eq!(shape.air_steps, [1, 1, 1, 1]);
        assert_eq!(shape.ports, vec![[2, 1, 1]]);
        assert_eq!(shape.taps, vec![[1, 1, 0]]);
        assert_eq!((shape.min, shape.max), ([0, 0, 0], [2, 2, 2]));
    }

    #[test]
    fn a_reactor_with_air_inside_forms() {
        // The interior air is not a part, so the box holds 98 shell blocks and
        // three rods: 101 parts, not 125. A 5×5×5 must still form.
        let shape = shape_of(&big_reactor()).expect("a 5x5x5 with an air interior forms");
        assert_eq!(shape.parts, 101);
        assert_eq!(shape.fuel_rods, 3);
        assert_eq!(shape.air_steps, [2, 2, 2, 2]);
        let design = Design::try_new([5, 5, 5], 101, 3, 1, shape.air_steps).expect("forms");
        assert_eq!(
            design.energy_capacity(),
            101 * 10_000 * 8 / 10,
            "partEnergyCapacity x parts x 0.8"
        );
    }

    #[test]
    fn a_hole_in_the_shell_is_refused() {
        let mut cells = book_reactor();
        cells.remove(&[0, 1, 0]);
        assert_eq!(shape_of(&cells), Err(ShapeError::HoleInTheShell));
    }

    #[test]
    fn a_box_with_no_fuel_rod_is_refused() {
        let mut cells = book_reactor();
        cells.remove(&[1, 1, 1]);
        assert_eq!(shape_of(&cells), Err(ShapeError::NoFuelRods));
    }

    #[test]
    fn a_control_rod_off_the_face_is_refused() {
        let mut cells = big_reactor();
        cells.insert([3, 2, 2], Part::ControlRod);
        assert_eq!(shape_of(&cells), Err(ShapeError::ControlRodNotOnAFace));
    }

    #[test]
    fn a_box_without_a_controller_is_refused() {
        let mut cells = book_reactor();
        cells.insert([0, 1, 1], Part::Casing);
        assert_eq!(shape_of(&cells), Err(ShapeError::NoController));
    }

    #[test]
    fn a_loose_fuel_rod_is_refused() {
        let mut cells = big_reactor();
        cells.insert([1, 2, 2], Part::FuelRod);
        assert_eq!(shape_of(&cells), Err(ShapeError::LooseFuelRod));
    }

    #[test]
    fn a_rod_column_that_stops_in_the_air_is_refused() {
        let mut cells = big_reactor();
        cells.remove(&[2, 1, 2]);
        assert_eq!(
            shape_of(&cells),
            Err(ShapeError::AssemblyDoesNotEndOnCasing)
        );
    }

    #[test]
    fn a_new_reactor_is_off_and_cold() {
        let design = first_reactor();
        let reactor = Reactor::new();
        assert!(!reactor.active);
        assert_eq!(reactor.fuel, 0);
        assert_eq!(reactor.waste, 0);
        assert!(reactor.fuel_heat.abs() < f64::EPSILON);
        assert!((reactor.reactor_heat - AMBIENT_HEAT).abs() < f64::EPSILON);
        assert_eq!((reactor.energy as i32), 0);
        assert_eq!(design.energy_capacity(), 216_000);
        assert_eq!(design.fuel_capacity(), 4_000);
    }

    #[test]
    fn one_ingot_is_a_thousand_millibuckets_and_the_rod_caps_the_tank() {
        let design = first_reactor();
        let mut reactor = Reactor::new();
        assert_eq!(reactor.insert_ingots(&design, 1), 1);
        assert_eq!(reactor.fuel, 1_000);
        assert_eq!(reactor.insert_ingots(&design, 9), 3, "4 ingots per rod");
        assert_eq!(reactor.fuel, 4_000);
        assert_eq!(reactor.insert_ingots(&design, 1), 0);
        assert_eq!(reactor.insert_ingots(&design, -4), 0);
    }

    #[test]
    fn an_inactive_reactor_burns_no_fuel_and_makes_no_energy() {
        let design = first_reactor();
        let mut reactor = Reactor::new();
        reactor.insert_ingots(&design, 4);
        run(&mut reactor, &design, 100);
        assert_eq!(reactor.fuel, 4_000);
        assert_eq!(reactor.waste, 0);
        assert_eq!((reactor.energy as i32), 0);
    }

    #[test]
    fn a_reactor_with_no_fuel_makes_no_energy() {
        let design = first_reactor();
        let mut reactor = Reactor::new();
        reactor.toggle();
        run(&mut reactor, &design, 100);
        assert_eq!((reactor.energy as i32), 0);
        assert_eq!(reactor.waste, 0);
        assert!((reactor.reactor_heat - AMBIENT_HEAT).abs() < f64::EPSILON);
    }

    #[test]
    fn the_first_tick_makes_half_a_fe() {
        let design = first_reactor();
        let mut reactor = Reactor::new();
        reactor.insert_ingots(&design, 4);
        reactor.toggle();
        let tick = reactor.tick(&design);
        assert!(
            (tick.energy_generated - 0.550_594).abs() < 1e-4,
            "first tick generated {}",
            tick.energy_generated
        );
        assert!((reactor.energy - 0.550_594).abs() < 1e-4);
        assert!((reactor.fuel_heat - 57.634_212).abs() < 1e-3);
        assert!((reactor.reactor_heat - 21.674_137).abs() < 1e-3);
    }

    #[test]
    fn the_reference_trajectory_matches_the_upstream_formulas() {
        let design = first_reactor();
        let mut reactor = Reactor::new();
        reactor.insert_ingots(&design, 4);
        reactor.toggle();
        run(&mut reactor, &design, 40);
        assert!(
            (reactor.energy - 1_557.820_795).abs() < 0.5,
            "40 ticks: {} FE",
            reactor.energy
        );
        assert_eq!((reactor.fuel, reactor.waste), (3_999, 1));
        run(&mut reactor, &design, 60);
        assert!(
            (reactor.energy - 8_026.375_571).abs() < 0.5,
            "100 ticks: {} FE",
            reactor.energy
        );
        assert_eq!((reactor.fuel, reactor.waste), (3_998, 2));
    }

    #[test]
    fn a_steady_reactor_makes_about_127_fe_a_tick() {
        let design = first_reactor();
        let mut reactor = Reactor::new();
        reactor.insert_ingots(&design, 4);
        reactor.toggle();
        run(&mut reactor, &design, 300);
        let before = reactor.energy;
        run(&mut reactor, &design, 100);
        let average = (reactor.energy - before) / 100.0;
        assert!(
            (average - 126.796_188).abs() < 0.05,
            "steady state averaged {average} FE/t"
        );
        assert_eq!(
            (reactor.fuel, reactor.waste),
            (3_989, 11),
            "0.028 mB/t of fuel"
        );
        assert!(
            (reactor.reactor_heat - 426.187_937).abs() < 0.5,
            "reactor heat {}",
            reactor.reactor_heat
        );
    }

    #[test]
    fn switching_off_stops_the_fuel_but_stored_heat_still_makes_energy() {
        let design = first_reactor();
        let mut reactor = Reactor::new();
        reactor.insert_ingots(&design, 4);
        reactor.toggle();
        run(&mut reactor, &design, 200);
        let fuel = reactor.fuel;
        let before = reactor.energy;
        reactor.toggle();
        assert!(!reactor.active);
        run(&mut reactor, &design, 20);
        assert_eq!(reactor.fuel, fuel, "an off reactor burns no fuel");
        assert!(
            reactor.energy > before,
            "stored heat kept generating: {} -> {}",
            before,
            reactor.energy
        );
        assert!(reactor.reactor_heat < 426.0, "the heat soaked away");
    }

    #[test]
    fn the_buffer_stops_at_its_capacity() {
        let design = first_reactor();
        let mut reactor = Reactor::new();
        reactor.insert_ingots(&design, 4);
        reactor.toggle();
        run(&mut reactor, &design, 4_000);
        assert_eq!(
            (reactor.energy as i32),
            design.energy_capacity(),
            "the buffer clamps at partEnergyCapacity × parts × efficiency"
        );
    }

    #[test]
    fn a_rod_with_no_moderator_path_makes_less_energy() {
        // Every horizontal cell around the rod is a block entity in upstream:
        // radiation crosses nothing, so only conduction heats the reactor.
        let blocked = Design::try_new([3, 3, 3], 27, 1, 1, [0, 0, 0, 0]).expect("forms");
        let open = first_reactor();
        let mut without = Reactor::new();
        let mut with = Reactor::new();
        without.insert_ingots(&blocked, 4);
        with.insert_ingots(&open, 4);
        without.toggle();
        with.toggle();
        run(&mut without, &blocked, 400);
        run(&mut with, &open, 400);
        assert!(without.fuel_heat > 0.0, "the fuel still heats up");
        assert!(
            without.energy < with.energy,
            "moderated {} FE vs unmoderated {} FE",
            with.energy,
            without.energy
        );
        assert!(
            without.energy > 0.0,
            "conduction alone still makes some power"
        );
    }

    #[test]
    fn a_shape_without_rods_is_refused() {
        assert_eq!(
            Design::try_new([3, 3, 3], 26, 1, 0, [1, 1, 1, 1]),
            Err(DesignError::NoControlRods)
        );
        assert_eq!(
            Design::try_new([3, 3, 3], 26, 0, 1, [1, 1, 1, 1]),
            Err(DesignError::NoFuelRods)
        );
        assert_eq!(
            Design::try_new([2, 3, 3], 18, 1, 1, [1, 1, 1, 1]),
            Err(DesignError::NoFuelRods),
            "a 2-wide box has no interior"
        );
    }

    #[test]
    fn a_bigger_box_makes_more_power_per_rod() {
        let small = first_reactor();
        let big = Design::try_new([5, 5, 5], 99, 1, 1, [2, 2, 2, 2]).expect("forms");
        let mut a = Reactor::new();
        let mut b = Reactor::new();
        a.insert_ingots(&small, 4);
        b.insert_ingots(&big, 4);
        a.toggle();
        b.toggle();
        run(&mut a, &small, 400);
        run(&mut b, &big, 400);
        assert!(
            b.energy > a.energy * 1.05,
            "5x5x5 {} FE vs 3x3x3 {} FE",
            b.energy,
            a.energy
        );
    }
}
