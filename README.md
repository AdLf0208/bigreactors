# bigreactors (Horizon port)

A Horizon port of [bigreactors](ZeroNoRyouki/ExtremeReactors2) — MIT.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- The smallest reactor the mod expresses end to end, with the jar's own numbers:
- | in | what |
- |---|---|
- | `bigreactors:basic_reactorcasing` | the shell |
- | `bigreactors:basic_reactorcontroller` | the machine: fuel, waste, heat, energy, on/off |
- | `bigreactors:basic_reactorfuelrod` | the fuel column |
- | `bigreactors:basic_reactorcontrolrod` | the assembly cap (top face) |
- | `bigreactors:basic_reactorsolidaccessport` | the inventory fuel goes in through |

## Not yet ported

- Turbine, reprocessor, energizer, fluidizer** — four more multiblocks, each
- its own machines slice; the reactor's active (steam) mode goes with the
- turbine.
- Fluids** — the 10 fluid blocks, coolants, steam, the fluid ports and the
- reactor's active cooling mode.
- Ores and the metal chain** — yellorite ore (worldgen), the ore blocks, raw
- yellorium, and the reprocessor's cyanite → yellorium loop. The ingots are
- obtainable only by `give` until then.

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: ZeroNoRyouki/ExtremeReactors2. Port: AdLf0208. Licence: MIT — see [LICENSE](LICENSE);
the original notices are preserved.
