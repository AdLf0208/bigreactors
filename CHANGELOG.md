# Changelog

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
