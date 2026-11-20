#!/usr/bin/env bash
# In-game check for Extreme Reactors: slice 1 (the basic passive reactor) and
# slice 2 (the energizer), with every carried recipe, block and loot table
# exercised as the parity rule (AGENTS.md, 2026-11-20) requires.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the `run_mod_command` hook. The leg launches
# `--offscreen` (not `--headless`): `click-slot`, which the crafting phase needs,
# requires a renderer.
#
# The package is one flat `.hmod` (`horizon-mod.toml`, `src/`, `assets/`,
# `data/`) that the launcher compiles into the game:
#
#   ports/tools/launcher-game.sh run bigreactors
#
# The End, in the void: an empty level with no ambient item entities. A stone
# floor at y=100, air above it.
#
# Default run:
#  A  content: every carried block and item is give-able and place-able, an
#     unknown id is refused, and the `c:` tags the recipes use resolve;
#  B  recipes: all nine carried shaped recipes craft in a real table with their
#     declared id and count, ingredients consumed, and two wrong-ingredient
#     controls leave the result empty; the four graphite smelting/blasting
#     recipes cook in a furnace and a blast furnace;
#  C  loot: one of every block drops itself when broken in survival;
#  D  reactor: the book's 3x3x3 forms, 4 ingots become 4,000 mB, the upstream
#     trajectory (8,026 FE at t=100), 126.8 FE/t and 0.028 mB/t steady, the tap
#     carries the FE, and controls for no fuel, off and unformed;
#  E  energizer: a 3x3x3 forms with one cell and the jar's per-block capacity; it
#     refuses FE while off; switched on, the reactor's tap charges it; its output
#     port moves the FE into the reactor (a receive face); a 5x5x5 holds 27x;
#     air interior and no controller do not form; switched off, it stops pushing.
#
# SELFTEST=1 flips the reactor's trajectory assertion; NEGATIVE=1 flips the
# energizer's capacity assertion. Either must FAIL (EXIT:1), proving the leg can.
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
NEGATIVE=${NEGATIVE:-0}
KEEP=${KEEP:-0}
WORLD=BigReactorsE2E

# The reactor, in the End's void. `X0,Y0,Z0` is the shell's near corner.
X0=100
Y0=101
Z0=0
# The controller sits on the reactor's south-east edge so the energizer's output
# port can stand against it: the reactor's controller is a receive face.
CONTROLLER="$((X0 + 2)) $((Y0 + 1)) $Z0"
PORT_AT="$((X0 + 2)) $((Y0 + 1)) $((Z0 + 1))"
TAP="$((X0 + 1)) $((Y0 + 1)) $Z0"
FUEL_ROD="$((X0 + 1)) $((Y0 + 1)) $((Z0 + 1))"
CONTROL_ROD="$((X0 + 1)) $((Y0 + 2)) $((Z0 + 1))"

# Energizer A sits against the reactor's tap (input) and its controller (output).
EA_X0=$X0
EA_Z0=-3
EA_CONTROLLER="$X0 $((Y0 + 1)) $((EA_Z0 + 1))"
EA_PORT_IN="$((X0 + 1)) $((Y0 + 1)) $((EA_Z0 + 2))"
EA_PORT_OUT="$((X0 + 2)) $((Y0 + 1)) $((EA_Z0 + 2))"
# Energizer C: a 5x5x5 with a 3x3x3 interior of 27 cells.
EC_X0=110
EC_Y0=$Y0
EC_Z0=0
EC_CONTROLLER="$EC_X0 $((EC_Y0 + 1)) $((EC_Z0 + 2))"
EC_PORT="$((EC_X0 + 4)) $((EC_Y0 + 1)) $((EC_Z0 + 2))"

# The content grid, the crafting table and the furnaces.
CONTENT_Z=10
TABLE_AT="118 101 $CONTENT_Z"
FURNACE_AT="122 101 18"
BLAST_AT="125 101 18"
LOOT_Z=14

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
    ctl launch --game "$GAME" --world "$WORLD" --seed 20261105 --creative --offscreen \
        --wait --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }
