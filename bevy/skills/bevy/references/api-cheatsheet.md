# Bevy 0.16 → 0.17 → 0.18 API rename cheatsheet

## Contents
- ECS communication (Event/Message/Observer split — 0.17)
- ECS data (required components, material wrappers, color arithmetic — 0.17)
- Children / hierarchy (`detach_*` rename, `children!` macro limit)
- Rendering and assets (`RenderTarget`, `AmbientLight`, `Atmosphere`, Aabb)
- Schedules and states (state set re-fire, system set naming, `RenderStartup`)
- UI (`Val` helpers, `UiTransform`, `BorderRadius` folding)
- Resources (lifetime requirements, `BindGroupLayoutDescriptor`)
- Cargo features (collections, picking-backend renames)
- Errors and entities (0.18 internal — `EntityIndex`, allocator rework)
- Search aids — common error messages and what they mean

Use this when modernizing existing code or interpreting compile errors that look version-mismatched. Symptom alongside the fix.

## ECS communication (the big one — 0.17)

| Old (≤0.16) | New (0.17+) | Notes |
| --- | --- | --- |
| `EventReader<E>` | `MessageReader<M>` | And the trait is now `Message`, not `Event`. |
| `EventWriter<E>` | `MessageWriter<M>` | `.send()` → `.write()`. |
| `Events<E>` resource | `Messages<M>` resource | Same double-buffer semantics. |
| `app.add_event::<E>()` | `app.add_message::<M>()` | For buffered communication. |
| `Trigger<E>` parameter | `On<E>` parameter | Same role, shorter name. |
| `OnAdd` | `Add` | Lifecycle event. Same for `OnInsert/OnReplace/OnRemove/OnDespawn` → `Insert/Replace/Remove/Despawn`. |
| `world.trigger_targets(E, entity)` | `commands.trigger(E { entity, .. })` where `E: EntityEvent` | Targeting is now baked into the event type. |
| `commands.add_observer(\|t: Trigger<E>\| ...)` | `commands.add_observer(\|e: On<E>\| ...)` | Name the binding after the *event*, not the trigger. |

If you see `MyEvent is not a Message` or `method 'send' not found for MessageWriter`, you have an `Event` being used as if it were a `Message`. Either switch the derive to `Message` (and use reader/writer), or move to observers (and use `add_observer` + `commands.trigger`).

## ECS data (0.17)

| Old | New | Notes |
| --- | --- | --- |
| `app.register_type::<NonGeneric>()` | (drop it) | `#[derive(Reflect)]` auto-registers via `inventory`. Keep only for generic instantiations like `register_type::<Container<Item>>()`. |
| Bundle structs (still exist) | `#[require(Other)]` on the component | Required components replace bundles for "always together" composition. Bundles still work as tuples for ad-hoc spawn. |
| `Query<&Handle<StandardMaterial>>` | `Query<&MeshMaterial3d<StandardMaterial>>` | The handle isn't a component; the wrapper is. Same for `Mesh3d`/`Mesh2d`/`MeshMaterial2d`. |
| `Color * f32` arithmetic | `LinearRgba::rgb(c.red * f, c.green * f, c.blue * f)` | Color arithmetic was removed; convert to a linear color space first. |
| `entity.insert(AnimationTarget { id, player })` | `entity.insert((AnimationTargetId(id), AnimatedBy(player)))` | (0.18) Split into two components for flexibility. |
| `query.get_many_mut([a, b])` | `query.get_many_mut([a, b])` | Still works; in 0.18 also `entity.get_components_mut::<(&mut A, &mut B)>()` for type-driven multi-component access on a single entity. |

## Children / hierarchy

| Old | New | Notes |
| --- | --- | --- |
| `entity.clear_children()` | `entity.detach_all_children()` | (0.18) Clearer that children aren't despawned. |
| `entity.remove_children(&[...])` | `entity.detach_children(&[...])` | Same renaming. |
| `entity.remove_child(child)` | `entity.detach_child(child)` | |
| `entity.clear_related::<R>()` | `entity.detach_all_related::<R>()` | Generalized relationships. |
| `children!` capped at 12 children | `children!` supports ~1400 | Rust recursion limit; for more, use `Children::spawn(SpawnIter(...))`. |
| `Parent` (component) | `ChildOf` (component) | And `Children` for the parent-side collection. The naming convention: name the component from the *holder's* perspective. |

## Rendering and assets

| Old | New | Notes |
| --- | --- | --- |
| `Camera { target: RenderTarget::Image(...), .. }` | Spawn `RenderTarget::Image(...)` as a separate component | (0.18) `RenderTarget` moved off `Camera`. |
| `AmbientLight` as a resource | `GlobalAmbientLight` resource + optional `AmbientLight` component (per camera) | (0.18) Split into world default and per-camera override. |
| `MaterialPlugin::<M> { prepass_enabled: false, .. }` | `impl Material for M { fn enable_prepass() -> bool { false } }` | (0.18) Per-material trait methods, not plugin fields. Same for `enable_shadows`. |
| `Atmosphere::default()` | `Atmosphere::earthlike(media.add(ScatteringMedium::default()))` | (0.18) Generalized scattering needs an asset. |
| `entity.remove::<Aabb>()` after mesh mutation | (drop it) | (0.18) Aabb auto-updates. Use `NoAutoAabb` to opt out. |
| `Image::reinterpret_size(...)` (panicking) | `Image::reinterpret_size(...)?` (returns `Result`) | (0.18) Made fallible instead of panicking. |
| `mesh.insert_attribute(...)` on a `RENDER_WORLD`-only mesh | `mesh.try_insert_attribute(...)?` | (0.18) The non-`try_` variants still exist but now panic if the mesh has been extracted to render world. Use `try_*` if there's any chance the mesh is render-only. |

