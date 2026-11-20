# Changelog

## 0.2.2 — 2026-11-20

The energizer, plus the recipe, tag and loot defects the parity rule exposed.

- **The energizer**: `energizercasing`, `energizercell`, `energizercontroller` and
  `energizerpowerportfe` (the jar's item id is `energizerpowerport_fe`) form a 3..32
  block box of casing around an interior of energy cells, with at least one
  controller. Capacity is the jar's `u64::MAX / 30³` per interior block (a 3×3×3
  holds 683,212,743,470,724 FE; a 5×5×5 holds 27×). The controller's right-click
  switches it on or off; the power port's right-click toggles input/output. An
  input port accepts a neighbour's FE into the machine, an output port pushes it
  out; an off machine refuses both. No GUI (upstream's two buttons are the
  right-clicks).
- **The reactor's controller is a receive face**: another machine's port can push
  FE into the reactor's buffer (upstream exposes the buffer on every part);
  extraction stays on the active tap. The tap no longer pushes into the reactor's
  own parts.
- **The nine carried shaped recipes now craft in game** with their declared id and
  count, and consume their ingredients; the four graphite smelting/blasting
  recipes cook. The `c:` common tags the jar's recipes name never resolve in
  Horizon (it mounts only the package's own namespace), so the recipes use direct
  item ids. The two `graphite_from_coalblock` recipes are dropped: their result
  block is not carried.
- **The ten blocks drop themselves**: they were not in
  `minecraft:mineable/pickaxe`, so `requires_tool` made every break harvest
  nothing; the vanilla tags are now declared.
- **Verified in game** end to end: content, all recipes, loot, the reactor's
  phases and the energizer's formation/charge/output, with a control leg and both
  counterfactuals (`SELFTEST=1`, `NEGATIVE=1`).

## 0.2.0 — 2026-11-18

The SDK package. Horizon's mod-API merge (#16941, launcher ≥ 0.2.70) retired the
`.hnmod` container, so this release republishes the same slice as one flat
`.hmod` — code and content in the same package.

- **Supersedes 0.1.0, submitted as PR #796 and closed by the 2026-11-18 format
  purge** ("republish this as a native `.hmod` with the SDK from the next
  Nightly"). This is a new version, not a patch of that submission.
- `src/runtime.rs` is **byte-identical** to the 0.1.0 file
  (sha256 `ffac1b95…05da8`): the simulation, the formation rule and their 20
  tests carry over unchanged.
- The glue is rewritten against the SDK's typed hooks: three `BlockEntityPart`s,
  `Registrar::block_container` for the access port, `Registrar::block_energy` for
  the power tap, `Hooks::right_click_block`, `Hooks::register_commands` /
  `run_mod_command`, `Hooks::level_tick_post` and `Hooks::world_lifecycle`.
- The embedded-content helper is gone: `assets/` and `data/` live in the package,
  so one install carries the six blocks, the two ingots and the code.
- **No in-game run for this version**: the SDK compiles code mods only through
  `horizon-launcher native-build`, and no launcher binary is installed on this
  machine. `tools/e2e.sh` is rewritten for that flow and refuses to start without
  `GAME`; the 0.1.0 run is the behaviour evidence (same `runtime.rs`, same glue).

## 0.1.0 — 2026-11-10

First release: the basic passive reactor (6 blocks, 2 items, the upstream
simulation, the energy tap, `/bigreactors status`), submitted as PR #796 in the
retired `.hnmod` format.