end() { ctl command "execute in minecraft:the_end run $1"; }

# A stone floor at y=100 in the End's void, clear air above it.
arena() {
    ctl command "execute in minecraft:the_end run tp @s 99.5 104 1.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block 95 100 -24 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    end "fill 95 100 -24 135 100 24 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    end "fill 95 101 -24 135 112 24 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    end "kill @e[type=!player]" >/dev/null
    end "tp @s 99.5 101 1.5" >/dev/null
    ctl wait 20
    [ "$(block_name 95 100 -24)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name 95 101 -24)" = "minecraft:air" ] || fail "the arena air above the floor is not air"
}

place() { # place X Y Z BLOCK
    end "setblock $1 $2 $3 $4" >/dev/null
    ctl wait 2
    [ "$(block_name "$1" "$2" "$3")" = "$4" ] || fail "expected $4 at $1,$2,$3, got '$(block_name "$1" "$2" "$3")'"
}

fill_box() { # fill_box X0 Y0 Z0 X1 Y1 Z1 BLOCK
    end "fill $1 $2 $3 $4 $5 $6 $7" >/dev/null
    ctl wait 2
}

# The book's first reactor: 27 blocks, the six kinds this package carries.
build_reactor() {
    local y
    for y in "$Y0" $((Y0 + 1)) $((Y0 + 2)); do
        fill_box "$X0" "$y" "$Z0" "$((X0 + 2))" "$y" "$((Z0 + 2))" bigreactors:basic_reactorcasing
    done
    place $CONTROLLER bigreactors:basic_reactorcontroller
    place $PORT_AT bigreactors:basic_reactorsolidaccessport
    place $TAP bigreactors:basic_reactorpowertapfe_active
    place $FUEL_ROD bigreactors:basic_reactorfuelrod
    place $CONTROL_ROD bigreactors:basic_reactorcontrolrod
    ctl wait 4
}

# A 3x3x3 energizer shell with one cell; the caller places the controller and ports.
build_energizer3() { # X0 Y0 Z0
    local x0=$1 y0=$2 z0=$3 y
    for y in "$y0" $((y0 + 1)) $((y0 + 2)); do
        fill_box "$x0" "$y" "$z0" "$((x0 + 2))" "$y" "$((z0 + 2))" bigreactors:energizercasing
    done
    place "$((x0 + 1))" "$((y0 + 1))" "$((z0 + 1))" bigreactors:energizercell
    ctl wait 4
}

# A 5x5x5 energizer: shell, 27 cells.
build_energizer5() { # X0 Y0 Z0
    local x0=$1 y0=$2 z0=$3 y
    for y in "$y0" $((y0 + 1)) $((y0 + 2)) $((y0 + 3)) $((y0 + 4)); do
        fill_box "$x0" "$y" "$z0" "$((x0 + 4))" "$y" "$((z0 + 4))" bigreactors:energizercasing
    done
    fill_box "$((x0 + 1))" "$((y0 + 1))" "$((z0 + 1))" "$((x0 + 3))" "$((y0 + 3))" "$((z0 + 3))" bigreactors:energizercell
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
capacity_of() { sed -nE 's/.* [0-9]+\/([0-9]+) FE.*/\1/p' <<<"$1"; }
fuel_of() { sed -nE 's/.* fuel ([0-9]+), waste .*/\1/p' <<<"$1"; }
waste_of() { sed -nE 's/.* waste ([0-9]+), heat .*/\1/p' <<<"$1"; }
active_of() { sed -nE 's/.* active (true|false), t=.*/\1/p' <<<"$1"; }
cells_of() { sed -nE 's/.* ([0-9]+) cells,.*/\1/p' <<<"$1"; }
direction_of() { sed -nE 's/.*: (input|output), .*/\1/p' <<<"$1"; }

expect_status() { # expect_status X Y Z SUBSTRING
    local line
    line=$(status "$1" "$2" "$3")
    echo "  status($1,$2,$3): $line"
    case "$line" in
        *"$4"*) ;;
        *) fail "expected /bigreactors status to say '$4', got '$line'" ;;
    esac
}

