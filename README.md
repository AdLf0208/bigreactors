# Extreme Reactors for Horizon — slice 1

A native Horizon port of **Extreme Reactors 2.4.28** (`ZeroNoRyouki/ExtremeReactors2`,
MIT), the mod ATM10 ships as `bigreactors`. This is a **partial port**: it
implements the basic passive **Reactor** and the **Energizer** end to end and
records the rest as gaps.

Local-only for now: MIT allows derivatives, but publishing needs the maintainer's
approval (`ARCHITECTURE.md` §8).

## The package

One flat **`.hmod`** (`bigreactors.hmod`), the SDK format of 2026-11-18: content and
code in one package, so nothing else has to be installed.

```text
horizon-mod.toml     the only manifest: id, version 0.2.3, api 0.2, the [native] pins
Cargo.toml           one crate; the only dependency is horizon-mod-api from .horizon/sdk/
src/lib.rs           the one #[mod_api::horizon_mod] impl Hooks (glue and Parts)
src/runtime.rs       the pure reactor and energizer logic, 38 tests
assets/bigreactors/  carried blockstates, models, textures, item definitions and lang
data/bigreactors/    carried block/item declarations, loot, recipes, a creative tab
data/c/              the c:ingots/yellorium and c:ingots/graphite item tags
LICENSE, LICENSE-NOTICE.md, icon.png
```

## What works

- **Build the book's first reactor**: a 3×3×3 (3 to 5 on each side) box of
  `basic_reactorcasing` with `basic_reactorcontroller`, `basic_reactorfuelrod`,
  `basic_reactorcontrolrod` on the top face above the rod,
  `basic_reactorsolidaccessport` and `basic_reactorpowertapfe_active`.
- **Right-click the controller** to switch the reactor on or off (upstream's GUI
  button). A shape that does not form tells you why.
- **Fuel** goes in through the Solid Access Port: one yellorium ingot is
  1,000 mB. The port's slot is a real container, so hoppers work.
- **The reactor runs the upstream simulation**: radiation, fuel heat, reactor
  heat, passive cooling and the FE buffer, with the mod's own constants. A 3×3×3
  with one fuel rod settles at **≈126.8 FE/t** and **≈0.028 mB of fuel per tick**
  (one ingot is about 35,700 ticks) at **≈426 °C**.
- **The power tap** is a real `horizon:energy` block buffer (the SDK's energy
  capability), filled from the reactor and pushed to whatever is beside it,
  50,000 FE/t.
- **Build an energizer**: a 3 to 32 block box of `energizercasing` around an
  interior filled with `energizercell`, with `energizercontroller` and at least
  one `energizerpowerportfe`. Its capacity is the jar's per-block amount times the
  interior volume: a 3×3×3 (one cell) holds **683,212,743,470,724 FE**, a 5×5×5
  (27 cells) 27× that.
- **Charge and discharge it**: right-click the controller to switch it on or off;
  right-click the power port to toggle **input** (a neighbour pushes FE in) and
  **output** (the port pushes FE out). An off energizer refuses both. The
  reactor's controller is also a receive face, so one machine can feed the other.
- `/bigreactors status <pos>` prints a read-only view of any of the blocks
  (upstream has no command; this is the harness and player view).
- Carried content: the ten blocks with their own models and textures, yellorium
  and graphite ingots, nine block recipes, graphite smelting/blasting from
  coal/charcoal, loot tables, the vanilla `minecraft:mineable/pickaxe` and
  `minecraft:needs_stone_tool` tags, a creative tab and English names.

A crafted chain exists: coal → graphite ingot; casing, controller, control rod,
fuel rod, access port and power tap from iron/graphite/yellorium ingots
(yellorium ingots come from yellorite ore, which is a worldgen gap — `give` them
until then).

## What does not (see `notes.md` for the full gap list)

- The **turbine**, **reprocessor** and **fluidizer** multiblocks; the
  **reinforced** variant; **fluids** (coolants, steam, active cooling);
  **ores/worldgen**; the energizer's charging and computer ports, and its cell
  recipe (it needs the ore chain's crystals).
- **Control-rod insertion** (fixed at 0: the rod is the assembly cap), **solid
  moderators** (air only), **waste ejection**, every **GUI**, the guide book.
- The reactor's energy buffer lives in the controller and is exposed through the
  tap only; upstream exposes it on every part.

## Build

`cargo horizon-mod` reads `.horizon/sdk/` (fetched by any `cargo horizon-mod`
command, never packed). `build-slot.sh` speaks the retired tools, so the SDK
ladder runs **directly in the port** — with a build-slot lease around it when the
campaign's compile slots are shared:

```sh
cd ports/bigreactors
CARGO_INCREMENTAL=0 cargo test                  # the pure runtime + glue tests
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
cargo fmt --check                               # not --all: it would read the fetched SDK
rm -f Cargo.lock                                # a packed lock changes the artifact
CARGO_INCREMENTAL=0 cargo horizon-mod build     # -> target/horizon-mod/bigreactors.hmod
cp target/horizon-mod/bigreactors.hmod ./bigreactors.hmod
cargo horizon-mod check bigreactors.hmod
cargo horizon-mod fits bigreactors.hmod --build nightly
```

The in-game scenario needs a game the launcher built with the package:
`horizon-launcher native-build ports/bigreactors --instance <instance>
--trust-native-code`, then `GAME=<that horizon-client> tools/e2e.sh`
(`SELFTEST=1` flips its trajectory assertion so it must fail). The 0.1.0 `.hnmod`
run stands as the behaviour's evidence; see `VALIDATION.md`.

## Evidence

`VALIDATION.md` has the ladder, the artifact's hash and the in-game logs
(`ports/_shared/evidence/bigreactors/`). `FINDINGS.md` has the jar inventory, the
upstream files read and every number with the file it came from.

## Licence

MIT, as the original. The original copyright and licence are the upstream
project's; this port is a clean-room Rust rewrite written from the mod's observed
behaviour, its own data files and its public source as an analysis aid.
