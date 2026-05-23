# Common pitfalls (and what to do instead)

## Contents
- Communication — polling for spawn-time setup; Event/Message confusion; `trigger_targets` removal; `Trigger<E>` rename; `Replace`→`Discard` (0.19)
- Change detection — mutable deref always marks changed; `RemovedComponents` in `FixedUpdate`
- Components and bundles — `Handle<T>` not a Component; bundle arity; `children!` macro limit; `clear_children` rename
- Schedules and states — `set` re-fires `OnEnter`/`DespawnOnExit`; `add_event` vs `add_message`; manual `register_type`; system ordering source-of-truth
- Resources and lifetimes — resources-as-components / no `Component+Resource` co-derive (0.19); broad-query conflicts; non-static lifetime forbidden; `AmbientLight` split
- Rendering — `Camera::target` move; `Atmosphere` now an entity (0.19); light `shadow_maps_enabled` (0.19); `MaterialPlugin` field-to-method; auto-Aabb; mesh `try_*` methods
- Color arithmetic — direct ops removed
- Assets — handle drop = unload; `SceneRoot`→`WorldAssetRoot` (0.19); `get_mut`→`AssetMut` (0.19); asset path resolution; `LoadContext::asset_path` removal
- UI — `Val::Px(...)` verbosity; `Transform` on UI nodes; text `FontSource`/`FontSize` (0.19); `InputFocus` fields private (0.19); widgets/Feathers de-experimentalized (0.19); non-text picking
- Cargo features — additive surprise; collection migration; `audio`/`ui` no longer implied (0.19); picking-backend renames
- Misc — `cargo clean` reflexes; same-system `next_state`/spawn ordering; reflection generics
- Performance traps — per-frame allocation; asset content cloning; shipping `dynamic_linking`

Patterns that look reasonable but bite later, or 0.16/0.17/0.18-era idioms that no longer work in 0.19. Each one has the symptom alongside the fix.

## Communication

### Polling for spawn-time setup in `Update`

**Symptom**: A system queries `Query<Entity, With<NeedsHookup>>` every frame, does work for matching entities, removes the marker. Cheap when no entities match, but executes the polling system every tick forever.

**Fix**: Use an observer.

```rust
commands.add_observer(|add: On<Add, NeedsHookup>, mut commands: Commands /* deps */| {
    // Hookup work, with full system params.
    commands.entity(add.entity).remove::<NeedsHookup>();
});
```

Fires once, immediately when `NeedsHookup` is added. No per-frame polling cost.

**Exception**: when hookup needs to wait for *both* an asset to finish loading *and* a child component to appear, polling each frame and bailing early is the simplest form. Observers don't help when the trigger condition isn't a single component add.

### `EventReader<E>` doesn't compile / `is not a Message`

**Symptom**: `MyEvent is not a Message`, or `method 'send' not found for MessageWriter`.

**Cause**: The 0.17 split of `Event` (observable) from `Message` (buffered).

**Fix**: Pick one.

For buffered: change `#[derive(Event)]` to `#[derive(Message)]`, change `EventReader`/`EventWriter` to `MessageReader`/`MessageWriter`, change `app.add_event::<E>()` to `app.add_message::<M>()`, change `.send(...)` to `.write(...)`.

For observable: keep `#[derive(Event)]` (and add `Clone` if not already there), replace the system using `EventReader` with an observer that takes `On<E>`, replace `events.send(...)` with `commands.trigger(E)`.

### Mixing `EventReader<Foo>` with `add_observer` for the same `Foo`

**Symptom**: Half the consumers work, half don't.

**Cause**: You can't have it both ways without explicitly implementing both `Event` and `Message` traits for the type. The default `#[derive(Event)]` produces an `Event`, not a `Message`.

**Fix**: Pick one (see above). If you genuinely need both — rare — implement both traits manually.

### `world.trigger_targets(E, entity)` doesn't exist

**Symptom**: `method trigger_targets not found`.

**Cause**: Removed in 0.17. Targeting is now baked into the event type via `EntityEvent`.

**Fix**: Make `E` an `EntityEvent` with an `entity` field (or another `Entity` field with `#[event_target]`):