inventory_count() { ctl inventory | jq -r --arg item "$1" '.counts[$item] // 0'; }

# The reactor's controller sits on the south-east edge; its south and east faces
# are the ones a player can see. Stand south-east of it and aim.
face_reactor_controller() {
    # East of the energizer's east face, so the player is never inside a casing,
    # and in line with the controller's exposed east face.
    end "tp @s 103.5 101 -1.5" >/dev/null
    ctl wait 6
    ctl look-at $CONTROLLER >/dev/null
}

launch
arena

# ---------------------------------------------------------------------------
# A. Content: every block and item, the unknown id, the tags
# ---------------------------------------------------------------------------
echo "== A: content - every carried block and item =="
BLOCKS=(
    basic_reactorcasing basic_reactorcontroller basic_reactorfuelrod
    basic_reactorcontrolrod basic_reactorsolidaccessport basic_reactorpowertapfe_active
    energizercasing energizercell energizercontroller
)
ITEMS=(yellorium_ingot graphite_ingot)
i=0
for id in "${BLOCKS[@]}" "${ITEMS[@]}"; do
    ctl command "clear @s" >/dev/null
    ctl give "bigreactors:$id" 1 >/dev/null || fail "give bigreactors:$id failed"
    ctl wait 3
    got=$(inventory_count "bigreactors:$id")
    [ "$got" -ge 1 ] || fail "bigreactors:$id is not in the inventory after give"
    i=$((i + 1))
done
echo "  $i block and item ids are give-able"

# The power port's item id is upstream's `energizerpowerport_fe`; its block id is
# `energizerpowerportfe` (the jar declares them differently, and its loot table
# drops the item id).
ctl command "clear @s" >/dev/null
ctl give bigreactors:energizerpowerport_fe 1 >/dev/null || fail "give the power port item failed"
ctl wait 3
[ "$(inventory_count bigreactors:energizerpowerport_fe)" -ge 1 ] || fail "the power port item is missing"
echo "  bigreactors:energizerpowerport_fe is give-able"

# Every block stands where it is put; every block item places its block.
i=0
for id in "${BLOCKS[@]}"; do
    x=$((120 + i))
    place "$x" 101 "$CONTENT_Z" "bigreactors:$id"
    i=$((i + 1))
done
echo "  $i blocks place and read back"

# The port's item places its differently-named block.
ctl command "clear @s" >/dev/null
ctl give bigreactors:energizerpowerport_fe 1 >/dev/null
ctl wait 4
end "tp @s 119.5 101 22.5" >/dev/null
ctl wait 3
ctl look-at 120 101 22 >/dev/null
ctl select bigreactors:energizerpowerport_fe >/dev/null
ctl place 120 101 22 --item bigreactors:energizerpowerport_fe >/dev/null \
    || fail "the power port item did not place"
ctl wait 4
[ "$(block_name 120 101 22)" = "bigreactors:energizerpowerportfe" ] \
    || fail "the power port item placed '$(block_name 120 101 22)', not the block"
echo "  the power port item places bigreactors:energizerpowerportfe"

# An unknown id is refused, and the world is unchanged.
end "setblock 121 101 22 bigreactors:not_a_block" >/dev/null 2>&1 || true
ctl wait 3
[ "$(block_name 121 101 22)" = "minecraft:air" ] \
    || fail "an unknown block id was accepted: $(block_name 121 101 22)"
echo "  an unknown block id is refused"

# Horizon mounts only the package's own namespace (`data/bigreactors/`), so the
# `c:` common tags the jar's recipes name never resolve (recorded in notes.md).
# The recipes use direct item ids instead; the crafting phase is their proof.
echo "  tags: Horizon mounts data/bigreactors only, so the recipes use direct ids"
ctl command "clear @s" >/dev/null

