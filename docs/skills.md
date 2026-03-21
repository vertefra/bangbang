# Skills

This document explains the current skill system in `bangbang`.

## Goal

Skills are data-driven definitions loaded from `assets/skills/*.json`.
Runtime state (HP, inventory, charges) stays in ECS components.

## Data model

Each skill file maps to `SkillDef` and contains:

- `id`: unique identifier
- `name`: UI label
- `category`: `permanent` or `usable`
- `subcategory`: free-form classifier (for example `weapon` or `consumable`)
- `charges_default`: required for usable skills, ignored for permanent
- `effects`: list of effect steps

Each effect step contains:

- `op`: `deal_damage` or `heal`
- `target`: `caster` or `opponent`
- `amount`: non-negative value

Current built-in skills:

- `sidearm` (`permanent` + `weapon`) -> `deal_damage` to `opponent`
- `beer` (`usable` + `consumable`) -> `heal` on `caster`

## Loading and validation

`SkillRegistry::load_from_dir()` natively discovers all ids from the `assets/skills/*.json` folder by reading file stems.

`SkillDef::load()` validates:

- file id matches requested id
- `effects` is not empty
- each `amount` is non-negative
- usable skills must set `charges_default > 0`

Invalid files return an explicit error.

## Runtime ECS state

Definitions are not duplicated on entities. Entities keep only mutable state:

- `Health { current, max }`
- `Backpack`
  - `permanent: Vec<String>` skill ids
  - `usable: Vec<UsableSkillStack { skill_id, charges }>`
  - `equipped_weapon_id: Option<String>` — must be a `permanent` id whose def has `subcategory == "weapon"`; used for hotkey **1**

`seed_demo_backpack()` seeds the player with:

- permanent: `sidearm`
- `equipped_weapon_id`: `sidearm`
- usable: `beer` with charges from `charges_default`

## Effect resolution

`apply_skill(skill, world, caster, opponent)` iterates `effects`.

For each effect:

1. Resolve target by role (`caster` or `opponent`)
2. Borrow target `Health`
3. Apply operation:
   - `deal_damage`: `current = max(0, current - amount)`
   - `heal`: `current = min(max, current + amount)`

Missing `Health` fails with an explicit error.

## Overworld controls (current demo wiring)

- `B` toggles backpack overlay (Overworld)
- When backpack is open:
  - `Tab` / `Shift+Tab` cycles the **equipped weapon** among permanent skills with `subcategory == "weapon"` (no-op if fewer than two weapons)
  - `1` uses the **equipped weapon** on the nearest NPC in range (invalid/missing equipped id is normalized to the first weapon in `permanent` order; logs if there is no weapon)
  - `2` uses the first usable skill (`beer`) on the player

While the backpack is open, `normalize_equipped_weapon` runs each frame so equipped state stays consistent with `permanent`.

Usable skill charges are decremented and the entry is removed when it reaches `0`.

## Backpack UI grouping

Permanent skills are split for display and hotkey **1** by `SkillDef.subcategory`:

- **`weapon`** — listed under **Weapons**; one row is highlighted as equipped (`equipped_weapon_id`)
- **anything else** — listed under **Passives** (not fired by hotkey **1**)

Helpers: `weapon_ids_in_order`, `passive_ids_in_order`, `normalize_equipped_weapon`, `cycle_equipped_weapon_in_backpack` in `src/skills/backpack_view.rs`; `cycle_equipped_weapon(world, …)` and panel text via `ui::backpack::backpack_panel_lines`.

## ECS boundary

Recommended pattern (current implementation):

- Keep skill **definitions** in data files and a shared registry
- Keep per-entity **state** in ECS components
- Run skill **application** in systems/functions that mutate ECS components

This avoids duplicating static definitions across entities and keeps balancing fully data-driven.
