---
name: bevy
description: Provides authoritative idioms for Bevy 0.20 game projects in Rust. Covers ECS data design (components, required components, queries, change detection, relationships, resources-as-components), communication (Event vs Message vs Observer, EntityEvent, lifecycle patterns), plugin organization, scheduling (run conditions, weak ordering, states, fixed timestep), assets, UI (text, widgets, text input), BSN scenes, rendering and WESL shaders, error handling, testing, performance tuning, and common pitfalls. Use when working with Bevy code — mentions of Bevy, ECS, Component, Query, Plugin, Observer, Event, Message, Schedule, or files using `bevy::prelude::*` — or when modernizing pre-0.20 idioms. Apply even when the user doesn't explicitly say "Bevy" if the task or file is clearly Bevy-shaped. Especially valuable on mixed-version codebases since the 0.16→0.20 rename surface is large.
---

# Bevy 0.20 — How to write idiomatic Bevy code

Bevy moves fast. Idioms that were correct in 0.16 are broken in 0.17, 0.17 idioms shifted in 0.18, 0.18 shifted again in 0.19, and 0.19 shifted again in 0.20. The most consequential shifts you must internalize:

- **0.17 split `Event` from `Message`.** Observers ↔ `Event`; `EventReader`/`EventWriter` was renamed to `MessageReader`/`MessageWriter` and lives on a distinct `Message` trait. Mixing them is the #1 source of confusion.
- **0.17 renamed `Trigger<E>` to `On<E>`** and renamed lifecycle events: `OnAdd` → `Add`, `OnInsert` → `Insert`, `OnRemove` → `Remove`, `OnDespawn` → `Despawn`, `OnReplace` → `Replace`. **In 0.19 `Replace` was renamed again to `Discard`** (and `on_replace` → `on_discard`). The current set is `Add`, `Insert`, `Discard`, `Remove`, `Despawn`, observed as `On<Add<MyComponent>>` (0.20 moved the component generic into the event; 0.17–0.19 wrote `On<Add, MyComponent>`).
- **0.17 made `#[derive(Reflect)]` auto-register** (via the `inventory` crate). `register_type::<T>()` is now needed only for concrete instantiations of generic types.
- **0.17 introduced required components** (`#[require(Other)]`) that effectively replaced bundles for "always together" composition. Bundles still exist as tuples, but new code should declare requirements on the component itself.
- **0.18 made `EntityEvent` immutable by default** (mutation moved to `SetEntityEventTarget`); moved `RenderTarget` off `Camera`; split `AmbientLight`/`GlobalAmbientLight`; renamed `clear_children`/`remove_child` to `detach_all_children`/`detach_child`; made same-value `next_state.set(X)` re-fire `OnEnter`/`OnExit` (use `set_if_different` — named `set_if_neq` in 0.18–0.19 — for the old behavior); and added cargo feature collections.
- **0.19 made resources components.** `Resource` is now a subtrait of `Component`, and `#[derive(Resource)]` *also* implements `Component`. You can no longer `#[derive(Component, Resource)]` on one type (duplicate `Component` impl). Resources gain hooks, observers, relationships, and can be made immutable. See `references/ecs.md`.
- **0.19 overhauled text** (migrated to `parley`): `TextFont.font` is now a `FontSource` (not `Handle<Font>`), `font_size` is a `FontSize` enum (not `f32`), with new `weight`/`width`/`style` fields and a `LetterSpacing` component.
- **0.19 promoted UI out of experiment.** `experimental_bevy_ui_widgets` → `bevy_ui_widgets` (in the `ui`/default features) and `experimental_bevy_feathers` → `bevy_feathers` (still opt-in: `features = ["bevy_feathers"]`); `UiWidgetsPlugins` + `InputDispatchPlugin` are in `DefaultPlugins`. A first-class text input landed (0.19 `EditableText`; split into `TextInput` + `EditableText` in 0.20).
- **0.19 renamed the old scene system to `bevy_world_serialization`.** `SceneRoot(...)` → `WorldAssetRoot(...)` for glTF scene spawning; `DynamicScene` → `DynamicWorld`. The `bevy_scene`/`bevy::scene` name now hosts the new **BSN** scene system (`bsn!`). See `references/assets.md` and the BSN section below.
- **0.19 renamed light shadow fields** (`shadows_enabled` → `shadow_maps_enabled`, plus new `contact_shadows_enabled`), made `Atmosphere` a standalone entity (`Atmosphere::earth(medium)`), and replaced the render-graph with **render-graph-as-systems**. See `references/rendering.md`.
- **0.20 moved the observer component generic onto the event.** `On<Add, T>` → `On<Add<T>>` (same for `Insert`/`Discard`/`Remove`/`Despawn`); `On` now takes one `E: EventPattern`. `On<Add<(A, B)>>` fires when *either* is added. Bare `On<Add>` is gone — dynamic-component observers use `On<Add<()>>` + `.with_component(id)`. `add.entity` and plain `On<MyEvent>` observers are unchanged.
- **0.20 flattened pointer events.** `On<Pointer<Press>>` → `On<PointerPress>`, `Pointer<Click>` → `PointerClick`, and so on for Over/Out/Enter/Leave/Move/Release/Cancel/Scroll/Drag*. `Pointer` is now a plain field: `ev.pointer.id`, `ev.pointer.position` (a `Vec2`), `ev.button`/`ev.hit`/`ev.count` directly on the event. Generic handlers: `fn f<E: PointerEvent>(e: On<E>)`.
- **0.20 tightened BSN syntax.** Scene includes need `@` (`@button("Ok")`, `@scene_var`, `@{expr}`); entities inside `Children [ … ]` / `bsn_list! { … }` are separated by `--`; `template_value(..)` is deprecated (write the value bare); `bsn!`/`bsn_list!` want brace delimiters. A new `Ready` event fires bottom-up once an entity *and its whole subtree* exist. See `references/bsn.md`.
- **0.20 reshaped UI.** `bevy_ui::Button` and `Interaction` are deprecated — import `bevy::ui_widgets::Button` and read `Hovered` + `Has<Pressed>`. A text field is now `TextInput` (behavior) + `EditableText` (state); `EditableText` alone renders but ignores input. `Val` gained `Em`/`Rem` (`em(1.5)`, `rem(2)`), `TextFont::default()` sizes text as `FontSize::Rem(1.)`, and `FontSource::SansSerif` became `FontSource::sans_serif()`.
- **0.20 moved shaders to WESL.** naga_oil is gone: custom shaders are `.wesl` with `import a::b::c;`, `@if(DEF)`/`@else`, and `constants::NAME`; module paths follow the crate file path (`bevy_pbr::forward_io` → `bevy_pbr::render::forward_io`). `#[derive(ExtractComponent)]` now requires `#[extract_app(RenderApp)]`. `Tonemapping::None` became a true passthrough (use the new `Tonemapping::Linear`), and `ScreenSpaceTransmission` is opt-in on `Camera3d`. See `references/rendering.md`.

