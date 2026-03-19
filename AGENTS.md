# AGENTS.md

## context
load_always: [docs/architecture.md, docs/game.md]
load_on_demand: [assets/ASSET_STYLE_GUIDE.md, docs/ui.md]
load_on_demand_when: assets|art|style|ui

## exploration (pre_impl, required)
where: module|file|component for feature
reuse: existing types|fns|assets; no duplicate logic
from_scratch: only what has no equivalent

## review (post_impl, required)
clarity: readable; obvious naming/structure
checks: no redundant conditionals
fallbacks: no unnecessary; fail explicit when precond unmet
errors: propagate|log with context; never swallow
separation: clear responsibilities; no cross-layer leak
interfaces: no internal types|impl at public boundary
refactor: dedupe; consolidate; simplify

## rules
1. exploration before coding
2. review after coding
3. explicit_fail > silent|hidden
