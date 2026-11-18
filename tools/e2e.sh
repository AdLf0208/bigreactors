#!/usr/bin/env bash
# In-game check for Extreme Reactors slice 1: the basic passive reactor.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the `run_mod_command` hook.
#
# Since the SDK migration (2026-11-18) the mod is one flat `.hmod` (`horizon-mod.toml`,
# `src/`, `assets/`, `data/`) that the launcher compiles into the game, so this script
# needs a game the launcher built with it:
#
#   horizon-launcher native-build ports/bigreactors --instance <instance> --trust-native-code
#   GAME=<that horizon-client> tools/e2e.sh
#
# There is no `--dev-mod` leg any more: the content (blocks, items, models, textures, loot
# tables, lang) is inside the same package as the code. The retired 0.1.0 `.hnmod` run is
# kept under `ports/_shared/evidence/bigreactors/retired-*/`.
#
# The End, in the void at x=100: an empty level with no ambient item entities.
# The book's first reactor stands at (100,101..103, 0..2): 3x3x3 of basic
# casing, the controller on the west face, the solid access port on the east
# face, the active FE power tap on the north face, one fuel rod in the
# 1x1x1 interior and the control rod on the top face above it.
#
# Default run:
#  A  the formed reactor reads 0 FE, no fuel, inactive;
#  B  control "no fuel": 40 ticks of an active reactor with an empty tank make
#     no FE and burn nothing;
#  C  four yellorium ingots in the access port become 4,000 mB of fuel;
#  D  right-clicking the controller switches it on (upstream's GUI button);
#  E  after ~100 ticks the reactor follows the upstream trajectory (8,026 FE at
#     t=100 +/- the table) and the housing is near 420 degrees;
#  F  the power tap holds the reactor's FE, so the buffer is on the machine's FE
#     face and not only in its own NBT;
#  G  the steady rate is 126.8 FE/t and 0.028 mB/t, the tap settles at 50,000 FE;
#  H  control "off": right-clicking again stops the fuel use while the stored
#     heat is still radiated away;
#  I  control "unformed": a 3x3x3 of casing with no fuel rod does not form, says
#     why, and makes no FE.
#
# SELFTEST=1 flips E's trajectory assertion (it must FAIL), proving the scenario
# can fail.
set -Eeuo pipefail
export LC_ALL=C

CLONE=${CLONE:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/bigreactors
PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/bigreactors-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=BigReactorsE2E

# The reactor, in the End's void at x=100. `X0,Y0,Z0` is the shell's near corner.
X0=100
Y0=101
Z0=0
CONTROLLER="$X0 $((Y0 + 1)) $((Z0 + 1))"
PORT_AT="$((X0 + 2)) $((Y0 + 1)) $((Z0 + 1))"
TAP="$((X0 + 1)) $((Y0 + 1)) $Z0"
FUEL_ROD="$((X0 + 1)) $((Y0 + 1)) $((Z0 + 1))"
CONTROL_ROD="$((X0 + 1)) $((Y0 + 2)) $((Z0 + 1))"

command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; exit 1; }
[ -n "$GAME" ] || { echo "set GAME=<horizon-client the launcher built with this .hmod>" >&2; exit 1; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/bigreactors-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 80 >&2 || true; exit 1; }

cleanup() {
    status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() {
    rm -f "$SOCK"
    cd "$SCRATCH"
    ctl launch --game "$GAME" --world "$WORLD" --seed 20261105 --creative --headless \
        --wait --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }

# A stone floor at y=100 in the End's void at x=100, clear air above it.
arena() {
    ctl command "execute in minecraft:the_end run tp @s $((X0 + 1)).5 104 1.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 3)) 100 -12 $((X0 + 6)) 100 14 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 3)) 101 -12 $((X0 + 6)) 108 14 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $((X0 - 2)).5 101 1.5" >/dev/null
    ctl wait 20
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X0" 101 0)" = "minecraft:air" ] || fail "the arena air above the floor is not air"
}

place() { # place X Y Z BLOCK
    ctl command "execute in minecraft:the_end run setblock $1 $2 $3 $4" >/dev/null
    ctl wait 2
    [ "$(block_name "$1" "$2" "$3")" = "$4" ] || fail "expected $4 at $1,$2,$3, got '$(block_name "$1" "$2" "$3")'"
}

