# BangBang — Antipatterns & Architectural Rules

To maintain the scalability, predictability, and cleanliness of the BangBang engine, adhere to the following anti-patterns when adding new features. 

---

## 1. The "Silent Fallback" Antipattern
**What it is:** Catching `std::io::Error` or serialization failures internally during asset loading and substituting a hardcoded "default" value (e.g., returning an empty dialogue line when `read_to_string` fails).
**Why it's bad:** It masks missing files, typos, and syntax errors. Content creators won't know their JSON is broken because the game keeps running silently with missing data.
**The Right Way:** Functions that touch disk or parse data must return `Result<T, Error>`. Bubble the error up to `main().expect()` so the engine halts loudly and prints the path that failed. 

## 2. Spawning Static Content into ECS Components
**What it is:** Copying the contents of a JSON dialogue tree or an entire character biography string directly into a component when calling `world.spawn()`.
**Why it's bad:** The ECS is for mutable, high-performance runtime numbers (velocity, HP, equipped item IDs). Static strings waste memory via per-entity duplication and become out of sync if the master data changes.
**The Right Way:** The component should only store the String ID (e.g. `conversation_id: "mom_intro"`). Systems dynamically query the `AssetStore`, `StoryState`, or a cache to render the content when required.

## 3. Game Logic in the Render Pass (`draw_frame`)
**What it is:** Querying `hecs::World` inside the GPU rendering code to calculate what a UI string should say (e.g. iterating the backpack vector to build text lines), or applying damage.
**Why it's bad:** Rendering must be read-only and side-effect free. Mixing physics/formatting into the rendering loop breaks decoupling, blocks fixed-timestep physics features, and creates massive, untestable monolithic `.rs` files.
**The Right Way:** The `App::update()` loop calculates logical data models (e.g., building a `BackpackPanelLines` struct). `draw_frame()` takes this pre-computed formatting overlay as a simple argument to draw.

## 4. Expanding the `App` State with Global Bools 
**What it is:** Adding fields like `is_in_shop: bool` or `is_minigame_active: bool` directly to the `App` struct. 
**Why it's bad:** These boolean toggles exist outside the overarching `AppState` enum geometry, meaning logic starts leaking: you end up in `AppState::Duel` where `is_in_shop` is accidentally left `true`. 
**The Right Way:** Add new scopes into `AppState` (e.g., `AppState::Shop` or update the `Overworld` variant to have a defined inner-state enum). All views of the world must collapse into one unambiguous Enum. 

## 5. CPU UI Rendering (The Software Canvas)
**What it is:** Drawing interactive menus by mutating `&mut [u32]` pixel buffers piecemeal, outside the GPU.
**Why it's bad:** Disjointed graphics pipelines. Changing screen scales causes pixel crunch, fonts do not map to the GPU atlas smoothly, and it diverges rendering implementations. 
**The Right Way:** All UI rendering must go through the grouped `wgpu` sub-batches initialized in `gpu::renderer` utilizing `ui::layout` geometry and `ui::theme` colors. 

## 6. Hardcoded Asset Paths
**What it is:** Typing `env!("CARGO_MANIFEST_DIR")` or `"/assets/"` in new subsystems.
**Why it's bad:** As the binary changes scopes or enters packaging / distribution logic, fixing raw strings across 10 distinct files is impossible.
**The Right Way:** Use `crate::paths::asset_root().join("foo")` exclusively.