```rust
#[derive(EntityEvent)]
struct Click { entity: Entity }

commands.trigger(Click { entity: target });
```

### Observer parameter `Trigger<E>` doesn't compile

**Symptom**: `cannot find type Trigger`.

**Cause**: Renamed to `On<E>` in 0.17.

**Fix**: `On<E>` instead of `Trigger<E>`. Bind name should match the event semantics (e.g., `click: On<Click>`) rather than `trigger`.

### `On<Replace, T>` / `on_replace` not found (0.19)

**Symptom**: `cannot find type Replace`, or `#[component(on_replace = ...)]` rejected.

**Cause**: 0.19 renamed the lifecycle event `Replace` → `Discard` (it fires when a component is removed *or* replaced by a new value).

**Fix**: `On<Discard, T>`, `ComponentHooks::on_discard`, `#[component(on_discard = ...)]`. The `Replace`/`OnReplace` doc-aliases remain only for search.

## Change detection

### Mutable deref always marks changed

**Symptom**: A downstream system gated on `Changed<T>` fires every frame even though the value hasn't actually changed.

**Cause**: `*foo = ...` always marks `foo` as changed, even if the new value equals the old value. `for mut x in q.iter_mut()` followed by *any* deref-write does this.

**Fix**: Use `set_if_neq` for components/resources that implement `PartialEq`:

```rust
foo.set_if_neq(NewValue);
```

Or guard the write:

```rust
if foo.value != new_value {
    foo.value = new_value;
}
```

For resources, `ResMut::set_if_neq(...)` works the same way.

### Reading via `Query<&T>` doesn't update change ticks correctly with `Mut<T>`

**Symptom**: `Changed<T>` filter misses changes that should have been picked up.

**Cause**: Less common — usually means a system is using `Mut<T>` directly without proper deref. The smart pointer's `DerefMut` is what marks changed; using `bypass_change_detection()` skips it.

**Fix**: Don't use `bypass_change_detection()` unless you specifically want to skip the marker. For normal mutation, plain `*foo = ...` is correct.

### `RemovedComponents` misses removals in `FixedUpdate`

**Symptom**: Removal-handling logic in `FixedUpdate` occasionally fails to fire.

**Cause**: `RemovedComponents` is cleared between `World` updates; in `FixedUpdate`, multiple updates can happen between reads and clears.

**Fix**: Use an `On<Remove, T>` observer or a `#[component(on_remove = ...)]` hook instead. These also give you access to the component's *value* before it's removed, which `RemovedComponents` doesn't.

## Components and bundles

### `Query<&Handle<StandardMaterial>>` doesn't compile

**Symptom**: `Handle<StandardMaterial> is not a Component`.

**Cause**: 0.17 made the wrapper components (`MeshMaterial3d<M>`) the actual Component. Bare `Handle<T>` is no longer a Component for materials/meshes.

**Fix**:

```rust
Query<&MeshMaterial3d<StandardMaterial>>
```

Access the underlying handle with `.0`:

```rust
for mat in &query {
    if let Some(m) = materials.get_mut(&mat.0) { /* ... */ }
}
```

Same applies for `MeshMaterial2d`, `Mesh3d`, `Mesh2d`. For sprites, use `Sprite::from_image(handle)`.

### Bundle with >12 components

**Symptom**: Compile error about exceeding tuple bundle size when spawning.

**Cause**: Tuple bundle support tops out around 12-16 elements (depends on Bevy version).

**Fix**: Nest tuples — `(A, B, C, (D, E, F))` is fine. Each inner tuple counts as one element of the outer.

Or extract a custom bundle with `#[derive(Bundle)]`:

```rust
#[derive(Bundle, Default)]
struct EnemyBundle {
    enemy: Enemy,
    health: Health,
    transform: Transform,
    visibility: Visibility,
    /* etc */
}

commands.spawn(EnemyBundle::default());
```

Or use required components (preferred for "always together"):

```rust
#[derive(Component)]
#[require(Health, Transform, Visibility)]
struct Enemy;
```

### `children!` macro hits limit

