#!/usr/bin/env python3
"""Craft every shaped recipe the package carries, in a real crafting table.

Called by `tools/e2e.sh` with the open game's control socket. The harness's
crafting table exposes the 3x3 grid as slots 100-108 (row-major) and the result
as 200 (`ctl screen`). For each recipe this:

1. resets the table and clears the inventory;
2. gives exactly the pattern's ingredients;
3. places them with right-clicks (one item per slot);
4. asserts the result's **id and count**;
5. shift-clicks the result into the inventory and asserts it arrived;
6. asserts the grid is empty and the distinctive ingredient was consumed.

Two deliberately wrong patterns must leave the result empty. Exits 1 on the
first mismatch, printing the observed result.

    python3 tools/craft_check.py SOCK CTL X Y Z
"""
import json
import subprocess
import sys

# The crafting table's grid, row-major, and its result.
GRID = {
    "tl": 100, "tm": 101, "tr": 102,
    "ml": 103, "mm": 104, "mr": 105,
    "bl": 106, "bm": 107, "br": 108,
}
RESULT = 200


def main():
    sock, ctl, x, y, z = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]

    def run(*args):
        out = subprocess.run([ctl, "--socket", sock, *args], text=True, capture_output=True)
        try:
            return json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")

    def wait(ticks=4):
        run("wait", str(ticks))

    def slots():
        screen = run("screen")
        if "slots" not in screen:
            raise SystemExit(f"no menu is open: {screen}")
        return screen["slots"]

    def find(item):
        for slot in slots():
            if slot.get("item") == item:
                return slot["slot"]
        raise SystemExit(f"missing ingredient {item} in the inventory")

    def result():
        stack = [s for s in slots() if s["slot"] == RESULT]
        if not stack or not stack[0].get("item"):
            return None
        return stack[0]

    def grid_is_empty():
        return all(not s.get("item") for s in slots() if 100 <= s["slot"] <= 108)

    def count(item):
        return run("inventory").get("counts", {}).get(item, 0)

    def place(item, targets):
        # One pickup per target: a max-stack-1 ingredient (a casing) cannot fill
        # four slots from one held stack, while a stack of four can.
        for target in targets:
            slot = find(item)
            run("click-slot", str(slot))
            wait(1)
            run("click-slot", str(target), "right")
            wait(1)
            run("click-slot", str(slot))
            wait(1)

    def setup(ingredients):
        try:
            run("close")
        except SystemExit:
            pass
        wait(4)
        # Replace the table so its block entity, and therefore the grid, is fresh.
        run("setblock", x, y, z, "minecraft:air")
        run("setblock", x, y, z, "minecraft:crafting_table")
        wait(2)
        run("command", f"tp @s {float(x) - 0.5} {y} {float(z) + 0.5}")
        wait(3)
        run("command", "clear @s")
        for item, n in ingredients:
            run("give", item, str(n))
        wait(3)
        run("interact", x, y, z)
        wait(5)

    failures = []

    def check(label, ingredients, placements, want_item, want_count, consumed_item):
        setup(ingredients)
        for item, targets in placements:
            place(item, targets)
        got = result()
        print(f"{label} -> {json.dumps(got)}")
        if not got or got.get("item") != want_item or got.get("count") != want_count:
            grid = [s for s in slots() if 100 <= s["slot"] <= 108]
            print(f"  grid: {json.dumps(grid)}")
            failures.append(f"{label}: expected {want_count} {want_item}, got {got}")
            return
        run("click-slot", str(RESULT), "shift")
        wait(4)
        made = count(want_item)
        if made < want_count:
            failures.append(f"{label}: the crafted stack reached the inventory as {made}")
        if not grid_is_empty():
            failures.append(f"{label}: the grid still holds ingredients after the craft")
        left = count(consumed_item)
        if left != 0:
            failures.append(f"{label}: {consumed_item} was not consumed ({left} left)")

    def empty_control(label, ingredients, placements):
        setup(ingredients)
        for item, targets in placements:
            place(item, targets)
        got = result()
        print(f"{label} -> {json.dumps(got)}")
        if got is not None:
            failures.append(f"{label}: the control crafted {got['item']} x{got['count']}")

    def every(*slots):
        return list(slots)

    # --- the reactor's six -------------------------------------------------
    check(
        "reactor casing: 4 iron + 4 graphite + 1 sand",
        [("minecraft:iron_ingot", 4), ("bigreactors:graphite_ingot", 4), ("minecraft:sand", 1)],
        [
            ("minecraft:iron_ingot", every(100, 102, 106, 108)),
            ("bigreactors:graphite_ingot", every(101, 103, 105, 107)),
            ("minecraft:sand", every(104)),
        ],
        "bigreactors:basic_reactorcasing", 1, "minecraft:iron_ingot",
    )
    check(
        "reactor controller: 4 casings + comparator + 2 yellorium + diamond + redstone",
        [
            ("bigreactors:basic_reactorcasing", 4),
            ("minecraft:comparator", 1),
            ("bigreactors:yellorium_ingot", 2),
            ("minecraft:diamond", 1),
            ("minecraft:redstone", 1),
        ],
        [
            ("bigreactors:basic_reactorcasing", every(100, 102, 106, 108)),
            ("minecraft:comparator", every(101)),
            ("bigreactors:yellorium_ingot", every(103, 105)),
            ("minecraft:diamond", every(104)),
            ("minecraft:redstone", every(107)),
        ],
        "bigreactors:basic_reactorcontroller", 1, "bigreactors:basic_reactorcasing",
    )
    check(
        "reactor control rod: 4 casings + redstone + 2 iron + piston + graphite",
        [
            ("bigreactors:basic_reactorcasing", 4),
            ("minecraft:redstone", 1),
            ("minecraft:iron_ingot", 2),
            ("minecraft:piston", 1),
            ("bigreactors:graphite_ingot", 1),
        ],
        [
            ("bigreactors:basic_reactorcasing", every(100, 102, 106, 108)),
            ("minecraft:redstone", every(101)),
            ("minecraft:iron_ingot", every(103, 105)),
            ("minecraft:piston", every(104)),
            ("bigreactors:graphite_ingot", every(107)),
        ],
        "bigreactors:basic_reactorcontrolrod", 1, "bigreactors:basic_reactorcasing",
    )
    check(
        "reactor fuel rod: 4 iron + 2 graphite + 2 glass + yellorium",
        [
            ("minecraft:iron_ingot", 4),
            ("bigreactors:graphite_ingot", 2),
            ("minecraft:glass", 2),
            ("bigreactors:yellorium_ingot", 1),
        ],
        [
            ("minecraft:iron_ingot", every(100, 102, 106, 108)),
            ("bigreactors:graphite_ingot", every(101, 107)),
            ("minecraft:glass", every(103, 105)),
            ("bigreactors:yellorium_ingot", every(104)),
        ],
        "bigreactors:basic_reactorfuelrod", 1, "bigreactors:yellorium_ingot",
    )
    check(
        "reactor solid access port: 4 casings + hopper + 2 iron + chest + piston",
        [
            ("bigreactors:basic_reactorcasing", 4),
            ("minecraft:hopper", 1),
            ("minecraft:iron_ingot", 2),
            ("minecraft:chest", 1),
            ("minecraft:piston", 1),
        ],
        [
            ("bigreactors:basic_reactorcasing", every(100, 102, 106, 108)),
            ("minecraft:hopper", every(101)),
            ("minecraft:iron_ingot", every(103, 105)),
            ("minecraft:chest", every(104)),
            ("minecraft:piston", every(107)),
        ],
        "bigreactors:basic_reactorsolidaccessport", 1, "bigreactors:basic_reactorcasing",
    )
    check(
        "reactor active tap: 4 casings + 4 redstone blocks + redstone",
        [
            ("bigreactors:basic_reactorcasing", 4),
            ("minecraft:redstone_block", 4),
            ("minecraft:redstone", 1),
        ],
        [
            ("bigreactors:basic_reactorcasing", every(100, 102, 106, 108)),
            ("minecraft:redstone_block", every(101, 103, 105, 107)),
            ("minecraft:redstone", every(104)),
        ],
        "bigreactors:basic_reactorpowertapfe_active", 1, "bigreactors:basic_reactorcasing",
    )

    # --- the energizer's three --------------------------------------------
    check(
        "energizer casing: 6 iron + 2 redstone blocks + gold block",
        [("minecraft:iron_ingot", 6), ("minecraft:redstone_block", 2), ("minecraft:gold_block", 1)],
        [
            ("minecraft:iron_ingot", every(100, 102, 103, 105, 106, 108)),
            ("minecraft:redstone_block", every(101, 107)),
            ("minecraft:gold_block", every(104)),
        ],
        "bigreactors:energizercasing", 1, "minecraft:iron_ingot",
    )
    check(
        "energizer controller: 4 casings + comparator + 2 glowstone + torch + gold",
        [
            ("bigreactors:energizercasing", 4),
            ("minecraft:comparator", 1),
            ("minecraft:glowstone", 2),
            ("minecraft:redstone_torch", 1),
            ("minecraft:gold_ingot", 1),
        ],
        [
            ("bigreactors:energizercasing", every(100, 102, 106, 108)),
            ("minecraft:comparator", every(101)),
            ("minecraft:glowstone", every(103, 105)),
            ("minecraft:redstone_torch", every(104)),
            ("minecraft:gold_ingot", every(107)),
        ],
        "bigreactors:energizercontroller", 1, "bigreactors:energizercasing",
    )
    check(
        "energizer power port: 4 casings + 2 iron + 2 glowstone dust + redstone block",
        [
            ("bigreactors:energizercasing", 4),
            ("minecraft:iron_ingot", 2),
            ("minecraft:glowstone_dust", 2),
            ("minecraft:redstone_block", 1),
        ],
        [
            ("bigreactors:energizercasing", every(100, 102, 106, 108)),
            ("minecraft:iron_ingot", every(101, 107)),
            ("minecraft:glowstone_dust", every(103, 105)),
            ("minecraft:redstone_block", every(104)),
        ],
        "bigreactors:energizerpowerport_fe", 1, "minecraft:redstone_block",
    )

    # --- controls: a wrong ingredient leaves the result empty -------------
    empty_control(
        "energizer casing with a stick where the gold block goes",
        [("minecraft:iron_ingot", 6), ("minecraft:redstone_block", 2), ("minecraft:stick", 1)],
        [
            ("minecraft:iron_ingot", every(100, 102, 103, 105, 106, 108)),
            ("minecraft:redstone_block", every(101, 107)),
            ("minecraft:stick", every(104)),
        ],
    )
    empty_control(
        "reactor casing with a stick where the sand goes",
        [("minecraft:iron_ingot", 4), ("bigreactors:graphite_ingot", 4), ("minecraft:stick", 1)],
        [
            ("minecraft:iron_ingot", every(100, 102, 106, 108)),
            ("bigreactors:graphite_ingot", every(101, 103, 105, 107)),
            ("minecraft:stick", every(104)),
        ],
    )

    run("close")
    wait(3)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        raise SystemExit(1)
    print("recipe PASS: all nine carried shaped recipes craft with their declared id and count, "
          "their ingredients are consumed, and both wrong-ingredient controls leave the result empty")


if __name__ == "__main__":
    main()