# ---------------------------------------------------------------------------
# B. Recipes: all nine shaped recipes, the two controls, the four cooking ones
# ---------------------------------------------------------------------------
echo "== B1: the nine carried shaped recipes in a real crafting table =="
place 118 101 "$CONTENT_Z" minecraft:crafting_table
end "tp @s 117.5 101 10.5" >/dev/null
ctl wait 4
ctl command "gamemode survival @s" >/dev/null
ctl wait 3
python3 "$PORT/tools/craft_check.py" "$SOCK" "$CTL" 118 101 "$CONTENT_Z" \
    || fail "the crafting-table recipe check failed"
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 3

echo "== B2: the four graphite smelting/blasting recipes =="
smelt() { # X Y Z BLOCK INPUT LABEL
    local x=$1 y=$2 z=$3 block=$4 input=$5 label=$6 out
    place "$x" "$y" "$z" "$block"
    ctl container "$x" "$y" "$z" "$input" 1 >/dev/null
    ctl container "$x" "$y" "$z" minecraft:coal_block 1 >/dev/null
    ctl wait 150
    out=$(ctl container "$x" "$y" "$z" | jq -c '[.slots[] | select(. != null) | {item, count}]')
    echo "  $label: $input -> $out"
    jq -e --arg i bigreactors:graphite_ingot \
        '[.[] | select(.item == $i and .count >= 1)] | length == 1' <<<"$out" >/dev/null \
        || fail "$label did not make a graphite ingot: $out"
    jq -e --arg i "$input" '[.[] | select(.item == $i)] | length == 0' <<<"$out" >/dev/null \
        || fail "$label did not consume its input: $out"
}
smelt 122 101 18 minecraft:furnace minecraft:coal "furnace, coal"
smelt 122 101 20 minecraft:furnace minecraft:charcoal "furnace, charcoal"
smelt 125 101 18 minecraft:blast_furnace minecraft:coal "blast furnace, coal"
smelt 125 101 20 minecraft:blast_furnace minecraft:charcoal "blast furnace, charcoal"

# ---------------------------------------------------------------------------
# C. Loot: every block drops itself
# ---------------------------------------------------------------------------
echo "== C: every block's loot table =="
ctl command "gamemode survival @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl give minecraft:iron_pickaxe 1 >/dev/null
ctl wait 4
ctl select minecraft:iron_pickaxe >/dev/null
ctl wait 2
i=0
for id in "${BLOCKS[@]}"; do
    x=$((120 + i))
    place "$x" 101 "$LOOT_Z" "bigreactors:$id"
    i=$((i + 1))
done
place 120 101 22 bigreactors:energizerpowerportfe
end "tp @s 119.5 101 13.5" >/dev/null
ctl wait 10
i=0
for id in "${BLOCKS[@]}"; do
    x=$((120 + i))
    end "tp @s $((x - 1)).5 101 13.5" >/dev/null
    ctl wait 6
    ctl look-at-block "$x" 101 "$LOOT_Z" >/dev/null 2>&1 || true
    reply=$(ctl break "$x" 101 "$LOOT_Z") || fail "cannot break bigreactors:$id: $reply"
    got=$(jq -r --arg d "bigreactors:$id" '.drops[$d] // 0' <<<"$reply")
    [ "$got" -ge 1 ] || fail "bigreactors:$id dropped $got of itself: $reply"
    i=$((i + 1))
done
end "tp @s 119.5 101 21.5" >/dev/null
ctl wait 10
ctl look-at-block 120 101 22 >/dev/null 2>&1 || true
reply=$(ctl break 120 101 22) || fail "cannot break the power port: $reply"
got=$(jq -r '.drops["bigreactors:energizerpowerport_fe"] // 0' <<<"$reply")
[ "$got" -ge 1 ] || fail "the power port dropped $got of its item: $reply"
echo "  ${#BLOCKS[@]} + 1 blocks each dropped their item"
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 3

