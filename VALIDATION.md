# Verification

Campaign pin: Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`,
horizon-mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` (api `0.2`, launcher 0.2.92).
Everything below was run at that pin; the logs named are the evidence. Sections older than
0.2.5 were run at the pins they name.

## 0.2.5 — one record, one visit, membership on access (2026-11-21)

Council review of PR #1501, four findings; each has a red proof already quoted in `notes.md`
(the F1 test on the **delivered 0.2.4 sources**, the F2 test with 0.2.4's loop policy restored
verbatim, and the G/H/I game phases against the **delivered 0.2.4 package**). This section is the
ladder that followed.

| step | command | result |
|---|---|---|
| repin | `[native] horizon = 0a875da…`, `rm -rf .horizon/sdk`, `cargo horizon-mod build` | `SDK 0a875da … in ./.horizon/sdk` |
| baseline | `cargo test --offline` + the unchanged e2e | **56 passed**; `e2e-0a875da-baseline.log` **PASS + EXIT:0** |
| tests | `cargo test --offline` | **62 passed, 0 failed** (`sdk-test.log`) |
| clippy | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| fmt | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| build ×2 | `rm -f Cargo.lock && cargo horizon-mod build` twice | `cmp` byte-identical (`sdk-reproducible.txt`) |
| check | `cargo horizon-mod check bigreactors.hmod` | `bigreactors 0.2.5 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, **87,747 B**, EXIT 0 (`sdk-check.log`) |
| fits (pin) | `cargo horizon-mod fits bigreactors.hmod --build 0a875da` | `0a875da  made for`, EXIT 0 (`sdk-fits.log`) |
| fits (another) | `cargo horizon-mod fits bigreactors.hmod --build 0b14f8a` | `0b14f8a  fits`, EXIT 0 (same log) |
| gate | `python3 ports/tools/verify-sdk-package.py bigreactors` | **1/1** |
| logs | `python3 ports/tools/check-e2e-logs.py bigreactors-e2e --strict` | `logs=10 ok=10 missing-exit=0 bad-exit=0` |

Delivered artifact: `bigreactors.hmod` = `extreme_reactors.hmod`, **87,747 B**, SHA-256
`0f5f3698be82cf9f13d745ac0c960d0fc92a152355d80b0825aa6f3e3ac920f2`, 107 entries, no `Cargo.lock`.

In game (`ports/_shared/evidence/bigreactors-e2e/`, `--offscreen`):

| log | leg | verdict |
|---|---|---|
| `e2e-launcher.log` | all earlier phases plus G (a 4-ingot stack into 3,000 mB of room leaves `yellorium_ingot x3`), H (5³ → 3³ around the same controller: old port `not formed`, buffer stays 0 for 200 ticks while the tap offers 15,470 FE, H2's tap then charges the new port 1,109 → 25,910), I (`visits=2` on, `visits=0` off) | **PASS + EXIT:0** |
| `e2e-launcher-control.log` | the same Nightly **without** the package | **FAIL + EXIT:1** |
| `e2e-launcher-selftest.log` | `SELFTEST=1` | **FAIL + EXIT:1** |
| `e2e-launcher-negative.log` | `NEGATIVE=1` | **FAIL + EXIT:1** |
| `e2e-launcher-peak-negative.log` | `PEAK_NEGATIVE=1` | **FAIL + EXIT:1** |
| `e2e-024-red-f3-detached-port.log` | phase H on the **delivered 0.2.4 package** | **FAIL + EXIT:1** (the counterfactual) |
| `e2e-024-red-f4-partial-fuel.log` | phase G on 0.2.4 | **FAIL + EXIT:1** |
| `e2e-024-red-f2-visits.log` | phase I on 0.2.4 | **FAIL + EXIT:1** |
| `e2e-0a875da-baseline.log` | the pre-fix tree at the new pin | **PASS + EXIT:0** |

### Runner notes this slice cost

- **A position's first `status` read must not abort the leg.** `status()`'s poll began with
  `line=$(… | grep " at X,Y,Z:" | head -1)`; with `set -e` + `pipefail`, the *first* poll of a
  position the chat window does not hold yet killed the leg silently (no `FAIL`, just `EXIT:1`).
  Both greps are `|| true` now and the window is 30 messages, because every `setblock`/`fill`
  adds a "Changed the block" chat line.
- **Do not stand in the content grid.** Phase A leaves blocks at `(120..128, 101, 10)`; the H
  phase's controller was first clicked from `(122.5,101,10.5)` — *inside* one of them — so
  `look-at-block` failed. H clicks it from the east `(123.5,101,9.5)`.
- **A detached port must survive the rebuild.** Filling the whole 5³ with air removed the very
  port the finding is about; the fill now goes around `(118,102,6)`, so the block keeps whatever
  its block entity remembers (which is what 0.2.4 trusted, and what the fix no longer reads).

## 0.2.4 — the tick budget covers capability scans (2026-11-20)

At the working pin **`0b14f8a`** + mixins **`e59da90`**. Every command ran directly in
the port with `CARGO_INCREMENTAL=0` inside a build-slot lease; the logs are
`ports/_shared/evidence/bigreactors/sdk-*.log` and `…/bigreactors-e2e/`.

| step | command | result |
|---|---|---|
| red | `cargo test --offline` before the fix | **54 passed, 1 failed** — `a_capability_scan_inside_a_tick_is_charged_to_the_tick`, `left: 6, right: 4` (`sdk-test-red.log`) |
| tests | `cargo test --offline` | **56 passed, 0 failed** (`sdk-test.log`) |
| clippy | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| fmt | `cargo fmt --check` | clean, no output (`sdk-fmt.log`) |
| build | `rm -f Cargo.lock && cargo horizon-mod build` | `Built ./target/horizon-mod/bigreactors.hmod` (`sdk-build.log`) |
| repeat | build again | `cmp` byte-identical (`sdk-reproducible.txt`) |
| check | `cargo horizon-mod check bigreactors.hmod` | `bigreactors 0.2.4 (native-mod, format native-1/0b14f8a/e59da90, for 26.3)`, **81,742 B**, **EXIT 0** (`sdk-check.log`) |
| fits (pin) | `cargo horizon-mod fits bigreactors.hmod --build 0b14f8a` | `0b14f8a  made for`, EXIT 0 (`sdk-fits.log`) |
| fits (another) | `cargo horizon-mod fits bigreactors.hmod` | `0a875da  fits`, EXIT 0 (same log) |
| package gate | `python3 ports/tools/verify-sdk-package.py bigreactors` | **1/1** |

Delivered artifact: `bigreactors.hmod` **81,742 B**, SHA-256
`c0168cedbb2a222886ec053ba0a4c6817956bb13ba5439eda04e9056e771c65b`, 107 entries, no
`Cargo.lock`. `src/runtime.rs` is untouched by this fix (sha256 `934fec99…b02b`).

In game (`ports/_shared/evidence/bigreactors-e2e/`, `--offscreen`):

| log | leg | verdict |
|---|---|---|
| `e2e-launcher.log` | every earlier phase unchanged, plus F1 (an energizer serves two adjacent reactors: B 9,253 → 216,000 FE), F2 (five machines invalidated in one tick → `scans=4`), F3 (five status reads outside a tick still scan freely), F4 (all five recover) | **PASS + EXIT:0** |
| `e2e-launcher-counterfactual-unbudgeted.log` | the same package with only `charge` reverted to 0.2.3's policy | **`FAIL: … ran 7 fresh scans, not the budgeted 4` + EXIT:1** |
| `e2e-launcher-control.log` | the same Nightly **without** the package | **FAIL + EXIT:1** |
| `e2e-launcher-selftest.log` | `SELFTEST=1`, the reactor trajectory assertion flipped | **FAIL + EXIT:1** |
| `e2e-launcher-negative.log` | `NEGATIVE=1`, the energizer capacity assertion flipped | **FAIL + EXIT:1** |
| `e2e-launcher-peak-negative.log` | `PEAK_NEGATIVE=1`, one fresh scan past the budget expected | **FAIL + EXIT:1** |

`python3 ports/tools/check-e2e-logs.py bigreactors-e2e --strict` → **6/6**;
`python3 ports/tools/check-e2e-logs.py bigreactors --strict` → **3/3** (the retired
0.1.0 logs).

## 0.2.3 — bounded formation scans (2026-11-20)

At the working pin **`0b14f8a`** + mixins **`e59da90`**. Every command ran directly in
the port with `CARGO_INCREMENTAL=0` inside a build-slot lease; the logs are
`ports/_shared/evidence/bigreactors/sdk-*.log`.

| step | command | result |
|---|---|---|
| tests | `cargo test --offline` | **53 passed, 0 failed** (`sdk-test.log`) |
| clippy | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| fmt | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| build | `rm -f Cargo.lock && cargo horizon-mod build` | `Built ./target/horizon-mod/bigreactors.hmod` (`sdk-build.log`) |
| repeat | build again | `cmp` byte-identical (`sdk-reproducible.txt`) |
| check | `cargo horizon-mod check bigreactors.hmod` | `bigreactors 0.2.3 (native-mod, format native-1/0b14f8a/e59da90, for 26.3)`, **79,837 B**, **EXIT 0** (`sdk-check.log`) |
| fits | `cargo horizon-mod fits bigreactors.hmod --build 0b14f8a` | `0b14f8a  made for`, EXIT 0 (`sdk-fits.log`) |
| package gate | `python3 ports/tools/verify-sdk-package.py bigreactors` | **1/1** |

Delivered artifact: `bigreactors.hmod` **79,837 B**, SHA-256
`e8dcf1df35d35341e56e128bc9ec9c35efff05ece485921349b018a65ee8ba46`, 107 entries, no
`Cargo.lock`. `src/runtime.rs` is untouched by this fix.

In game (`ports/_shared/evidence/bigreactors-e2e/`, `--offscreen`):

| log | leg | verdict |
|---|---|---|
| `e2e-launcher.log` | every earlier phase unchanged, plus E8 (an unrelated break leaves the formed energizer's readout identical) and E8b (a broken casing reads "not formed (the shell has a hole in it)" and forms again when replaced) | **PASS + EXIT:0** |
| `e2e-launcher-control.log` | the same Nightly **without** the package | **FAIL + EXIT:1** |
| `e2e-launcher-selftest.log` | `SELFTEST=1`, the reactor trajectory assertion flipped | **FAIL + EXIT:1** |
| `e2e-launcher-negative.log` | `NEGATIVE=1`, the energizer capacity assertion flipped | **FAIL + EXIT:1** |

`python3 ports/tools/check-e2e-logs.py bigreactors-e2e --strict` → **4/4**.

## 0.2.2 — the energizer and the parity leg (2026-11-20)

At the working pin **`0b14f8a`** + mixins **`e59da90`**. Every command ran
directly in the port with `CARGO_INCREMENTAL=0` inside a build-slot lease; the
logs are `ports/_shared/evidence/bigreactors/sdk-*.log`.

| step | command | result |
|---|---|---|
| tests | `cargo test --offline` | **47 passed, 0 failed** (`sdk-test.log`) |
| clippy | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| fmt | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| build | `cargo horizon-mod build` | `Built ./target/horizon-mod/bigreactors.hmod` (`sdk-build.log`) |
| repeat | build again | `cmp` byte-identical |
| check | `cargo horizon-mod check bigreactors.hmod` | `bigreactors 0.2.2 (native-mod, format native-1/0b14f8a/e59da90, for 26.3)`, **75,918 B**, **EXIT 0** (`sdk-check-fits.log`) |
| fits | `cargo horizon-mod fits bigreactors.hmod --build nightly` | `fits`, EXIT 0 (same log) |
| package gate | `python3 ports/tools/verify-sdk-package.py bigreactors` | **1/1** |

Delivered artifact: `bigreactors.hmod` **75,918 B**, SHA-256
`bdebad9f6389ee725fbab8f80b06be7c6b388217517497e2ee58c5f4efb4b611`, 107
entries, no `Cargo.lock`.

The in-game leg was built from the same package's `src/`, `data/`, `assets/`,
`horizon-mod.toml` and `Cargo.toml`; the delivered file differs only in
`README.md`/`CHANGELOG.md` (written after the run, and not compiled by the
launcher). A byte comparison of every packed file except those two and
`uses.json`/`icon.png` reports **0 differences** against the package the game
carries (`~/.cache/horizon-e2e/0b14f8a/bigreactors/horizon-native-mods/bigreactors/`).

### In game (`ports/_shared/evidence/bigreactors-e2e/`)

`--offscreen` (the crafting phase needs `click-slot`), the launcher-built game
carrying the package.

| log | leg | verdict |
|---|---|---|
| `e2e-launcher.log` | content, all 9 shaped recipes + 2 controls, 4 cooking recipes, 10 loot tables, the reactor phases, the energizer phases | **PASS + EXIT:0** |
| `e2e-launcher-control.log` | the same Nightly **without** the package | **FAIL + EXIT:1** (`give bigreactors:basic_reactorcasing failed`) |
| `e2e-launcher-selftest.log` | `SELFTEST=1`, the reactor trajectory assertion flipped | **FAIL + EXIT:1** |
| `e2e-launcher-negative.log` | `NEGATIVE=1`, the energizer capacity assertion flipped | **FAIL + EXIT:1** |

`python3 ports/tools/check-e2e-logs.py bigreactors-e2e --strict` → **4/4**.

Measured: the reactor unchanged (123.7 FE/t over 333 ticks, 125.5 FE/t steady,
3 mB/114 ticks, 425.8 °C, the tap 43,790 → 50,000); the energizer formed at
683,212,743,470,724 FE, refused the tap while off, charged 164,911 → 173,254 when
on, pushed **125,529 FE** into the reactor through its output port, held 27× at
5×5×5, and stopped when off.

### Defects the leg found (fixed, recorded in `notes.md`)

1. the `c:` common tags never resolve in Horizon → the recipes use direct ids;
2. the blocks were not in `minecraft:mineable/pickaxe` → every break dropped
   nothing; the tags are declared;
3. the reactor's tap pushed its own buffer into the controller's new receive
   face → `push_out` skips the reactor's own parts;
4. the two `graphite_from_coalblock` recipes were dead data → dropped.

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
