# Plugins and project organization

## Contents
- The plugin trait — struct or fn-as-plugin form
- Plugin lifecycle — `build`, `ready`, `finish`, `cleanup` phases
- Configuration: resources vs plugin fields — when to use which
- Plugin groups — for libraries; `PluginGroup::set` for overrides
- Plugin uniqueness — `is_unique()` for multi-instance plugins
- Plugin ordering and dependencies — duck typing pattern
- System sets and centralized ordering — the canonical `SystemSet` enum pattern
- Project structure — by-feature beats by-file-kind; folder → crate progression
- Visibility — `pub(crate)` default; reasons to take it seriously
- Project growth example — from `main.rs` to multi-crate workspace

The `Plugin` trait is the unit of composition in Bevy. Use it aggressively — one plugin per feature is the standard idiom.

## The plugin trait

```rust
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DamageDealt>()
           .add_message::<DeathOccurred>()
           .add_observer(check_death_threshold)
           .add_systems(Update, (apply_damage, drop_loot).chain().in_set(GameSet::Combat));
    }
}
```

Then in `main.rs` (or wherever you build the `App`):

```rust
App::new()
    .add_plugins((DefaultPlugins, CombatPlugin, MovementPlugin, UiPlugin))
    .run();
```

`build` takes `&self`, so the plugin can hold config:

```rust
pub struct CombatPlugin {
    pub use_observer_routing: bool,
}

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        if self.use_observer_routing {
            app.add_observer(/* ... */);
        } else {
            app.add_systems(Update, /* ... */);
        }
    }
}
```

For zero-config plugins, the function-as-plugin form is shorter:

```rust
fn movement_plugin(app: &mut App) {
    app.add_systems(Update, (apply_velocity, friction).chain());
}

App::new().add_plugins(movement_plugin);
```

Functions taking `&mut App` are blanket-impl'd as `Plugin`. Use this when you don't need a struct; switch to a struct when you eventually need configuration or state on the plugin.

## Plugin lifecycle

When `App::run` starts, it goes through phases:

1. `Plugin::build` for each plugin (when `add_plugins` is called).
2. `Plugin::ready` is polled until all plugins return `true` (default: always `true`).
3. `Plugin::finish` for each plugin (deferred init that needs other plugins to have built).
4. `Plugin::cleanup` for each plugin.
5. The runner function takes over and starts the game loop.

Most plugins only need `build`. `finish` is occasionally useful for renderer subsystems that need other plugins' resources to exist (although 0.17 introduced the `RenderStartup` schedule, which is now the recommended path — it lets renderer init happen as ordinary systems in `build`).

## Configuration: resources vs plugin fields

Default to **resources** for plugin config:

```rust
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CombatSettings>()
           .add_systems(Update, apply_damage);
    }
}

#[derive(Resource)]
pub struct CombatSettings {
    pub crit_chance: f32,
    pub damage_multiplier: f32,
}

impl Default for CombatSettings {
    fn default() -> Self { Self { crit_chance: 0.1, damage_multiplier: 1.0 } }
}
```

The user tunes the resource at runtime, sees its value in any inspector, and serializes it for save games — none of that works if the value lives on the plugin struct.

Use **plugin struct fields** only for things that *only* make sense at construction time:

- Which schedule the plugin's systems should land in.
- Whether to register a particular set of plugins (e.g., a `dev_tools` flag).
- One-time setup paths (filesystem locations, embedded asset roots).

If you find yourself adding a plugin field for something that might change at runtime, make it a resource instead.

## Plugin groups

For plugins that ship as a unit but should remain individually toggleable, use a `PluginGroup`:

```rust
pub struct GamePlugins;

impl PluginGroup for GamePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(CombatPlugin)
            .add(MovementPlugin)
            .add(UiPlugin)
    }
}

App::new()
    .add_plugins(GamePlugins.build().disable::<UiPlugin>())  // disable one
    .run();
```

`PluginGroupBuilder::set` lets users override the configuration of a contained plugin without forking your code:

```rust
DefaultPlugins.set(WindowPlugin {
    primary_window: Some(Window { title: "My Game".into(), ..default() }),
    ..default()
})
```

For application code, a parent plugin adding child plugins via `add_plugins` is fine — it's only libraries that should prefer `PluginGroup` (so users can opt out cleanly).

## Plugin uniqueness

By default, adding the same plugin type twice panics. To allow multiple instances:

```rust
impl Plugin for ConfigurablePlugin {
    fn build(&self, app: &mut App) { /* ... */ }
    fn is_unique(&self) -> bool { false }
}
```

Useful when the same plugin might be added with different configs (e.g., one `RenderPipelinePlugin<MyMaterial>` per material type).

## Plugin ordering and dependencies

Plugins build in the order they're added. There's no formal dependency declaration — if `B` needs a resource `A` adds, you need to add `A` before `B`.

The "least bad" workaround for "plugin X requires plugin Y" is duck typing: in `B::build`, check whether `A`'s resources exist, and either bail with an error or fall back gracefully. There's been a long-running RFC for proper plugin dependencies; not landed yet.