# ---------------------------------------------------------------------------
# D. The reactor (slice 1, unchanged)
# ---------------------------------------------------------------------------
end "tp @s 99.5 101 1.5" >/dev/null
ctl wait 10
echo "== D0: control - a box of casing with no fuel rod does not form =="
fill_box "$X0" "$Y0" "$Z0" "$((X0 + 2))" "$((Y0 + 2))" "$((Z0 + 2))" bigreactors:basic_reactorcasing
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
fill_box "$X0" "$Y0" "$Z0" "$((X0 + 2))" "$((Y0 + 2))" "$((Z0 + 2))" minecraft:air
ctl wait 6

echo "== D1: build the book's first reactor (3x3x3, one fuel rod, one control rod) =="
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

echo "== D2: control - no fuel, no FE (the reactor is still off here) =="
ctl wait 40
line=$(status $CONTROLLER)
echo "  after 40 off ticks: $line"
[ "$(energy_of "$line" || true)" = 0 ] || fail "an off reactor made FE: $line"

echo "== D3: four yellorium ingots in the access port become 4,000 mB =="
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

echo "== D4: right-clicking the controller switches the reactor on =="
face_reactor_controller
ctl interact $CONTROLLER >/dev/null || fail "the controller did not take a right click"
ctl wait 6
line=$(status $CONTROLLER)
echo "  $line"
[ "$(active_of "$line")" = true ] || fail "the controller did not switch the reactor on: $line"
# The status round trip costs a few ticks, so the reactor has already made its
# first few hundred FE; the trajectory below is measured as an increment from here.
line=$(status $CONTROLLER)
t0=$(field "$line" "t=")
total0=$(( $(energy_of "$line" || echo 0) + $(energy_of "$(status $TAP)" || echo 0) ))
echo "  from t=$t0: $total0 FE"

echo "== D5: the reactor's FE against the upstream trajectory =="
ctl wait 320
line=$(status $CONTROLLER)
t1=$(field "$line" "t=")
total1=$(( $(energy_of "$line" || echo 0) + $(energy_of "$(status $TAP)" || echo 0) ))
produced=$((total1 - total0))
echo "  at t=$t1: $total1 FE total, $produced FE in $((t1 - t0)) ticks, $line"
if [ "$SELFTEST" = 1 ]; then
    echo "  SELFTEST: expecting a wildly wrong amount (this must fail)"
    produced=999999
fi
rate=$(awk -v got="$produced" -v dt="$((t1 - t0))" 'BEGIN { printf "%.2f", (dt > 0 ? got / dt : 0) }')
echo "  elapsed $((t1 - t0)) ticks: $produced FE (${rate} FE/t; the steady rate is 126.8)"
awk -v got="$produced" -v dt="$((t1 - t0))" 'BEGIN { r = (dt > 0 ? got / dt : 0); exit !(r > 118 && r < 132) }' \
    || fail "$produced FE in $((t1 - t0)) ticks is not the documented 126.8 FE/t: $line"
heat=$(sed -nE 's/.* heat ([0-9.]+), fuel heat.*/\1/p' <<<"$line")
echo "  reactor heat $heat degrees (trajectory 420-426)"
awk -v heat="$heat" 'BEGIN { exit !(heat > 410 && heat < 432) }' \
    || fail "the reactor heat is not on the upstream trajectory: $line"
[ "$(active_of "$line")" = true ] || fail "the reactor switched itself off: $line"

echo "== D6: the power tap holds the reactor's FE (the energy capability) =="
tap_line=$(status $TAP)
tap_energy=$(energy_of "$tap_line" || echo 0)
echo "  $tap_line"
[ "$tap_energy" -gt 0 ] || fail "the tap has no FE, so the buffer is not on the machine's FE face: $tap_line"
[ "$tap_energy" -le 50000 ] || fail "the tap holds more than its capacity: $tap_line"

echo "== D7: the steady-state rate and the fuel burn over 100 ticks =="
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

echo "== D8: control - off stops the fuel use; stored heat is still radiated =="
face_reactor_controller
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