# The book's first reactor: 27 blocks, the six kinds this slice carries.
build_reactor() {
    local y
    for y in "$Y0" $((Y0 + 1)) $((Y0 + 2)); do
        ctl command "execute in minecraft:the_end run fill $X0 $y $Z0 $((X0 + 2)) $y $((Z0 + 2)) bigreactors:basic_reactorcasing" >/dev/null
    done
    ctl wait 4
    place $CONTROLLER bigreactors:basic_reactorcontroller
    place $PORT_AT bigreactors:basic_reactorsolidaccessport
    place $TAP bigreactors:basic_reactorpowertapfe_active
    place $FUEL_ROD bigreactors:basic_reactorfuelrod
    place $CONTROL_ROD bigreactors:basic_reactorcontrolrod
    ctl wait 4
}

# status X Y Z: this block's status line, after a fresh reply arrives (the t=
# field makes each reply unique).
status() { # status X Y Z
    local out line before
    before=$(ctl messages 20 | jq -r '.chat[]' | grep -E " at $1,$2,$3:" | head -1)
    say "bigreactors status $1 $2 $3"
    for _ in $(seq 1 25); do
        ctl wait 4 >/dev/null
        out=$(ctl messages 20)
        line=$(jq -r '.chat[]' <<<"$out" | grep -E " at $1,$2,$3:|No reactor block" | head -1)
        if [ -n "$line" ] && [ "$line" != "$before" ]; then
            echo "$line"
            return
        fi
    done
    echo "NO-STATUS $(jq -c '{chat,action_bar}' <<<"$out")"
}

field() { sed -nE "s/.*$2([0-9]+).*/\1/p" <<<"$1"; }
energy_of() { sed -nE 's/.* ([0-9]+)\/[0-9]+ FE.*/\1/p' <<<"$1"; }
fuel_of() { sed -nE 's/.* fuel ([0-9]+), waste .*/\1/p' <<<"$1"; }
waste_of() { sed -nE 's/.* waste ([0-9]+), heat .*/\1/p' <<<"$1"; }
active_of() { sed -nE 's/.* active (true|false), t=.*/\1/p' <<<"$1"; }

expect_status() { # expect_status X Y Z SUBSTRING
    local line
    line=$(status "$1" "$2" "$3")
    echo "  status($1,$2,$3): $line"
    case "$line" in
        *"$4"*) ;;
        *) fail "expected /bigreactors status to say '$4', got '$line'" ;;
    esac
}

echo "== H: control - a box of casing with no fuel rod does not form =="
ctl command "execute in minecraft:the_end run fill $X0 $Y0 $Z0 $((X0 + 2)) $((Y0 + 2)) $((Z0 + 2)) bigreactors:basic_reactorcasing" >/dev/null
ctl wait 4
ctl command "execute in minecraft:the_end run setblock $FUEL_ROD minecraft:air" >/dev/null
ctl wait 4
place $CONTROLLER bigreactors:basic_reactorcontroller
place $CONTROL_ROD bigreactors:basic_reactorcontrolrod
line=$(status $CONTROLLER)
echo "  status: $line"
case "$line" in
    *"not formed"*) ;;
    *) fail "a box with no fuel rod formed: $line" ;;
esac
case "$line" in
    *" FE"*) fail "an unformed reactor reads an FE buffer: $line" ;;
esac
ctl command "execute in minecraft:the_end run fill $X0 $Y0 $Z0 $((X0 + 2)) $((Y0 + 2)) $((Z0 + 2)) minecraft:air" >/dev/null
ctl wait 6

echo "== build the book's first reactor (3x3x3, one fuel rod, one control rod) =="
build_reactor
line=$(status $CONTROLLER)
echo "  status: $line"
case "$line" in
    *"formed 3x3x3, 1 fuel rods, 1 control rods"*) ;;
    *) fail "the 3x3x3 reactor did not form: $line" ;;
esac
[ "$(energy_of "$line" || true)" = 0 ] || fail "a fresh reactor holds FE: $line"
[ "$(fuel_of "$line")" = 0 ] || fail "a fresh reactor holds fuel: $line"
[ "$(active_of "$line")" = false ] || fail "a fresh reactor is on: $line"