`references/api-cheatsheet.md` has the full rename table (0.16 → 0.20).

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
- **Component lifecycle observers** — `On<Add<T>>`, `On<Insert<T>>`, `On<Discard<T>>`, `On<Remove<T>>`, `On<Despawn<T>>` (0.20; `On<Add, T>` in 0.17–0.19). Run when a component shows up, gets re-inserted, gets discarded (removed or replaced by a new value), gets removed, or the entity is despawned. The generic is a bundle used as an OR filter: `On<Add<(A, B)>>` fires when either is added. Prefer these over polling `Query<Entity, Added<T>>` in `Update` for spawn-time wiring.
- **Observers can take run conditions (0.19).** `add_observer(on_damage.run_if(|paused: Res<Paused>| !paused.0))` skips the observer when the condition is false. Works with `add_observer`, entity `.observe(...)`, and the `Observer` builder; multiple `.run_if` AND together.
- **`#[component(on_add = fn_path)]`** — register the same hook directly on the component type. Use this when "every time this component appears, do X" is a fundamental property of the type, not a behavior some plugin opts into.

**Heuristic:** if the work is "respond to a thing happening right now to one entity," reach for an observer (or a component hook). If it's "many producers feed a queue that some system drains," reach for messages. If you find yourself spawning an entity and then in the next frame querying for it to attach more state, that's an `On<Add<MarkerComponent>>` observer waiting to happen. (For BSN scenes, `On<Add<T>>` runs top-down before children exist — use `On<Ready>` when you need the whole subtree.)

`references/communication.md` has full examples, propagation, custom triggers, and the lifecycle ordering rules.

## ECS data design