## Schedules and states

| Old | New | Notes |
| --- | --- | --- |
| `next_state.set(X)` (idempotent) | Now re-fires `OnEnter`/`OnExit` even if equal | (0.18) Use `next_state.set_if_neq(X)` for the old "skip if equal" behavior. |
| `prepass_enabled` plugin field | `Material::enable_prepass()` trait method | (0.18) See above. |
| `bevy_internal::*Set` mixed naming | `*Systems` suffix on system sets | (0.17) Convention shift — `PickSet` → `PickingSystems`, `Animation` → `AnimationSystems`, `GizmoRenderSystem` → `GizmoRenderSystems`. |
| `RenderApp` finish-time init | `RenderStartup` schedule + systems in `Plugin::build` | (0.17) Renderer plugins use a normal startup schedule now. Old `Plugin::finish` patterns still work but new code should use `RenderStartup`. |

## UI

| Old | New | Notes |
| --- | --- | --- |
| `Val::Px(200.0)` | `px(200)` | (0.17) `px`/`percent`/`vw`/`vh`/`vmin`/`vmax` helpers. The originals still work. |
| `UiRect { left: Val::Px(10.0), .. default() }` | `px(10).left()` (or `.right()/.top()/.bottom()/.all()/.horizontal()/.vertical()`) | (0.17) Fluent UiRect builder. |
| `Transform`/`GlobalTransform` on UI nodes | `UiTransform`/`UiGlobalTransform` | (0.17) UI got its own specialized 2D transform; don't reach for `Transform` on UI entities. |
| `BorderRadius` as a component | `Node { border_radius: BorderRadius { .. }, .. }` field | (0.18) Folded into `Node`. |
| `Text` non-text areas pickable | Only text glyphs pickable; wrap in a parent `Node` for hit-test on the full node | (0.18) Picking precision change. |
| `BorderColor(Color::WHITE)` | `BorderColor::all(Color::WHITE)` (or per-side: `.set_left(...)` etc.) | (0.17) Per-side border colors. |

## Resources

| Old | New | Notes |
| --- | --- | --- |
| `#[derive(Resource)] struct Foo<'a> { .. }` | `#[derive(Resource)] struct Foo { .. }` | (0.18) Resources require `'static`. |
| `AmbientLight` resource (see above) | `GlobalAmbientLight` resource | |
| `BindGroupLayout` field on pipeline descriptor | `BindGroupLayoutDescriptor` field | (0.18) Lazy creation; use `pipeline_cache.get_bind_group_layout(&desc)` to materialize. |

## Cargo features (0.18)

| Old | New | Notes |
| --- | --- | --- |
| Hand-listing 30+ Bevy features | `bevy = { default-features = false, features = ["3d", "ui"] }` | (0.18) Top-level collections: `2d`, `3d`, `ui`, `audio`, `dev`. Plus mid-level `2d_api`, `3d_api`, `default_app`, `default_platform`. |
| `bevy_sprite_picking_backend` | `sprite_picking` | (0.18) Feature renamed for consistency. Same for `bevy_ui_picking_backend` → `ui_picking`, `bevy_mesh_picking_backend` → `mesh_picking`. |
| `animation` feature | `gltf_animation` | (0.18) Renamed to make the scope clear. |

## Errors and entities (0.18 internal)

These rarely matter at the gameplay layer but show up if you're touching ECS internals:

- `Entity::row` → `Entity::index`; `Entity::from_row` → `Entity::from_index`; `EntityRow` → `EntityIndex`
- `EntityDoesNotExistError` → split into `InvalidEntityError`, `EntityValidButNotSpawnedError`, `EntityNotSpawnedError`
- `Entities::alloc/free/reserve/flush` → moved to `EntitiesAllocator`; `World::spawn_at` replaces flush-after-reserve
- `QueryEntityError::EntityDoesNotExist` → `QueryEntityError::NotSpawned`
- `EntityEvent::from` and `EntityEvent::event_target_mut` → moved to a separate `SetEntityEventTarget` trait (immutable by default)
- `clear_children` → `detach_all_children` etc. (see above)
- `bevy_gizmos` rendering split into `bevy_gizmos_render` (separate feature)

## Search aids

When a Bevy compile error surfaces in old code, these phrases usually mean a version skew:

- `is not a Message` / `is not an Event` — Event vs Message split (0.17)
- `Handle<X> is not a Component` — needs the `MeshMaterial3d` etc. wrapper (0.17)
- `cannot multiply Color by f32` — color arithmetic removed (0.17)
- `method 'send' not found for MessageWriter` — old `EventWriter::send` replaced by `MessageWriter::write` (0.17)
- `next_state.set` re-firing transitions — same-state re-fire change (0.18)
- `field 'target' not found on Camera` — `RenderTarget` moved off `Camera` (0.18)
- `enable_prepass` not found on `MaterialPlugin` — moved to `Material` trait method (0.18)
- `Atmosphere: !Default` — `ScatteringMedium` asset required (0.18)