echo "== B: control - no fuel, no FE (the reactor is still off here) =="
ctl wait 40
line=$(status $CONTROLLER)
echo "  after 40 off ticks: $line"
[ "$(energy_of "$line" || true)" = 0 ] || fail "an off reactor made FE: $line"

echo "== C: four yellorium ingots in the access port become 4,000 mB =="
ctl container $PORT_AT bigreactors:yellorium_ingot 4 >/dev/null
ctl wait 6
line=$(status $CONTROLLER)
echo "  $line"
[ "$(fuel_of "$line")" = 4000 ] || fail "the port did not convert the ingots: $line"
line=$(status $PORT_AT)
echo "  $line"
case "$line" in
    *"empty"*) ;;
    *) fail "the access port kept its ingots: $line" ;;
esac

echo "== D: right-clicking the controller switches the reactor on =="
ctl look-at $X0 $((Y0 + 1)) $((Z0 + 1)) >/dev/null
ctl interact $CONTROLLER >/dev/null || fail "the controller did not take a right click"
ctl wait 6
line=$(status $CONTROLLER)
echo "  $line"
[ "$(active_of "$line")" = true ] || fail "the controller did not switch the reactor on: $line"
[ "$(energy_of "$line")" = 0 ] || fail "the reactor made FE before a tick: $line"
t0=$(field "$line" "t=")

echo "== E: the reactor's FE against the upstream trajectory =="
# The status round trip costs ticks, so the run measures elapsed game time (the
# `t=` field) instead of assuming how many ticks a `ctl wait` took. The
# expectations are the upstream formulas transcribed in FINDINGS.md.
ctl wait 320
line=$(status $CONTROLLER)
t1=$(field "$line" "t=")
total1=$(( $(energy_of "$line" || echo 0) + $(energy_of "$(status $TAP)" || echo 0) ))
echo "  at t=$t1: $total1 FE total, $line"
expect=$(awk -v t="$t1" -v t0="$t0" 'BEGIN {
    dt = t - t0;
    split("20 340 40 1558 60 3423 80 5639 100 8026 120 10494 140 12999 160 15521 180 18051 200 20585 220 23121 240 25657 260 28193 280 30730 300 33266 320 35802 340 38338 360 40874 380 43410 400 45946", table, " ");
    lo = table[1]; hi = table[length(table)];
    if (dt <= lo) { print table[2]; exit }
    if (dt >= hi) { print table[length(table)]; exit }
    for (i = 1; i < length(table); i += 2) {
        a = table[i]; b = table[i + 2];
        if (dt >= a && dt <= b) {
            ea = table[i + 1]; eb = table[i + 3];
            print ea + (eb - ea) * (dt - a) / (b - a); exit
        }
    }
}')
echo "  elapsed $((t1 - t0)) ticks: $total1 FE, the trajectory says $expect"
if [ "$SELFTEST" = 1 ]; then
    echo "  SELFTEST: expecting a wildly wrong total (this must fail)"
    total1=999999
fi
awk -v got="$total1" -v want="$expect" 'BEGIN { exit !(got > want * 0.92 && got < want * 1.08) }' \
    || fail "$total1 FE after $((t1 - t0)) ticks is not the upstream trajectory's $expect: $line"
heat=$(sed -nE 's/.* heat ([0-9.]+), fuel heat.*/\1/p' <<<"$line")
echo "  reactor heat $heat degrees (trajectory 420-426)"
awk -v heat="$heat" 'BEGIN { exit !(heat > 410 && heat < 432) }' \
    || fail "the reactor heat is not on the upstream trajectory: $line"
[ "$(active_of "$line")" = true ] || fail "the reactor switched itself off: $line"

echo "== F: the power tap holds the reactor's FE (the energy capability) =="
tap_line=$(status $TAP)
tap_energy=$(energy_of "$tap_line" || echo 0)
echo "  $tap_line"
[ "$tap_energy" -gt 0 ] || fail "the tap has no FE, so the buffer is not on the machine's FE face: $tap_line"
[ "$tap_energy" -le 50000 ] || fail "the tap holds more than its capacity: $tap_line"

