# Carried content

> **SDK layout (2026-11-18):** these files now live at the port root as
> `assets/bigreactors/` and `data/bigreactors/` + `data/c/` and are **inside the one
> `.hmod`** with the code. The builder below is retired
> (`ports/_shared/evidence/bigreactors/retired-tools/carry_content.py`); the tables
> are kept as the record of what was carried and from where.

Everything here comes out of the ATM10 jar
`ExtremeReactors2-1.21.1-2.4.28.jar` (MIT) and was **regenerated** by
`tools/carry_content.py` — never hand-edited, so a later slice can extend it and
a reviewer can diff the script instead of 69 files.

```sh
python3 tools/carry_content.py     # (retired) rewrote content/ from the jar
bash tools/package.sh              # (retired) built content/bigreactors.hmod
```

## What is carried

| kind | files |
|---|---|
| blocks | `basic_reactorcasing`, `basic_reactorcontroller`, `basic_reactorfuelrod`, `basic_reactorcontrolrod`, `basic_reactorsolidaccessport`, `basic_reactorpowertapfe_active` |
| items | `yellorium_ingot`, `graphite_ingot` |
| assets | the jar's blockstates and block models (the fuel rod's `reactor/fuel_rod` parent included), 7 block textures and 2 item textures, 26.3 item definitions in `assets/bigreactors/items/`, `lang/en_us.json` (only this slice's names), a creative tab |
| data | `horizon/block/*.json` (templates), `horizon/item/*.json`, the 6 block loot tables, 6 block recipes, 6 graphite smelting/blasting recipes, the `c:ingots/yellorium` and `c:ingots/graphite` tags |

## Patches the carry applies (1.21.1 → 26.3)

- **Loot tables**: a pool's `"conditions": [{ "condition": "…" }]` becomes the
  singular `"condition": { "type": "…" }`. The plural form silently evaluates
  false in 26.3 and the block drops nothing.
- **Recipes**: `{"tag": "c:ingots/iron"}` becomes `"#c:ingots/iron"` and
  `{"item": "x"}` becomes `"x"` (26.3's ingredient form).
- **Item models**: the 1.21.1 `models/item/*.json` are kept as the model, and a
  26.3 item definition is added beside them in `assets/bigreactors/items/`.
- **Block templates** (the engine builds a mod block from a vanilla one):
  - `basic_reactorcasing`, `basic_reactorfuelrod`, `basic_reactorcontrolrod` →
    `minecraft:iron_block` (a plain full block, no states, no block entity);
  - `basic_reactorcontroller`, `basic_reactorsolidaccessport`,
    `basic_reactorpowertapfe_active` → `minecraft:beacon` (a block entity with a
    single empty blockstate variant, so the jar's one-variant blockstate is
    enough). The mod's own block entity replaces the vanilla one; the container
    and the FE buffer are declared by the code.
  All six keep upstream's hardness 3.0 / blast resistance 6.0 and the
  `minecraft:iron_block` sounds, and stack to 1.

## What is not carried, and why

- **Blockstates with properties** (the controller's on/off and facing, the
  access port's in/out, the fuel rod's `axis`): the slice keeps one state per
  block, so the state matrix those need is a gap, not a silent simplification.
- **The glass, charging ports, redstone port, computer port, creative water
  generator**, the reinforced variant, the turbine/reprocessor/energizer/
  fluidizer blocks, the fluids and the ores: other slices.
- **The Patchouli guide book** (`assets/bigreactors/patchouli_books/`):
  Patchouli is not ported.
- **The `c:ingots/uranium`, `c:gems/diamond`, `c:dusts/redstone`,
  `c:sands/colorless`, `c:glass_blocks` and `c:chests/wooden` tags** the recipes
  name: they are vanilla/common tags in the pack. A tag that does not resolve is
  an empty ingredient, so the affected recipe silently cannot match — the six
  block recipes are recorded as working only where those tags exist.