**Symptom**: `children!` panics or fails to compile with many children. Old code may say "limited to 12."

**Fix**: 0.17+ supports up to ~1400 children per `children!` call (Rust recursion limit). For more:

```rust
Children::spawn(SpawnIter(items.into_iter().map(|item| /* spawn fn */)))
```

### `clear_children` / `remove_child` don't exist

**Symptom**: `method clear_children not found on EntityCommands`.

**Cause**: Renamed in 0.18 to `detach_*` to make it clear that children are detached, not despawned.

**Fix**:

| Old | New |
| --- | --- |
| `entity.clear_children()` | `entity.detach_all_children()` |
| `entity.remove_children(&[...])` | `entity.detach_children(&[...])` |
| `entity.remove_child(c)` | `entity.detach_child(c)` |
| `entity.clear_related::<R>()` | `entity.detach_all_related::<R>()` |

## Schedules and states

### `next_state.set(X)` re-fires `OnEnter` even when already in `X`

**Symptom**: A system in `OnEnter(State::X)` runs more times than expected.

**Cause**: 0.18 always re-fires `OnEnter`/`OnExit` on `set`, even when the state was already `X`.

**Fix**: `next_state.set_if_neq(X)` for the old "skip if equal" behavior.

This is sometimes intentional — you might want to re-fire setup on every state-change attempt. Just be explicit about which behavior you want.

### `app.add_event::<E>()` for buffered events doesn't work

**Cause**: Renamed to `add_message`.

**Fix**: `app.add_message::<M>()` for messages. (See Communication section.)

### Manual `register_type::<T>()` calls

**Symptom**: Doesn't fail, just unnecessary noise in plugin code.

**Cause**: 0.17 made `Reflect` auto-register via the `inventory` crate.

**Fix**: Drop `register_type::<T>()` for non-generic types. Keep them only for concrete instantiations of generic types: `register_type::<Container<Item>>()`.

If you're on a platform without `inventory` support, use the static-registration variant from the `auto_register_static` example.

### System ordering surprises after splitting plugins

**Symptom**: Two plugins both call `configure_sets` and the relative ordering depends on plugin add order.

**Fix**: One source of truth for ordering. Define a `SystemSet` enum centrally. `configure_sets` only in `app.rs`. Plugins drop systems into named sets via `.in_set(...)`, never call `configure_sets`.

## Resources and lifetimes

### `#[derive(Component, Resource)]` on one type doesn't compile (0.19)

**Symptom**: "conflicting implementations of trait `Component`".

**Cause**: 0.19 made `Resource` a subtrait of `Component`, and `#[derive(Resource)]` now *also* generates the `Component` impl. Deriving both gives two `Component` impls.

**Fix**: Pick one, or split into two types. If you only ever used it as a resource, drop the `Component` derive. If you genuinely need both a per-entity component and a singleton resource, make `FooComponent` and `FooResource`.

### `#[reflect(Resource)]` no longer carries reflection data (0.19)

**Symptom**: Reflection-driven code (BRP, `bevy_world_serialization`) can't find/operate on a resource.

**Cause**: `ReflectResource` became a zero-sized marker in 0.19; the real machinery lives in `ReflectComponent` (since resources are components).

**Fix**: Use `#[reflect(Component)]` / `ReflectComponent` for resources too. (You can also drop `#[derive(MapEntities)]` from resources — entity mapping is automatic for components.)

### Broad `Query<EntityMut>` conflicts with `Res<T>` (0.19)

**Symptom**: A system that worked before now fails to build with an access conflict between a catch-all query and a resource.

**Cause**: Resources are entities now, so broad queries (`Query<Entity>`, `Query<EntityMut>`, `Query<EntityRef>`, `Query<Option<&T>>`) match resource entities and can collide with `Res`/`ResMut`/`NonSend` in the same system.

**Fix**: Exclude resource entities: `Query<EntityMut, Without<IsResource>>` (or `Without<TheSpecificResource>`).

### `#[derive(Resource)] struct Foo<'a>` doesn't compile

**Symptom**: 0.18 onward refuses to derive `Resource` for types with non-static lifetimes.

