# Extreme Reactors → Horizon — slice 1: the basic passive reactor

Ported from **Extreme Reactors 2.4.28** (`ZeroNoRyouki/ExtremeReactors2`), the
exact ATM10 jar `ExtremeReactors2-1.21.1-2.4.28.jar` (2,518,864 B, SHA-256
`1807c6db…da8341`, read from the pack's own record
`ports/_shared/licence-cache/atm10-modlist.json`). Upstream behaviour was read
from tag `1.21.1-2.4.27` (commit `f6cc6514…`) — analysis only, this is a
clean-room Rust rewrite. What was read and every number extracted:
[`FINDINGS.md`](FINDINGS.md).

**Licence: MIT → rank 0 (permissive, publishable).** The campaign still keeps
every port local-only until the maintainer approves publishing
(`ARCHITECTURE.md` §8). Ported now because it is a P0 `content`/`machines` core
mod at licence rank 0 with no unported dependency.

**Class + tier:** `content` · **machines** (`ports/ledger.csv`, mirrored from
`ports/_shared/source-map/atm10-sources.csv`). Approach per the pin's
`docs/modpacks.md` ("Tiers (the API a port needs)"): a **machines** shape — block
entities with a server tick, a declared FE buffer (API 142), a container and
formation logic — not a data-only carry, and not a `deep` port (no renderer, no
custom model, no dimension).

## Slice 1 scope

The smallest reactor the mod expresses end to end, with the jar's own numbers:

| in | what |
|---|---|
| `bigreactors:basic_reactorcasing` | the shell |
| `bigreactors:basic_reactorcontroller` | the machine: fuel, waste, heat, energy, on/off |
| `bigreactors:basic_reactorfuelrod` | the fuel column |
| `bigreactors:basic_reactorcontrolrod` | the assembly cap (top face) |
| `bigreactors:basic_reactorsolidaccessport` | the inventory fuel goes in through |
| `bigreactors:basic_reactorpowertapfe_active` | the FE face (API 142 buffer, 50,000 FE/t) |
| `bigreactors:yellorium_ingot`, `bigreactors:graphite_ingot` | fuel, and the crafting chain's graphite |

A 3×3×3 basic reactor with one fuel rod runs the upstream simulation and reaches
**≈126.8 FE/t** at steady state, burning **≈0.028 mB of fuel per tick** (one
ingot ≈ 35,700 ticks), exactly as the reference trajectory in `FINDINGS.md`
says. The e2e pins 100 ticks at 8,026 FE ± 60, 2 mB of fuel turned into 2 mB of
waste, and a housing at ~410 °C.

Everything else in the mod is a recorded gap (below): the turbine, the
reprocessor, the energizer, the fluidizer, the reinforced variant, fluids, ores
and worldgen, the guide book, and every GUI.

## Behaviour (upstream → this port)

| aspect | upstream | this port |
|---|---|---|
| simulation | `ReactorLogic.update`: irradiate → decay → refuel → fuel→reactor heat → reactor→coolant → passive loss | the same order, in `runtime::Reactor::tick` |
| numbers | `ENERGY_PER_RADIATION_UNIT` 10, `10 FE/°C/volume`, `AMBIENT_HEAT` 20, `Moderator.AIR` (0.1/0.25/1.1), `CONDUCTIVITY_IRON` 0.6, `REACTOR_HEAT_LOSS_CONDUCTIVITY` 0.001, passive 0.2/0.5, basic efficiency 0.8, 10,000 FE per part, 4,000 mB per rod, Yellorium `FuelProperties.DEFAULT`, reactivity 1.05, TTL 4 | the same constants, each named with the file it came from in `FINDINGS.md` |
| formation | `MultiblockReactor.isMachineWhole`: ≥1 control rod, ≥1 controller, control rods on one face, each assembly an unbroken run of fuel rods ending on casing, no loose fuel rods | the same checks on a 3..5 block box, with the reason shown to the player |
| reactor on/off | a button in the controller's GUI | right-click the controller (no GUI in this slice); `/bigreactors status` reads |
| fuel | Solid Access Port: Yellorium ingot = 1,000 mB reactant | the same, through the port's one slot |
| waste | produced 1:1 with the fuel, ejected through the port | produced 1:1 and kept in the tank (no ejection yet: gap) |
| energy | the buffer is on every part, extracted by the active tap at 50,000 FE/t | the controller holds the buffer; the tap's own 50,000 FE buffer is filled from it and pushes out at `level_tick_post` |
| control rods | insertion slider per rod scales the radiation | insertion fixed at 0 (fully retracted): the cap is geometry |
| moderators | air plus 20+ registered solid moderators | air only (the casing and interior air moderate with `Moderator.AIR`) |

