# BangBang — Asset Style Guide

## Mood & reference
- **Mood**: Pokémon-like — warm, readable, slightly cartoon; characters and props read clearly at a glance.
- **Setting**: Far West (Bang! style) — dusty towns, saloons, desert, cowboys; not grim, keep it light and adventurous.

## Art type
- **Pixel art** only. No anti-aliasing on edges; clean pixel boundaries. Optional 1px black or dark outline for characters/UI if it improves readability.

## Resolution & frames
- **Target frame size**: 96×96 px (final quality).
- **Prototyping**: Smaller frames (e.g. 32×32, 48×48) are fine; keep aspect and layout consistent so assets can be swapped to 96×96 later.

## Environment
- **Far West**: Desert, sand, wooden buildings, tumbleweeds, cacti, saloons, dusty roads, corrals.
- **Tiles**: Mostly **sandy** — dirt/sand ground, occasional stone or wood (boardwalks, porches). Grass only as rare accent (oasis, edge of town).

## Color palette

Palette is limited and shared across tiles, characters, and props for a cohesive look.

| Role        | Hex       | Use |
|------------|-----------|-----|
| Sand light | `#E8D4A8` | Main ground, paths |
| Sand mid   | `#C9B896` | Sand variation, shadows on ground |
| Sand dark  | `#A68B5E` | Sand shadows, dirt |
| Dust/earth | `#8B7355` | Dark dirt, tracks |
| Wood light | `#D4A574` | Planks, beams |
| Wood mid   | `#8B6914` | Wood shadow, leather |
| Wood dark  | `#5C4A2E` | Dark wood, saloon trim |
| Sky        | `#87CEEB` | Default sky (warm blue) |
| Sky warm   | `#B8D4E8` | Dawn/dusk tint |
| Foliage    | `#6B8E23` | Cacti, rare grass (muted) |
| Accent     | `#C41E3A` | Bandana, signs, UI highlight |
| Accent 2   | `#2E5A1C` | Felt, cloth, secondary UI |
| Skin mid   | `#E8C4A0` | Skin base |
| Skin shadow| `#C9A87A` | Skin shade |
| Black/line | `#2C2416` | Outlines, deep shadow (optional) |
| White      | `#F5F0E6` | Highlights, eyes, teeth |

**Rules**: Prefer 3–5 colors per sprite/surface. Reuse palette colors; avoid new one-off hues. Slightly warm and dusty overall; avoid cold blues or neon.

## Tiles
- **Ground**: Sandy tones (Sand light/mid/dark, Dust) with small variation (cracks, pebbles, tracks) to avoid flat look.
- **Collision**: Mark impassable tiles in map data; art can use Wood dark, stone, or fences to read as solid.
- **Tile size**: Match map cell size (e.g. 32×32 or 48×48 for prototype; 96×96 for final). One tile = one cell.

## Buildings (structures)
- **Camera**: **High top-down** (same as ground tilesets and map objects). Vertical walls show a sliver of roof; door reads on the **south** face unless the map calls for another facing.
- **Size**: Building footprints may differ. A clinic can be smaller than a bank or saloon; variation in width/height is fine when it matches the intended gameplay footprint and street importance.
- **Scale**: Width/height in pixels should be a **multiple of the map `tile_size`** (e.g. 32) so one building lines up with collision rectangles. Prototype: match footprint in tiles (e.g. 11×8 cells → 352×256 px art at 32 px/tile). Different building sizes are acceptable; inconsistent **pixel density** is not.
- **Materials**: **Wood** frontier construction — planks and beams use Wood light/mid/dark (`#D4A574`, `#8B6914`, `#5C4A2E`); trim against sand uses Sand light/mid. Stone civic buildings (bank, jail) can add Dust/`#8B7355` and cool gray sparingly; still keep the warm dusty overall read.
- **Roof**: Simple pitched roof, slightly darker than walls; avoid neon or saturated reds except small Accent accents (signage only).
- **Readability**: Clear silhouette at gameplay zoom; door and porch readable in **3–5** major tones per surface; same outline rule as characters (all outlined or all lineless per area).
- **Integration**: Deliver as a **single PNG**; engine prefers `assets/props/{id}.prop/sheet.png` + `sheet.json` (`rows`/`cols`, usually `1`×`1`) for map props, and `assets/props/{id}.door/` for door props. **Walkable yard / paths must be transparent pixels** — the map tilemap draws the ground and owns collision. Opaque pixels in the prop sit on top: if you paint sand into the PNG, it is **not** walkable in any special way (and can hide the player); cut it away or match `map.json` walkable tiles and rely on transparency so the tilemap shows through. Collision stays in `map.json`. Place `doors.json` rects on walkable tiles at the real door/steps.

### Building style specification
- **Town building perspective**: Use one shared projection for a building set. For Dustfall-style street buildings, the target is a **south-facing facade with a shallow roof reveal**. Avoid mixing that with steep three-quarter or isometric-looking roof masses in the same street row.
- **Pixel density**: Doors, windows, signs, boards, stairs, and trim should be painted at the same apparent pixel scale across buildings. A larger building should have **more repeated modules**, not tinier pixels.
- **Side walls**: Keep visible side walls minimal and consistent. Show them only when placement really needs a corner read, and keep the angle/shading language aligned with the rest of the set.
- **Ground treatment**: Do **not** bake a large ground rectangle, yard patch, or full backdrop into one building when neighboring buildings rely on transparent surroundings. Porch steps, posts, awnings, and tight contact shadows are fine; the surrounding terrain should usually stay transparent so the map tilemap remains the ground.
- **Shadow direction**: Use one light direction for the whole set. Exterior buildings should cast shadows the same way; do not mix left-cast and right-cast shadows within one town frontage.
- **Line/outline treatment**: Keep outline weight and contrast consistent. Do not mix very heavy black contouring on one building with soft outline-less rendering on the next unless the whole area uses that treatment.
- **Signage**: Signs can differ in width and wording, but should share a common treatment for border thickness, letter sizing, and readability at gameplay zoom.
- **Surface detail**: Match texture complexity across the set. Do not place one building with dense plank/grain/window detail next to flatter simplified buildings unless you intentionally restyle the full set to that higher detail level.

