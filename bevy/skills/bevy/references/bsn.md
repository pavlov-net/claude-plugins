# BSN — next-generation scenes (0.19)

## Contents
- What BSN is, and what shipped in 0.19 (vs not yet)
- The `bsn!` macro — what it produces
- Spawning a scene — `spawn_scene`, `queue_spawn_scene`, the `.spawn()` startup idiom
- Syntax reference — patches, children, observers, names, inheritance, scene components
- Composition — fragments composed via bare scene-fn calls
- `SceneComponent` and props — `@Type { @prop: … }`
- Relationship to the old scene system (`bevy_world_serialization`)
- Honest limitations in 0.19

**BSN** (Bevy Scene Notation) is Bevy's next-generation way to describe multi-entity assemblages declaratively. It landed a first usable slice in 0.19, living in the `bevy_scene` / `bevy::scene` crate (the *old* scene system was renamed to `bevy_world_serialization` to make room — see `references/assets.md`). BSN is the foundation of the future `.bsn` asset format and the Bevy editor, and Feathers widgets are already built on it.

**Use it with eyes open.** BSN is incomplete and will see breaking changes. It's a genuine win for UI assemblages and widget composition (a slider is a track + fill + thumb + label wired together — BSN expresses that in one place). It is *not* a wholesale replacement for `commands.spawn(...)` yet, and the file/asset workflow isn't ready.

## The `bsn!` macro

`bsn! { ... }` produces a value implementing the `Scene` trait (an anonymous type — you don't name it). `bsn_list![a, b, c]` produces a `SceneList` (multiple root scenes). Both are in the prelude.

A `Scene` describes one root entity (plus children); a `SceneList` describes several roots. A function returning `impl Scene` is itself a reusable scene fragment — this is how you compose.

```rust
fn ui() -> impl Scene {
    bsn! {
        Node { width: percent(100), height: percent(100) }
        BackgroundColor(Color::BLACK)
    }
}
```

## Spawning a scene

`bsn!` only *describes* a scene; you spawn it:

```rust
// From Commands or World — immediate (errors if asset deps aren't loaded):
commands.spawn_scene(bsn! { Camera2d });
world.spawn_scene(bsn! { Camera2d }).unwrap();

// Queued — waits for asset dependencies before applying:
commands.queue_spawn_scene(bsn! { /* ... */ });

// Startup-system idiom: turn a scene-returning fn into a system with .spawn()
app.add_systems(Startup, scene.spawn());

fn scene() -> impl SceneList {
    bsn_list![Camera2d, ui()]
}
```

There is **no `SceneRoot`-style component** for BSN — you spawn via `spawn_scene`, not by attaching a component. (`WorldAssetRoot`, the glTF/old-scene spawn component, is a different system in `bevy_world_serialization`.)

## Syntax reference

Inside `bsn! { ... }`, a root entity is a whitespace-separated list of component patches, optionally followed by `Children [ ... ]` and `on(...)` observers:

| Form | Meaning |
| --- | --- |
| `ComponentName` | Insert the component's default value. |
| `ComponentName { field: value, .. }` | Patch *only* the listed fields; unlisted fields keep prior/default values. |
| `ComponentName(value)` | Tuple-struct patch (`BackgroundColor(Color::BLACK)`). |
| `Children [ scene_a, scene_b ]` | Comma-separated child scenes. |
| `on(\|e: On<SomeEvent>, …\| { … })` | Attach an observer to this entity. |
| `#Name` | Assign `Name("Name")` and make the entity referenceable within the scene. |
| `#{expr}` | Dynamic name from an expression. |
| `{expr}` | An arbitrary Rust expression that is itself a `Scene` / value. |
| `other_fn(args)` | Compose another scene fn — a **bare call** inside `bsn!`; patches written after it layer on top. |
| `:"path.bsn"` | Asset inheritance — **parses but does not load in 0.19** (no `.bsn` format yet). |
| `@SceneComponent { @prop: value, field: value }` | Spawn a `SceneComponent`, setting its props (`@prop`) and components. |

Patch semantics matter: `Node { width: px(10) }` sets *only* `width` and leaves everything else at its prior value, so fragments compose by layering partial patches rather than overwriting whole components.

## Composition

The idiomatic pattern is small scene-returning functions composed by **calling them bare** inside `bsn!` (and nesting via `Children`):

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, scene.spawn())
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, menu()]
}