**Fix**: Make the type `'static`. If it needs a borrowed reference, store an `Arc<...>` or owned data instead.

### `AmbientLight` as a resource doesn't work

**Symptom**: `app.insert_resource(AmbientLight { .. })` panics or behaves wrong in 0.18.

**Cause**: 0.18 split `AmbientLight` (per-camera component) from `GlobalAmbientLight` (world-level resource).

**Fix**:

```rust
// World default:
app.insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 2000.0, ..default() });

// Per-camera override:
commands.spawn((Camera3d::default(), AmbientLight { /* ... */ }));
```

## Rendering

### `Camera { target: RenderTarget::Image(...), .. }`

**Symptom**: `field 'target' not found on Camera`.

**Cause**: 0.18 moved `RenderTarget` off `Camera` into its own component.

**Fix**:

```rust
commands.spawn((
    Camera3d::default(),
    RenderTarget::Image(image_handle.into()),
));
```

### `Atmosphere::default()` / `Atmosphere::earthlike` / `Atmosphere` on the camera

**Symptom**: `Atmosphere does not implement Default`; `no method earthlike`; or atmosphere doesn't render when added to the camera.

**Cause**: 0.18 generalized atmospheric scattering to require a `ScatteringMedium` asset. **0.19 moved `Atmosphere` to `bevy_light`, made it a standalone entity (not a camera component), and renamed `earthlike` → `earth`** (and `bottom_radius`/`top_radius` → `inner_radius`/`outer_radius`). `AtmosphereSettings` stays on the camera and is what enables atmosphere rendering for that view.

**Fix** (0.19):

```rust
use bevy::light::{atmosphere::ScatteringMedium, Atmosphere};
use bevy::pbr::AtmosphereSettings;

fn setup(mut commands: Commands, mut media: ResMut<Assets<ScatteringMedium>>) {
    // Atmosphere is its own entity now.
    commands.spawn(Atmosphere::earth(media.add(ScatteringMedium::earth(256, 256))));
    // The camera opts in via AtmosphereSettings.
    commands.spawn((Camera3d::default(), AtmosphereSettings::default()));
}
```

### `PointLight { shadows_enabled: true }` field not found (0.19)

**Symptom**: `no field shadows_enabled on PointLight` (also `DirectionalLight`, `SpotLight`).

**Cause**: 0.19 renamed `shadows_enabled` → `shadow_maps_enabled` (because lights now also support *contact* shadows, with a separate `contact_shadows_enabled` field).

**Fix**: Rename the field to `shadow_maps_enabled`. Set `contact_shadows_enabled: true` to add screen-space contact shadows (the camera needs a `ContactShadows` component).

### `MaterialPlugin::<M> { prepass_enabled: false, .. }` field doesn't exist

**Symptom**: Field name not found.

**Cause**: 0.18 moved these to `Material` trait methods.

**Fix**:

```rust
impl Material for MyMaterial {
    fn enable_prepass() -> bool { false }
    fn enable_shadows() -> bool { false }
    // ...
}
```

### `entity.remove::<Aabb>()` after mutating mesh

**Symptom**: Workaround code that removes `Aabb` to force regeneration is no longer needed.

**Cause**: 0.18 auto-updates `Aabb` for mutated meshes/sprites.

**Fix**: Remove the workaround. Use `NoAutoAabb` to opt out of auto-management for specific entities.

### `mesh.insert_attribute(...)` panics on some meshes

**Symptom**: Panic with "mesh data has been extracted to render world."

**Cause**: 0.18: meshes with `RenderAssetUsages::RENDER_WORLD` only retain their data on the GPU. The non-`try_*` mesh-mutation methods now panic in this case.

**Fix**: Either use `try_*` variants and handle the `MeshAccessError`:

```rust
mesh.try_insert_attribute(Mesh::ATTRIBUTE_POSITION, positions)?;
```

Or set `RenderAssetUsages::all()` (default) when creating the mesh to keep CPU data alongside the GPU upload.

## Color arithmetic

### `let dim = color * 0.5;` doesn't compile

**Cause**: Direct color arithmetic was removed (it was unclear which color space the operation happened in).