### Building review checklist
- **Allowed**: Different footprints, different heights, different frontage widths, different numbers of windows/doors.
- **Must match across a set**: projection, pixel density, outline treatment, shadow direction, ground transparency treatment, and overall detail level.
- **Fail examples**: one building includes a painted sand backdrop while others are transparent; one uses a steep angled roof while the rest are facade-first; one uses much finer pixels for boards/signs than the rest.

## Characters & props
- **Silhouette**: Clear and readable at target resolution; cowboy hat, bandana, gun belt, etc. should read at a glance.
- **Animation**: Idle + walk (4 or 8 directions if needed). Frame count and layout per `sheet.json` (rows/cols).
- **Consistency**: Same pixel scale as tiles; same outline style (all with or all without).

### World scale (player, NPCs, PixelLab)
The overworld draws each frame at **world size** ≈ **`frame_width` × `scale.x`** and **`frame_height` × `scale.y`**, where **`scale`** comes from **`assets/npc/{id}.npc/config.json`** (see [docs/npc.md](../docs/npc.md)). The **player** uses **48×48** px frames at **`scale` 1.0** — that is the **reference** for human-scale actors on the ground.

- **Match NPCs to the player**: use **~48×48** frames with **`"scale": [1.0, 1.0]`** (same as `mom`, `bandit`), **or** larger source art with a **smaller** scale so the product stays ~48 px tall. Example: PixelLab **`create_character`** with **`size`: 96** → set **`"scale": [0.5, 0.5]`** so \(96 × 0.5 = 48\) world pixels wide per frame.
- **Rule of thumb**: \(\texttt{scale} \approx 48 / \texttt{frame\_width}\) when you want the same on-screen height as the player (adjust for deliberate tall/short characters).
- **Tiles**: Map cells are often **32** px; a ~**48** px-tall character reads as “one figure” next to buildings — avoid shipping **96×96** frames at **`scale` 1.0** unless you intentionally want a giant.

## Prop and asset ids (naming)
- **Reuse generic ids** for furniture, clutter, and anything that could appear in multiple interiors or towns: `bed`, `table`, `dresser`, `stove`, `cactus`, `barrels` — not character- or quest-tied names like `mumBed`.
- **Named landmark props** (unique buildings tied to one place) may use a descriptive id (`billyHouse`, `dustfallSaloon`) when a generic name would be misleading. When in doubt for small reusable props, stay generic even if only one map references them today.

## Top-down furniture vs isometric generators
- Interior props must match the **high top-down** camera used with wang interior tilesets. Tools that only emit **isometric cubes** (e.g. PixelLab MCP `create_isometric_tile`) usually read as **crates/blocks**, not beds or dressers — use orthographic top-down art instead.

## File layout
- **Player only**: `assets/characters/player/` — `sheet.png`, `sheet.json` (not an NPC; no `npc.json` entry).
- **NPCs and scene characters** (any id with `assets/npc/{id}.npc/config.json` or placed in `npc.json`): **`assets/npc/{id}.npc/`** — `config.json`, optional `sheet.png`, `sheet.json`, `portrait.png`. Legacy **`assets/npc/{id}.npc.json`** (single file) is still supported with a deprecation warning; prefer the folder layout.
- **Dialogue portraits** (`assets/npc/{id}.npc/portrait.png`): optional **bust** image for the dialogue panel (see [docs/npc.md](../docs/npc.md)). **128×128** RGBA matches existing **`mom`**; avoid reusing the overworld **down** frame as the portrait—it reads as a miniature full-body icon. PixelLab **`create_map_object`** (e.g. **128×128**, description: head-and-shoulders, transparent bg) is a workable fit for character busts.
- **Tiles**: `assets/tiles/` (or per-map tile sets as used by map loader).
- **Maps**: `assets/maps/{id}.map/` — `map.json`, `npc.json`, optional `props.json`; art in same folder or shared tiles.
- **Props (buildings, large objects)**: `assets/props/{id}.prop/` — `sheet.png`, `sheet.json` (grid). Referenced from map `props.json` by `id`. Use **generic** functional names for reusable props (`bed`, `dresser`); camelCase for multi-word ids (`billyHouse` for a named structure only when it is a specific landmark).
- **Map doors (transitions)**: `doors.json` field **`prop`** → `assets/props/{id}.door/` by convention (for example `"south"` → `assets/props/south.door/`, `"southHeavy"` → `assets/props/southHeavy.door/`). Use **`"none"`** or omit the field for no sprite when the doorway is already drawn on a building prop or tiles. Same palette/outline rules as buildings.
- **Skills (including weapons)**: `assets/skills/{id}.skill/` — **`config.json`** (`SkillDef`; weapons use `subcategory: "weapon"`), optional **`skill_image.png`** (96×96 RGBA, transparent background) for the backpack icon. Same folder-per-id pattern as `assets/npc/{id}.npc/`.

---

*Summary: Pokémon-like pixel art, Far West setting, sandy tiles, 96×96 target (smaller OK for prototype), limited warm dusty palette.*
