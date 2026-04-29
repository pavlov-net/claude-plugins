---
name: bevy
description: Provides authoritative idioms for Bevy 0.18 game projects in Rust. Covers ECS data design (components, required components, queries, change detection, relationships), communication (Event vs Message vs Observer, EntityEvent, lifecycle hooks), plugin organization, scheduling (run conditions, states, fixed timestep), assets, UI, error handling, testing, performance tuning, and common pitfalls. Use when working with Bevy code — mentions of Bevy, ECS, Component, Query, Plugin, Observer, Event, Message, Schedule, or files using `bevy::prelude::*` — or when modernizing pre-0.17 idioms. Apply even when the user doesn't explicitly say "Bevy" if the task or file is clearly Bevy-shaped. Especially valuable on mixed-version codebases since the 0.16→0.17 rename surface is large.
---

# Bevy 0.18 — How to write idiomatic Bevy code

Bevy moves fast. Idioms that were correct in 0.16 are broken in 0.17, and 0.17 idioms have already shifted in 0.18. The most consequential shifts you must internalize:

- **0.17 split `Event` from `Message`.** Observers ↔ `Event`; `EventReader`/`EventWriter` was renamed to `MessageReader`/`MessageWriter` and lives on a distinct `Message` trait. Mixing them is the #1 source of confusion.
- **0.17 renamed `Trigger<E>` to `On<E>`** and renamed lifecycle events: `OnAdd` → `Add`, `OnInsert` → `Insert`, `OnRemove` → `Remove`, `OnDespawn` → `Despawn`, `OnReplace` → `Replace`. They're observed as `On<Add, MyComponent>`.
- **0.17 made `#[derive(Reflect)]` auto-register** (via the `inventory` crate). `register_type::<T>()` is now needed only for concrete instantiations of generic types.
- **0.17 introduced required components** (`#[require(Other)]`) that effectively replaced bundles for "always together" composition. Bundles still exist as tuples, but new code should declare requirements on the component itself.
- **0.18 made `EntityEvent` immutable by default** (mutation moved to `SetEntityEventTarget`).
- **0.18 moved `RenderTarget` off `Camera`** into its own component, split `AmbientLight` (component, per-camera override) from `GlobalAmbientLight` (resource, world default), turned `MaterialPlugin::prepass_enabled`/`shadows_enabled` into `Material::enable_prepass`/`enable_shadows` trait methods, renamed `clear_children`/`remove_child` to `detach_all_children`/`detach_child`, made `Atmosphere` require a `ScatteringMedium` asset, and made same-value `next_state.set(X)` re-fire `OnEnter`/`OnExit` (use `set_if_neq` for the old behavior).
- **0.18 added cargo feature collections**: `2d`, `3d`, `ui`, `audio`, `dev`, plus mid-level `2d_api`, `3d_api`, `default_app`, `default_platform`. Prefer these over hand-listing every feature.

`references/api-cheatsheet.md` has the full rename table.

## The three rules everything else follows

1. **Data lives in components and resources. Logic lives in systems and observers.** A method on a component is fine if it's a pure projection of its own fields (`Health::is_alive`, `Vec3::length`). Anything that touches another entity, spawns, despawns, or reads a resource belongs in a system or observer. The advice "components are just data" has limits — small impl blocks for invariant-preserving setters and convenient accessors are good — but anything that walks the world goes in a system.
2. **One plugin per domain.** Each feature gets a `XPlugin` struct that registers its messages, resources, observers, and systems. Plugins are composable, and breaking work into plugins is the canonical way to keep a Bevy project navigable as it grows. Drop plugins into `App` from a small `main.rs` (or a binary crate that depends on a library crate); resist the urge to put everything in one file.
3. **Centralize ordering with a `SystemSet` enum.** Define one enum with variants for each ordered phase of your game (`InputGather`, `AiBrain`, `Locomotion`, `CameraFollow`, `UpdateUi`, etc.), `chain()` them once in `app.rs`, and have plugins drop systems *into* those sets via `.in_set(...)`. Don't sprinkle `configure_sets` calls across plugins — that splits the source of truth and ordering becomes nondeterministic in practice.

The rest of this document is the canonical idiom for each area, with pointers to references for depth.

## Communication: Event, Message, Observer