**Fix**: Convert to a linear color space first:

```rust
let linear = color.to_linear();
let dim = LinearRgba::new(linear.red * 0.5, linear.green * 0.5, linear.blue * 0.5, linear.alpha);
```

Or extract sRGBA components:

```rust
let srgb = color.to_srgba();
let dim = Color::srgba(srgb.red * 0.5, srgb.green * 0.5, srgb.blue * 0.5, srgb.alpha);
```

## Assets

### Handle dropped immediately after `load`

**Symptom**: Asset loads, you spawn an entity using it, the entity despawns, and now the asset is gone — re-spawning triggers a fresh load.

**Cause**: `Handle<T>` is reference-counted. When all clones drop, the asset is unloaded.

**Fix**: Hold the handle in a resource:

```rust
#[derive(Resource)]
struct EnemyAssets { sprite: Handle<Image> }

fn preload(server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(EnemyAssets {
        sprite: server.load("enemy.png"),
    });
}
```

Then clone from the resource when spawning. Cloning a handle is cheap (refcount bump). The asset stays loaded as long as `EnemyAssets` lives.

### `cargo run` panics with "asset not found" on first launch

**Symptom**: Panic referencing an asset path.

**Cause**: Bevy looks for assets in `assets/` next to your `Cargo.toml`. If it isn't there, or if you're running from a different directory, paths break.

**Fix**: Either ensure you `cargo run` from the project root, set `BEVY_ASSET_ROOT=/absolute/path/to/assets`, or override `AssetPlugin::file_path` in your `App` setup.

For shipped builds, the `assets/` folder needs to be alongside the final binary.

### `SceneRoot(...)` for glTF doesn't compile (0.19)

**Symptom**: `cannot find SceneRoot` / `cannot find DynamicScene`.

**Cause**: 0.19 renamed the old scene crate `bevy_scene` → `bevy_world_serialization` (the `bevy_scene` name now hosts the new BSN system).

**Fix**: `WorldAssetRoot(asset_server.load("scene.gltf#Scene0"))` (was `SceneRoot`). `DynamicScene` → `DynamicWorld`, `DynamicSceneBuilder` → `DynamicWorldBuilder`, etc. All in the prelude. Direct material sub-asset loads now yield `Handle<GltfMaterial>`, not `Handle<StandardMaterial>`.

### `assets.get_mut(&h)` returns the wrong type / fires Modified constantly (0.19)

**Symptom**: `let Some(img) = images.get_mut(...)` won't let you mutate (`img` is not `&mut`), or a material re-extracts to the render world every frame.

**Cause**: 0.19 changed `Assets::get_mut` to return an `AssetMut<A>` smart pointer (like `Mut<T>`) that emits `AssetEvent::Modified` on deref-mutation.

**Fix**: Bind it `mut` (`let Some(mut img) = ...`). Guard writes so you only mutate on real change — `if mat.base_color != new { mat.base_color = new; }` — to avoid needless `Modified` events (a real cost for materials). `bypass_change_detection()` skips the event entirely.

### `LoadContext::asset_path` doesn't exist

**Cause**: 0.18 removed `LoadContext::asset_path`. `LoadContext::path` now returns `AssetPath` (it used to return `Path`).

**Fix**: `load_context.path()` for the `AssetPath`. If you really need a `Path`, `load_context.path().path()` — but prefer `AssetPath` for custom-asset-source compatibility.

## UI

### `Val::Px(...)` everywhere

Not a bug, but verbose. 0.17 helpers are concise and equivalent:

```rust
// Old
Node {
    width: Val::Px(200.0),
    padding: UiRect::all(Val::Px(10.0)),
    ..default()
}

// New
Node {
    width: px(200),
    padding: px(10).all(),
    ..default()
}
```

### `Transform` on UI nodes

**Symptom**: Layout fights against your Transform changes.

**Cause**: UI uses `UiTransform`/`UiGlobalTransform` (0.17+), not the regular `Transform` propagation.

**Fix**: Don't put `Transform` on UI entities. Use `Node` for layout. If you need to animate UI position, animate `Node.left`/`Node.top` (with `position_type: Absolute`) or `UiTransform` directly.

