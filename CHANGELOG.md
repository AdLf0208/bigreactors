# Changelog

## 0.2.5 — 2026-11-21

Council review findings on PR #1501 (one record, one visit, membership on access; plus the
0.2.1 partial-fuel follow-up). All four were real and are fixed.

- **One formation record per machine.** The cache stored `Result<T, String>` and cloned the whole
  record for **every controller** of a machine (`Formations::insert`); a 32³ energizer's shell can
  be 5,768 controller/port blocks, so 0.2.4 held 5,768 × 5,768 coordinates ≈ **381 MiB**. The
  answer is now `Result<Arc<T>, Arc<str>>` and the hand-written `Proven::clone` copies two `Arc`s:
  one record per machine, shared by every controller and every port it answers through. A port's
  capability lookup is the same cache hit a controller's tick gets, because a scan now registers
  for the machine's controllers **and** its ports.
- **Ports are visited once per machine per tick.** Every controller used to walk every port (and
  wrote `cx/cy/cz` into each one even while the machine was off): a controller/port split cost
  N × P visits — over 8 million on a 32³ shell. The machine's tick now runs on one **master
  controller** (the lowest coordinate) and `visit_machine_ports` claims the machine once per level
  tick, so the cost is P; a switched-off machine walks nothing. `announce_port` is deleted.
  Status gains `visits=`, the ports the machine served in the last tick.
- **Membership on port access.** A port answered from the controller coordinates it remembered plus
  the controller's `formed` byte, so after a machine was rebuilt smaller around the same controller
  a detached old port still reached the energy buffer. The port now floods the formation **from its
  own position** and must be in the current shape: not a member → 0 capacity, 0 accepted. The saved
  coordinates, the `formed` bit and the saved `capacity` are gone; an off machine reports 0 both
  ways, as upstream's inactive machine refuses both.
- **Partial fuel take (0.2.1 follow-up).** The access port cleared its whole stack and the reactor
  dropped what did not fit. `Reactor::ingots_that_fit` now bounds the take to whole ingots of the
  tank's remaining room and `take_fuel` writes the remainder back into the slot with
  `ItemStack::split`.
- **Any controller is the control surface**: a click toggles the machine's master, so a second
  controller shows and switches the same machine.
- **Verified**: 62 tests (the two unit reds: 4 deep copies → 0, and 8 visits → 2);
  `fits` against the pin (`made for`) and against `0b14f8a` (`fits`); in game a 4-ingot stack into
  3,000 mB of room leaves three ingots in the port, a 5³ → 3³ rebuild leaves the old port
  `not formed` and the buffer at 0 while the tap beside it offers 15,470 FE (the new port then
  charges 1,109 → 25,910), and a two-controller machine reads `visits=2` on and `visits=0` off.
  The same three phases against the delivered 0.2.4 package fail.

## 0.2.4 — 2026-11-20

The tick's scan budget now covers capability scans (registry review finding).

- **A capability call made inside a level tick is charged to the tick.** An
  energizer's push asks each neighbour for its `capacity`, and a reactor's receive
  face asks for its own when it accepts FE; both went through the one-scan path and
  never touched `SCANS_PER_TICK`, so a tick could run one fresh box walk per
  neighbour on top of its bounded share. `level_tick_pre` now opens the tick's
  window and `level_tick_post` closes it after the tap pushes, and `ScanBudget`
  charges an in-tick capability scan to the same share as the machines' own ticks.
- **Nothing changed outside a tick**: a click, a command and a capability call
  still cost one scan, so a player's readout is never refused.
- **A budget refusal is a pause**: the capability answers 0 capacity / 0 accepted
  (nothing moves into an unproven shape) and the machine is re-proven on a later
  tick; the machine's own tick does not simulate either.
- **The status readout gains `scans=`**: the busiest level tick's fresh-scan count
  since the last read (a diagnostic like `t=`). A paused machine and a broken one
  answer a capability call the same way, so this is how the game leg proves the
  bound.
- **Verified in game**: five machines invalidated in one tick run four fresh scans
  with the fix and **seven** with only the policy reverted (the counterfactual leg);
  an energizer serves two adjacent reactors; five status reads outside a tick are
  still free; all five machines recover.

## 0.2.3 — 2026-11-20

Bounded tick cost for the formation scan (registry review finding).

- **A block change no longer drops every machine's formation.** Each controller
  keeps the proof of its machine's shape with the box that proof depends on; a
  committed block change drops only the proofs it can touch — a change inside the
  box (the shell or the interior) always, a machine part in the one-block ring
  because it can join the connected component and grow the box, and nothing
  further out. An unrelated break now leaves every controller's scan standing, so
  a busy world no longer makes every reactor and energizer re-walk its box every
  tick.
- **One scan answers a whole machine.** The scan names every controller of the
  component, so a reactor with two controllers is scanned once, not once per
  controller.
- **Total work per tick is bounded.** At most four fresh formation scans run in a
  level tick (`SCANS_PER_TICK`), refilled at `level_tick_post`; a scan that
  starts always finishes. A machine the budget cannot reach reports "not formed
  (the shape is waiting to be re-checked)" for that tick instead of running on a
  shape nothing re-checked — a pause, never stale energy.
- **A world change clears the proofs** (`world_lifecycle`), not only the tap
  queue.
- **Verified in game**: the energizer's readout is unchanged by a break two blocks
  outside its shell, and a broken casing is re-proven as "not formed (the shell
  has a hole in it)" and forms again when the casing is replaced (phases E8/E8b).

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