## Hooks

| Hook (API) | Helper | What it does |
|---|---|---|
| `block_entity` (15) | `block_entity.rs` | The controller, the access port and the tap get a block entity. |
| `block_entity_ticks` (15) | `ticks.rs` | Those three tick on the server. |
| `block_entity_container` (16) | `container.rs` | The access port's one fuel slot. |
| `block_entity_energy` (142) | `energy.rs` | The tap's 50,000 FE buffer, give-only. |
| `block_entity_tick` (17) | `tick.rs` | Formation scan, fuel intake, one reactor tick, FE to the taps. |
| `level_tick_post` (23) | `level_tick_post.rs` | The taps push FE to their neighbours (`native_move_energy`). |
| `right_click_block` (21) | `right_click.rs` | Switch the reactor on or off; say why a shape did not form. |
| `register_commands` (8) | `register.rs` | `/bigreactors status <x> <y> <z>` |
| `perform_command` (8) | `perform.rs` | The command body (read-only). |
| `world_lifecycle` (v1) | `world.rs` | Drop the queued tap positions per world. |
| `content_root` (AE2 pattern) | `embedded_content.rs` | Writes the embedded `.hmod` and appends it to the dev list. |

11 injections, all stamped at the pin (`native-hooks.json`); no pending hook is
named. Two pin facts shaped the design: API 142's FE amount is the int `energy`
in the block entity's saved compound, and a block entity is out of the world
during its own tick — so the taps move FE at `level_tick_post`, as `powah` does.

## Content

`content/` is carried from the jar by `tools/carry_content.py` (reproducible),
then built by the pin's own `cargo-horizon-mod`. 6 blocks + 2 items, their
blockstates/models/textures (from the jar), 26.3 item definitions, 6 block
recipes + 6 graphite smelting/blasting recipes, 8 loot tables, the two `c:`
ingot tags, a creative tab and `en_us` names. The 1.21.1 shapes are patched to
26.3 ones: loot pool `conditions` → the singular typed `condition`, ingredient
objects → the string form (`#c:ingots/iron`).

## Gaps (slice 2 and later)

- **Turbine, reprocessor, energizer, fluidizer** — four more multiblocks, each
  its own machines slice; the reactor's active (steam) mode goes with the
  turbine.
- **Fluids** — the 10 fluid blocks, coolants, steam, the fluid ports and the
  reactor's active cooling mode.
- **Ores and the metal chain** — yellorite ore (worldgen), the ore blocks, raw
  yellorium, and the reprocessor's cyanite → yellorium loop. The ingots are
  obtainable only by `give` until then.
- **The reinforced variant** — a second block set with 1.0 efficiency, 1,000
  block max size and a 30,000 FE part capacity.
- **Control-rod insertion** — the signature feature of the mod; needs a GUI or a
  command surface, and the per-rod radiation model it feeds.
- **Solid moderators** — graphite/iron/diamond blocks registered as moderators
  (the API can read them; the slice reads air only).
- **GUIs** — the controller's screen (fuel/waste/heat/energy bars), the port's
  in/out mode, the redstone port, the computer ports (CC:Tweaked is not ported).
- **Waste ejection** — waste currently fills the tank and then stops the
  reaction; upstream ejects cyanite through the access port.
- **Guide book** — the Patchouli `erguide` (Patchouli is not ported).
- **Item reactants by tag** — only `bigreactors:yellorium_ingot` is accepted;
  the `c:ingots/yellorium`/`c:ingots/uranium` tag route upstream uses is not
  read.
- **Charging ports, creative water generator, glass** — carried blocks not yet
  in the slice.

## Deviations

- The buffer lives in the controller's NBT; upstream keeps a share per part and
  exposes it on every part. Only the tap declares the capability here.
- The tap's own buffer caps at 50,000 FE (one tick's extraction rate); upstream
  exposes the whole reactor buffer through it.
- One fuel assembly per reactor (all fuel rods in one vertical column). Upstream
  allows several assemblies and horizontal ones.
- Radiation between fuel rods is not modelled (a fuel rod is `MODERATOR_NONE`
  upstream); the port takes the tightest rod's moderation reach for the whole
  reactor.
- Right-click toggles instead of the GUI button, and `/bigreactors status` is a
  read-only harness/player view upstream does not have.
- Waste never leaves the tank (no ejection path yet).
- No config file: the upstream defaults (`fuelUsageMultiplier`,
  `powerProductionMultiplier`, `reactorPowerProductionMultiplier` = 1.0) are
  baked in.

