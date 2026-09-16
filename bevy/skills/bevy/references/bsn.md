# BSN — next-generation scenes (0.19; syntax revised in 0.20)

## Contents
- What BSN is, and what shipped (vs not yet)
- The `bsn!` macro — what it produces
- Spawning a scene — `spawn_scene`, `queue_spawn_scene`, `spawn_scene_list`, the `.spawn()` startup idiom
- Syntax reference (0.20) — patches, `@` includes, `--` separators, observers, names, scene components
- Composition — fragments included with `@`; loops and conditionals via list splicing
- `Ready` (0.20) — the whole-subtree hook
- `SceneComponent` and props — `@Type { @prop: … }`, derives
- Relationship to the old scene system (`bevy_world_serialization`)
- Honest limitations (still true in 0.20)

**BSN** (Bevy Scene Notation) is Bevy's next-generation way to describe multi-entity assemblages declaratively. It landed a first usable slice in 0.19, living in the `bevy_scene` / `bevy::scene` crate (the *old* scene system was renamed to `bevy_world_serialization` to make room — see `references/assets.md`). BSN is the foundation of the future `.bsn` asset format and the Bevy editor, and Feathers widgets are already built on it.

**Use it with eyes open.** BSN is incomplete and will see breaking changes. It's a genuine win for UI assemblages and widget composition (a slider is a track + fill + thumb + label wired together — BSN expresses that in one place). It is *not* a wholesale replacement for `commands.spawn(...)` yet, and the file/asset workflow isn't ready.

## The `bsn!` macro

