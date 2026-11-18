# Verification

Campaign pin: Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`,
horizon-mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6).
Everything below was run at that pin; the logs named are the evidence.

## 0.2.0 — the SDK package (2026-11-18)

The format migration: one flat `.hmod` built against SDK nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`** with mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**. Every command ran directly in the
port (the retired `tools/horizon.py` staging does not apply) with
`CARGO_INCREMENTAL=0`, inside a build-slot lease; logs are in
`ports/_shared/evidence/bigreactors/`.

| step | command | result |
|---|---|---|
| runtime tests, before glue | `cargo test` with only `pub mod runtime;` | **20 passed, 0 failed** (`sdk-test-runtime.log`) |
| all tests | `cargo test` | **23 passed, 0 failed** (`sdk-test-glue.log`) |
| clippy | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| fmt | `cargo fmt --check` | clean (`sdk-build.log`) |
| build | `cargo horizon-mod build` | `Built ./target/horizon-mod/bigreactors.hmod` (`sdk-build.log`) |
| check | `cargo horizon-mod check bigreactors.hmod` | `bigreactors 0.2.0 (native-mod, format native-1/0b14f8a/e59da90, for 26.3)`, **EXIT 0** (`sdk-check-fits.log`) |
| fits | `cargo horizon-mod fits bigreactors.hmod --build nightly` | `0b14f8a  made for`, **EXIT 0** |
| runtime untouched | `unzip -p bigreactors.hmod src/runtime.rs \| sha256sum` | `ffac1b95…da8`, `cmp` clean against the retired file |
| repeat build | test + clippy + `cargo horizon-mod build` again | 23 passed, 0 warnings, `cmp target/… ./bigreactors.hmod` clean — reproducible (`sdk-final-repro.log`) |

Delivered artifact: `bigreactors.hmod` **55,528 B**, SHA-256
`8eea3653872249b8d5e7fff818f3ab99a3b91f414f6b4631b5b20cf001aa3a83`, **77 entries**,
`LICENSE` + `LICENSE-NOTICE.md` + `icon.png` inside (`sdk-sha256.txt`).

Not re-run: the in-game e2e (no `horizon-launcher` on this machine; `tools/e2e.sh`
now needs a launcher-built `GAME`) and the staged ladder (its tools are deleted with
the format). The 0.1.0 run below stays the behaviour evidence — the simulation and
its tests are byte-identical.

## 0.1.0 — the retired `.hnmod`

### The ladder

| step | command | result |
|---|---|---|
| fmt | `cargo fmt --check` | clean (`FMT_OK`) |
| clippy | `cargo clippy --all-targets --locked --offline -- -D warnings` (standalone crate) | exit 0, no warnings |
| unit | `cargo test --locked --offline` (standalone crate) | **20 passed, 0 failed** |
| stage | `python3 tools/horizon.py stage` | staged `1d63e0a`, 0 errors |
| check | `python3 tools/horizon.py check` | `cargo_exit 0`, **0 warnings** |
| test | `python3 tools/horizon.py test` | **25 passed, 0 failed** (20 `game::bigreactors::` + 5 `game::bigreactors_state::`), 1,240 filtered |
| build | `python3 tools/horizon.py build` (`fast`) | `cargo_exit 0`, **0 warnings** |

Red first, twice:

1. the runtime's twelve simulation tests ran against `todo!()` bodies before the
   implementation (`0 passed; 12 failed`, every one `not yet implemented`), then
   `12 passed`;
2. the formation checker's eight tests ran against the new `check_shape` before
   its body was written: **6 failed** (`the book's first reactor forms`,
   `a reactor with air inside forms`, `a hole in the shell is refused`,
   `a box with no fuel rod is refused`, `a control rod off the face is refused`,
   `a box without a controller is refused`), then green. The first failure was a
   real defect the game found first: the port rejected every reactor whose
   interior is air (a 3×3×3 shell is 26 parts, not 27), so only a box whose
   interior was solid with rods could form. The interior test is now
   "strictly inside on all three axes", and the part count is the connected
   parts, which is also what upstream's `getPartsCount` counts.

## Artifacts (in the port, never `target/`)

| file | size | SHA-256 | entries |
|---|---:|---|---:|
| `content/bigreactors.hmod` | 25,969 B | `5a6412cf3513ecfc3ffd0c79be4522ebb01112282b3612043ed9c47c4142f03f` | — |
| `bigreactors.hnmod` | 34,256 B | `7310d7985c8da8e10fb8f114644c1ebcb1dc9e8dc63ff081178e34f9e575c330` | 16 |