## Upstream files read (analysis only)

`api/reactor/radiation/EnergyConversion.java`, `api/reactor/radiation/IrradiationData.java`,
`api/reactor/radiation/RadiationPacket.java`, `api/reactor/FuelProperties.java`,
`api/reactor/Reaction.java`, `api/reactor/IHeatEntity.java`,
`api/reactor/Moderator.java`, `api/reactor/ReactantMappingsRegistry.java`,
`gamecontent/ReactorGameData.java`, `gamecontent/multiblock/reactor/ReactorLogic.java`,
`gamecontent/multiblock/reactor/MultiblockReactor.java`,
`gamecontent/multiblock/reactor/FuelContainer.java`,
`gamecontent/multiblock/reactor/Heat.java`,
`gamecontent/multiblock/reactor/variant/ReactorVariant.java`,
`gamecontent/multiblock/reactor/part/ReactorFuelRodEntity.java`,
`config/General.java`, `config/Reactor.java`. Repo clone deleted after the
findings were recorded (the jar stays as evidence).

## Migration to the SDK format (2026-11-18)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, PR #17992) **retired `.hnmod`**; the
registry closed this port's 0.1.0 submission (**PR #796**) with one reason: *"republish this as
a native `.hmod` with the SDK from the next Nightly"*. This port is now **0.2.0**, one flat
`.hmod` in the SDK layout (`ports/_shared/AGENT_MOD_STRUCTURE.md` §0). The slice, every number
and **`src/runtime.rs` are unchanged** — sha256 `ffac1b9517711acc75add3d6dee4763fce83cab68b2d99c53a285c895a705da8`,
`cmp` against the retired copy **and** `unzip -p bigreactors.hmod src/runtime.rs` both identical.

### File map