fn menu() -> impl Scene {
    bsn! {
        Node {
            width: percent(100), height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: px(5),
        }
        Children [
            ( button("Ok")     on(|_: On<Pointer<Press>>| println!("Ok")) ),
            ( button("Cancel") on(|_: On<Pointer<Press>>| println!("Cancel"))
                               BackgroundColor(Color::srgb(0.4, 0.15, 0.15)) ),
        ]
    }
}

fn button(label: &str) -> impl Scene {
    bsn! {
        Button
        Node { width: px(150), height: px(65), border: px(5),
               justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        BorderColor::from(Color::BLACK)
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
        Children [( Text(label) TextColor(Color::srgb(0.9, 0.9, 0.9)) )]
    }
}
```

Note in the `menu()` example that `button("Cancel")` is patched *after the fact* with an extra `BackgroundColor` — the fragment provides a base, the call site layers patches on top. The same works for argument-less fragments: call the fn bare and write overrides after it (there is no `:fn` syntax for this — a colon prefix is asset inheritance, `:"path.bsn"`).

```rust
fn plain_button() -> impl Scene {
    bsn! { Button Node { width: px(150), height: px(65) } BackgroundColor(Color::srgb(0.15, 0.15, 0.15)) }
}

fn fancy_button() -> impl Scene {
    bsn! {
        plain_button()                        // compose the base button (bare call)
        BorderColor::from(Color::GOLD)        // then override
    }
}
```

## `SceneComponent` and props

A `SceneComponent` is a component that brings its own scene (a multi-entity widget). Feathers widgets are `SceneComponent`s. You spawn one with the `@Type { @prop: value }` syntax — `@prop` sets *props* (inputs that aren't plain components, like a caption that's itself a list of entities), plain entries set components:

```rust
bsn! {
    @FeathersCheckbox {
        @caption: { bsn! { Text("Enable shadows") ThemedText } }
    }
    MyMarker
    on(|change: On<ValueChange<bool>>, mut config: ResMut<ShadowConfig>| {
        config.enabled = change.value;
    })
}
```

Define your own with `#[derive(SceneComponent, FromTemplate)]` and a `#[scene(MyProps)]` props struct; the derive wires the props into the spawned scene. Field-level patching for your own components comes from `#[derive(FromTemplate)]`.

## Relationship to the old scene system

| Concern | 0.19 home |
| --- | --- |
| BSN, `bsn!`, `spawn_scene`, `SceneComponent` | `bevy_scene` / `bevy::scene` |
| glTF scene spawning (`WorldAssetRoot`), `World` (de)serialization (`DynamicWorld`) | `bevy_world_serialization` / `bevy::world_serialization` |

For glTF you still use `WorldAssetRoot(asset_server.load("scene.gltf#Scene0"))` — the glTF loader hasn't been ported to BSN. See `references/assets.md`.

## Honest limitations in 0.19

What works: the `bsn!` / `bsn_list!` macros (inline Rust scenes), `spawn_scene`/`queue_spawn_scene`, the `.spawn()` startup idiom, `Children`/`on(...)`, scene-fn composition (bare calls), `@SceneComponent { @prop }`, and `ScenePatchInstance` for deferred application.

What is **not** ready:

- **`.bsn` files don't load.** The asset format isn't released — `:"path.bsn"` and `#[scene("file.bsn")]` parse and compile, but won't load at runtime.
- **glTF isn't ported to BSN.** Use `WorldAssetRoot` from `bevy_world_serialization`.
- **No `World` → BSN round-trip.** Writing a world out is still `bevy_world_serialization`'s `DynamicWorld`.

So in 0.19, treat BSN as a *code-first* scene/composition tool — excellent for UI and widgets, premature for asset-driven scene files. When the `.bsn` format lands (targeted for a later release) the same `bsn!` syntax becomes portable to files.