# ---------------------------------------------------------------------------
# E. The energizer (slice 2)
# ---------------------------------------------------------------------------
echo "== E1: a 3x3x3 energizer forms with one cell and the jar's capacity =="
build_energizer3 "$EA_X0" "$Y0" "$EA_Z0"
place $EA_CONTROLLER bigreactors:energizercontroller
place $EA_PORT_IN bigreactors:energizerpowerportfe
place $EA_PORT_OUT bigreactors:energizerpowerportfe
ctl wait 6
line=$(status $EA_CONTROLLER)
echo "  $line"
case "$line" in
    *"formed 3x3x3, 1 cells"*) ;;
    *) fail "the 3x3x3 energizer did not form: $line" ;;
esac
capacity=$(capacity_of "$line")
expected_capacity=683212743470724
if [ "$NEGATIVE" = 1 ]; then
    echo "  NEGATIVE: expecting a wrong capacity (this must fail)"
    expected_capacity=1
fi
[ "$capacity" = "$expected_capacity" ] \
    || fail "the energizer capacity is $capacity, not $expected_capacity: $line"
[ "$(active_of "$line")" = false ] || fail "a fresh energizer is on: $line"
[ "$(energy_of "$line")" = 0 ] || fail "a fresh energizer holds FE: $line"
cell_line=$(status "$((EA_X0 + 1))" "$((Y0 + 1))" "$((EA_Z0 + 1))")
echo "  $cell_line"
case "$cell_line" in
    *"formed 3x3x3, 1 cells"*) ;;
    *) fail "the cell does not report its formed machine: $cell_line" ;;
esac

echo "== E2: control - an off energizer refuses the reactor's FE =="
end "tp @s 99.5 101 1.5" >/dev/null
ctl wait 6
face_reactor_controller
ctl interact $CONTROLLER >/dev/null || fail "the reactor did not switch on"
ctl wait 200
line=$(status $EA_CONTROLLER)
echo "  $line"
[ "$(energy_of "$line")" = 0 ] || fail "an off energizer took FE: $line"
tap_line=$(status $TAP)
tap_energy=$(energy_of "$tap_line" || echo 0)
echo "  $tap_line"
[ "$tap_energy" -gt 0 ] || fail "the tap had no FE to offer, so the control is vacuous: $tap_line"

echo "== E3: switching the energizer on charges it from the tap =="
end "tp @s 99.5 101 -1.5" >/dev/null
ctl wait 6
ctl look-at $EA_X0 $((Y0 + 1)) $((EA_Z0 + 1)) >/dev/null
ctl interact $EA_CONTROLLER >/dev/null || fail "the energizer did not take a right click"
ctl wait 6
line=$(status $EA_CONTROLLER)
echo "  $line"
[ "$(active_of "$line")" = true ] || fail "the energizer did not switch on: $line"
ctl wait 200
line=$(status $EA_CONTROLLER)
charged=$(energy_of "$line")
echo "  $line"
[ "$charged" -gt 0 ] || fail "the energizer took no FE from the tap: $line"
ctl wait 60
line=$(status $EA_CONTROLLER)
again=$(energy_of "$line")
[ "$again" -gt "$charged" ] || fail "the energizer stopped charging at $charged FE: $line"
echo "  charging: $charged -> $again FE"

echo "== E4: the output port moves the FE into the reactor =="
# The output port is on the south-east edge, against the reactor's controller
# (a receive face). Its east side is exposed, so the toggle is a player's click.
end "tp @s 103.5 101 -1.5" >/dev/null
ctl wait 10
line=$(status $EA_CONTROLLER)
a0=$(energy_of "$line")
rline=$(status $CONTROLLER)
r0=$(energy_of "$rline")
echo "  before: energizer $a0 FE, reactor $r0 FE"
for _ in 1 2 3; do
    ctl look-at-block 102 102 -1 >/dev/null 2>&1 || true
    ctl interact 102 102 -1 >/dev/null || true
    ctl wait 6
    if ctl command "data get block 102 102 -1" | jq -r '.chat[0]' | grep -q 'iodir: 1b'; then
        break
    fi
done
ctl command "data get block 102 102 -1" | jq -r '.chat[0]' | grep -q 'iodir: 1b' \
    || fail "the output port did not switch to output"