### `TextFont { font: handle, font_size: 24.0 }` doesn't compile (0.19)

**Symptom**: type mismatch on `font` (expected `FontSource`, found `Handle<Font>`) or `font_size` (expected `FontSize`, found `f32`).

**Cause**: 0.19 migrated text to `parley`; `font` is now `FontSource` and `font_size` is `FontSize`.

**Fix**: `font: handle.into()` (or `FontSource::Family("…")` / `FontSource::Monospace`), `font_size: FontSize::Px(24.0)`. New `weight`/`width`/`style` fields cover variable fonts; `LetterSpacing` is its own component. `TextLayout::new_with_justify` → `TextLayout::justify`.

### `input_focus.0 = Some(entity)` doesn't compile (0.19)

**Symptom**: field `.0` of `InputFocus` is private.

**Cause**: 0.19 made `InputFocus` fields private and added a `FocusCause`.

**Fix**: `input_focus.set(entity, FocusCause::Navigated)`, `input_focus.get()`, `input_focus.clear()`. (Core `InputFocus` setup also moved from `InputDispatchPlugin` to `InputFocusPlugin`, both in `DefaultPlugins`.)

### `experimental_bevy_ui_widgets` / `experimental_bevy_feathers` feature not found (0.19)

**Symptom**: unknown feature, or `UiWidgetsPlugins` added twice / `FeathersPlugin` not found.

**Cause**: Both graduated from experimental in 0.19. `experimental_bevy_ui_widgets` → `bevy_ui_widgets` (now in `ui`/default features and `DefaultPlugins`); `experimental_bevy_feathers` → `bevy_feathers`; `FeathersPlugin` → `FeathersCorePlugin`.

**Fix**: Drop the `experimental_` prefix from feature names; remove manual `add_plugins(UiWidgetsPlugins)`/`InputDispatchPlugin` when you have `DefaultPlugins`; rename `FeathersPlugin` → `FeathersCorePlugin`. Widget components also lost the `Core` prefix (`CoreScrollbarThumb` → `ScrollbarThumb`).

### Non-text areas of `Text` no longer pickable (0.18)

**Symptom**: A click handler on a `Text` node only fires when the user clicks directly on a glyph.

**Cause**: 0.18 narrowed picking precision to just the glyph rectangles.

**Fix**: Wrap the `Text` in a parent `Node`, put the picking observer on the parent.

## Cargo features

### Mysterious feature appears enabled despite not being in your `Cargo.toml`

**Symptom**: Code paths gated on a feature run, but you didn't enable it.

**Cause**: A dependency enables it for you. Cargo features are *additive* — once anything in the dependency tree enables a feature, it's on for everyone.

**Fix**: `cargo tree -f "{p} {f}"` shows what's enabled and by whom. If the upstream enabling is incidental (a default feature you didn't want), check whether you can disable default features for that crate. Worst case, file an issue with the upstream maintainer.

### Hand-listing 30+ features

Not a bug, but maintenance burden. Feature collections (`3d`, `ui`, etc.) cover most use cases. Switch to:

```toml
bevy = { version = "0.19", default-features = false, features = ["3d", "ui"] }
```

Add specific features only when the collection is missing something. The collections are designed to be the right size for typical apps.

### `audio`/`ui` missing in a non-default build (0.19)

**Symptom**: After upgrading to 0.19 with `default-features = false, features = ["3d"]`, audio or UI stops working.

**Cause**: 0.19 stopped having `2d`/`3d` implicitly pull in `audio` and `ui`, so you could swap those subsystems without a feature soup.

**Fix**: List them: `features = ["3d", "ui", "audio"]`. (Conversely, dropping `bevy_audio` is now just leaving `audio` off.) Also: `bevy_window`/`bevy_input_focus`/`custom_cursor` left the `default_app` collection, and the Android activity backend is no longer a default — add what you need explicitly.

### Renamed feature names (0.18)

| Old | New |
| --- | --- |
| `bevy_sprite_picking_backend` | `sprite_picking` |
| `bevy_ui_picking_backend` | `ui_picking` |
| `bevy_mesh_picking_backend` | `mesh_picking` |
| `animation` | `gltf_animation` |

