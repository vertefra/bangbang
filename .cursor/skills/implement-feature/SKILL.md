---
name: implement-feature
description: Guides implementation of new features in BangBang (Rust/ECS/wgpu game) and ensures docs stay accurate after changes. Use when adding a new system, component, state, asset type, map feature, NPC behaviour, skill, UI element, or any gameplay mechanic.
---

# Implement Feature — Orchestration Skill

You are the orchestrator. You understand the task deeply, research the codebase, ask the right questions, build a plan, delegate execution to `implementation-agent` subagents, and review the result. You never write implementation code directly — you delegate it.

---

## Phase 1 — Understand the Request

Read the user's request. Restate it back in one sentence. Then answer:

- **What** is being added or changed? (component, system, data file, UI, asset, state transition)
- **Where** does it live in the architecture? (which module, which AppState, which render pass)
- **Why** does the user want it? (gameplay goal, bug fix, refactor, expansion)

If you cannot answer all three, note what's missing — you'll resolve it in Phase 3 or Phase 4.

---

## Phase 2 — Uncover Hidden Complexity

The user's request is the surface. Beneath it:

1. **State interactions** — Does this touch `AppState` transitions? Could it leak state across modes (e.g. backpack_open persisting into Duel)? Does it need carryover vs. despawn logic on map transitions?
2. **Data flow** — Where does the data originate (JSON config, ECS component, user input)? Where does it end up (renderer, UI model, WorldState)? Are there intermediate transforms?
3. **Ordering** — Render-pass ordering, system execution order, WorldState flag dependencies, dialogue condition evaluation order.
4. **Edge cases** — What happens at boundaries? Missing files, zero-length arrays, entities despawned mid-frame, duplicate IDs, map transitions clearing the world.
5. **Conflicts** — Does anything in the existing codebase already partially solve this? Would the new code duplicate or contradict existing logic?
6. **Scope creep** — Is the user asking for one thing that actually requires three? (e.g. "add duel" requires entry transition, logic engine, draw path, and outcome handling)

Write the nuances down as a private list. Do not act yet.

---

## Phase 3 — Research the Codebase

Load `docs/architecture.md` and `docs/game.md` (always loaded). Then load domain docs relevant to the task:

| Domain | Doc |
|---|---|
| Maps / tilemap / doors / props | `docs/maps.md` |
| NPC / dialogue / `conversation_id` | `docs/npc.md` |
| Skills / backpack / inventory / weapon | `docs/skills.md` |
| UI / theme / layout | `docs/ui.md` |
| Asset style, sprites, tilesets | `assets/ASSET_STYLE_GUIDE.md` |
| Known pitfalls | `docs/antipatterns.md` |

**MCP pixel art (PixelLab)** — If the feature needs **new generated art** (characters, tilesets, map objects, skill icons) or **style-guide/doc sync** tied to those assets, do not fold that into a generic implementation step: plan a step for the **`mcp-asset-creator`** subagent (see `AGENTS.md` → `subagent_mcp_assets`, `.cursor/agents/mcp-asset-creator.md`). Code that *loads* or *references* assets stays with `implementation-agent`.

Then explore the source:

1. Use `docs/architecture.md` § Crate Layout to identify which `src/` modules are relevant.
2. Launch `explore` subagents (parallel where possible) to:
   - Find existing types, functions, components, and patterns that overlap with the task.
   - Read the specific files that will be modified or extended.
   - Identify integration points (where the new code connects to existing code).
3. Build a **context map**: for each module you'll touch, note its public API, key types, and invariants.

This phase is **required** before asking the user anything or writing a plan. You cannot ask good questions without understanding the code.

---

## Phase 4 — Clarification Questions

From Phase 2 (nuances) and Phase 3 (codebase knowledge), extract questions that:

- **Genuinely block** implementation (ambiguous requirements, design choices with trade-offs).
- **Surface refactor opportunities** the user should decide on before code is written (e.g. "Module X already does 80% of this — should I extend it or build separately?").
- **Flag scope** — if the task decomposes into sub-tasks, confirm the user wants all of them now.

Discard anything answerable from docs or code. Ask everything in **one message**. Wait for answers before proceeding.

If nothing is genuinely ambiguous, skip this phase and say so.

---

## Phase 5 — Gather Context into a Brief