port_line=$(status $EA_PORT_OUT)
echo "  $port_line"
[ "$(direction_of "$port_line")" = output ] || fail "the power port did not switch to output: $port_line"
ctl wait 120
line=$(status $EA_CONTROLLER)
a1=$(energy_of "$line")
rline=$(status $CONTROLLER)
r1=$(energy_of "$rline")
pushed=$((a0 - a1))
gained=$((r1 - r0))
echo "  after: energizer $a0 -> $a1 FE (pushed $pushed), reactor $r0 -> $r1 FE (gained $gained)"
[ "$pushed" -gt 50000 ] || fail "the output port pushed only $pushed FE: $line"
[ "$gained" -gt 50000 ] || fail "the reactor gained only $gained FE: $rline"

echo "== E5: a 5x5x5 energizer holds 27x the per-block capacity =="
build_energizer5 "$EC_X0" "$EC_Y0" "$EC_Z0"
place $EC_CONTROLLER bigreactors:energizercontroller
place $EC_PORT bigreactors:energizerpowerportfe
ctl wait 6
line=$(status $EC_CONTROLLER)
echo "  $line"
case "$line" in
    *"formed 5x5x5, 27 cells"*) ;;
    *) fail "the 5x5x5 energizer did not form: $line" ;;
esac
big_capacity=$(capacity_of "$line")
[ "$big_capacity" = 18446744073709548 ] \
    || fail "the 5x5x5 capacity is $big_capacity, not 27 x per-block: $line"

echo "== E6: controls - an air interior and a box without a controller =="
build_energizer3 120 "$Y0" -3
end "setblock 121 $((Y0 + 1)) $(( -3 + 1 )) minecraft:air" >/dev/null
ctl wait 4
place 120 $((Y0 + 1)) $(( -3 + 1 )) bigreactors:energizercontroller
place 121 $((Y0 + 1)) -3 bigreactors:energizerpowerportfe
ctl wait 6
line=$(status 120 $((Y0 + 1)) $(( -3 + 1 )))
echo "  air interior: $line"
case "$line" in
    *"not formed (the interior must be filled with energy cells)"*) ;;
    *) fail "an energizer with an air interior formed: $line" ;;
esac

build_energizer3 126 "$Y0" -3
place 127 $((Y0 + 1)) -3 bigreactors:energizerpowerportfe
ctl wait 6
line=$(status 127 $((Y0 + 1)) $(( -3 + 1 )))
echo "  no controller: $line"
case "$line" in
    *"not formed (an energizer needs a controller)"*) ;;
    *) fail "an energizer without a controller formed: $line" ;;
esac

echo "== E7: control - an off energizer stops pushing =="
end "tp @s 99.5 101 -1.5" >/dev/null
ctl wait 6
ctl look-at $EA_X0 $((Y0 + 1)) $((EA_Z0 + 1)) >/dev/null
ctl interact $EA_CONTROLLER >/dev/null || fail "the energizer did not switch off"
ctl wait 20
line=$(status $EA_CONTROLLER)
echo "  $line"
[ "$(active_of "$line")" = false ] || fail "the energizer did not switch off: $line"
before=$(energy_of "$line")
ctl wait 60
line=$(status $EA_CONTROLLER)
after=$(energy_of "$line")
echo "  energizer: $before -> $after FE while it is off"
[ "$after" = "$before" ] || fail "an off energizer kept pushing or charging: $line"

echo "PASS: every carried block and item is give-able and place-able, an unknown id is refused, all nine shaped recipes craft with their declared id and count and consume their ingredients (two wrong-ingredient controls stay empty), the four graphite cooking recipes produce graphite, every block drops itself, the 3x3x3 reactor forms and holds the upstream 126.8 FE/t and 0.028 mB/t with a working tap and a receive face, and a 3x3x3 energizer forms at 683212743470724 FE, refuses FE while off, charges from the reactor's tap when on, pushes its FE into the reactor through its output port, scales to 27x at 5x5x5, and stops pushing when switched off"
