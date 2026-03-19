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

## Characters & props
- **Silhouette**: Clear and readable at target resolution; cowboy hat, bandana, gun belt, etc. should read at a glance.
- **Animation**: Idle + walk (4 or 8 directions if needed). Frame count and layout per `sheet.json` (rows/cols).
- **Consistency**: Same pixel scale as tiles; same outline style (all with or all without).

## File layout
- **Characters**: `assets/characters/{id}/` — `sheet.png`, `sheet.json`.
- **NPCs**: `assets/npc/{id}.npc/` or `assets/npc/{id}.npc.json` plus sprites as referenced.
- **Tiles**: `assets/tiles/` (or per-map tile sets as used by map loader).
- **Maps**: `assets/maps/{id}.map/` — `map.json`, `npc.json`; art in same folder or shared tiles.

---

*Summary: Pokémon-like pixel art, Far West setting, sandy tiles, 96×96 target (smaller OK for prototype), limited warm dusty palette.*