- **Small, focused components.** `Health`, `Armor`, `Speed` — separate. Group fields only when an invariant binds them (current ≤ max, or you need methods that span the values). A god-component is hard to query into pieces and wastes memory on entities that don't need every field.
- **Marker components are free filtering.** `Player`, `Enemy`, `Burning`, `NeedsHookup` — unit structs that drive `With<T>`/`Without<T>` filters. Adding/removing them is a cheap way to switch behavior; observers can fire on the add/remove transitions.
- **Required components replace bundles for "always together" composition.** `#[require(Transform, Visibility)]` on a component means inserting it auto-inserts the others with `Default` values. Use `#[require(Foo(value))]` for a non-default initializer. Bundles (tuples of components) still exist for ad-hoc spawning, but durable composition belongs on the component.
- **Relationships, not raw `Vec<Entity>`.** `ChildOf`/`Children` for parent-child; for anything else (containment, ownership, targeting, ability-of, contained-by) define a custom pair with `#[relationship]`/`#[relationship_target]`. Despawning the parent automatically despawns children when the relationship uses `linked_spawn`. Naming convention is unambiguous: name the component on the *holder* side from the holder's perspective (`ContainedBy`, not `Container`). For purely semantic, non-hierarchical relationships that may point at their own entity (`Likes(self)`), opt in with `#[relationship(relationship_target = ..., allow_self_referential)]` (0.19).
- **Change detection is cheap. Use it.** `Query<&T, Changed<T>>` for "react when this changed," `Ref<T>` if you need to access all entities and check `is_changed()` per row. `set_if_neq` for "mutate but only mark changed if value actually differs" — load-bearing when downstream gates check `Res::is_changed`. Mutable deref unconditionally marks changed, even if you write the same value.
- **`Reflect` auto-registers (since 0.17).** Don't write `app.register_type::<Foo>()` for non-generic types. You *do* still register concrete instantiations of generic types: `app.register_type::<Container<Item>>()`. The `inventory`-based registration doesn't work on a few niche platforms; the workaround is the static-registration variant in the reflect example.
- **Singleton entity vs resource.** Resource when the data is truly singular and isn't part of any larger ECS query (audio settings, world clock, time of day). Component on a singleton entity when it might one day participate in a query, get rendered/simulated alongside other entities, or grow to a small collection (the player, the camera, the active level).
- **Resources are components now (0.19).** `Resource` is a subtrait of `Component` and `#[derive(Resource)]` *also* implements `Component` — so you can't `#[derive(Component, Resource)]` on one type (split them), and broad queries like `Query<EntityMut>` match resource entities (exclude with `Without<IsResource>`). The upside is resources can now carry hooks, observers, relationships, and immutability. `references/ecs.md` covers the reflection, non-send, and generic-bound consequences.

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
- **Weak ordering (0.20).** `.chain_weak()` / `.before_weak(set)` / `.after_weak(set)` keep an ordering only between systems whose tracked ECS accesses actually conflict (run conditions count as access); non-conflicting neighbours may run in parallel. Systems that use `Commands` and exclusive systems always stay ordered. Reach for it on wide phase-set chains; keep `.chain()` when systems talk through anything the scheduler can't see (channels, atomics, interior mutability behind `Res<T>`, statics).
- **Exclusive systems are just systems (0.20).** `&mut World` is a plain `SystemParam`: it can sit anywhere in the parameter list (after `In<T>`, which stays first) alongside `Local<T>`, `&mut SystemState<P>` and `&mut QueryState<D, F>`. `ExclusiveSystemParam`/`ExclusiveFunctionSystem` are gone. Mixing `&mut World` with `Commands`/`Query`/`Res`/`WorldId` is now a schedule-build panic (`error[B0002]`), not a compile error — use `world.id()` or `Local<WorldId>` for the world id.
- **Run conditions over early-return.** `.run_if(in_state(GameState::Playing))` and `.run_if(resource_exists::<MyConfig>)` skip the system entirely (no dispatch, parallelism preserved). Returning early still pays the dispatch cost. Bevy ships dozens of common conditions — search docs.rs for `common_conditions`.
- **Fallible system params skip silently.** `Single<...>` succeeds only when exactly one entity matches, and the system is skipped otherwise — perfect for "no player exists yet" cases. `Option<Res<T>>` for "may not be loaded yet" resources. Use these instead of returning `Err` for cases that aren't really errors.
- **`Result`-returning systems** carry recoverable failures to a global handler; an unannotated `Err` panics, and (0.20) a panic inside a system routes through the same handler. Downgrade one call site with `.with_severity(Severity::Warning)?`. **Never override the global handler in a library plugin.** See the Errors section below.
- **States and sub-states.** `init_state::<S>()` for top-level, `add_sub_state::<S>()` for sub-states gated on a parent state, `add_computed_state::<S>()` for derived states with no manual setter. Computed states beat `or`-chained run conditions when "is the game in any of these states" gets repeated; they also automatically update when new variants get added.
- **Time:** `Res<Time>` adapts to the schedule (virtual time in `Update`, fixed time in `FixedUpdate`). For specific flavors: `Time<Real>` (wall clock, ignores pause), `Time<Virtual>` (in-game, pausable, scalable), `Time<Fixed>` (fixed timestep). Use `time.delta_secs()` to scale per-frame motion. For periodic logic, use `Timer` components (tick them yourself) or the `on_timer(Duration::from_secs(N))` run condition.
- **`DelayedCommands`** (0.19) wraps `Commands` with a delay: `commands.delayed().secs(1.0).spawn(Foo)` queues a spawn for one second from now, ticked automatically.

`references/scheduling.md` has the full schedule list, fixed-timestep gotchas, and a longer example of states + computed states.

## Assets