## System sets and centralized ordering

The single most important architecture decision in a Bevy project: **declare your ordering in one place**. Pattern:

```rust
// src/schedule.rs
#[derive(SystemSet, Clone, Eq, PartialEq, Hash, Debug)]
pub enum GameSet {
    // PreUpdate phase
    InputGather,
    ClockTick,

    // Update phase
    AiBrain,
    Combat,
    Locomotion,
    CameraFollow,

    // PostUpdate phase
    Animation,
    UpdateHud,
}

// src/app.rs
fn configure_schedule(app: &mut App) {
    app.configure_sets(PreUpdate, (GameSet::InputGather, GameSet::ClockTick).chain());
    app.configure_sets(Update, (
        GameSet::AiBrain,
        GameSet::Combat,
        GameSet::Locomotion,
        GameSet::CameraFollow,
    ).chain());
    app.configure_sets(PostUpdate, (GameSet::Animation, GameSet::UpdateHud).chain());
}
```

Then plugins drop their systems into named sets:

```rust
impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (apply_damage, check_death).chain().in_set(GameSet::Combat));
    }
}
```

**Key rule:** plugins should never call `configure_sets`. That's the app's job. If two plugins both configure the same set, the relative ordering becomes a function of plugin add order, which is fragile and surprising.

For sub-phases within a set:

```rust
#[derive(SystemSet, Clone, Eq, PartialEq, Hash, Debug)]
pub enum CombatPhase {
    PreDamage,
    ApplyDamage,
    PostDamage,
}

app.configure_sets(Update, (
    CombatPhase::PreDamage,
    CombatPhase::ApplyDamage,
    CombatPhase::PostDamage,
).chain().in_set(GameSet::Combat));
```

This nests the combat sub-phases inside the `Combat` set, which itself is ordered relative to other top-level sets.

In 0.17 Bevy renamed its own internal sets to a `*Systems` suffix (`PickSet` → `PickingSystems`, `Animation` → `AnimationSystems`, `GizmoRenderSystem` → `GizmoRenderSystems`, etc.). Following the same convention in your own code is recommended for consistency.

## Project structure

The community-converged advice: **organize by feature, not by file kind.**

```
src/
├── main.rs                 # App build, plugin registration
├── app.rs                  # build_app(), set ordering
├── schedule.rs             # SystemSet enum
├── state.rs                # AppState, sub-states
├── combat/
│   ├── mod.rs              # CombatPlugin, public API
│   ├── components.rs       # Health, Armor
│   ├── messages.rs         # DamageDealt
│   └── systems.rs          # apply_damage, check_death
├── movement/
│   └── ...
└── ui/
    └── ...
```

Avoid:

```
src/
├── components/
│   ├── combat.rs
│   ├── movement.rs
│   └── ui.rs
├── systems/
│   ├── combat.rs
│   ├── movement.rs
│   └── ui.rs
```

The "by kind" layout doubles the navigation work and forces feature changes to touch multiple folders. By-feature keeps related code colocated and supports the plugin-per-folder convention.

When a folder gets large, recurse: `src/combat/{plugin,components,systems,messages}.rs`. When a feature is mature and stable, consider extracting it to its own crate — Cargo parallelizes compilation at the crate level, and incremental builds skip unchanged crates entirely.

A common workspace shape for non-trivial Bevy projects:

```
Cargo.toml          # workspace root
crates/
├── core/           # pure logic (no Bevy deps) — fast tests
└── ...
src/                # main game crate (Bevy plugins)
tools/              # supporting binaries (CLIs, asset processors)
```

Splitting pure logic into a non-Bevy-depending crate lets `cargo test -p core` link without the Bevy graph (much faster).

## Visibility

Default to `pub(crate)`. Pure `pub` for items that are part of the plugin's public API (the plugin struct itself, public components, public messages). Private for internal implementation.

Reasons to take visibility seriously even in a small game:

- Rust can't dead-code-detect `pub` items. Restricting visibility lets the compiler prune unused functions.
- Restricting access enforces invariants: if a component's fields are private, only methods can mutate them, and you can guarantee invariants like `current ≤ max`.
- Makes refactoring safer: changing a private item is local.

Don't `pub` reflexively. If something's only used inside the plugin, keep it module-local.

## Project growth example

A typical project's evolution:

1. **Single `main.rs`** — fine for a few hundred lines.
2. **Top-level modules** (`mod player; mod ui;`) — split when files exceed ~1000 lines.
3. **Per-feature folders** (`src/player/{plugin,components,systems}.rs`) — when modules need internal organization.
4. **Workspace + library crate** — when compile times start hurting iteration. `wild_west_game` (binary, just `main.rs`) + `wild_west_lib` (library, all the gameplay code).
5. **Multi-crate workspace** — split out independent subsystems (`wild_west_ai`, `wild_west_ui`) when they're stable enough that you don't refactor across the boundary often.

Don't try to skip ahead. A multi-crate workspace before you know the shape of your domain is wasted effort. Refactor opportunistically, when files actively get in your way. Rust's compiler makes the refactor safe, and version control makes it cheap.