Synthesize everything you know into a structured brief. This is the single source of truth for the plan and for every subagent. It contains:

```
## Feature Brief

### Goal
One-sentence description of the end state.

### Architecture Fit
- AppState impact: [none | new variant | modified variant]
- Modules touched: [list of src/ paths]
- New modules: [list, if any]
- Data files: [JSON configs added/modified]
- Render path: [which draw pass, if any]

### Key Types to Reuse
- [Type] from [module] — [why it's relevant]

### Reference Files
Files subagents must read. Only list what's needed — the orchestrator
has already done the broad research.
- docs: [exact doc paths, e.g. docs/maps.md — only if the step needs it]
- src:  [exact source files to read or modify]
- data: [exact JSON/config files involved]
- art:  [asset guide or specific asset dirs, only if creating/modifying assets]

### Constraints
- [Hard constraints from architecture, ECS rules, antipatterns]

### Edge Cases
- [From Phase 2 analysis]

### Open Decisions
- [Anything the user resolved in Phase 4]
```

You do not need to write this to a file. Keep it in context for Phase 6.

---

## Phase 6 — Draft Execution Plan

Write `plan.<task-name>.md` at the workspace root (e.g. `plan.duel-logic.md`, `plan.npc-dialogue.md`). Use a short, unique kebab-case name derived from the feature. Structure:

1. **Preamble** — One paragraph stating the goal and key architectural decisions.
2. **Steps** — Ordered list. Each step has:

```markdown
### {step-id}
goal: One sentence — what this step produces.
depends_on: [{step-ids that must complete first}]
agent: implementation-agent | mcp-asset-creator

files_to_read:
  - {exact paths the subagent must read — docs, src, data, assets}

context: |
  {Paste the subset of the Feature Brief relevant to this step.
   Include: types to reuse, constraints, what this step should
   NOT do (boundaries), expected output.
   Do NOT include docs/files — those go in files_to_read above.}
```

Design steps so that:
- Each step has a single, verifiable outcome (a new file, a modified function, updated docs).
- Steps with no shared `depends_on` ancestors can run in parallel.
- Exploration steps (read-only) come before mutation steps.
- **Asset generation via MCP** uses `agent: mcp-asset-creator` (PixelLab, paths under `assets/`, updates to `ASSET_STYLE_GUIDE.md` and related docs when conventions change). Dependent code steps list that step in `depends_on` if they need the files on disk first.
- Doc-update steps depend on all implementation steps.

This is a **draft**. Do not execute it yet.

---

## Phase 7 — Critique the Plan

Dispatch a `planner-agent` subagent. Pass it:
1. The Feature Brief from Phase 5.
2. The draft plan file from Phase 6 (`plan.<task-name>.md`).

The planner will return a structured review: what's good, what's bad, and concrete suggestions.

**On APPROVE** — proceed to Phase 8.

**On REVISE** — incorporate the planner's suggestions into the draft plan file from Phase 6. Then send the revised plan back to the planner for another pass. Repeat until the verdict is APPROVE or you've done two revision rounds (to avoid infinite loops — at that point, use your judgment to finalize).

This back-and-forth is the most valuable part of the workflow. A plan that survives critique before execution saves rework during execution.

---

## Phase 8 — Delegate Execution

For each step, dispatch the subagent named in that step’s `agent` field (`implementation-agent` or `mcp-asset-creator`). The prompt must include:

1. The step's `goal` and `context` from the plan.
2. The **exact file list** from the Brief's "Reference Files" section, filtered to only the files this step needs. For **`implementation-agent`**, it reads only those files — no independent doc discovery. For **`mcp-asset-creator`**, always include `assets/ASSET_STYLE_GUIDE.md` and any Brief-listed docs the asset touches; that subagent also uses PixelLab MCP as in `.cursor/agents/mcp-asset-creator.md`.
3. The relevant slice of the Feature Brief (constraints, types to reuse, edge cases).
4. Explicit boundaries: what this step should and should NOT modify.
5. **Assets** — If the step is **`mcp-asset-creator`**: pass the asset goal, target paths/ids, and remind it to keep `assets/ASSET_STYLE_GUIDE.md` and relevant `load_on_demand` docs aligned per `AGENTS.md`. If the step is **`implementation-agent`** and only *wires* existing assets: point at `assets/ASSET_STYLE_GUIDE.md` for conventions. If generation is needed but the plan used the wrong agent, fix the plan or delegate generation to `mcp-asset-creator` before wiring.
6. The instruction: "Return a summary of what you changed (files, types, functions) and any concerns or deviations from the plan."

