#!/usr/bin/env python3
"""Carry the reactor slice's content out of the ATM10 jar into `content/`.

The jar is the evidence (`ports/_shared/licence-cache/ExtremeReactors2-…jar`,
MIT); this script is reproducible: running it again rewrites the same files, so
`content/` can be audited or extended a block at a time. It also patches the
1.21.1 shapes into 26.3 ones, which is why the carried files are not byte copies
of the jar's:

- loot pools: `"conditions": [{"condition": …}]` → `"condition": {"type": …}`
  (the plural form evaluates false in 26.3 and a block would drop nothing);
- recipes: `{"tag": "c:ingots/iron"}` → `"#c:ingots/iron"`, `{"item": "x"}` → `"x"`;
- item models become 26.3 item definitions in `assets/<ns>/items/`.

Usage: `python3 tools/carry_content.py [--check]`
"""
import json
import pathlib
import shutil
import sys
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
JAR = ROOT.parent / "_shared" / "licence-cache" / "ExtremeReactors2-1.21.1-2.4.28.jar"
CONTENT = ROOT / "content"
NS = "bigreactors"

BLOCKS = {
    "basic_reactorcasing": ("minecraft:iron_block", "Reactor Casing (Basic)"),
    "basic_reactorcontroller": ("minecraft:beacon", "Reactor Controller (Basic)"),
    "basic_reactorfuelrod": ("minecraft:iron_block", "Reactor Fuel Rod (Basic)"),
    "basic_reactorcontrolrod": ("minecraft:iron_block", "Reactor Control Rod (Basic)"),
    "basic_reactorsolidaccessport": ("minecraft:beacon", "Reactor Solid Access Port (Basic)"),
    "basic_reactorpowertapfe_active": (
        "minecraft:beacon",
        "Reactor Active Forge Energy Power Tap (Basic)",
    ),
}

ITEMS = {
    "yellorium_ingot": "minecraft:diamond",
    "graphite_ingot": "minecraft:coal",
}

# Model files the blocks' blockstates name, plus their textures.
MODELS = [
    "models/block/reactor/basic/casing_00_single.json",
    "models/block/reactor/basic/controller.json",
    "models/block/reactor/basic/fuelrod.json",
    "models/block/reactor/fuel_rod.json",
    "models/block/reactor/basic/controlrod.json",
    "models/block/reactor/basic/accessport_solid_in.json",
    "models/block/reactor/basic/powertap_fe_active.json",
]
TEXTURES = [
    "textures/block/reactor/basic/casing_single.png",
    "textures/block/reactor/basic/controller.png",
    "textures/block/reactor/basic/fuelrod_cap.png",
    "textures/block/reactor/basic/fuelrod_vertical.png",
    "textures/block/reactor/basic/controlrod.png",
    "textures/block/reactor/basic/accessport_solid_in.png",
    "textures/block/reactor/basic/powertap_fe_active.png",
    "textures/item/yellorium_ingot.png",
    "textures/item/graphite_ingot.png",
]
RECIPES = [
    "reactor/basic/casing",
    "reactor/basic/controller_ingots_yellorium",
    "reactor/basic/controlrod",
    "reactor/basic/fuelrod_ingots_yellorium",
    "reactor/basic/solidaccessport",
    "reactor/basic/activetap_fe",
    "smelting/graphite_from_coal",
    "smelting/graphite_from_charcoal",
    "smelting/graphite_from_coalblock",
    "blasting/graphite_from_coal",
    "blasting/graphite_from_charcoal",
    "blasting/graphite_from_coalblock",
]
TAGS = ["data/c/tags/item/ingots/graphite.json", "data/c/tags/item/ingots/yellorium.json"]

MANIFEST = """# Slice 1 of the Extreme Reactors port: the basic passive reactor.
# Assets and data are carried from the ATM10 jar (ExtremeReactors2-1.21.1-2.4.28.jar);
# licence MIT (ZeroNoRyouki/ExtremeReactors2), local-only until the maintainer
# approves publishing.
id = "bigreactors"
name = "Extreme Reactors"
version = "0.1.0"
description = "Basic passive reactor: casing, controller, fuel rod, control rod, solid access port, active FE power tap (slice 1)"
authors = ["ZeroNoRyouki"]
api = "0.2"
side = "both"
capabilities = ["registries"]
"""

BLOCK_PROPERTIES = {
    "hardness": 3.0,
    "explosion_resistance": 6.0,
    "sounds": "minecraft:iron_block",
    "requires_tool": True,
}


def write(relative, data):
    path = CONTENT / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    if isinstance(data, bytes):
        path.write_bytes(data)
    elif isinstance(data, str):
        path.write_text(data)
    else:
        path.write_text(json.dumps(data, indent=2) + "\n")


