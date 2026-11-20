#!/usr/bin/env python3
"""Carry the energizer's assets out of the ATM10 jar, reproducibly.

Every file below is a byte-for-byte copy of the jar's entry: the energizer's
blockstates, block models, item models and the four textures they reference.
The data files (`data/`, `assets/bigreactors/items/`, `lang/`) are written by
hand in 26.3's shape and are not in this list.

    python3 tools/carry_content.py --check   # prove every carried file (default)
    python3 tools/carry_content.py --write   # (re)extract them from the jar

The jar is the pack's own `ExtremeReactors2-1.21.1-2.4.28.jar` (MIT), kept as
evidence under `ports/_shared/licence-cache/`. Its `neoforge.mods.toml` says
`version="1.21.1-2.4.9"`: the artefact wins over the file name, and 2.4.9's
energizer has one passive power port (see `notes.md`).
"""
import argparse
import hashlib
import pathlib
import sys
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
PORT = pathlib.Path(__file__).resolve().parents[1]
JAR = ROOT / "ports/_shared/licence-cache/ExtremeReactors2-1.21.1-2.4.28.jar"

# jar entry -> port path, relative to the port root
FILES = {
    "assets/bigreactors/blockstates/energizercasing.json": "assets/bigreactors/blockstates/energizercasing.json",
    "assets/bigreactors/blockstates/energizercell.json": "assets/bigreactors/blockstates/energizercell.json",
    "assets/bigreactors/blockstates/energizercontroller.json": "assets/bigreactors/blockstates/energizercontroller.json",
    "assets/bigreactors/blockstates/energizerpowerportfe.json": "assets/bigreactors/blockstates/energizerpowerportfe.json",
    "assets/bigreactors/models/block/energizer/casing_00_single.json": "assets/bigreactors/models/block/energizer/casing_00_single.json",
    "assets/bigreactors/models/block/energizercell.json": "assets/bigreactors/models/block/energizercell.json",
    "assets/bigreactors/models/block/energizer/controller.json": "assets/bigreactors/models/block/energizer/controller.json",
    "assets/bigreactors/models/block/energizer/powerportfe_input.json": "assets/bigreactors/models/block/energizer/powerportfe_input.json",
    "assets/bigreactors/models/item/energizercasing.json": "assets/bigreactors/models/item/energizercasing.json",
    "assets/bigreactors/models/item/energizercell.json": "assets/bigreactors/models/item/energizercell.json",
    "assets/bigreactors/models/item/energizercontroller.json": "assets/bigreactors/models/item/energizercontroller.json",
    "assets/bigreactors/models/item/energizerpowerport_fe.json": "assets/bigreactors/models/item/energizerpowerport_fe.json",
    "assets/bigreactors/textures/block/energizer/casing_single.png": "assets/bigreactors/textures/block/energizer/casing_single.png",
    "assets/bigreactors/textures/block/energizercell.png": "assets/bigreactors/textures/block/energizercell.png",
    "assets/bigreactors/textures/block/energizer/controller.png": "assets/bigreactors/textures/block/energizer/controller.png",
    "assets/bigreactors/textures/block/energizer/powerportfe_input.png": "assets/bigreactors/textures/block/energizer/powerportfe_input.png",
}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--write", action="store_true", help="extract the files from the jar")
    group.add_argument("--check", action="store_true", help="compare the port's files to the jar")
    args = parser.parse_args()

    if not JAR.is_file():
        print(f"missing jar: {JAR}", file=sys.stderr)
        return 1

    bad = 0
    with zipfile.ZipFile(JAR) as jar:
        for entry, relative in sorted(FILES.items()):
            try:
                expected = jar.read(entry)
            except KeyError:
                print(f"MISSING IN JAR: {entry}", file=sys.stderr)
                bad += 1
                continue
            target = PORT / relative
            if args.write:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(expected)
                print(f"wrote {relative} {digest(expected)[:16]}")
                continue
            if not target.is_file():
                print(f"MISSING IN PORT: {relative}", file=sys.stderr)
                bad += 1
                continue
            actual = target.read_bytes()
            if actual != expected:
                print(
                    f"DIFFERS: {relative} (jar {digest(expected)[:16]}, port {digest(actual)[:16]})",
                    file=sys.stderr,
                )
                bad += 1
    if args.write:
        print(f"carried {len(FILES)} files from {JAR.name}")
        return 0
    if bad:
        print(f"{bad} of {len(FILES)} carried files do not match the jar", file=sys.stderr)
        return 1
    print(f"{len(FILES)} carried files match {JAR.name} byte for byte")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
