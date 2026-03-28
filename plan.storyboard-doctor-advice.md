# Plan: Storyboard Part 3 — Doctor’s Advice (Clinic tutorial)

Deliver the third beat from `story/0.TheDustfallIncident/storyboard.csv`: **Doc Sawbones** at the **Clinic** explains the **respawn / penalty** loop (5 LP, wake at clinic, gold cost, gear if broke). **No new AppState** — this is **data + one gameplay constant**: dialogue-driven tutorial on the existing overworld. Full duel-loss respawn logic remains future work (see `docs/game.md` Phase 5); the script sets player expectations only.

**Decisions baked in:** Player **starting** health is **5 / 5** Life Points so the spoken line matches mechanics and the HUD. NPCs keep the existing default (10 / 10). Doc is placed on **dustfall.junction** in front of the **clinic** prop (exterior-only town; no clinic interior map yet).

---

### player-hp-and-clinic-content

goal: Player spawns with 5/5 LP; Doc Sawbones exists on `dustfall.junction` with a full tutorial conversation matching the storyboard.

depends_on: []

agent: implementation-agent

files_to_read:
  - docs/npc.md
  - docs/game.md
  - src/ecs/world.rs
  - assets/dialogue/mom.json
  - assets/maps/dustfall.junction.map/npc.json
  - assets/maps/dustfall.junction.map/props.json
  - assets/npc/mom.npc/config.json

context: |
  Split player starting Health from NPC default in setup_world: introduce a constant e.g. DEFAULT_PLAYER_HEALTH { current: 5, max: 5 } and keep DEFAULT_ACTOR_HEALTH { 10, 10 } for NPC spawns only. Player branch uses carryover when present, else DEFAULT_PLAYER_HEALTH.
  Add assets/npc/docSawbones.npc/config.json (scale/color; no sheet required — solid sprite like other NPCs if no art).
  Add assets/dialogue/docSawbones.json: map npc id must match assets/npc/{id}.npc/ and assets/dialogue/{conversation_id}.json when conversation_id is omitted (defaults to id).
  Start node with lines paraphrasing the storyboard (Doc Sawbones: hold it, green, 5 LP, duel loss → wake here, gold for stitches, gear if broke). Use multi-line "lines" array like mom.json; speaker prefix on first line or each line as fits existing style.
  Set dustfall.junction.map/npc.json to one entry: id docSawbones, position on walkable street immediately south of clinic prop (clinic prop ~[544,496]; place doc so proximity interaction is clearly "at the clinic" — verify against map.json tile_size and walkable tiles).
  Refresh comments on Health constants in world.rs so they describe player vs NPC defaults and carryover (planner feedback).
  Run cargo check after Rust edits.
  Do not implement respawn-on-duel-loss or gold deduction — tutorial text only.
  Do not add interior clinic map.

---

### docs-update

goal: Document the new beat and LP default in player-facing docs.

depends_on: [player-hp-and-clinic-content]

agent: implementation-agent

files_to_read:
  - docs/game.md
  - docs/npc.md

context: |
  Update docs/game.md "Current state" (or roadmap): Dustfall Junction includes Doc Sawbones near the clinic prop; starting player LP is 5; tutorial dialogue explains respawn/penalty as design intent.
  Add a short npc.md note if useful (e.g. example id docSawbones) — only if it improves authoring; avoid redundant prose.

---