- `cargo-horizon-mod horizon-mod check` on the `.hmod` → `no code: content only`,
  API 0.2, both sides, 26.3.
- `cargo-horizon-mod horizon-mod check bigreactors.hnmod --kind native-mod` →
  `bigreactors 0.1.0 (native-mod, format native-1/1d63e0a5…/5a64af93…, for 26.3)`.
- The embedded copy in the bundle is byte-identical to `content/bigreactors.hmod`
  (`5a6412cf…`), checked by `tools/package.py` and by the e2e's own `cmp`.
- Reproducible: `python3 tools/package.py` run twice gives the same `7310d798…`.

## In-game evidence (`ports/_shared/evidence/bigreactors/`)

`check-e2e-logs.py bigreactors --strict` → **3/3 ok** (each log carries a
verdict and an exit status).

| log | leg | verdict |
|---|---|---|
| `e2e-default.log` | The End, `--dev-mod content`: the unformed control, formation, fuel intake, on/off, the trajectory, the tap, the steady rate, the off control | **PASS + EXIT:0** |
| `e2e-selftest.log` | the same with the trajectory assertion flipped (`SELFTEST=1`) | **FAIL + EXIT:1** (`999999 FE after 329 ticks is not the upstream trajectory's 36943.2`) |
| `e2e-embedded.log` | no `--dev-mod`: first the *same binary* with `.horizon-cache` unusable (`no casing item, the block stayed minecraft:air`), then the whole scenario from the bundle's own content | **PASS + EXIT:0** |

Measured in game (3×3×3 basic reactor, one fuel rod, passive cooling):

| measurement | in game | upstream formulas |
|---|---|---|
| 329 ticks after switching on | 39,479 FE | 36,943 FE (the ramp is convex; the check allows ±8 %) |
| steady-state generation | **126.8 FE/t** over 118 ticks | 126.8 FE/t |
| fuel burn | **3 mB in 118 ticks** | 0.028 mB/t |
| housing heat | 425.8 °C | 426.4 °C |
| power tap (API 142) | 39,352 → 50,000 FE (its capacity) | the buffer is on the machine's FE face |
| off control | fuel stays 3,978 mB, heat 397 → 140 °C, the buffer still grows | the stored heat radiates |

The controls in the same log: a 3×3×3 of casing with a controller and a control
rod but **no fuel rod** reports
`not formed (a reactor needs at least one fuel rod)` and holds no FE; a reactor
with no fuel makes `0/216000 FE` over 40 ticks; switching the reactor off stops
the fuel use while the stored heat keeps generating.

## Gates

| gate | result |
|---|---|
| `python3 ports/tools/check-e2e-logs.py bigreactors --strict` | exit 0 — 3 logs, `PASS`/`FAIL` + `EXIT:<n>` in every one |
| `python3 ports/tools/check-skills.py --notes ports/bigreactors` | `SKILLS GATE: PASS` |
| `python3 ports/tools/upstream_ready.py bigreactors` | exit 1 with **one blocker**: `status is pending, not ported` — the ledger row is the orchestrator's to record. Every other gate reads green: licence `MIT` reviewable (jar, `licence-precheck.csv`), `bigreactors.hnmod` 34,256 B delivered in the port and newer than its sources, `notes.md` `Skills:` line, pinned to `1d63e0a5`, `e2e-default.log: PASS + EXIT:0`. |

## Runner notes (things that cost runs)

- `build-slot.sh run … --port ports/bigreactors` (campaign-relative) forwards
  correctly from inside the port. `--port ../bigreactors` looked like it worked
  but `sync_port` strips the `ports/` prefix, so it forwarded **without syncing
  the port** and the far box ran a stale copy.
- Three `cargo test` runs died with
  `could not parse/generate dep info at: … (os error 2)`. The campaign recorded
  the cause on 2026-11-10: a lease fd closed in the parent let two jobs share a
  slot and delete each other's `stage/modded`. The runs after that fix were
  green.
- The green test run used `RUSTFLAGS="-C debuginfo=0"` and
  `CARGO_BUILD_JOBS=1` while the far box was at 9 GB of 13 GB; without them
  `horizon-client-lib` (which does not include this port's code) failed the same
  dep-info way.
- A `tee` in the e2e command hid a failing exit status once (`… | tee file`
  exits with `tee`'s status). The legs now redirect and read the log instead.
