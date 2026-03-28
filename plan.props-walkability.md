# plan.props-walkability — Prop vs walkable collision audit

**Goal:** For every map with `props.json`, ensure collision in `map.json` matches prop placement: small props sit on walkable tiles; building wall rects cover the prop sprite bounding box in tile space on north/east/west (and roof), leaving an extra southern row walkable only when the sheet uses transparent porch/facade pixels (per [docs/maps.md](docs/maps.md)).

**Audit result:** Only `dustfall.junction.map` needed a data fix — the **saloon** wall rect was two tiles too short on the **north** side relative to `sheet.png` size and `position`. Interior mumhome maps are consistent.

## Steps

### junction-saloon-wall-doc
goal: Extend saloon wall rect in `dustfall.junction.map/map.json` to cover sprite rows y=8–16; add a short authoring note in `docs/maps.md` under props/collision; normalize stray indent in `mumhome.secondFloor.map/map.json` `tiles.rects`.
depends_on: []
agent: implementation-agent

files_to_read:
  - docs/maps.md
  - assets/maps/dustfall.junction.map/map.json
  - assets/maps/dustfall.junction.map/props.json
  - assets/maps/mumhome.secondFloor.map/map.json

context: |
  Saloon prop: position [1456, 400], scale [1,1], sheet 352×256 (world half-extents 176×128, tile_size 32).
  Sprite tile AABB: x 40–50, y 8–16. Current wall rect `{ "id": 1, "x": 40, "y": 10, "w": 11, "h": 7 }` misses y=8,9 (walkable under opaque art).
  Change to `{ "id": 1, "x": 40, "y": 8, "w": 11, "h": 9 }` so rows 8–16 block. Do not change other rects or prop positions.
  Other junction buildings: southern row y=17 inside sprite AABB is intentionally walkable (porch/street); do not extend those rects without art review.
  docs/maps.md: add one concise bullet (props + collision) explaining N/E/W coverage vs optional transparent south; no large rewrite.
  mumhome.secondFloor map.json: fix indentation of `rects` under `tiles` only.
  Do NOT change Rust code. Run `cargo check` after edits if you touch anything beyond JSON/docs (not required for JSON-only).
