# Carried content

> **SDK layout (2026-11-18):** these files live at the port root as
> `assets/bigreactors/` and `data/bigreactors/` (plus `data/minecraft/tags/`)
> and are **inside the one `.hmod`** with the code. The retired `.hnmod`
> builder is in `ports/_shared/evidence/bigreactors/retired-tools/`.

Everything here comes out of the ATM10 jar
`ExtremeReactors2-1.21.1-2.4.28.jar` (MIT). Its `neoforge.mods.toml` says
`version="1.21.1-2.4.9"`, so the artefact is 2.4.9 despite the file name: that is
the version whose energizer has one passive power port (`notes.md`).

The energizer's own assets are copied byte for byte by
`tools/carry_content.py`; `--check` (the default) compares every carried file's
SHA-256 with its jar entry and fails on a difference:

```sh
python3 tools/carry_content.py --check   # prove every carried file
python3 tools/carry_content.py --write   # (re)extract them from the jar
```

The data files (`data/`, `assets/bigreactors/items/`, `lang/`) are written by
hand in 26.3's shape; the carry script covers the blockstates, models, item
models and textures.

## What is carried

| kind | files |
|---|---|
| blocks | reactor: `basic_reactorcasing`, `basic_reactorcontroller`, `basic_reactorfuelrod`, `basic_reactorcontrolrod`, `basic_reactorsolidaccessport`, `basic_reactorpowertapfe_active`; energizer: `energizercasing`, `energizercell`, `energizercontroller`, `energizerpowerportfe` |
| items | `yellorium_ingot`, `graphite_ingot`, `energizerpowerport_fe` (the power port's item id; the block id has no underscore, as in the jar) |
| assets | the jar's blockstates and block models, their textures, 26.3 item definitions in `assets/bigreactors/items/`, `lang/en_us.json` (only the carried names), a creative tab |
| data | `horizon/block/*.json` (templates), `horizon/item/*.json`, the 10 block loot tables, 9 block recipes, 4 graphite smelting/blasting recipes, `data/minecraft/tags/block/mineable/pickaxe.json` and `needs_stone_tool.json` |

## Patches the carry applies (1.21.1 → 26.3)

- **Loot tables**: a pool's `"conditions": [{ "condition": "…" }]` becomes the
  singular `"condition": { "type": "…" }` (the vanilla 26.3 shape), and the
  `random_sequence` field is dropped.
- **Recipes**: `{"tag": "c:ingots/iron"}` becomes `"minecraft:iron_ingot"` (see
  below), and `{"item": "x"}` becomes `"x"` (26.3's ingredient form).
- **Item models**: the 1.21.1 `models/item/*.json` are kept as the model, and a
  26.3 item definition is added beside them in `assets/bigreactors/items/`.
- **Block templates** (the engine builds a mod block from a vanilla one):
  - casing/cell/fuel rod/control rod → `minecraft:iron_block` (a plain full
    block, no states, no block entity);
  - controller/access port/taps → `minecraft:beacon` (a block entity with a
    single empty blockstate variant).
  All keep upstream's hardness 3.0 / blast resistance 6.0 and the
  `minecraft:iron_block` sounds, and stack to 1.
- **Tool tags**: every carried block is in `minecraft:mineable/pickaxe` and
  `minecraft:needs_stone_tool`. Without the first, `requires_tool` makes a
  player's break harvest nothing (the loot phase caught it); the second matches
  the iron-block tier.

## Why the recipes use direct ids, not `c:` tags

The jar's recipes name NeoForge's common tags (`c:ingots/iron`,
`c:storage_blocks/gold`, `c:dusts/glowstone`, `c:sands/colorless`, …). Horizon
mounts **only the package's own namespace** as content (`data/bigreactors/`), so
those tags never resolve and the recipes cannot match. The port therefore
expresses the same ingredients with direct item ids:

| tag | direct id | difference |
|---|---|---|
| `c:ingots/iron`, `c:ingots/gold` | `minecraft:iron_ingot`, `minecraft:gold_ingot` | none |
| `c:storage_blocks/gold` | `minecraft:gold_block` | none |
| `c:dusts/glowstone`, `c:dusts/redstone` | `minecraft:glowstone_dust`, `minecraft:redstone` | none |
| `c:gems/diamond` | `minecraft:diamond` | none |
| `c:glass_blocks` | `minecraft:glass` | coloured glass is not accepted |
| `c:chests/wooden` | `minecraft:chest` | a trapped chest is not accepted |
| `c:sands/colorless` | `minecraft:sand` | red sand is not accepted |
| `c:ingots/graphite`, `c:ingots/yellorium` | `bigreactors:graphite_ingot`, `bigreactors:yellorium_ingot` | the port's own items |

## What is not carried, and why

- **`graphite_from_coalblock`** (smelting and blasting): their result
  `bigreactors:graphite_block` is not carried (it belongs to the ore/metal-chain
  gap), so the two recipes were dead data and are dropped.
- **The energizer's energy cell and energy core recipes**: they need
  `anglesite_crystal`, `benitoite_crystal` and the magentite block tag, all part
  of the ore/metal chain; the cell is give-able until that chain is ported.
- **Blockstates with properties** (the controller's on/off and facing, the
  access port's in/out, the fuel rod's `axis`): the slice keeps one state per
  block, so the state matrix those need is a gap.
- **The glass, charging ports, redstone port, computer port, creative water
  generator**, the reinforced variant, the turbine/reprocessor/fluidizer blocks,
  the fluids and the ores: other slices.
- **The Patchouli guide book** (`assets/bigreactors/patchouli_books/`):
  Patchouli is not ported.
