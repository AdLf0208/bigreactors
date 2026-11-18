# Big Reactors (Extreme Reactors 2) — findings

Jar: `ports/_shared/licence-cache/ExtremeReactors2-1.21.1-2.4.28.jar`
(2,518,864 B, SHA-256 `1807c6dbe8458768641170a7eaee6383cab40a8ed4a5da4cf84a873f83da8341`,
sha1 + pack record: `ports/_shared/licence-cache/atm10-modlist.json`).
Licence read from the jar's `META-INF/neoforge.mods.toml`: **MIT** (rank 0) —
`python3 ports/tools/precheck-mod.py bigreactors` prints
`jar licence: MIT (rank 0) from ExtremeReactors2-1.21.1-2.4.28.jar`.

Upstream analysis (authorised by `AGENTS.md` §"Upstream source analysis"): repo
<https://github.com/ZeroNoRyouki/ExtremeReactors2>, tag `1.21.1-2.4.27`
(commit `f6cc65140a8abbfc87137f5c54fc641085f66048`), shallow clone at
`ports/_shared/upstream/bigreactors`. The pack ships 2.4.28 (the tag after that
one); every file read below is unchanged between the two releases for the
behaviour used here (the 2.4.28 diff is recipes/data). The port stays a
clean-room Rust rewrite: only behaviour and numbers were extracted.

## What the jar actually contains (inventory first, not the name)

`unzip -l`: **2,306 entries** — 99 blockstates, 114 item models, 89 block loot
tables, 22 + 18 Patchouli reactor/turbine guide pages.

Families (blockstates, `assets/bigreactors/blockstates/`):

| family | blocks |
|---|---|
| **reactor** (basic + reinforced) | `casing`, `controller`, `controlrod`, `fuelrod`, `glass`, `solidaccessport`, `fluidaccessport`, `powertapfe_active`, `powertapfe_passive`, `chargingportfe`, `redstoneport`, `computerport`, `creativewatergenerator` |
| **turbine** | `casing`, `controller`, `glass`, `rotorbearing`, `rotorblade`, `rotorshaft`, `fluidport_forge_{active,passive}`, `powertapfe_{active,passive}`, `chargingportfe`, `redstoneport`, `computerport`, `creativesteamgenerator` |
| **reprocessor** | `casing`, `controller`, `glass`, `collector`, `fluidinjector`, `wasteinjector`, `outputport`, `powerport` |
| **energizer** | `casing`, `cell`, `controller`, `chargingportfe`, `computerport`, `powerportfe` |
| **fluidizer** | `casing`, `controller`, `glass`, `fluidinjector`, `solidinjector`, `outputport`, `powerport` |
| ores / storage | `yellorite_ore`, `deepslate_yellorite_ore`, `anglesite_ore`, `benitoite_ore`, `yellorium_block`, `blutonium_block`, `cyanite_block`, `graphite_block`, `ludicrite_block`, `magentite_block`, `inanite_block`, `insanite_block`, `ridiculite_block`, `raw_yellorium_block` |
| fluids | `yellorium_fluid`, `cyanite_fluid`, `blutonium_fluid`, `magentite_fluid`, `verderium_fluid`, `rossinite_fluid`, `cryomisi_fluid`, `tangerium_fluid`, `redfrigium_fluid`, `steam` |

So the mod is five multiblock machines plus an ore/metal chain — a `machines`
tier mod, and **the reactor is one of five machines**, not the whole mod. Slice 1
takes the smallest complete reactor (basic variant, passive cooling); the
turbine, reprocessor, energizer, fluidizer, the reinforced variant, fluids, ores
and the guide book are recorded gaps in `notes.md`.

## The reactor's numbers (from the upstream source)

All file references are under
`src/main/java/it/zerono/mods/extremereactors/` at the tag above.

| value | source | used |
|---|---|---|
| `ENERGY_PER_RADIATION_UNIT = 10` | `api/reactor/radiation/EnergyConversion.java` | FE per radiation unit |
| `ENERGY_PER_CENTIGRADE_PER_UNIT_VOLUME = 10` | same | heat ↔ FE (volume × °C × 10) |
| `AMBIENT_HEAT = 20` | `api/reactor/IHeatEntity.java` | cold reactor = 20 °C |
| `CONDUCTIVITY_IRON = 0.6` | same | reactor→coolant coefficient = `0.6 × internal surface` |
| `REACTOR_HEAT_LOSS_CONDUCTIVITY = 0.001` | `multiblock/reactor/MultiblockReactor.java:1586` | passive loss = `0.001 × external surface` |
| `PASSIVE_COOLING_TRANSFER_EFFICIENCY = 0.2`, `PASSIVE_COOLING_POWER_EFFICIENCY = 0.5` | `multiblock/reactor/ReactorLogic.java:471,474` | 20 % of the heat differential per tick, at 50 % power |
| basic variant: `getPartEnergyCapacity = 10_000`, `getEnergyGenerationEfficiency = 0.8`, `getMaxEnergyExtractionRate = 50_000` | `multiblock/reactor/variant/ReactorVariant.java` | buffer, efficiency, tap rate |
| energy buffer capacity = `partEnergyCapacity × partsCount × (powerMultiplier × efficiency)` | `MultiblockReactor.java:836` | 3×3×3 ⇒ 10 000 × 27 × 0.8 = **216 000 FE** |
| `FUEL_CAPACITY_PER_FUEL_ROD = 4 × 1000` | `multiblock/reactor/part/ReactorFuelRodEntity.java:50` | 4 ingots per rod |
| `STANDARD_SOLID_REACTANT_AMOUNT = 1000` (1 item = 1000 mB) | `api/reactor/ReactantMappingsRegistry.java:56` | one yellorium ingot = 1000 mB |
| Yellorium uses `FuelProperties.DEFAULT = (moderation 1.5, absorption 0.5, hardness 1.0, fission 0.01, fuelUnitsPerFission 0.0007)` | `api/reactor/FuelProperties.java`, `gamecontent/ReactorGameData.java:47` | fission maths |
| Yellorium reaction: `STANDARD_REACTIVITY = 1.05`, `STANDARD_FISSIONRATE = 0.01` | `api/reactor/Reaction.java:40-41`, `ReactorGameData.java:89` | reactivity exponent |
| `Moderator.AIR = (absorption 0.1, heatEfficiency 0.25, moderation 1.1, conductivity 0.05)` | `api/reactor/Moderator.java` | what the casing and interior air moderate with |
| irradiation travels `ttl = 4` blocks in each of the 4 directions perpendicular to the rod axis | `ReactorLogic.java:396-410` | radiation reach |
| config multipliers `fuelUsageMultiplier`, `powerProductionMultiplier`, `reactorPowerProductionMultiplier` default **1.0** | `config/General.java`, `config/Reactor.java` | no config file carried; defaults only |