echo "== G: the steady-state rate and the fuel burn over 100 ticks =="
# The tap's 50,000 FE buffer is a window on the reactor's; once it is full the
# reactor's own buffer grows again, so the rate is read from the controller over
# a window after the tap has filled. Every read is timestamped from the same
# reply as the number it carries: a status round trip costs ticks.
ctl wait 300
line=$(status $CONTROLLER)
tap_line=$(status $TAP)
fuel0=$(fuel_of "$line")
t2_start=$(field "$tap_line" "t=")
total2_start=$(( $(energy_of "$line" || echo 0) + $(energy_of "$tap_line" || echo 0) ))
echo "  t=$t2_start: $total2_start FE total ($(energy_of "$line") in the reactor, $(energy_of "$tap_line") in the tap), fuel $fuel0 mB"
ctl wait 100
line=$(status $CONTROLLER)
tap_line=$(status $TAP)
t2=$(field "$tap_line" "t=")
total2=$(( $(energy_of "$line" || echo 0) + $(energy_of "$tap_line" || echo 0) ))
dt=$((t2 - t2_start))
rate=$(awk -v a="$total2_start" -v b="$total2" -v d="$dt" 'BEGIN { printf "%.1f", (b - a) / d }')
echo "  $dt ticks: $total2_start -> $total2 FE, $rate FE/t (upstream 126.8)"
awk -v rate="$rate" 'BEGIN { exit !(rate > 123 && rate < 130) }' \
    || fail "the steady-state rate is $rate FE/t, not the upstream 126.8: $line"
fuel_used=$(( fuel0 - $(fuel_of "$line") ))
echo "  fuel $fuel0 -> $(fuel_of "$line") mB ($fuel_used mB in $dt ticks, upstream 0.028 mB/t)"
awk -v used="$fuel_used" -v d="$dt" 'BEGIN { exit !(used >= d * 0.02 && used <= d * 0.04) }' \
    || fail "the reactor burned $fuel_used mB in $dt ticks, not ~0.028 mB/t: $line"
[ "$(waste_of "$line")" -gt 0 ] || fail "no waste was produced: $line"
heat=$(sed -nE 's/.* heat ([0-9.]+), fuel heat.*/\1/p' <<<"$line")
echo "  steady heat $heat degrees (upstream 426.4)"
awk -v heat="$heat" 'BEGIN { exit !(heat > 420 && heat < 432) }' \
    || fail "the steady-state heat is $heat, not the upstream 426.4: $line"
[ "$(energy_of "$tap_line")" = 50000 ] || fail "the tap did not settle at its 50,000 FE capacity: $tap_line"

echo "== H: control - off stops the fuel use; stored heat is still radiated =="
ctl look-at $X0 $((Y0 + 1)) $((Z0 + 1)) >/dev/null
ctl interact $CONTROLLER >/dev/null || fail "the controller did not take a right click"
ctl wait 6
line=$(status $CONTROLLER)
echo "  $line"
[ "$(active_of "$line")" = false ] || fail "the controller did not switch the reactor off: $line"
fuel_before=$(fuel_of "$line")
energy_before=$(( $(energy_of "$line" || echo 0) + $(energy_of "$(status $TAP)" || echo 0) ))
ctl wait 60
line=$(status $CONTROLLER)
energy_after=$(( $(energy_of "$line" || echo 0) + $(energy_of "$(status $TAP)" || echo 0) ))
echo "  after 60 off ticks: $line (energy $energy_before -> $energy_after)"
[ "$(fuel_of "$line")" = "$fuel_before" ] || fail "an off reactor burned fuel: $line"
[ "$energy_after" -ge "$energy_before" ] || fail "the stored heat was lost instead of radiated: $line"

echo "PASS: a 3x3x3 basic reactor formed, 4 ingots became 4,000 mB of fuel, the reactor followed the upstream trajectory ($total1 FE at t=$t1), held the steady 126.8 FE/t and 0.028 mB/t with the housing at ~426 C, its power tap carried the FE through the energy capability, a box with no fuel rod did not form, and switching it off stopped the fuel use while the stored heat still radiated"