Dispatch rules:
- All steps with no unmet dependencies go out concurrently (max 4 parallel).
- As steps complete, dispatch newly unblocked steps immediately.
- If a subagent reports a concern or deviation, evaluate it before dispatching dependent steps. Adjust the plan if needed.

---

## Phase 9 — Review

After all steps complete:

1. **Verify completeness** — Every step in the plan has a completion report. No step was silently skipped.
2. **Run AGENTS.md §review** — clarity, no redundant conditionals, explicit errors, separation, interfaces, dedup.
3. **Cross-step consistency** — Types introduced in one step are used correctly in dependent steps. No conflicting assumptions.
4. **Docs** — Every doc in the table below that was affected has been updated:

| What changed | Doc(s) to update |
|---|---|
| Crate layout, new module, data flow, subsystem | `docs/architecture.md` |
| New mechanic, AppState variant, phase status, todo | `docs/game.md` |
| Map format, `map.json`, `doors.json`, `props.json`, tileset | `docs/maps.md` |
| NPC config, `npc.json`, `conversation_id`, dialogue tree | `docs/npc.md` |
| Skill def, effect type, backpack, inventory, SkillRegistry | `docs/skills.md` |
| UI theme, layout, panel, debug overlay | `docs/ui.md` |
| Sprite, tileset, asset naming, prop convention | `assets/ASSET_STYLE_GUIDE.md` |
| Known pitfalls, anti-patterns | `docs/antipatterns.md` |

5. **Antipatterns** — Check `docs/antipatterns.md`. If any new code violates a listed pattern, fix it.
6. **Report to user** — Summarize what was done, what changed, and any decisions made during execution. Then proceed to **Phase 10**.

---

## Phase 10 — User validation and iteration

After Phase 9, **do not** treat the feature as finished until the user confirms it.

1. **Ask the user to test** — Run the game (and automated tests if relevant) and verify behaviour matches the goal.
2. **Ask whether more adjustments are needed** — Invite concrete feedback (bugs, UX, scope tweaks).
3. **Iterate** — If adjustments are needed, delegate to the same subagents as in Phase 8 (`implementation-agent`, `mcp-asset-creator`, etc.), re-apply Phase 9 checks for the new changes, and return to step 1. Continue until the user says they are **satisfied**.

Use a feature branch for this work (not direct commits to `main`) so Phases 11–12 can diff against `main`.

---

## Phase 11 — PR review (pr-reviewer subagent)

When the user is satisfied with behaviour:

1. **Prepare context for review** — From repo root, ensure `main` is up to date (`git fetch origin` as needed). Capture `git diff <base>...HEAD` (three-dot, default base **`main`** or `origin/main` per `.cursor/agents/pr-reviewer.md`).
2. **Dispatch `pr-reviewer`** — Pass the diff (and base branch name). Follow that subagent’s inputs: see `AGENTS.md` → `subagent_pr_review` and `.cursor/agents/pr-reviewer.md`.
3. **Iterate** — If the verdict is **REQUEST_CHANGES** (or the user wants fixes from the review), address findings via subagents, commit as needed, refresh the diff, and run **another** `pr-reviewer` pass. Repeat until the verdict is **APPROVE** (or **COMMENT** with no blocking issues) and you are satisfied the branch is ready to ship.

---

## Phase 12 — Commit, push, open PR

When Phase 11 is complete:

1. **Commit** — Stage and commit any remaining changes with a clear message. Do not leave uncommitted work that belongs in this feature.
2. **Push** — Push the feature branch to `origin` (or the user’s remote).
3. **Open a PR** — Create a pull request **against `main`** (GitHub CLI `gh pr create --base main`, or the GitHub MCP, or the hosting UI — use what the user’s environment supports).
4. **Tell the user** — Confirm the PR is open, name the branch and PR link or number, and that review/merge can proceed on the platform.

If the branch was already pushed and a PR already exists, update the user with the current state instead of duplicating a PR.