### The tick, as the source has it (`ReactorLogic.update`)

1. **Irradiate** (only while the reactor is *active*) from **one** fuel rod per
   tick, round-robin: `rawRadIntensity = (fuel + waste/100) × 0.01`;
   `scaled = raw^1.05`, then `(scaled / controlRods)^1.05 × controlRods`, times
   `(100 − insertion)/100`; heat penalties
   `exp(-15·exp(-0.0025·fuelHeat))` and `1 − 0.95·exp(-10·exp(-0.0012·fuelHeat))`;
   `fuelUsed = 0.0007 × rawRadIntensity / fertilityModifier`.
   Fuel heat gains `10 × effectiveIntensity / fuelRodCount` °C, and each of the
   4 × 4 cells along the perpendicular plane absorbs
   `intensity × 0.1 × (1 − hardness)` (the AIR moderator), adding
   `0.25 × absorbed × 10` FE of environment heat.
2. **Decay**: `fertility -= max(0.1, fertility/20)`.
3. **Refuel**: the fuel accumulator converts whole mB of fuel into the same
   number of mB of waste.
4. **Fuel → reactor heat**: `energy = (fuelHeat − reactorHeat) × Σ rod transfer
   rate` (0.05 °C/FE per air-facing side).
5. **Reactor → coolant (passive)**: `e = (reactorHeat − 20) × 0.6 × internal
   surface × 0.2`; `generateEnergy(e × 0.5)`; the reactor loses `e`.
6. **Passive loss**: `max(1, (reactorHeat − 20) × 0.001 × external surface)`.

`WideEnergyBuffer.grow` clamps at the capacity.

### Reference trajectory (independent transcription, 3×3×3, one rod)

The upstream formulas above were transcribed a second time into a standalone
script (not the Rust port) to obtain expected values for the tests and the e2e.
3×3×3 basic reactor, one fuel rod in the 1×1×1 interior, one control rod on the
top face, insertion 0, 4000 mB of yellorium, passive cooling:

| tick | fuel heat °C | reactor heat °C | fuel mB | waste mB | FE made this tick |
|---:|---:|---:|---:|---:|---:|
| 1 | 57.63 | 21.55 | 4000 | 0 | 0.51 |
| 5 | 277.29 | 36.06 | 4000 | 0 | 5.01 |
| 10 | 529.65 | 66.25 | 4000 | 0 | 14.43 |
| 20 | 965.30 | 141.11 | 4000 | 0 | 37.79 |
| 40 | 1551.72 | 273.09 | 3999 | 1 | 78.97 |
| 100 | 2033.39 | 410.25 | 3998 | 2 | 121.77 |
| 200 | 2084.90 | 426.18 | 3995 | 5 | 126.74 |
| 400 | 2084.80 | 426.30 | 3989 | 11 | 126.78 |

Steady state: **≈126.8 FE/t** and **≈0.028 mB/t** of fuel (one ingot ≈ 35 700
ticks). That is the number the port must reproduce.

## Behaviours extracted (not code)

- Multiblock formation (`MultiblockReactor.isMachineWhole`): ≥1 control rod, ≥1
  controller, all control rods on the same face, every control rod's assembly
  (scanning inward) is one or more fuel rods ending on a casing, and the total
  fuel-rod count equals the sum of the assemblies (no loose fuel rods). One
  power-system type.
- Basic variant: max 5×5×5 outer (`ReactorVariant.Basic`), controller furthest
  from the centre of its face is the reference.
- Passive cooling is the default operational mode; the Reactor only generates
  while it is switched on in the controller, but stored heat keeps radiating
  after it is switched off.
- Fuel enters through a Solid Access Port (an inventory: ingots in, reactant
  out) and waste is ejected through the same port.
- The Active Forge Energy Power Tap pushes the buffer to whatever is connected,
  at most 50,000 FE/t.

## Where the rewrite differs (recorded again in `notes.md`)

- One fuel assembly, vertical, insertion fixed at 0: the control rod is the
  assembly cap, not a movable insert. Fine insertion, solid moderators and the
  guide book's reactor planner are gaps.
- Air moderation only (the port does not read solid moderator blocks).
- Fuel/waste heat and the energy buffer live in the controller's NBT; upstream
  splits them per part.
- The reactor's own energy capability is declared on the controller; upstream
  declares it on every part.
- No GUI: right-clicking the controller toggles it (upstream's GUI button), and
  `/bigreactors status` is the read-only harness view (upstream has no command).