- **`Handle<A>` is reference-counted.** When the last handle drops, the asset is unloaded — even if a system is about to need it again. Always store your handles in a resource or component immediately after `asset_server.load(...)`.
- **Preload at startup.** Eagerly load asset handles into a resource and use them from there. Without this, the first time you need an enemy sprite after all enemies despawn, you pay the load cost again — and may render a frame with the asset still loading.
- **Wait for completion before gating gameplay.** `asset_server.is_loaded_with_dependencies(&handle)` checks recursively. Derive `VisitAssetDependencies` on your asset struct (annotating each handle field with `#[dependency]`) and use `asset_server.are_dependencies_loaded(&self)` — automatic and update-safe when you add fields.
- **Render-component wrappers wrap the handle.** `MeshMaterial3d<StandardMaterial>` (not `Handle<StandardMaterial>`) is the component. Same for `Mesh3d`, `Mesh2d`, `MeshMaterial2d`. `Sprite::from_image(handle)` for sprites. The bare handle is *not* a component.
- **Mutating asset vs handle.** Replacing the handle on an entity points that one entity at a different asset. Mutating through `assets.get_mut(&handle)` mutates the underlying data shared by all handles. Pick deliberately. (0.19: `get_mut` returns an `AssetMut<A>` — bind it `mut`; see `references/assets.md` for the change-detection caveat.)
- **glTF scene spawning moved (0.19).** Use `WorldAssetRoot(asset_server.load("scene.gltf#Scene0"))` (was `SceneRoot`) — the old scene system is now `bevy_world_serialization`; `bevy::scene` is the new BSN system. One load entry point too: `asset_server.load(path)`, or `load_builder()` for the advanced cases the old `load_acquire`/`load_untyped` variants used to cover.
- **Embedded assets** for assets shipped inside the binary: register with `EmbeddedAssetRegistry::insert_asset(path, &path, bytes)`, load via `embedded://<crate>/<path>` URLs. Pair with the `embedded_watcher` feature for hot reload during dev.
- **Web assets** in 0.17+: enable the `http`/`https` features and `asset_server.load("https://example.com/foo.png")` works. Optional `web_asset_cache` feature for filesystem caching.
- **Hot reload** via the `file_watcher` feature flag. Listen for `AssetEvent::Modified` or filter with `AssetChanged<T>`. This is also the foundation of asset-driven gameplay (RON manifests of items/abilities) — leaning into hot reload for tuning is a powerful pattern for data-heavy games.
- **Persistent user settings (0.19).** Separate from assets, `bevy_settings` (an opt-in feature) is a first-party persistence layer for volume, graphics and window placement. Derive `(Resource, SettingsGroup, Reflect, Default)` **and** annotate `#[reflect(Resource, SettingsGroup, Default)]` — the derive alone is inert — then add `SettingsPlugin::new("com.example.app")` and save with the debounced `SaveSettingsDeferred(Duration)` command. See `references/assets.md`.

`references/assets.md` has full coverage including the mutation semantics, render-asset GPU-only pitfalls, asset-driven gameplay setup, and the `bevy_settings` persistence layer.

## UI

Bevy UI is a flexbox-style layout system. The essentials:

