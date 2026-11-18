# Extreme Reactors — a Horizon port

**Extreme Reactors** is a Minecraft mod. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [ZeroNoRyouki/ExtremeReactors2](https://github.com/ZeroNoRyouki/ExtremeReactors2) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Extreme Reactors ([source](https://github.com/ZeroNoRyouki/ExtremeReactors2)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `bigreactors.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

### What works

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
- `/bigreactors status <pos>` prints a read-only view of any of the three blocks
  (upstream has no command; this is the harness and player view).
- Carried content: the six blocks with their own models and textures, yellorium
  and graphite ingots, six block recipes, graphite smelting/blasting from
  coal/charcoal, loot tables, the `c:ingots/yellorium` and `c:ingots/graphite`
  tags, a creative tab and English names.

A crafted chain exists: coal → graphite ingot; casing, controller, control rod,
fuel rod, access port and power tap from iron/graphite/yellorium ingots
(yellorium ingots come from yellorite ore, which is a worldgen gap — `give` them
until then).

## Not yet ported

- **Turbine, reprocessor, energizer, fluidizer** — four more multiblocks, each
- **Fluids** — the 10 fluid blocks, coolants, steam, the fluid ports and the
- **Ores and the metal chain** — yellorite ore (worldgen), the ore blocks, raw
- **The reinforced variant** — a second block set with 1.0 efficiency, 1,000
- **Control-rod insertion** — the signature feature of the mod; needs a GUI or a
- **Solid moderators** — graphite/iron/diamond blocks registered as moderators
- **GUIs** — the controller's screen (fuel/waste/heat/energy bars), the port's
- **Waste ejection** — waste currently fills the tank and then stops the

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `bigreactors.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Extreme Reactors was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/ZeroNoRyouki/ExtremeReactors2

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run bigreactors` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/bigreactors/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