This is the single most-confused area in modern Bevy. Three distinct communication tools, three distinct uses:

- **`#[derive(Message)]`** — buffered, frame-deferred, scales to N writers and N readers. Emit with `MessageWriter<M>`, consume with `MessageReader<M>`. Stored in a double-buffered `Messages<M>` resource (a message is readable for one full frame after writing, then dropped). Best when many producers feed a queue that some system drains in batch — damage events, scoreboard updates, log lines.
- **`#[derive(Event)]` + `add_observer(...)`** — runs immediately at `world.trigger(E)`, or on command flush at `commands.trigger(E)`. The handler takes `On<E>` as its first parameter (not `Trigger<E>` — that's the old name). Best when a single explicit consumer needs to act *now*, in response to a discrete moment.
- **`#[derive(EntityEvent)]`** — like `Event`, but targeted at a specific entity. Put `entity: Entity` on the struct (or another `Entity` field with `#[event_target]`). Trigger with `commands.trigger(MyEvent { entity, .. })`. Observe globally with `world.add_observer(...)`, or per-entity with `commands.entity(e).observe(...)`. Opt into hierarchical bubbling with `#[entity_event(propagate)]` (defaults to walking `ChildOf`; you can specify a different relationship).
- **Component lifecycle observers** — `On<Add, T>`, `On<Insert, T>`, `On<Replace, T>`, `On<Remove, T>`, `On<Despawn, T>`. Run when a component shows up, gets re-inserted, gets replaced, gets removed, or the entity is despawned. Add and Insert run on add (Add only on first add); Replace and Remove run on remove (Replace before Remove); Despawn runs last. Prefer these over polling `Query<Entity, Added<T>>` in `Update` for spawn-time wiring.
- **`#[component(on_add = fn_path)]`** — register the same hook directly on the component type. Use this when "every time this component appears, do X" is a fundamental property of the type, not a behavior some plugin opts into.

**Heuristic:** if the work is "respond to a thing happening right now to one entity," reach for an observer (or a component hook). If it's "many producers feed a queue that some system drains," reach for messages. If you find yourself spawning an entity and then in the next frame querying for it to attach more state, that's an `On<Add, MarkerComponent>` observer waiting to happen.

`references/communication.md` has full examples, propagation, custom triggers, and the lifecycle ordering rules.

## ECS data design

- **Small, focused components.** `Health`, `Armor`, `Speed` — separate. Group fields only when an invariant binds them (current ≤ max, or you need methods that span the values). A god-component is hard to query into pieces and wastes memory on entities that don't need every field.
- **Marker components are free filtering.** `Player`, `Enemy`, `Burning`, `NeedsHookup` — unit structs that drive `With<T>`/`Without<T>` filters. Adding/removing them is a cheap way to switch behavior; observers can fire on the add/remove transitions.
- **Required components replace bundles for "always together" composition.** `#[require(Transform, Visibility)]` on a component means inserting it auto-inserts the others with `Default` values. Use `#[require(Foo(value))]` for a non-default initializer. Bundles (tuples of components) still exist for ad-hoc spawning, but durable composition belongs on the component.
- **Relationships, not raw `Vec<Entity>`.** `ChildOf`/`Children` for parent-child; for anything else (containment, ownership, targeting, ability-of, contained-by) define a custom pair with `#[relationship]`/`#[relationship_target]`. Despawning the parent automatically despawns children when the relationship uses `linked_spawn`. Naming convention is unambiguous: name the component on the *holder* side from the holder's perspective (`ContainedBy`, not `Container`).
- **Change detection is cheap. Use it.** `Query<&T, Changed<T>>` for "react when this changed," `Ref<T>` if you need to access all entities and check `is_changed()` per row. `set_if_neq` for "mutate but only mark changed if value actually differs" — load-bearing when downstream gates check `Res::is_changed`. Mutable deref unconditionally marks changed, even if you write the same value.
- **`Reflect` auto-registers in 0.18.** Don't write `app.register_type::<Foo>()` for non-generic types. You *do* still register concrete instantiations of generic types: `app.register_type::<Container<Item>>()`. The `inventory`-based registration doesn't work on a few niche platforms; the workaround is the static-registration variant in the reflect example.
- **Singleton entity vs resource.** Resource when the data is truly singular and isn't part of any larger ECS query (audio settings, world clock, time of day). Component on a singleton entity when it might one day participate in a query, get rendered/simulated alongside other entities, or grow to a small collection (the player, the camera, the active level).

`references/ecs.md` has full coverage of queries, change detection details, relationships, and custom `QueryData`/`SystemParam`.

## Plugins and project organization

- **Plugin per feature.** Each `XPlugin` registers messages, resources, observers, and systems for that feature. Keep the plugin's internals private; the plugin and a small set of public components/messages are the API.
- **Centralize ordering.** One `SystemSet` enum, chained in `app.rs`. Plugins drop systems into named variants with `.in_set(...)`. Don't call `configure_sets` outside the app builder — ordering should have one source of truth.
- **Resources for plugin config.** Anything the user might tune at runtime, or that survives plugin teardown, should be a resource. Reserve plugin struct fields for "this only makes sense at app construction time" (e.g., choosing which schedule the plugin's systems live in).
- **Don't nest plugins for libraries.** For application code, a parent plugin adding child plugins via `add_plugins` is fine and convenient. For libraries, use a `PluginGroup` instead — it lets users disable individual plugins from your group without forking your code.
- **Project structure follows feature, not file kind.** `src/combat/{plugin,components,systems}.rs` beats `src/components/combat.rs` + `src/systems/combat.rs`. Code that changes together should live together. When a folder is consistently >1000 lines and pulling in only one part forces compilation of the rest, split it into its own crate (workspaces help compile time because Cargo parallelizes at the crate level).
- **`pub(crate)` is the right default visibility.** Pure `pub` is for items in the plugin's public API. Private is for implementation details. Don't `pub` everything reflexively — Rust can't dead-code-detect `pub` items, and excess visibility leaks complexity.

`references/plugins.md` has full coverage including the `PluginGroup` API and a worked example of a project growing from a single `main.rs` to a multi-crate workspace.

## Systems and scheduling

- **Schedules in tick order:** `First` → `PreUpdate` → `StateTransition` → `RunFixedMainLoop` (which iterates `FixedFirst`/`FixedPreUpdate`/`FixedUpdate`/`FixedPostUpdate`/`FixedLast` zero or more times) → `Update` → `PostUpdate` → `Last`. Application logic almost always lives in `Update` (or `OnEnter`/`OnExit` for state hooks). `PreUpdate` is for things prepping state for `Update` (input, clocks). `PostUpdate` is for things consuming `Update`'s output (animation drivers, transform propagation, uniform uploads).
- **System ordering via named `SystemSet`s.** `.in_set(MySet::Brain).before(MySet::Locomotion)` reads cleaner than `.before(specific_function)` and survives refactors. For a tight local sequence inside one `add_systems` call, `.chain()` on a tuple is fine.
- **Run conditions over early-return.** `.run_if(in_state(GameState::Playing))` and `.run_if(resource_exists::<MyConfig>)` skip the system entirely (no dispatch, parallelism preserved). Returning early still pays the dispatch cost. Bevy ships dozens of common conditions — search docs.rs for `common_conditions`.
- **Fallible system params skip silently.** `Single<...>` succeeds only when exactly one entity matches, and the system is skipped otherwise — perfect for "no player exists yet" cases. `Option<Res<T>>` for "may not be loaded yet" resources. Use these instead of returning `Err` for cases that aren't really errors.
- **`Result`-returning systems** carry recoverable failures to a global handler. The default handler panics on `Err` (loud during development). You can downgrade with `.with_severity(Severity::Warn)?` on a per-call basis, or change the global handler before release with `app.set_error_handler(warn)`. **Never override the global handler in a library plugin.**
- **States and sub-states.** `init_state::<S>()` for top-level, `add_sub_state::<S>()` for sub-states gated on a parent state, `add_computed_state::<S>()` for derived states with no manual setter. Computed states beat `or`-chained run conditions when "is the game in any of these states" gets repeated; they also automatically update when new variants get added.
- **Time:** `Res<Time>` adapts to the schedule (virtual time in `Update`, fixed time in `FixedUpdate`). For specific flavors: `Time<Real>` (wall clock, ignores pause), `Time<Virtual>` (in-game, pausable, scalable), `Time<Fixed>` (fixed timestep). Use `time.delta_secs()` to scale per-frame motion. For periodic logic, use `Timer` components (tick them yourself) or the `on_timer(Duration::from_secs(N))` run condition.
- **`DelayedCommands`** wraps `Commands` with a delay: `commands.delayed().secs(1.0).spawn(Foo)` queues a spawn for one second from now, ticked automatically.

`references/scheduling.md` has the full schedule list, fixed-timestep gotchas, and a longer example of states + computed states.

## Assets

- **`Handle<A>` is reference-counted.** When the last handle drops, the asset is unloaded — even if a system is about to need it again. Always store your handles in a resource or component immediately after `asset_server.load(...)`.
- **Preload at startup.** Eagerly load asset handles into a resource and use them from there. Without this, the first time you need an enemy sprite after all enemies despawn, you pay the load cost again — and may render a frame with the asset still loading.
- **Wait for completion before gating gameplay.** `asset_server.is_loaded_with_dependencies(&handle)` checks recursively. Derive `VisitAssetDependencies` on your asset struct (annotating each handle field with `#[dependency]`) and use `asset_server.are_dependencies_loaded(&self)` — automatic and update-safe when you add fields.
- **Render-component wrappers wrap the handle.** `MeshMaterial3d<StandardMaterial>` (not `Handle<StandardMaterial>`) is the component. Same for `Mesh3d`, `Mesh2d`, `MeshMaterial2d`. `Sprite::from_image(handle)` for sprites. The bare handle is *not* a component.
- **Mutating asset vs handle.** Replacing the handle on an entity points that one entity at a different asset. Mutating through `assets.get_mut(&handle)` mutates the underlying data shared by all handles. Pick deliberately.
- **Embedded assets** for assets shipped inside the binary: register with `EmbeddedAssetRegistry::insert_asset(path, &path, bytes)`, load via `embedded://<crate>/<path>` URLs. Pair with the `embedded_watcher` feature for hot reload during dev.
- **Web assets** in 0.17+: enable the `http`/`https` features and `asset_server.load("https://example.com/foo.png")` works. Optional `web_asset_cache` feature for filesystem caching.
- **Hot reload** via the `file_watcher` feature flag. Listen for `AssetEvent::Modified` or filter with `AssetChanged<T>`. This is also the foundation of asset-driven gameplay (RON manifests of items/abilities) — leaning into hot reload for tuning is a powerful pattern for data-heavy games.

`references/assets.md` has full coverage including the mutation semantics, render-asset GPU-only pitfalls, and asset-driven gameplay setup.

## UI

Bevy UI is a flexbox-style layout system. The 0.18 essentials:

- **`Node` is the layout component.** `position_type`, `display`, `flex_direction`, `justify_content`, `align_items`, plus the size/spacing fields (`width`, `height`, `padding`, `margin`, `border`).
- **`UiTransform`/`UiGlobalTransform`** are 2D-specialized; UI nodes don't share regular `Transform` propagation any more (since 0.17). Don't reach for `Transform` on UI entities.
- **`Val` helpers**: `px(200)`, `percent(20)`, `vw(10)`, `vh(10)`, `vmin(5)`, `vmax(5)`, `auto()`. Plus fluent `UiRect` builders: `px(2).all()`, `percent(20).horizontal().with_top(px(10))`, `vw(10).left()`.
- **Headless widgets** (experimental, behind `experimental_bevy_ui_widgets`): `Button`, `Slider`, `Scrollbar`, `Checkbox`, `RadioButton`, `RadioGroup`. Handle behavior (events: `Activate`, `ValueChange<T>`); you provide style. State-tracking components: `Hovered`, `Pressed`, `Checked`, `InteractionDisabled`.
- **Feathers** (experimental, behind `experimental_bevy_feathers`): an opinionated themed widget set built on the headless widgets, intended for the future Bevy Editor. Useful for tools and inspectors; use sparingly in shipped games.
- **Marker-component pattern for HUD elements**: spawn with `Node` + `BackgroundColor` + a marker component (`HealthBar`), then update it via `Query<&mut Node, With<HealthBar>>` or `Query<&mut Text, With<ScoreDisplay>>`.
- **Pickable text spans** (0.18) — observers on a `TextSpan` entity fire when the user clicks within that section's glyph rectangle. Useful for hyperlink-like behavior. Note that non-text areas of `Text` nodes are no longer pickable in 0.18 — wrap in a parent node if you need that.

`references/ui.md` has flexbox tips, positioning recipes, the Val helper reference, and headless widget examples.

## Errors

- **`Result` is `Result<(), BevyError>`** in Bevy's prelude. Systems can return it directly and `?` works on any error implementing `std::error::Error`.
- **Default handler panics on `Err`** — loud, helpful in development. Configure for release: `app.set_error_handler(warn)` (other presets: `error`, `info`, `debug`, `trace`, `ignore`). Use a feature flag to switch between dev (panic) and release (warn) policies. Library plugins must never override the global handler.
- **Per-error severity**: `.with_severity(Severity::Warn)?` downgrades a single call site without affecting the global default. `.map_severity(|e| match e { ... })` varies by error variant.
- **System piping for custom handling**: `.add_systems(Update, update.pipe(handle_error))`. The piped handler takes `In<Result>` (or `In<Result<T, E>>`).
- **Commands can return errors too**: `commands.queue_handled(cmd, |err, ctx| ...)` for explicit handling, `queue_silenced` to drop them.

`references/errors.md` covers patterns, severity choices, and integration with `thiserror`.

## Testing

Bevy testing fans out by fidelity (cheap → expensive):

- **Test pure methods directly.** If `Health::heal` is a method, write `let mut h = Health::new(100); h.heal(50); assert_eq!(...)` — no `World`, no `App`. The fastest tests you can write.
- **`World::new()` for setup helpers.** Spawn entities, mutate them, read state back. Useful when the function under test takes `&mut World`.
- **`World::run_system_once(my_system)`** runs a system once against a constructed world. Good for testing real systems in isolation. *No* `Local`, no `Added`/`Changed` filters work the way they would in a real schedule (the system is fresh every call).
- **`Schedule` for ordering tests.** `let mut s = Schedule::default(); s.add_systems((a, b).chain()); s.run(&mut world);` — verifies the *interaction* between systems.
- **`App::update()` for plugin-level tests.** Add `MinimalPlugins` + your plugin; loop `app.update()` to advance frames. Highest fidelity, most fragile.
- **Headless feature flag**: gate `add_plugins(DefaultPlugins)` behind `#[cfg(not(feature = "headless"))]` and add a CI variant that disables `AudioPlugin`/`UiRenderPlugin`/etc. Lets you run integration tests on machines without a GPU.

`references/testing.md` has the full ladder, mocking input, and a brief on visual-regression testing.

## Performance and profiling

- **Change detection is the cheapest optimization.** A system that runs over 10,000 entities every frame becomes free when most of them haven't changed: `Query<&T, Changed<T>>`.
- **Filter at the query, not in the loop.** `Query<&A, (With<B>, Without<C>)>` is a no-cost filter; `if has_b && !has_c { ... }` inside a loop costs every iteration.
- **`par_iter_mut`** for parallel iteration when the body is independent across entities. Combine with `ParallelCommands::command_scope` to issue commands from parallel work.
- **Fixed timestep** for physics, networking, anything where reproducibility matters. `Time<Fixed>` runs zero or more times per frame; interpolate visual transforms between fixed ticks to avoid jitter.
- **Profile before optimizing.** Tracy is the canonical tool: enable the `trace_tracy` feature, run the Tracy GUI capture tool (`capture-release`), launch the app. Bevy's built-in spans show every system. Add custom spans with `info_span!("name")`. Memory tracking adds significant overhead; enable only when chasing allocation issues.
- **Compile profile for release.** `[profile.release]`: `opt-level = 3` for desktop or `'z'`/`'s'` for wasm/mobile binary size, `lto = "fat"`, `codegen-units = 1`, `strip = "debuginfo"`. Add `[profile.dev.package."*"] opt-level = 3` so dev builds run dependencies (including Bevy) at full optimization while keeping your code unoptimized for fast incremental compiles.
- **Dev iteration speed**: `bevy/dynamic_linking` feature is the single biggest compile-time win for development. Don't ship it. Use the `lld` linker on Linux (Rust 1.90+ defaults to it on `x86_64-unknown-linux-gnu`), `mold` if you want to push further. Cranelift codegen on nightly is faster but the binary is slower — fine for `cargo run`, not for benchmarking.
- **Cargo feature collections (0.18)** mean you rarely need to hand-pick features any more. `bevy = { default-features = false, features = ["3d", "ui"] }` is the new shape.

`references/performance.md` has Tracy walkthrough, GPU profiling pointers, and compile-time tooling (cargo-bloat, cargo-llvm-lines, cargo --timings).

## Common pitfalls and what to do instead

- **Polling for spawn-time setup**: `Query<Entity, With<NeedsHookup>>` running every frame. Use `On<Add, NeedsHookup>` instead — fires once, immediately, with full access. (Exception: when hookup needs to wait for *both* an asset to load *and* a child component to appear, polling each frame and bailing early is the simplest form. But "do thing once on spawn" is observer territory.)
- **Mutable deref triggering change detection unintentionally**: `for mut t in q.iter_mut()` then a conditional write — every write *unconditionally* marks changed. If downstream gates check `Changed<T>`, use `set_if_neq` or guard the write. Same applies to `ResMut<T>`.
- **`EventReader<Foo>` next to `add_observer(...)` for the same `Foo`** — pick one. `Event` is for observers; `Message` (with `MessageReader`) is for buffered communication.
- **`world.trigger_targets(E, entity)`** — gone. Make `E` an `EntityEvent` with an `entity: Entity` field, then `commands.trigger(E { entity, .. })`.
- **`Query<&Handle<StandardMaterial>>`** — doesn't compile. Use `Query<&MeshMaterial3d<StandardMaterial>>` and dereference `.0` to get the handle.
- **`children!` macro hitting an arity limit** — old code may have hit 12-child cap. 0.17+ supports ~1400 in one macro. For more, `Children::spawn(SpawnIter(..))`.
- **`clear_children` / `remove_child` calls** — renamed in 0.18 to `detach_all_children` / `detach_child` (the children aren't despawned, just detached).
- **`next_state.set(State::X)` expecting no-op when already there** — 0.18 always re-fires `OnEnter`/`OnExit`. Use `set_if_neq` if you want the old behavior.
- **`#[derive(Resource)] struct Foo<'a> { ... }`** — stopped compiling in 0.18; resources require `'static`.
- **`AmbientLight` as a resource** — that's the old API. In 0.18 `AmbientLight` is a per-camera component, `GlobalAmbientLight` is the world resource.
- **`Atmosphere::default()`** — gone in 0.18; needs a `ScatteringMedium` asset. `Atmosphere::earthlike(media.add(ScatteringMedium::default()))` is the modern shape.
- **`Camera { target: RenderTarget::Image(...) }`** — `RenderTarget` is its own component now. Spawn it alongside `Camera3d`.
- **Auto-Aabb workarounds**: `entity.remove::<Aabb>()` after mutating mesh/sprite. Drop those — 0.18 updates `Aabb` automatically. Use `NoAutoAabb` to opt out.
- **Manual `register_type::<Foo>()` calls** — for non-generic types in 0.17+, `Reflect` auto-registers. Keep these only for generic instantiations.

`references/pitfalls.md` lists more, with the symptom alongside each fix.

## Reference index

Load these as the task lands in their area:

- `references/api-cheatsheet.md` — version-rename table; old → new at-a-glance
- `references/ecs.md` — components, required components, queries, change detection, relationships, custom `QueryData`/`SystemParam`
- `references/communication.md` — Event vs Message vs Observer, `EntityEvent`, propagation, lifecycle hooks
- `references/plugins.md` — plugin pattern, project organization, system-set centralization, plugin groups
- `references/scheduling.md` — schedules, ordering, run conditions, states/sub-states/computed states, time and timers
- `references/assets.md` — handles, asset framework, preloading, hot reloading, embedded/web assets, render wrappers
- `references/ui.md` — `Node`, `UiTransform`, `Val` helpers, headless widgets, Feathers
- `references/errors.md` — `Result` systems, `BevyError`, severity, fallible params, command error handling
- `references/testing.md` — unit tests through plugin-level tests, headless setup, mocking input
- `references/performance.md` — change detection, query optimization, fixed timestep, Tracy/perf, compile profiles
- `references/pitfalls.md` — anti-patterns and their fixes

When a task spans multiple areas (e.g., "add a damage system"), pull the relevant references together — design data and pick the messaging path in one go, don't separate them.