- **`Node` is the layout component.** `position_type`, `display`, `flex_direction`, `justify_content`, `align_items`, plus the size/spacing fields (`width`, `height`, `padding`, `margin`, `border`). 0.19 adds `direction` (inline axis, `InlineDirection::Ltr` by default).
- **`UiTransform`/`UiGlobalTransform`** are 2D-specialized; UI nodes don't share regular `Transform` propagation any more (since 0.17). Don't reach for `Transform` on UI entities.
- **`Val` helpers**: `px(200)`, `percent(20)`, `vw(10)`, `vh(10)`, `vmin(5)`, `vmax(5)`, `em(1.5)`, `rem(2)` (0.20), `auto()`. `Val::Rem` resolves against the `RemSize` resource (one knob for the whole UI); `Val::Em` resolves against the node's own `EmSize` component and is *not* inherited from parents. Plus fluent `UiRect` builders: `px(2).all()`, `percent(20).horizontal().with_top(px(10))`, `vw(10).left()`.
- **Text changed in 0.19** (parley migration): `TextFont.font` is a `FontSource` — `asset_server.load(...).into()`, `FontSource::family("FiraMono")`, or a generic constructor `FontSource::sans_serif()`/`monospace()`/… (0.20: constructors, not variants — `FontSource::SansSerif` no longer exists). `font_size` is a `FontSize` enum (`FontSize::Px(24.0)`, `Vh`, `Rem`, …), and (0.20) `TextFont::default()` is `FontSize::Rem(1.)`, so default-sized text follows `RemSize`. New `weight: FontWeight::BOLD`, `width`, `style: FontStyle::Italic` fields and a `LetterSpacing` component. `TextLayout::justify`/`linebreak`/`no_wrap` (was `new_with_*`).
- **Headless widgets are no longer experimental (0.19).** Feature renamed `experimental_bevy_ui_widgets` → `bevy_ui_widgets` (now in default features and `DefaultPlugins` via `UiWidgetsPlugins`): `Button`, `Slider`, `Scrollbar`, `Checkbox`, `RadioButton`/`RadioGroup`, `ListBox`, menus, `Popover`, `ScrollArea`; 0.20 adds `TabList`/`Tab` and `Dialog`/`ModalDialog`. Behavior only (events: `Activate`, `ValueChange<T>`); you provide style. State components: `Hovered` (a `bool`) plus the unit markers `Pressed`, `Checked`, `InteractionDisabled`. **(0.20) import the widget explicitly** — `use bevy::ui_widgets::Button;` — because `bevy::prelude::Button` is the deprecated `bevy_ui` marker that never fires `Activate`.
- **Text input: `TextInput` + `EditableText` (0.20 split).** `bevy::ui_widgets::TextInput` is the widget (keyboard editing, selection, IME, clipboard); it requires `bevy::text::EditableText`, which now holds only state, so `EditableText` alone renders but ignores all input. Read `editable.value()`, cap with `max_characters`; read-only is the separate `TextReadWriteMode` component. Focus runs through the `InputFocus` resource (`get()`/`set(entity, FocusCause::…)`/`clear()`). `FeathersTextInput` is the themed wrapper.
- **Feathers is no longer experimental (0.19).** Feature `experimental_bevy_feathers` → `bevy_feathers` (still opt-in, not a default feature); plugin `FeathersPlugin` → `FeathersCorePlugin` (group `FeathersPlugins`). **(0.20) Feathers is BSN-only**: the deprecated `*_bundle` spawn fns are gone — compose `@FeathersButton`, `@FeathersCheckbox { @caption: bsn! { @caption("…") } }` inside `bsn!`. Editor/tooling aesthetic — use sparingly in shipped games.
- **Marker-component pattern for HUD elements**: spawn with `Node` + `BackgroundColor` + a marker component (`HealthBar`), then update it via `Query<&mut Node, With<HealthBar>>` or `Query<&mut Text, With<ScoreDisplay>>`.
- **Pickable text spans** (0.18) — observers on a `TextSpan` entity fire when the user clicks within that section's glyph rectangle. Note that non-text areas of `Text` nodes are no longer pickable — wrap in a parent node if you need that.

`references/ui.md` has flexbox tips, positioning recipes, the Val helper reference, and headless widget examples.

## Scenes: BSN (next-generation scenes, 0.19; syntax revised 0.20)

0.19 landed the first usable slice of **BSN** (Bevy Scene Notation) in the `bevy_scene` / `bevy::scene` crate — a declarative way to describe multi-entity assemblages in code. It's the foundation of the future `.bsn` asset format and the Bevy editor, and Feathers widgets are already built on it. It is **incomplete and will churn** — use it where it earns its keep (UI assemblages, widget composition), not as a wholesale replacement for spawning yet.

The `bsn!` macro produces an `impl Scene`; you spawn it with `commands.spawn_scene(...)` (or `queue_spawn_scene` to wait on asset deps), or turn a scene-returning fn into a startup system with `.spawn()`. Functions returning `impl Scene` compose as reusable fragments, **included with an `@` prefix (0.20)**:

```rust
use bevy::{prelude::*, ui_widgets::Button};

fn button(label: &str) -> impl Scene {
    bsn! {
        Button
        Node { width: px(150), height: px(65) }
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
        Children [ Text(label) TextColor(Color::WHITE) ]
    }
}

commands.spawn_scene(bsn! {
    Node { /* layout */ }
    Children [
        @button("Ok")     on(|_: On<PointerPress>| println!("Ok!"))
        --
        @button("Cancel")
    ]
});
```

0.20 syntax rules in one breath: `@` on every scene include (a bare `button("Ok")` is now a component-value insert and fails to compile); `--` between sibling entities inside `Children [ … ]` / `bsn_list! { … }` (commas and `( … )` groups still parse but warn); brace delimiters on the macros themselves; bare component values instead of `template_value(..)`; `#Name` takes an identifier only.

**`Ready` (0.20)** is an `EntityEvent` fired per entity once it *and its whole subtree* are spawned (bottom-up) — the counterpart to `On<Add<T>>`, which runs top-down before children exist. Import it: `use bevy::scene::Ready;`.