## Misc

### `cargo clean` to "fix" weird build errors

**Symptom**: A build is failing in confusing ways. Reflex: `cargo clean`.

**Cost**: Bevy from-scratch is multi-minute. Each `cargo clean` costs you a real chunk of time.

**Fix**: Investigate the actual error first. Cargo's incremental build is reliable; it's rarely the cause. Check for:

- Mismatched dependency versions (`cargo tree -d`).
- Feature flag conflicts (something enabling a feature you didn't want).
- Stale `target/cargo-timings` reports referencing old plugin builds.

If you genuinely need to nuke caches, target a specific package: `cargo clean -p my_crate`. Reserve full `cargo clean` for actual corruption (very rare).

### Setting `next_state` then querying state in the same system

**Symptom**: The state appears unchanged after `next_state.set(X)`.

**Cause**: State transitions don't apply until the `StateTransition` schedule runs (between `PreUpdate` and `Update`). Within a single system, you can't set the state and immediately observe the transition.

**Fix**: Either restructure to read state in the next system that runs after `StateTransition`, or use a custom schedule run via `world.run_schedule(StateTransition)` for sub-frame transitions.

### Spawning then querying within the same system

**Symptom**: `commands.spawn(...).id()` then `query.get(spawned_entity)` returns nothing.

**Cause**: Commands flush at the end of the schedule (or at explicit sync points), not at the end of the system.

**Fix**: Capture the entity ID and act on it later (next system, or in the response observer triggered by `On<Add, T>`). For "spawn and immediately use," use `world.spawn(...)` directly via `&mut World` or a `Commands::queue` closure — but those only work in exclusive systems.

### Forgotten `register_type::<MyEnum>()` for variants

**Symptom**: Reflection sees `MyEnum` but not its variants in serialization, or specific generic instantiations are missing.

**Cause**: While `Reflect` auto-registers types, *generic instantiations* still need manual registration:

```rust
app.register_type::<Container<Item>>();
```

Without this, the specific monomorphized type isn't visible to reflection.

### Custom `SystemParam::validate_param` no longer exists (0.19)

**Symptom**: `method validate_param is not a member of trait SystemParam`, or `SystemState::get` now returns a `Result` you didn't expect.

**Cause**: 0.19 merged param validation into fetching — `validate_param` was removed and `get_param` now returns `Result<Self::Item, SystemParamValidationError>`.

**Fix**: Delete `validate_param`; move its logic into `get_param`, returning `Err(SystemParamValidationError::skipped::<Self>("…"))` to skip or `::invalid::<Self>("…")` to error. Wrap the success case in `Ok(...)`. `SystemState::get`/`get_mut` now return `Result` — add `.unwrap()` or handle it.

## Performance traps

### Per-frame allocation of large data structures

**Symptom**: Inexplicable GC-like stutter in profiling traces.

**Cause**: A system allocates a `Vec`, fills it, drops it, every frame.

**Fix**: Use a `Local<Vec<T>>` and clear/reuse:

```rust
fn process(mut buffer: Local<Vec<i32>>) {
    buffer.clear();
    // fill and use
}
```

The buffer's allocation is amortized across frames.

### Unnecessary asset clones

**Symptom**: Memory grows over time despite handle refcounting.

**Cause**: Cloning the asset *contents* (e.g., `Image::clone()`) is expensive. The asset framework clones handles cheaply (refcount); cloning the underlying asset duplicates the data.

**Fix**: Clone the handle, not the asset. If you need to mutate one entity's view of an asset without affecting others, ask whether the design really needs that — usually the right answer is per-entity components, not asset duplication.

### Enabling `dynamic_linking` in release builds

**Symptom**: Released binary depends on `libbevy_dylib`. Won't run unless that file is shipped alongside.

**Fix**: Make `dynamic_linking` a dev-only feature. The standard pattern:

```toml
[features]
fast-compile = ["bevy/dynamic_linking"]
```

Use `cargo run --features fast-compile` for development. Release builds without the flag produce standalone binaries.