| 0.1.0 | 0.2.0 | why |
|---|---|---|
| `mixins/bigreactors/horizon-mixin.toml` + 11 helpers | `src/lib.rs` (one `impl Hooks` + three Parts) | hooks are typed methods now; no plan/injection schema |
| `mixins/bigreactors/runtime.rs` | `src/runtime.rs` (`pub mod runtime;`) | byte-identical; the module keeps its 20 tests |
| `mixins/bigreactors/support.rs` | folded into `src/lib.rs` | the glue was already the module; the SDK has no support module |
| `mixins/bigreactors/data/bigreactors-content.hmod` | none | `assets/` and `data/` are **in the package**; no `content_root` helper |
| `content/horizon-mod.toml` | `horizon-mod.toml` at the port root | one manifest for content and code |
| `content/assets/bigreactors/` | `assets/bigreactors/` | same files, port root |
| `content/data/bigreactors/` + `content/data/c/` | `data/bigreactors/` + `data/c/` | same files, port root |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` in `horizon-mod.toml` | the pin and toolchain come from the SDK |
| `bigreactors.hnmod` | `bigreactors.hmod` | one package; the `.hnmod` does not load |
| `tools/horizon.py`, `package.py`, `package.sh`, `test_*.py`, `stamp_license.py`, `audit/`, `carry_content.py` | deleted | staging, `.hnmod` packing and the content builder are retired; `cargo horizon-mod build/check/fits` replaces them |
| — | `CHANGELOG.md`, `icon.png`, `.gitignore` | the changelog names #796; the icon is the published one; `/Cargo.lock` is ignored (trap 5) |

The retired files are kept as evidence under
`ports/_shared/evidence/bigreactors/retired-hnmod-0.1.0/`, `retired-content-0.1.0/` and
`retired-tools/`; the 0.1.0 in-game logs stay beside them.

### Helper → hook

| 0.1.0 helper (hook) | SDK 0.2 home | Kept? |
|---|---|---|
| `embedded_content.rs` (`content_root`) | none: `assets/` + `data/` are in the package | **retired** (format change) |
| `block_entity.rs` (`block_entity`) | `BlockEntityPart::block_entity` on the controller, port and tap Parts | kept |
| `ticks.rs` (`block_entity_ticks`) | `BlockEntityPart::block_entity_ticks` (`event.ticks = true`) | kept |
| `container.rs` (`block_entity_container`) | `Registrar::block_container` on `basic_reactorsolidaccessport` (1 slot, input) | kept |
| `energy.rs` (`block_entity_energy`) | `Registrar::block_energy` on the tap (50,000 FE, extract only) | kept |
| `tick.rs` (`block_entity_tick`) | `BlockEntityPart::block_entity_tick` per Part (controller = the reactor, tap = queue, port = idle) | kept |
| `level_tick_post.rs` (`level_tick_post`) | `Hooks::level_tick_post` (`HostSide`, `event.level`) | kept |
| `right_click.rs` (`right_click_block`) | `Hooks::right_click_block` (`HostSide`, `BlockClick`) | kept |
| `register.rs` (`register_commands`) | `Hooks::register_commands` (collect: one `Command`) | kept |
| `perform.rs` (`perform_command`) | `Hooks::run_mod_command` (typed `pos:block_pos`) | kept |
| `world.rs` (`world_lifecycle`) | `Hooks::world_lifecycle` (clears the tap queue) | kept |
| `mark_modified` (`world.modified`) | retired: a tick's `Changed` reply dirties the chunk; `merge_data` dirties its own write and an item write calls `set_changed` | **new, required** |

The tap still queues because a ticked block entity is **out of the world during its own tick**;
the queue is now `static TAP_QUEUE: Mutex<Vec<(LevelRef, [i32; 3])>>`, keyed by the `LevelRef`
handle (epoch + dimension + generation), and `world_lifecycle` clears it.

### Traps this migration hit

1. **A leftover old path refuses the package.** `mixins/`, `content/` and the `.hnmod` were
   moved to evidence **before** the first build; otherwise `cargo horizon-mod build` answers
   `unexpected package path`.
2. **`level_tick_post` is not the old server hook.** Its generated signature is
   `fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent)` — `HostSide`
   (both sides), no lifetime on the event. Taking a `&mut dyn ServerHost` fails the trait.
3. **`BlockClick.player` is a `u128` uuid, not a `PlayerRef`.** Resolve it with
   `host.player(uuid)` before `action_bar`; the first draft called `action_bar` on the uuid.
4. **The energy capability exposes amount and capacity, not NeoForge's per-side
   `canInsert`/`maxInsert`.** `ModEnergyBuffer`'s `insert_faces`/`extract_faces` and per-call
   caps still gate the move, and `insert` answers with what it took, so the tap's push is
   **one transaction**: insert into the neighbour, then extract exactly that many from the tap,
   and abort if the two disagree — a refused insert creates no FE.
5. **`cargo test`/`clippy` write `Cargo.lock`, and the packer would include it**, changing the
   artifact (the imfast trap). `rm -f Cargo.lock` before the build; `.gitignore` lists it.
6. **`cargo fmt --check`, never `--all`**: `.horizon/sdk` is a path dependency and its own files
   are not ours to format.
7. **Two glue tests retire with the game-side crate.** `the_port_gives_up_its_yellorium` and
   `another_item_is_not_fuel` built an `ItemStack` from a string; the SDK's `ItemKey` is an
   opaque registry key only the game can mint. The fuel decision is still covered by the
   runtime's `insert_ingots`/capacity tests and by the e2e's 4-ingot phase; the other three
   glue tests (the block→part map, the NBT round trip, the cold default) carry over. Total:
   **23 tests** (20 runtime + 3 glue).
8. **`build-slot.sh` speaks the retired tools**, so it has no `stage`/`check`/`build` step for
   this layout and must not be handed the old `horizon.py` commands. The SDK ladder runs
   directly in the port with `CARGO_INCREMENTAL=0`, no staging and no `--target-dir`; each run
   below was still wrapped in a **lease** (`BUILD_LOCAL=1 build-slot.sh run --job sdk-br
   --port ports/bigreactors --step … -- bash -c 'cd ports/bigreactors && …'`) so the campaign's
   slot accounting holds.
9. **The artifact now carries `README.md`, `CHANGELOG.md` and `icon.png`** as root files, plus
   `uses.json`; docs such as `notes.md`/`FINDINGS.md`/`VALIDATION.md` are not packed. 77
   entries, 55,528 B.

### Not re-run in this migration

- **The in-game e2e** — the SDK compiles code mods only through
  `horizon-launcher native-build`, and no launcher binary is installed here. `tools/e2e.sh` is
  rewritten for that flow (it refuses to start without `GAME`) and keeps every 0.1.0 assertion,
  including the `SELFTEST=1` inverted-trajectory run. The 0.1.0 run remains the behaviour
  evidence: `runtime.rs` and the simulation are byte-identical, and only the transport of the
  same calls changed.
- **The staged ladder** (`tools/horizon.py …`) — those tools are deleted with the format.

## Verification

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

### 0.2.0 — the SDK package

| Check | Command | Result |
|---|---|---|
| Runtime tests before glue | `cargo test` with only `pub mod runtime;` | **20 passed**, 0 failed (`evidence/bigreactors/sdk-test-runtime.log`) |
| All tests | `cargo test` | **23 passed**, 0 failed (`evidence/bigreactors/sdk-test-glue.log`) |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`evidence/bigreactors/sdk-clippy.log`) |
| Formatting | `cargo fmt --check` | clean (inside `sdk-build.log`) |
| Package | `cargo horizon-mod build` | `Built ./target/horizon-mod/bigreactors.hmod`, EXIT 0 (`evidence/bigreactors/sdk-build.log`) |
| Registry check | `cargo horizon-mod check bigreactors.hmod` | `bigreactors 0.2.0 (native-mod, format native-1/0b14f8a/e59da90, for 26.3)`, **55,528 B**, sha256 `8eea3653…3a83`, **EXIT 0** (`evidence/bigreactors/sdk-check-fits.log`) |
| Fits the pinned build | `cargo horizon-mod fits bigreactors.hmod --build nightly` | `0b14f8a  made for`, EXIT 0 (same log) |
| `runtime.rs` untouched | `unzip -p bigreactors.hmod src/runtime.rs \| sha256sum` | `ffac1b95…da8` = the retired file (`cmp` clean) |
| Skills line | `python3 ports/tools/check-skills.py --notes ports/bigreactors` | present (see the line above) |

### 0.1.0 — the retired `.hnmod`

Ladder at the pin, all green: `cargo fmt --check` clean; `cargo clippy
--all-targets -- -D warnings` clean; **20 unit tests** (standalone crate);
`stage`/`check`/`test`/`build` `cargo_exit 0` with **0 warnings**, **25
embedded tests** (20 `game::bigreactors::` + 5 `game::bigreactors_state::`).
Red first: the twelve simulation tests failed on `todo!()` before the
implementation, and the eight formation-checker tests failed (6 red) before
`check_shape` — the first of those reds was a real defect the game found: every
reactor with an air interior was rejected (a 3×3×3 shell is 26 parts, not 27).

Artifacts in the port: `content/bigreactors.hmod` **25,969 B**
`5a6412cf…f03f`; `bigreactors.hnmod` **34,256 B** `7310d798…c330`, 16 entries,
reproducible on a second `tools/package.py` run; `horizon-mod check` accepts it
as `native-mod` for 26.3.

In game (The End, headless), three legs under
`ports/_shared/evidence/bigreactors/`: `e2e-default.log` **PASS + EXIT:0** —
formed 3×3×3, 4 ingots → 4,000 mB, **126.8 FE/t** steady over 118 ticks, **3 mB
of fuel in 118 ticks**, housing 425.8 °C, the tap filled to its 50,000 FE, the
off control stopped the fuel while the stored heat radiated, and a box with no
fuel rod did not form (`a reactor needs at least one fuel rod`);
`e2e-selftest.log` **FAIL + EXIT:1** (the trajectory assertion flipped);
`e2e-embedded.log` **PASS + EXIT:0** with no `--dev-mod`, and the same binary
with `.horizon-cache` unusable has no casing item and the block stays
`minecraft:air`. `check-e2e-logs.py --strict` 3/3; `check-skills.py --notes`
PASS; `upstream_ready.py` has one blocker, the ledger status, which the
orchestrator records. Details and the runner traps: [`VALIDATION.md`](VALIDATION.md).

## Reviewer repairs (2026-11-11) — PR review items and what changed


Each item below is quoted from `ports/_shared/horizon-mods/reviews/pr-*-bigreactors.md` (the review is the specification), with what this repair did about it.

- **Reviewer:** attribution and upstream licence evidence
  **Repair:** `LICENSE-NOTICE.md` now names the exact jar (`ExtremeReactors2-1.21.1-2.4.28.jar`), the MIT declaration in its `neoforge.mods.toml` and ZeroNoRyouki's copyright, and states that upstream's repository has no licence file (why GitHub reports `license: null`). The entry credits ZeroNoRyouki and links the absolute URL.
- **Reviewer:** licence notice in the bundle
  **Repair:** Both archives now carry the notice (`mixins/bigreactors/LICENSE` and `data/bigreactors/license/`).
- **Reviewer:** registry artifact delivery
  **Repair:** **Staging (orchestrator, not ours)**: the artifact is not in registry release/staging as `<sha256>.<ext>`; the orchestrator answers it with `horizon-mods#686` (no write access to the registry's releases). Every repackaged artifact below has a fresh sha256 for that re-push, and the author release must be re-uploaded from the port repository.

Skills: superpowers/using-superpowers,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port