What's **still not ready in 0.20**: `.bsn` files don't load (the asset format isn't released) and the glTF loader isn't ported — so for glTF you still use `WorldAssetRoot(asset_server.load("scene.gltf#Scene0"))` from `bevy_world_serialization`, and there's no `World`→BSN round-trip yet.

`references/bsn.md` has the full 0.20 syntax table (patches, `@` includes, `--` separators, `#Name`, `on`, `@SceneComponent` props), composition patterns, and the honest list of what works. `references/assets.md` covers the `bevy_scene` → `bevy_world_serialization` rename and glTF scene spawning.

## Rendering

Most gameplay code just spawns `Camera3d`, `Mesh3d` + `MeshMaterial3d`, and lights, and lets Bevy render. Reach into rendering for custom passes, post-processing, camera/light config, and dev tooling. The 0.19/0.20 surface:

- **Render-graph-as-systems.** The `RenderGraph` `Node`/label/edge API is gone — render passes are ordinary systems in the `Core3d`/`Core2d` schedules, ordered with the `Core3dSystems`/`Core2dSystems` sets (`Prepass`/`MainPass`/`EarlyPostProcess`/`PostProcess`) and the `ViewQuery` + `RenderContext` system params. Initialize render resources in `RenderStartup`.
- **Lights & shadows.** `shadows_enabled` → `shadow_maps_enabled`; new `contact_shadows_enabled`. `GlobalAmbientLight` (resource) vs `AmbientLight` (per-camera component).
- **Atmosphere & sky.** `Atmosphere` is now its own entity in `bevy_light` (`Atmosphere::earth(medium)`); `AtmosphereSettings` stays on the camera. `Skybox.image` is `Option`.
- **Post-processing.** New `Vignette` and `LensDistortion` camera components (`bevy::post_process::effect_stack`). Bloom's luma fix may make scenes look dimmer — bump `Bloom::intensity`.
- **Render recovery.** `RenderErrorHandler` lets you recover from GPU device loss instead of crashing.
- **Dev tools (0.19).** Infinite grid, diagnostics overlay, interactive transform gizmo, and world-space text gizmos.
- **Shaders are WESL (0.20).** Custom shaders are `.wesl`: `import bevy_pbr::render::forward_io::VertexOutput;` (imports first, module paths mirror the crate's file layout), `@if(DEF)`/`@elif`/`@else` for `#ifdef`, `constants::NAME` for integer defs. A plain `.wgsl` still loads but can't import, and shader defs passed to it are ignored with a warning; GLSL and the `shader_format_wesl`/`shader_format_glsl` features are gone.
- **Tonemapping (0.20).** `Tonemapping::None` is now a full passthrough — no `ColorGrading`, no `DebandDither`, no negative-channel clamp. Use the new `Tonemapping::Linear` for "no tone curve, keep grading/dither"; `Camera2d` now defaults to `Linear`.
- **Cameras (0.20).** `ScreenSpaceTransmission` is no longer auto-added to `Camera3d` — spawn `(Camera3d::default(), ScreenSpaceTransmission::default())` or `StandardMaterial::specular_transmission` silently renders with no refraction. An editor-style orbit camera ships behind the off-by-default `pan_orbit_camera` feature; it needs `MeshPickingPlugin` **and** an input plugin you write yourself.
- **Sprites are `Mesh2d` entities (0.20).** Every `Sprite` gets `Mesh2d` + `MeshMaterial2d<SpriteMeshMaterial>` auto-inserted, so `With<Mesh2d>` queries now match sprites (add `Without<Sprite>`). `Sprite` gained `alpha_mode: SpriteAlphaMode` (`Blend` default, `Opaque`, `Mask(f32)`), and custom sprite shaders arrive via `SpriteMaterial<M>` + `SpriteMaterialPlugin::<M>`.

`references/rendering.md` covers the render-world model, the custom-render-system shape, materials (`bevy_material`, bindless on Metal), skinned-mesh culling, and the dev tools in depth.

## Errors

- **`Result` is `Result<(), BevyError>`** in Bevy's prelude. Systems can return it directly and `?` works on any error implementing `std::error::Error`.
- **The default handler is `match_severity`**, which dispatches on `err.severity()`; unannotated errors default to `Severity::Panic`, so an `Err` panics — loud, helpful in development. Configure for release: `app.set_error_handler(warn)` (other presets: `error`, `info`, `debug`, `trace`, `ignore`). **Leave the default in debug builds** — any other preset makes `.with_severity(...)`/`.map_severity(...)` inert. Library plugins must never override the global handler.
- **Panics route through the handler too (0.20).** A panic in a system, run condition or command is caught, wrapped as a `BevyError` with `Severity::Panic`, and handed to the fallback handler. The default re-raises it, so dev behaviour is unchanged; a logging handler keeps a long-running tool alive through a panicking system.
- **Add context (0.20)** with `.context("loading save")?` / `.with_context(|| format!("reading {path}"))?` (`ContextExt`, in the prelude). Works on `Result<_, E: Into<BevyError>>` and lifts `Option<T>` into a `Result`; stacked contexts print a `Caused by:` chain.
- **Per-error severity**: `.with_severity(Severity::Warning)?` downgrades a single call site without affecting the global default. `.map_severity(|e| match e { ... })` varies by error variant.
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
- **Contiguous iteration for SIMD (0.19).** `query.contiguous_iter_mut()` hands you whole table slices (`ContiguousMut<T>`) instead of one row at a time, so LLVM can auto-vectorize tight numeric loops (`position += velocity` over thousands of entities). Returns `Err(QueryNotDenseError)` if the query isn't dense (a sparse-set component; `Changed`/`Added` filters are rejected at compile time); `bypass_change_detection()` gives the raw `&mut [T]`. (0.20) `contiguous_par_iter_mut()` combines this with the task pool. Reach for it on CPU-heavy bulk updates (physics-like workloads).
- **Fixed timestep** for physics, networking, anything where reproducibility matters. `Time<Fixed>` runs zero or more times per frame; interpolate visual transforms between fixed ticks to avoid jitter.
- **Profile before optimizing.** Tracy is the canonical tool: enable the `trace_tracy` feature, run the Tracy GUI capture tool (`capture-release`), launch the app. Bevy's built-in spans show every system. Add custom spans with `info_span!("name")`. Memory tracking adds significant overhead; enable only when chasing allocation issues.
- **Compile profile for release.** `[profile.release]`: `opt-level = 3` for desktop or `'z'`/`'s'` for wasm/mobile binary size, `lto = "fat"`, `codegen-units = 1`, `strip = "debuginfo"`. Add `[profile.dev.package."*"] opt-level = 3` so dev builds run dependencies (including Bevy) at full optimization while keeping your code unoptimized for fast incremental compiles.
- **Dev iteration speed**: `bevy/dynamic_linking` feature is the single biggest compile-time win for development. Don't ship it. Use the `lld` linker on Linux (Rust 1.90+ defaults to it on `x86_64-unknown-linux-gnu`), `mold` if you want to push further. Cranelift codegen on nightly is faster but the binary is slower — fine for `cargo run`, not for benchmarking.
- **Cargo feature collections** mean you rarely need to hand-pick features any more. `bevy = { default-features = false, features = ["3d", "ui"] }` is the shape. In 0.19 `audio` is no longer pulled in implicitly by the `2d`/`3d`/`ui` collections — it's now its own default feature (so a non-default build that wants it must list `"audio"`); disabling default features and listing only `["3d", "ui"]` is the clean way to drop `bevy_audio`.

`references/performance.md` has Tracy walkthrough, GPU profiling pointers, and compile-time tooling (cargo-bloat, cargo-llvm-lines, cargo --timings).

## Common pitfalls and what to do instead

- **Polling for spawn-time setup**: `Query<Entity, With<NeedsHookup>>` running every frame. Use `On<Add<NeedsHookup>>` instead — fires once, immediately, with full access. (Exception: when hookup needs to wait for *both* an asset to load *and* a child component to appear, polling each frame and bailing early is the simplest form. But "do thing once on spawn" is observer territory.)
- **Mutable deref triggering change detection unintentionally**: `for mut t in q.iter_mut()` then a conditional write — every write *unconditionally* marks changed. If downstream gates check `Changed<T>`, use `set_if_neq` or guard the write. Same applies to `ResMut<T>`.
- **`EventReader<Foo>` next to `add_observer(...)` for the same `Foo`** — pick one. `Event` is for observers; `Message` (with `MessageReader`) is for buffered communication.
- **`world.trigger_targets(E, entity)`** — gone. Make `E` an `EntityEvent` with an `entity: Entity` field, then `commands.trigger(E { entity, .. })`.
- **`Query<&Handle<StandardMaterial>>`** — doesn't compile. Use `Query<&MeshMaterial3d<StandardMaterial>>` and dereference `.0` to get the handle.
- **`children!` macro hitting an arity limit** — old code may have hit 12-child cap. 0.17+ supports ~1400 in one macro. For more, `Children::spawn(SpawnIter(..))`.
- **`clear_children` / `remove_child` calls** — renamed in 0.18 to `detach_all_children` / `detach_child` (the children aren't despawned, just detached).
- **`next_state.set(State::X)` expecting no-op when already there** — 0.18 always re-fires `OnEnter`/`OnExit` (and 0.19 also re-runs `DespawnOnEnter`/`DespawnOnExit`). Use `next_state.set_if_different(X)` (0.20 rename of `set_if_neq`) if you want the old skip-if-equal behavior.
- **`On<Replace, T>` / `on_replace` (0.19)** — the lifecycle event `Replace` is now `Discard`; the hook is `on_discard` and the attribute `#[component(on_discard = ...)]`. In 0.20 the spelling is `On<Discard<T>>`.
- **`#[derive(Component, Resource)]` on one type (0.19)** — duplicate `Component` impl, won't compile. `#[derive(Resource)]` now implies `Component`. Split into two types.
- **`#[reflect(Resource)]` for reflection (0.19)** — `ReflectResource` is now a marker only; reflection code (BRP, world serialization) should use `ReflectComponent`.
- **`TextFont { font: handle, font_size: 24.0 }` (0.19)** — `font` is now a `FontSource` (`handle.into()`) and `font_size` a `FontSize` (`FontSize::Px(24.0)`).
- **`PointLight { shadows_enabled: true }` (0.19)** — renamed `shadow_maps_enabled` (same for `DirectionalLight`/`SpotLight`); `contact_shadows_enabled` is the new contact-shadow toggle.
- **`SceneRoot(...)` for glTF (0.19)** — the old scene crate is `bevy_world_serialization`; spawn glTF with `WorldAssetRoot(...)`. `DynamicScene` → `DynamicWorld`.
- **`assets.get_mut(&h)` binding (0.19)** — returns `AssetMut<A>`; bind `mut`, and guard writes so you don't fire `AssetEvent::Modified` (and re-extract materials) on no-op writes.
- **Custom `SystemParam::validate_param` (0.19)** — removed. Move validation into `get_param`, which now returns `Result<Self::Item, SystemParamValidationError>`.
- **`#[derive(Resource)] struct Foo<'a> { ... }`** — stopped compiling in 0.18; resources require `'static`.
- **`AmbientLight` as a resource** — that's the old API. In 0.18 `AmbientLight` is a per-camera component, `GlobalAmbientLight` is the world resource.
- **`Atmosphere::default()` / `Atmosphere` on the camera** — gone. In 0.18 it needed a `ScatteringMedium` asset; in 0.19 `Atmosphere` is its own entity (`Atmosphere::earth(medium)`), moved to `bevy_light`, with `AtmosphereSettings` staying on the camera.
- **`Camera { target: RenderTarget::Image(...) }`** — `RenderTarget` is its own component now. Spawn it alongside `Camera3d`.
- **Auto-Aabb workarounds**: `entity.remove::<Aabb>()` after mutating mesh/sprite. Drop those — 0.18 updates `Aabb` automatically. Use `NoAutoAabb` to opt out.
- **Manual `register_type::<Foo>()` calls** — for non-generic types in 0.17+, `Reflect` auto-registers. Keep these only for generic instantiations.
- **`On<Add, T>` (0.20)** — the bundle generic moved onto the event: `On<Add<T>>`. Symptom: "struct takes 1 generic argument but 2 generic arguments were supplied".
- **`On<Pointer<Press>>` (0.20)** — pointer events are flat structs (`PointerPress`, `PointerClick`, …); `ev.pointer_id` → `ev.pointer.id`.
- **Bare `Button` from the prelude (0.20)** — that's the deprecated `bevy_ui` marker; `use bevy::ui_widgets::Button;` and insert `Hovered::default()` yourself.
- **`EditableText` alone (0.20)** — renders but ignores typing; add `bevy::ui_widgets::TextInput`.
- **`for x in q.iter_many(list)` (0.20)** — items are `Result<_, QueryEntityError>` now; add `.matched()` for the old skip-missing behaviour.
- **`With<Mesh2d>` queries now match sprites (0.20)** — add `Without<Sprite>`, and give overlapping sprites distinct Z.

`references/pitfalls.md` lists more, with the symptom alongside each fix.

## Reference index

Load these as the task lands in their area:

- `references/api-cheatsheet.md` — version-rename table (0.16→0.20); old → new at-a-glance
- `references/ecs.md` — components, required components, queries, change detection, relationships, resources-as-components, custom `QueryData`/`SystemParam`, exclusive systems (0.20)
- `references/communication.md` — Event vs Message vs Observer, `EntityEvent`, propagation, lifecycle patterns (`On<Add<T>>`, `Discard`)
- `references/plugins.md` — plugin pattern, project organization, system-set centralization, plugin groups
- `references/scheduling.md` — schedules, ordering (weak ordering, 0.20), run conditions, states/sub-states/computed states, time and timers
- `references/assets.md` — handles, asset framework, preloading, hot reloading, embedded/web assets, render wrappers, `bevy_world_serialization`/glTF, BSN split, mesh compression, `bevy_settings` persistence
- `references/ui.md` — `Node`, `UiTransform`, `Val` helpers (`em`/`rem`, 0.20), text (`FontSource`/`FontSize`), headless widgets, `TextInput`, Feathers
- `references/bsn.md` — BSN (`bsn!`) 0.20 syntax (`@`, `--`), scene composition, `SceneComponent` props, spawning, `Ready`, limitations
- `references/rendering.md` — render-graph-as-systems, WESL shaders (0.20), `Core3d`/`Core2d` schedules, cameras/lights/shadows, atmosphere, sprites and 2D materials, post-processing, dev tools
- `references/errors.md` — `Result` systems, `BevyError`, severity, error context (0.20), fallible params, command error handling
- `references/testing.md` — unit tests through plugin-level tests, headless setup, mocking input, schedule shuffling (0.20)
- `references/performance.md` — change detection, query optimization, contiguous/SIMD iteration, fixed timestep, Tracy/perf, compile profiles
- `references/pitfalls.md` — anti-patterns and their fixes

When a task spans multiple areas (e.g., "add a damage system"), pull the relevant references together — design data and pick the messaging path in one go, don't separate them.