def patch_loot(table):
    for pool in table.get("pools", []):
        if "conditions" in pool:
            pool["condition"] = [
                {"type": condition["condition"]} for condition in pool.pop("conditions")
            ][0]
        for entry in pool.get("entries", []):
            if "conditions" in entry:
                entry["condition"] = [
                    {"type": condition["condition"]} for condition in entry.pop("conditions")
                ][0]
    return table


def patch_recipe(recipe):
    for key, value in list(recipe.get("key", {}).items()):
        if isinstance(value, dict):
            if "tag" in value:
                recipe["key"][key] = "#" + value["tag"]
            elif "item" in value:
                recipe["key"][key] = value["item"]
    ingredient = recipe.get("ingredient")
    if isinstance(ingredient, dict):
        if "tag" in ingredient:
            recipe["ingredient"] = "#" + ingredient["tag"]
        elif "item" in ingredient:
            recipe["ingredient"] = ingredient["item"]
    return recipe


def carry():
    if not JAR.is_file():
        raise SystemExit(f"missing jar: {JAR}")
    with zipfile.ZipFile(JAR) as jar:
        write("horizon-mod.toml", MANIFEST)
        for name, (template, title) in BLOCKS.items():
            write(
                f"assets/{NS}/blockstates/{name}.json",
                json.loads(jar.read(f"assets/{NS}/blockstates/{name}.json")),
            )
            write(
                f"assets/{NS}/items/{name}.json",
                {"model": {"type": "minecraft:model", "model": f"{NS}:item/{name}"}},
            )
            write(
                f"assets/{NS}/models/item/{name}.json",
                {"parent": f"{NS}:block/" + block_model(name)},
            )
            write(
                f"data/{NS}/horizon/block/{name}.json",
                {
                    "template": template,
                    "properties": dict(BLOCK_PROPERTIES),
                    "item": True,
                    "components": {"minecraft:max_stack_size": 1},
                },
            )
            write(
                f"data/{NS}/loot_table/blocks/{name}.json",
                patch_loot(json.loads(jar.read(f"data/{NS}/loot_table/blocks/{name}.json"))),
            )
        for name, template in ITEMS.items():
            write(
                f"assets/{NS}/items/{name}.json",
                {"model": {"type": "minecraft:model", "model": f"{NS}:item/{name}"}},
            )
            write(
                f"assets/{NS}/models/item/{name}.json",
                json.loads(jar.read(f"assets/{NS}/models/item/{name}.json")),
            )
            write(
                f"data/{NS}/horizon/item/{name}.json",
                {"template": template, "rarity": "common"},
            )
        for model in MODELS:
            write(f"assets/{NS}/{model}", json.loads(jar.read(f"assets/{NS}/{model}")))
        for texture in TEXTURES:
            write(f"assets/{NS}/{texture}", jar.read(f"assets/{NS}/{texture}"))
        for recipe in RECIPES:
            write(
                f"data/{NS}/recipe/{recipe}.json",
                patch_recipe(json.loads(jar.read(f"data/{NS}/recipe/{recipe}.json"))),
            )
        for tag in TAGS:
            write(tag, json.loads(jar.read(tag)))
        lang = json.loads(jar.read(f"assets/{NS}/lang/en_us.json"))
        keys = {f"block.{NS}.{name}": title for name, (_, title) in BLOCKS.items()}
        keys |= {
            f"item.{NS}.yellorium_ingot": lang[f"item.{NS}.yellorium_ingot"],
            f"item.{NS}.graphite_ingot": lang[f"item.{NS}.graphite_ingot"],
            f"itemGroup.{NS}": lang.get(f"itemGroup.{NS}", "Extreme Reactors"),
        }
        write(f"assets/{NS}/lang/en_us.json", keys)
        write(
            f"data/{NS}/horizon/creative_tab/reactor.json",
            {
                "title": f"itemGroup.{NS}",
                "icon": f"{NS}:basic_reactorcontroller",
                "items": [f"{NS}:{name}" for name in BLOCKS] + [f"{NS}:{name}" for name in ITEMS],
            },
        )


def block_model(name):
    return {
        "basic_reactorcasing": "reactor/basic/casing_00_single",
        "basic_reactorcontroller": "reactor/basic/controller",
        "basic_reactorfuelrod": "reactor/basic/fuelrod",
        "basic_reactorcontrolrod": "reactor/basic/controlrod",
        "basic_reactorsolidaccessport": "reactor/basic/accessport_solid_in",
        "basic_reactorpowertapfe_active": "reactor/basic/powertap_fe_active",
    }[name]


if __name__ == "__main__":
    if "--check" in sys.argv:
        print(f"jar: {JAR} ({'present' if JAR.is_file() else 'MISSING'})")
        print(f"content: {CONTENT}")
    carry()
    print(f"carried {len(list(CONTENT.rglob('*.json')))} json files and "
          f"{len(list(CONTENT.rglob('*.png')))} textures into {CONTENT}")