`bsn! { … }` produces a value implementing the `Scene` trait (an anonymous type — you don't name it). `bsn_list! { a -- b -- c }` produces a `SceneList` (multiple root scenes). Both are in the prelude. **(0.20) Use brace delimiters and `--` separators** — `bsn_list![…]` / `bsn!(…)` still compile but warn ("rustfmt can mangle the outputs"), and commas between entities warn too.

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
// World — immediate; Err(SpawnSceneError) if asset deps aren't loaded:
world.spawn_scene(bsn! { Camera2d })?;
let roots: Vec<Entity> = world.spawn_scene_list(bsn_list! { Camera2d -- @ui() })?;

// Commands — queued as a command; spawn errors are logged, not returned:
commands.spawn_scene(bsn! { Camera2d });
commands.spawn_scene_list(bsn_list! { Camera2d -- @ui() });

// queue_* — waits for asset deps, then spawns in the `SpawnScene` schedule
// (between Update and PostUpdate):
commands.queue_spawn_scene(bsn! { /* ... */ });

// Add BSN children to an existing entity (any RelationshipTarget, usually Children):
commands.entity(e).queue_spawn_related_scenes::<Children>(bsn_list! { @button("x") });

// Patch a scene onto an existing entity — replaces and orphans its current children:
commands.entity(e).apply_scene(bsn! { /* ... */ });   // or queue_apply_scene

// Startup-system idiom: a fn returning `impl Scene`/`impl SceneList` becomes a system
app.add_systems(Startup, scene.spawn());

fn scene() -> impl SceneList {
    bsn_list! {
        Camera2d
        --
        @ui()
    }
}
```

There is **no `SceneRoot`-style component** for BSN — you spawn via `spawn_scene`, not by attaching a component. (`WorldAssetRoot`, the glTF/old-scene spawn component, is a different system in `bevy_world_serialization`.)

## Syntax reference

Inside `bsn! { … }`, a root entity is a whitespace-separated list of entries; related entities live in `Children [ … ]` (or any `RelationshipTarget [ … ]`) separated by `--`.

| Form | Meaning |
| --- | --- |
| `Comp` / `Comp(v, …)` / `Comp { field: v, … }` | Patch a component: only listed fields change; unlisted keep prior/default. `Comp { name }` is field shorthand; `Comp { a: 1, ..base }` is struct update (last, structs only). Path-qualified `my_mod::Comp { … }` works. |
| `MyEnum::Variant { x: 1, y: 0 }` | Enum component — **(0.20)** every field of the variant must be given, as in plain Rust (`VariantDefaults` is gone); `..expr` on a variant is a macro error. Struct values *inside* a variant still patch (`Shape::Rect(Size { height: 5 })` leaves `width` default). |
| `comp_var`, `make()`, `var.clone()`, `Transform::from_xyz(..).looking_at(..)` | Insert a component *value* as-is (whole overwrite, not a field patch). **(0.20)** replaces the deprecated `template_value(..)` wrapper. A bare `Type::ctor(..)` resolves via `<Type as FromTemplate>::Template::ctor(..)` — fine for `Default + Clone` components. |
| `~{expr}` | Force `expr` in as a `Template` value when the bare form doesn't resolve — the escape hatch the `template_value` deprecation points at (0.20). `~Tmpl { … }` / `~Tmpl::ctor(..)` treat `Tmpl` as the `Template` itself. |
| `@scene_fn(args)` / `@scene_var` / `@{expr}` | Include another `Scene`; entries written after it patch on top. **(0.20) the `@` is required** — a bare `scene_fn()` is now a component-value insert and fails to compile, and a bare `{expr}` at entry position no longer parses. |
| `@MySceneComp { @prop: v, field: v }` | Include a `SceneComponent`; `@prop` sets a prop, plain names set component fields. |
| `:"file.bsn"` | *Cached* scene-asset include; must be the first entry, and `:` accepts only a string literal (`:fn()` / `:Type` is a compile error — "Consider replacing `:` with `@`"). Parses, but no first-party `.bsn` loader ships. |
| `#Name` | Inserts `Name("Name")` and makes the entity referenceable in this `bsn!`/`bsn_list!` scope: `Comp(#Name)`, `Comp { entity: #Name }`, `@scene(#Name)`, `ChildOf(#Name)`. **Identifier only** — `#{expr}` does not parse. For a runtime name write `#Root Name({format!("Entity {i}")})`. |
| `on(\|e: On<Ev>, …\| { … })` / `on(fn_name)` | Attach an entity observer for any `EntityEvent` (`Ready`, `PointerPress`, `Activate`, `ValueChange<T>`, your own). Multiple `on(..)` allowed. |
| `template(\|ctx\| Ok(Comp(..)))` | Ad-hoc template with `TemplateContext` (world/resource access); `template` is in `bevy::prelude`. |
| `Children [ A B -- C ]` / `MyRel [ … ]` | Related entities via any `RelationshipTarget`. Whitespace joins components on **one** entity; `--` starts the next entity. **(0.20)** `,` and `( … )` still parse but emit deprecation warnings. |
| `Children [ A -- {list} -- B ]` | Splice a `SceneList` expression between entities (`Vec<impl Scene>`, `Vec<Box<dyn SceneList>>`, `Box<dyn SceneList>`, a `bsn_list!` value, a tuple, or `Option<impl SceneList>`). A single `impl Scene` is not a `SceneList` — write `@{scene}` as its own list item. |
| `Sprite { image: "player.png" }` | A string literal where a `Handle<T>` field is expected becomes a `HandleTemplate`, loaded via `AssetServer` at resolve time (the component must derive `FromTemplate`). |
| `{ expr }` as a *field value* | Arbitrary Rust expression (`{hp / 2}`). Plain idents, literals, consts, `func(..)`, `mac!(..)`, tuples, closures and `a..b` ranges need no braces. |

Patch semantics matter: `Node { width: px(10) }` sets *only* `width` and leaves everything else at its prior value, so fragments compose by layering partial patches rather than overwriting whole components — except bare values (`comp_var`, `make()`), which replace the whole component.

**Pitfall (0.20):** removing a deprecated comma without adding `--` silently merges two intended siblings into one entity (all the components land on the first). There is no warning for that, only for the old `,`/`( … )` syntax.

## Composition

The idiomatic pattern is small scene-returning functions included with `@` inside `bsn!` (and nested via `Children`):

```rust
use bevy::{prelude::*, ui_widgets::Button};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, scene.spawn())
        .run();
}

fn scene() -> impl SceneList {
    bsn_list! {
        Camera2d
        --
        @menu()
    }
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
            @button("Ok")
            on(|_: On<PointerPress>| println!("Ok"))
            --
            @button("Cancel")
            on(|_: On<PointerPress>| println!("Cancel"))
            BackgroundColor(Color::srgb(0.4, 0.15, 0.15))
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
        Children [ Text(label) TextColor(Color::srgb(0.9, 0.9, 0.9)) ]
    }
}
```

Note in `menu()` that `@button("Cancel")` is patched *after the fact* with an extra `BackgroundColor` — the fragment provides a base, the call site layers patches on top. The same works for argument-less fragments:

```rust
fn plain_button() -> impl Scene {
    bsn! { Button Node { width: px(150), height: px(65) } BackgroundColor(Color::srgb(0.15, 0.15, 0.15)) }
}

fn fancy_button() -> impl Scene {
    bsn! {
        @plain_button()                       // include the base button (note the @)
        BorderColor::from(Color::WHITE)       // then override
    }
}
```

**Import the right `Button`.** `bevy::prelude::Button` is the deprecated `bevy_ui` marker, which never fires `Activate`; `use bevy::ui_widgets::Button;` shadows it.

**Loops and conditionals.** `bsn!` has no `for`/`if`; build the list in Rust and splice it:

```rust
let rows: Vec<_> = (0..n).map(|i| bsn! { Text::new(format!("row {i}")) }).collect();
let footer: Option<_> = show_footer.then(|| bsn_list! { #Footer Node });
bsn! {
    Node
    Children [
        #Header
        --
        {rows}     // Vec<impl Scene> is a SceneList
        --
        {footer}   // Option<impl SceneList> resolves to nothing when None
    ]
}
```

## `Ready` (0.20)

`bevy::scene::Ready` (**not** in the prelude — `use bevy::scene::Ready;`) is an `EntityEvent` triggered for each entity of a scene *after* its components are written **and** its whole descendant subtree is spawned. It fires bottom-up (grandchildren, children, then root) and only once the scene's asset dependencies have resolved, making it the counterpart to `On<Add<T>>`, which runs top-down before children exist.

```rust
use bevy::scene::Ready;

fn widget() -> impl Scene {
    bsn! {
        Node { width: px(100), height: px(100) }
        on(|ready: On<Ready>, children: Query<&Children>| {
            // ready.entity's full subtree exists here
        })
        Children [ Text("hello") -- Text("world") ]
    }
}
```

Attach it with `on(…)` inside `bsn!` (Feathers does this for grid layout and popups) or globally with `app.add_observer(|r: On<Ready>| …)` — a global observer fires for *every* entity in the subtree, so filter by a marker.

**Pitfall:** `commands.spawn_scene(scene).observe(|r: On<Ready>| …)` never fires — `spawn_scene` applies the scene (and triggers `Ready`) inside its own queued command, before the later `.observe` command runs. `.observe` after `queue_spawn_scene` is fine. Only the scene machinery emits `Ready` (`spawn_scene`, `queue_spawn_scene`, `apply_scene`); plain `commands.spawn` and glTF do not.

## `SceneComponent` and props

A `SceneComponent` is a component that brings its own scene (a multi-entity widget). Feathers widgets are `SceneComponent`s. You spawn one with the `@Type { @prop: value }` syntax — `@prop` sets *props* (inputs that aren't plain components, like a caption that's itself a list of entities), plain entries set components:

```rust
use bevy::feathers::{controls::FeathersCheckbox, display::caption};

bsn! {
    @FeathersCheckbox {
        // `@caption:` sets a prop; `@caption("…")` includes the feathers `caption()`
        // scene fn (0.20; = `Text(text) ThemedText`). Scene refs need the `@`.
        @caption: bsn! { @caption("Enable shadows") }
    }
    MyMarker
    on(|change: On<ValueChange<bool>>, mut config: ResMut<ShadowConfig>| {
        config.enabled = change.value;
    })
}
```

Define your own with `#[derive(SceneComponent, Default, Clone)]` and `#[scene(MyProps)]` plus `fn scene(props: MyProps) -> impl Scene` on the type (`#[scene(other_fn)]` or `#[scene(Other::scene)]` names a different fn). The derive also implements `Component` — don't add `#[derive(Component)]`.

**Derives:** `Default + Clone` is all a component needs to be patchable — `FromTemplate` is blanket-implemented for `Clone + Default` types. Reach for `#[derive(FromTemplate)]` only when a field needs spawn-time context: a `Handle<T>` so `Sprite { image: "player.png" }` resolves through the `AssetServer`, or an `Entity` filled from a `#Name` reference. Deriving `FromTemplate` overrides the blanket impl, and adding `Default` alongside it is fine (the engine's own `Sprite` does). Enums need only `Default + Clone` too, but every field of a variant must be written out.

## Relationship to the old scene system

| Concern | Home |
| --- | --- |
| BSN, `bsn!`, `spawn_scene`, `SceneComponent` | `bevy_scene` / `bevy::scene` |
| glTF scene spawning (`WorldAssetRoot`), `World` (de)serialization (`DynamicWorld`) | `bevy_world_serialization` / `bevy::world_serialization` |

For glTF you still use `WorldAssetRoot(asset_server.load("scene.gltf#Scene0"))` — the glTF loader hasn't been ported to BSN. See `references/assets.md`.

## Honest limitations (still true in 0.20)

What works: the `bsn!` / `bsn_list!` macros (inline Rust scenes), `spawn_scene`/`spawn_scene_list`/`queue_spawn_scene`, the `.spawn()` startup idiom, `Children`/`on(...)`, `Ready`, scene-fn composition via `@` includes, `@SceneComponent { @prop }`, and `ScenePatchInstance` for deferred application.

What is **not** ready:

- **`.bsn` files don't load.** The asset format isn't released — `:"path.bsn"` and `#[scene("file.bsn")]` parse and compile, but won't load at runtime (the only `AssetLoader` in `bevy_scene` is a test-only fake).
- **glTF isn't ported to BSN.** Use `WorldAssetRoot` from `bevy_world_serialization`.
- **No `World` → BSN round-trip.** Writing a world out is still `bevy_world_serialization`'s `DynamicWorld`.

So in 0.20, treat BSN as a *code-first* scene/composition tool — excellent for UI and widgets, premature for asset-driven scene files. When the `.bsn` format lands (targeted for a later release) the same `bsn!` syntax becomes portable to files.
