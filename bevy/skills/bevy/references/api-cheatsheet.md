# Bevy 0.16 → 0.17 → 0.18 → 0.19 → 0.20 API rename cheatsheet

## Contents
- 0.19 → 0.20 (the latest hop — read first if you're upgrading from 0.19)
- 0.18 → 0.19 (`Replace`→`Discard`, `FontSize`/`FontSource`, resources-as-components)
- ECS communication (Event/Message/Observer split — 0.17; `Replace`→`Discard` — 0.19)
- ECS data (required components, material wrappers, color arithmetic — 0.17; resources-as-components — 0.19)
- Children / hierarchy (`detach_*` rename, `children!` macro limit)
- Rendering and assets (`RenderTarget`, `AmbientLight`, `Atmosphere`, Aabb; lights, scenes, text — 0.19)
- Schedules and states (state set re-fire, system set naming, `RenderStartup`, `set_executor` — 0.19)
- UI (`Val` helpers, `UiTransform`, `BorderRadius` folding; text `FontSource`/`FontSize`, widgets — 0.19; `em`/`rem`, `CornerRadius`, `FontSource::sans_serif()` — 0.20)
- Resources (lifetime requirements; resources-as-components — 0.19)
- Cargo features (collections, picking-backend renames; `audio`/`ui` no longer implied — 0.19; WESL/GLSL, `compressed_image_saver`, `bevy_curve` — 0.20)
- Errors and entities (`EntityIndex`, allocator rework — 0.18; `FallbackErrorHandler`, `validate_param` — 0.19)
- Search aids — common error messages and what they mean

Use this when modernizing existing code or interpreting compile errors that look version-mismatched. Symptom alongside the fix.

## 0.19 → 0.20 (the latest hop)

| 0.19 | 0.20 | Notes |
| --- | --- | --- |
| `On<Add, T>` (and `Insert`/`Discard`/`Remove`/`Despawn`) | `On<Add<T>>` | Bundle generic moved onto the lifecycle pattern; `On<E>` now takes one `E: EventPattern`. Underlying event structs are `AddEvent`/`InsertEvent`/`DiscardEvent`/`RemoveEvent`/`DespawnEvent` (not in the prelude: `bevy::ecs::lifecycle::AddEvent`); `On<Add<T>>` still derefs to `.entity`. A tuple is an OR filter. |
| `On<Add>` + `Observer::with_component(id)` | `On<Add<()>>` + `.with_component(id)` | `Add<B>` has no default generic. |
| `for c in q.iter_many(list)` | `for c in q.iter_many(list).matched()` (or `.unwrapped()` / `let c = r?;`) | `iter_many*`, `iter_many_unique*`, their `.sort*()` and `par_iter_many*` closures now yield `Result<Item, QueryEntityError>`; `.matched()` restores the old skip-on-miss behaviour. Sorted many-iters have no `.matched()` — use `.flat_map(Result::ok)`. |
| `impl ExclusiveSystemParam for X` | `impl SystemParam for X` | Exclusive systems unified with function systems; `&mut World` can sit anywhere after `In<T>`. Conflicts (`&mut World` + `Commands`/`Res`/`WorldId`) now panic at schedule build with `error[B0002]`, not at compile time. |
| `next_state.set_if_neq(X)`, `NextState::PendingIfNeq`, `commands.set_state_if_neq(X)` | `set_if_different(X)`, `PendingIfDifferent`, `set_state_if_different(X)` | Hard rename — no deprecated shim ships, despite the migration guide claiming one. `Mut::set_if_neq` / `ResMut::set_if_neq` (change detection) are **unchanged**. |
| `cond.and(b)` / `.or(b)` / `.nand(b)` / `.nor(b)` | `and_then`/`and_eager`, `or_else`/`or_eager`, `nand_then`/`nand_eager`, `nor_else`/`nor_eager` | Deprecated in 0.19, removed in 0.20. `_then`/`_else` short-circuit; `_eager` always runs both. `xor`/`xnor` unchanged. `.run_if(a).run_if(b)` is `a.and_eager(b)`. |
| `BevyError::panic(err)` | `BevyError::panic(err, payload)` | Payload is `Box<dyn Any + Send>`; new `with_payload`/`take_payload`. |
| `DefaultErrorHandler` (deprecated alias) | `FallbackErrorHandler` | Alias removed in 0.20. |
| `App`/`World::*_non_send_resource*`, `non_send_resource(_mut)` | `*_non_send`, `non_send(_mut)` | Deprecated in 0.19, **removed** in 0.20 (hard compile error). |
| `asset_server.load_acquire/load_untyped/load_with_settings(...)` | `asset_server.load_builder()….load(path)` | Deprecated in 0.19, **removed** in 0.20. Builder also carries `.load_untyped(path)` / `.load_erased(type_id, path)`. |
| `entity.clear_children()` / `remove_children()` / `remove_child()` | `detach_all_children()` / `detach_children()` / `detach_child()` | 0.18 rename; the deprecated shims were **deleted** in 0.20. |
| `impl FromType<T> for ReflectMyTrait { fn from_type() }` | `impl CreateTypeData<T> for ReflectMyTrait { fn create_type_data(_: ()) }` | `FromType` removed (`bevy_reflect::CreateTypeData`, not in the prelude); `#[reflect_trait]` users unaffected. |
| `Name::from(runtime_str)` (non-`'static` `&str`) | `Name::new(runtime_str.to_owned())` | `Name` impls `From<&'static str>`, not `From<&str>`, to avoid hidden allocations. Literals like `Name::from("Player")` still work. |
| `Entities::alloc/free/...` → `EntitiesAllocator` | → `EntityAllocator` | Typo fix: the type has always been `EntityAllocator`. |
| `use bevy::math::primitives::Cuboid`, `bevy::math::bounding::{Aabb3d, IntersectsVolume}`, `bevy::math::{Ray3d, ShapeSample}` | `use bevy::shape::{Cuboid, Aabb3d, IntersectsVolume, Ray3d, ShapeSample}` (flat) or `bevy::shape::prelude::*` | Split into `bevy_shape` (always on). There is **no** `bevy::shape::bounding::` / `::primitives::` path — everything is flat at the crate root (`sampling` and `prelude` are the only submodules), and `bevy::prelude::*` now also exports `Aabb2d`/`Aabb3d`, `BoundingVolume`, `IntersectsVolume`, `RayCast2d`/`RayCast3d`. `Dir2`/`Dir3`, `Isometry*`, `Rot2`, `Rect`, `ops` stay in `bevy::math`. |
| `bevy::math::curve::*`, `bevy::math::cubic_splines::*`, `bevy::math::Curve` | `bevy::curve::*`, `bevy::curve::cubic_splines::*`, `bevy::curve::Curve` | Split into `bevy_curve`; `bevy::prelude::*` still exports `Curve`, `EasingCurve`, `EaseFunction`, `interval`, so prelude users are unaffected. Feature `bevy_math/curve` → `bevy_curve` (in `common_api`). |
| `transform.rotation * (transform.scale * v)`, `global_transform.affine().transform_vector3(v)` | `transform.transform_vector(v)`, `global_transform.transform_vector(v)` | Direction counterpart to `transform_point`: scale + rotation, no translation. |
| `On<Pointer<Press>>`, `On<Pointer<Click>>`, `On<Pointer<Drag>>`, … | `On<PointerPress>`, `On<PointerClick>`, `On<PointerDrag>`, … | All 17 picking events are flat structs in `bevy::prelude`; the inner `Press`/`Click`/… types and the `Deref` are gone. |
| `ev.pointer_id`, `ev.pointer_location.position`, `ev.pointer_location` | `ev.pointer.id`, `ev.pointer.position` (`Vec2`), `ev.pointer.location()` | `button`/`hit`/`count`/`delta` sit directly on the event. |
| `fn f<E: Debug + Clone + Reflect>(e: On<Pointer<E>>)` | `fn f<E: PointerEvent>(e: On<E>)` | Add `E: for<'a> Event<Trigger<'a> = PropagateEntityTrigger<true, E, PointerTraversal>>` if the handler calls `propagate()`. |
| `bevy::ui::widget::Button` (the prelude `Button`) | `bevy::ui_widgets::Button` | Old one is `#[deprecated]` and is a type alias, so a bare `Button` value fails with `error[E0423]: expected value, found type alias`. `ui_widgets::Button` requires only `AccessibilityNode` — add `Node` and `Hovered::default()` yourself. |
| `Query<&Interaction, Changed<Interaction>>` + `match Interaction::…` | `Query<(&Hovered, Has<Pressed>), With<Button>>`, or `On<Activate>` | `Interaction` deprecated. `Hovered` is `bevy::picking::hover::Hovered` (a `bool`, read `.get()`); `Pressed` is the `bevy::ui::Pressed` unit marker. Neither is in the prelude. |
| `EditableText::default()` alone as a text field | `(TextInput, EditableText { .. })` | `bevy::ui_widgets::TextInput` requires `EditableText` and carries all the observers; `EditableText` alone renders but ignores input. Plugin `EditableTextInputPlugin` → `TextInputPlugin`. Read-only is the separate `bevy::text::TextReadWriteMode` component, not a `TextInput` field. |
| `bevy_ui::widget::TextScroll` | `EditableText::viewport` (`TextViewport { offset, size }`) | `scroll_editable_text` removed; scroll with `TextEdit::ScrollBy`/`ScrollTo`/`ScrollByLines` or write `viewport.offset`. |
| `FontSource::SansSerif` / `::Monospace` / other generic variants | `FontSource::sans_serif()` / `monospace()` / … (or `GenericFontFamily::SansSerif.into()`) | Generic families moved to `FontSource::Generic(GenericFontFamily)`; new `FontSource::families("Arial, 'Noto Sans', sans-serif")` / `list([...])` fallback chains. `"x".into()` now yields `Families`, so use `FontSource::family("x")` for an exact name. |
| `TextFont::default()` = `FontSize::Px(20.)` | `TextFont::default()` = `FontSize::Rem(1.)` | Renders identically at the default `RemSize` (20 px), but default-sized text now follows `RemSize` (UI `Text` and `Text2d` alike). Pin with `FontSize::Px(20.)`. |
| `BorderRadius { top_left: px(10), .. }` | `BorderRadius { top_left: px(10).into(), .. }` (or `BorderRadius::all(px(10))`) | Fields are `CornerRadius { x, y }` (elliptical corners). `From<Val>` gives a circular corner. `BorderRadius::all/new/top_left/with_*` take `impl Into<CornerRadius>` and are no longer `const` (`BorderRadius::px`/`percent` still are). `[px(10), px(20)]` for elliptical. |
| `val.resolve(scale, base, target)` | `val.resolve(scale, base, target, node.em_size, node.rem_size)` | All `Val`-family resolvers (`Val`, `Val2`, `UiPosition`, `CornerRadius`, `BorderRadius`, `UiTransform::compute_affine`) take trailing `EmSize`, `RemSize`; read both from `ComputedNode`. `FontSize::eval(vp, RemSize(px))` likewise. |
| `commands.trigger(UpdateNumberInput { entity, value })` | `commands.entity(entity).insert(NumberInputValue::F32(v))` | `UpdateNumberInput` removed; `NumberInputValue` is an immutable component required by `FeathersNumberInput`. It emits `ValueChange<f32>` (matching the variant), not `ValueChange<NumberInputValue>`. |
| `button_bundle(..)` / `checkbox_bundle(..)` / `slider_bundle(..)` / `*_bundle(..)` | `bsn! { @FeathersButton { .. } }` etc. | Feathers `*_bundle` fns and `ButtonBundleProps` are **removed**; Feathers is BSN-only. Themed labels: `@caption("…")` (`bevy::feathers::display::caption`). |
| `ThemeProps { color: HashMap<ThemeToken, Color> }` | `ThemeProps { token_assignments, semantic_base, semantic_overrides }` | Contextual theming. `ThemeProps::new_non_contextual(old_map)` is the one-line port. Subtree tinting needs `Propagate::<ThemeContext>(ThemeContext(SurfaceLevel::…))`; a bare `ThemeContext` affects only its own entity. |
| `bevy::feathers::cursor::{EntityCursor, DefaultCursor, OverrideCursor, CursorIconPlugin}` | `bevy::picking::cursor::{…}` | Per-entity hover cursors no longer need Feathers. `CursorIconPlugin` is **not** in `DefaultPickingPlugins` — add it yourself (Feathers still adds it). The `custom_cursor` feature now routes through `bevy_picking`. |
| `bsn! { scene_fn() }`, `{expr}` as an entity entry, `template_value(x)` | `bsn! { @scene_fn() }`, `@{expr}`, bare `x` (or `~{x}`) | `@` is mandatory on scene includes; a bare call is now a component-value insert. `template_value` is `#[deprecated(since = "0.20.0")]`. |
| `Children [ a, b ]`, `Children [ ( a ), ( b ) ]`, `bsn_list![a, b]` | `Children [ a -- b ]`, `bsn_list! { a -- b }` | `--` separates entities; commas and parens still parse but emit deprecation warnings, as do `bsn_list![…]` / `bsn!(…)` delimiters. **Pitfall:** deleting a comma without adding `--` silently merges two siblings into one entity, with no warning. |
| `#[derive(VariantDefaults)]` on a bsn! enum | (removed) `#[derive(Component, Default, Clone)]` | Enum variants are constructed verbatim: list every field (`Foo::A { x: 1, y: 0 }`); `..expr` on a variant is a macro error. Struct values *inside* a variant still patch. |
| `my_material.wgsl` with `#import bevy_pbr::forward_io::VertexOutput`, `#ifdef FLAG`, `@group(#{MATERIAL_BIND_GROUP})` | `my_material.wesl` with `import bevy_pbr::render::forward_io::VertexOutput;`, `@if(FLAG)`, `@group(constants::MATERIAL_BIND_GROUP)` | naga_oil removed; shaders are WESL. Rename the file **and** the Rust-side path string. Module path = crate name + file path (`bevy_pbr::prepass_utils` → `bevy_pbr::prepass::utils`); `#import "shaders/util.wgsl"::f` → `import super::util::f;`; `#define_import_path` gone. Only `Int`/`UInt` defs appear in `constants::`; `Bool` defs are `@if` flags. A directive-free `.wgsl` still loads but its shader defs are silently ignored. Full module-path list and example in `references/rendering.md`. |
| `#[derive(ExtractComponent)]` / `#[derive(ExtractResource)]` | + `#[extract_app(RenderApp)]` | Compile error without it. Manual impls take the label: `impl SyncComponent<RenderApp> for T`. Lives in the new `bevy_extract` crate, but the `bevy::render::{extract_component, extract_resource, sync_component, sync_world}` paths and `ExtractComponentPlugin::<T>::default()` are unchanged. |
| `commands.spawn(TemporaryRenderEntity)` | `commands.spawn(TemporaryRenderEntity::default())` | Now a `TemporaryEntity<RenderApp>` alias. |
| `Tonemapping::None` (to skip the tone curve) | `Tonemapping::Linear` | `None` is now a full passthrough (no `ColorGrading`, no `DebandDither`, no negative-channel clamp), and Bevy warns when it is paired with `DebandDither::Enabled`. `Camera2d` defaults to `Linear`. Reflected type path is now `bevy_render::view::Tonemapping` (Rust imports still work via re-export). |
| `Camera3d::default()` (transmission implicit) | `(Camera3d::default(), ScreenSpaceTransmission::default())` | `bevy::pbr::ScreenSpaceTransmission` is no longer required by `Camera3d`. Without it, `specular_transmission` materials fall back to `alpha_mode` routing — opaque by default, no refraction. Not in the prelude. |
| `ShaderBuffer::new(&bytes, asset_usage)`, `ShaderBuffer::from(my_struct)`, `buffer.set_data(x)`, `buffer.buffer_description.usage`, `buffer.resize_in_place(n)` | `ShaderBuffer::new(vec_of_t, asset_usage)`, `ShaderBuffer::from(vec![x])`, `buffer.clear(); buffer.extend([x])`, `buffer.buffer_usage`, `buffer.resize_buffer(n)` | Element type bound moved from encase `ShaderType` to `bytemuck::NoUninit` (`#[repr(C)]`; bevy does not re-export bytemuck). `extend` **appends** — `clear()` first or the buffer grows every frame. Read back with `cast_slice::<T>()`; `resize_buffer` no longer sets `copy_on_resize` for you. |
| `MeshTag(n)`, `tag.0` | `MeshTag::new(n)`, `tag.value` (`*tag` still derefs) | `MeshTag::with_type::<T>(n)` attaches a debug-only type id so Bevy can warn when a tag is overwritten. |
| `mesh.compute_aabb()` (`MeshAabb`) | `mesh.get_aabb()` | Rename only; still `Option<Aabb>`, and now returns `Mesh::final_aabb` when set. Trait is not in the prelude: `use bevy::camera::primitives::MeshAabb;`. |
| `ViewDepthTexture`, `DepthAttachment`, `ViewPrepassTextures::depth: Option<ColorAttachment>` | `ViewDepthStencilTexture`, `DepthStencilViewAttachment`, `Option<DepthStencilAttachment>` | In custom render passes: `get_attachment(StoreOp::Store)` is unchanged, the public `texture` field became `texture()`, and `depth_view()`/`previous_depth_view()` are now `depth_only_view()`/`previous_depth_only_view()`. |
| `impl Material2d for M { fn specialize(descriptor, layout, key) }` | `fn specialize(pipeline: &Material2dPipeline, descriptor, layout, key)` | New leading arg, matching 3D. `Material2dPipeline` is no longer generic; import from `bevy::sprite_render`. `Material2dKey<Self>` unchanged. Not in any migration guide. |
| `#ifdef OIT_ENABLED` + `#import bevy_core_pipeline::oit::oit_draw` | `@if(MATERIAL_OIT_ENABLED)` + `import bevy_core_pipeline::oit::draw::oit_draw;` | `OIT_ENABLED` is now the per-view def. `oit_draw` expects premultiplied colour: `oit_draw(in.position, vec4(c.rgb * c.a, c.a))`. Opt a material out with `fn enable_oit() -> bool { false }`. |
| `SpriteMesh { image, alpha_mode, .. }` (0.19 experimental) | `Sprite { image, alpha_mode: SpriteAlphaMode::{Opaque, Mask(f32), Blend}, .. }` | `SpriteMesh` deleted; `Sprite` renders via `Mesh2d`. Default alpha mode is `Blend` (0.19's `SpriteMesh` defaulted to `Mask(0.5)` — check ported code). `SpriteAlphaMode` is not in the prelude: `use bevy::sprite::SpriteAlphaMode;`. |
| `MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR` | `Res<MouseScrollPixelsPerLine>` + `AccumulatedMouseScroll::to_lines(&ratio)` / `to_pixels(&ratio)` | Also on `PointerScroll`. `MouseWheel` has no helper — multiply/divide by the resource (`wheel.y * *ratio`). `bevy::input::mouse`, not the prelude; default 100.0, override via `DerefMut`. |
| `AssetId::invalid()`, `AssetId::INVALID_UUID` | `Option<AssetId<A>>` + `None` | Deprecated since 0.20.0. Don't reach for `AssetId::default()` — it can map to a valid asset. |
| `Query<&Sprite, AssetChanged<Image>>` | `Query<&Sprite, AssetChanged<Sprite>>` | `AssetChanged<C>` is parameterised by the **component** holding the handle (`C: AsAssetId`), never the asset type. (True in 0.19 too.) |

**New in 0.20 (additions, not renames)** — see the topical references for each: `chain_weak`/`before_weak`/`after_weak` (scheduling), `ScheduleBuildSettings::shuffle_seed` behind the `debug` feature (testing), `resource_exists_and` and `OnAppExitSystems` (scheduling), `ContextExt::context`/`with_context` and the `bevy_error!`/`bail!`/`ensure!` macros (errors), `Commands::despawn_all`/`despawn_all_where` and `contiguous_par_iter` (ecs/performance), `Val::Em`/`Val::Rem`, `FixedNode`, `InlineBox`/`InlineImage`, `TabList`/`Tab`, `Dialog`/`ModalDialog` (ui), `bevy::scene::Ready` (bsn), `SpriteMaterial<M>` + `MaterialExtension2d` + `ExtendedMaterial2d` and `PanOrbitCamera` (rendering).

## 0.18 → 0.19

| 0.18 | 0.19 | Notes |
| --- | --- | --- |
| `On<Replace, T>`, `on_replace`, `#[component(on_replace=...)]` | `On<Discard, T>` (**0.20: `On<Discard<T>>`**), `on_discard`, `#[component(on_discard=...)]` | Lifecycle event `Replace` renamed `Discard`; the hook attribute name is unchanged in 0.20. |
| `#[derive(Component, Resource)]` | `#[derive(Resource)]` (implements both) | `Resource: Component` now; co-deriving is a duplicate impl. |
| `#[reflect(Resource)]` machinery | `ReflectComponent` | `ReflectResource` is a marker only. |
| `init_non_send_resource` / `insert_non_send_resource` | `init_non_send` / `insert_non_send` | Non-send "resources" → non-send "data". Deprecated shims **removed in 0.20**. |
| `TextFont { font: handle, font_size: 24.0 }` | `TextFont { font: handle.into(), font_size: FontSize::Px(24.0) }` | `font`→`FontSource`, `font_size`→`FontSize`. New `weight`/`width`/`style`, `LetterSpacing`. |
| `TextLayout::new_with_justify(...)` | `TextLayout::justify(...)` | Also `new_with_linebreak`→`linebreak`, `new_with_no_wrap`→`no_wrap`. |
| `Font::try_from_bytes(bytes)` | `Font::from_bytes(bytes)` | No longer `Result`; family-name arg also dropped (font auto-registers its embedded family + an internal alias). |
| `experimental_bevy_ui_widgets` feature | `bevy_ui_widgets` | Now in `ui`/default features; `UiWidgetsPlugins` in `DefaultPlugins`. |
| `experimental_bevy_feathers` feature | `bevy_feathers` | `FeathersPlugin` → `FeathersCorePlugin`. |
| `CoreScrollbarThumb`, `CoreScrollbarDragState`, `CoreSliderDragState` | `ScrollbarThumb`, `ScrollbarDragState`, `SliderDragState` | `Core` prefix dropped from widget components. |
| `input_focus.0 = Some(e)` | `input_focus.set(e, FocusCause::Navigated)` | Fields private; `get()`/`set(e, cause)`/`clear()`. |
| `SceneRoot(...)`, `DynamicScene`, `bevy::scene::*` (old) | `WorldAssetRoot(...)`, `DynamicWorld`, `bevy::world_serialization::*` | Old scene crate renamed; `bevy::scene` is now BSN. |
| `assets.get_mut(&h) -> Option<&mut A>` | `-> Option<AssetMut<A>>` | Bind `mut`; only fires `Modified` on real mutation. |
| `asset_server.load_acquire/load_untyped/load_with_settings(...)` | `asset_server.load_builder()....load(path)` | Old variants deprecated in 0.19, **removed in 0.20**. |
| `PointLight { shadows_enabled }` | `PointLight { shadow_maps_enabled }` | Same for `DirectionalLight`/`SpotLight`; new `contact_shadows_enabled`. |
| `Atmosphere::earthlike(m)` on camera | `Atmosphere::earth(m)` as its own entity | Moved to `bevy::light`; `AtmosphereSettings` stays on camera. |
| `Skybox { image: handle }` | `Skybox { image: Some(handle) }` | `image` is `Option`; moved to `bevy::light`. |
| custom `SystemParam::validate_param` | (removed) — validate inside `get_param` (returns `Result`) | `SystemState::get` now returns `Result`. |
| `DefaultErrorHandler` resource | `FallbackErrorHandler` | `set_error_handler` unchanged. The deprecated alias was removed in 0.20. |
| `ExecutorKind` / `Schedule::set_executor_kind` | `Schedule::set_executor(SingleThreadedExecutor::new())` etc. | `default_executor()` for the default. |
| `RenderGraph` `Node`/`add_render_graph_node` | render systems in `Core3d`/`Core2d` schedules | See `references/rendering.md`. |

## ECS communication (the big one — 0.17)

| Old (≤0.16) | New (0.17+) | Notes |
| --- | --- | --- |
| `EventReader<E>` | `MessageReader<M>` | And the trait is now `Message`, not `Event`. |
| `EventWriter<E>` | `MessageWriter<M>` | `.send()` → `.write()`. |
| `Events<E>` resource | `Messages<M>` resource | Same double-buffer semantics. |
| `app.add_event::<E>()` | `app.add_message::<M>()` | For buffered communication. |
| `Trigger<E>` parameter | `On<E>` parameter | Same role, shorter name. |
| `OnAdd` | `Add` | Lifecycle event. Same for `OnInsert/OnRemove/OnDespawn` → `Insert/Remove/Despawn`. `OnReplace` → `Replace` in 0.17, then → `Discard` in 0.19 (see the 0.18→0.19 table). (0.20) written `On<Add<T>>`, not `On<Add, T>`; the event struct is `AddEvent`. |
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
| `entity.clear_children()` | `entity.detach_all_children()` | (0.18) Clearer that children aren't despawned. Deprecated shims deleted in 0.20. |
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
| `MaterialPlugin::<M> { prepass_enabled: false, .. }` | `impl Material for M { fn enable_prepass() -> bool { false } }` | (0.18) Per-material trait methods, not plugin fields. Same for `enable_shadows`. (0.20) adds `fn enable_oit() -> bool` (default `true`; only meaningful when the camera has `OrderIndependentTransparencySettings`). |
| `Atmosphere::default()` | `Atmosphere::earthlike(media.add(ScatteringMedium::default()))` | (0.18) Generalized scattering needs an asset. |
| `entity.remove::<Aabb>()` after mesh mutation | (drop it) | (0.18) Aabb auto-updates. Use `NoAutoAabb` to opt out. |
| `Image::reinterpret_size(...)` (panicking) | `Image::reinterpret_size(...)?` (returns `Result`) | (0.18) Made fallible instead of panicking. |
| `mesh.insert_attribute(...)` on a `RENDER_WORLD`-only mesh | `mesh.try_insert_attribute(...)?` | (0.18) The non-`try_` variants still exist but now panic if the mesh has been extracted to render world. Use `try_*` if there's any chance the mesh is render-only. |

## Schedules and states

| Old | New | Notes |
| --- | --- | --- |
| `next_state.set(X)` (idempotent) | Now re-fires `OnEnter`/`OnExit` even if equal | (0.18) Use `next_state.set_if_different(X)` (named `set_if_neq` in 0.18–0.19) for the old "skip if equal" behavior. |
| `prepass_enabled` plugin field | `Material::enable_prepass()` trait method | (0.18) See above. |
| `bevy_internal::*Set` mixed naming | `*Systems` suffix on system sets | (0.17) Convention shift — `PickSet` → `PickingSystems`, `Animation` → `AnimationSystems`, `GizmoRenderSystem` → `GizmoRenderSystems`. |
| `RenderApp` finish-time init | `RenderStartup` schedule + systems in `Plugin::build` | (0.17) Renderer plugins use a normal startup schedule now. Old `Plugin::finish` patterns still work but new code should use `RenderStartup`. |

## UI

| Old | New | Notes |
| --- | --- | --- |
| `Val::Px(200.0)` | `px(200)` | (0.17) `px`/`percent`/`vw`/`vh`/`vmin`/`vmax` helpers; (0.20) plus `em`/`rem` (`Val::Em` resolves against the node's `EmSize` component, `Val::Rem` against the `RemSize` resource). The originals still work. |
| `UiRect { left: Val::Px(10.0), .. default() }` | `px(10).left()` (or `.right()/.top()/.bottom()/.all()/.horizontal()/.vertical()`) | (0.17) Fluent UiRect builder. |
| `Transform`/`GlobalTransform` on UI nodes | `UiTransform`/`UiGlobalTransform` | (0.17) UI got its own specialized 2D transform; don't reach for `Transform` on UI entities. |
| `BorderRadius` as a component | `Node { border_radius: BorderRadius { .. }, .. }` field | (0.18) Folded into `Node`. (0.20) Its fields are `CornerRadius`, not `Val` — write `BorderRadius::all(px(8))` or `px(8).into()` per field. |
| `Text` non-text areas pickable | Only text glyphs pickable; wrap in a parent `Node` for hit-test on the full node | (0.18) Picking precision change. |
| `BorderColor(Color::WHITE)` | `BorderColor::all(Color::WHITE)` (or per-side: `.set_left(...)` etc.) | (0.17) Per-side border colors. |

## Resources

| Old | New | Notes |
| --- | --- | --- |
| `#[derive(Resource)] struct Foo<'a> { .. }` | `#[derive(Resource)] struct Foo { .. }` | (0.18) Resources require `'static`. |
| `AmbientLight` resource (see above) | `GlobalAmbientLight` resource | |
| `BindGroupLayout` field on pipeline descriptor | `BindGroupLayoutDescriptor` field | (0.18) Lazy creation; use `pipeline_cache.get_bind_group_layout(&desc)` to materialize. |

## Cargo features (0.18–0.20)

| Old | New | Notes |
| --- | --- | --- |
| Hand-listing 30+ Bevy features | `bevy = { default-features = false, features = ["3d", "ui"] }` | (0.18) Top-level collections: `2d`, `3d`, `ui`, `audio`, `dev`. Plus mid-level `2d_api`, `3d_api`, `default_app`, `default_platform`. |
| `["3d"]` implicitly gave you audio + UI | list `["3d", "ui", "audio"]` explicitly | (0.19) `audio`/`ui` no longer implied by `2d`/`3d`; `audio` is its own default; `bevy_window`/`bevy_input_focus`/`custom_cursor` left `default_app`; Android activity backend no longer default. |
| `bevy_sprite_picking_backend` | `sprite_picking` | (0.18) Feature renamed for consistency. Same for `bevy_ui_picking_backend` → `ui_picking`, `bevy_mesh_picking_backend` → `mesh_picking`. |
| `animation` feature | `gltf_animation` | (0.18) Renamed to make the scope clear. |
| `features = ["shader_format_wesl"]` / `["shader_format_glsl"]` | (remove them) | (0.20) WESL is always compiled in (`wesl` is a required dep of `bevy_shader`); GLSL loading is gone. `.wgsl` and `.spv` still load, but `shader_defs` apply only to `.wesl`. `shader_format_spirv` unchanged. |
| `compressed_image_saver` (Basis UASTC) | `compressed_image_saver_universal` to keep Basis; `compressed_image_saver` now = BCn/ASTC via `ctt` | (0.20) Same name, different backend — the BCn output does not transcode, so use `_universal` for web. Default processed extensions grew from `png` to `png`/`jpeg`/`jpg`; trim via `ImagePlugin::default_compressed_image_processor_extensions`. |
| `bevy_math/curve` | `bevy_curve` | (0.20) Curves split into their own crate; `bevy_curve` is in `common_api`, so `2d`/`3d`/`ui` builds already have it. |
| `dev` = `debug` + `bevy_dev_tools` + `file_watcher` | `dev` also enables `render_dev_tools` | (0.20) `render_dev_tools` = `bevy_dev_tools/render`; without it `bevy_dev_tools` now builds render-free (no overlays, screenshots or infinite grid). |
| — | `pan_orbit_camera`, `complex_script_segmentation` | (0.20) New opt-ins: an editor-style orbit camera (see `references/rendering.md`) and dictionary line-breaking for CJK/Thai/Khmer/Lao/Myanmar (bigger binary). |

## Errors and entities (0.18 internal)

These rarely matter at the gameplay layer but show up if you're touching ECS internals:

- `Entity::row` → `Entity::index`; `Entity::from_row` → `Entity::from_index`; `EntityRow` → `EntityIndex`
- `EntityDoesNotExistError` → split into `InvalidEntityError`, `EntityValidButNotSpawnedError`, `EntityNotSpawnedError`
- `Entities::alloc/free/reserve/flush` → moved to `EntityAllocator` (`World::entity_allocator()`: `alloc`, `alloc_many`, `free`, `free_many`); `World::spawn_at` replaces flush-after-reserve
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
- `next_state.set` re-firing transitions — same-state re-fire change (0.18; extended to `DespawnOnEnter`/`DespawnOnExit` in 0.19)
- `field 'target' not found on Camera` — `RenderTarget` moved off `Camera` (0.18)
- `enable_prepass` not found on `MaterialPlugin` — moved to `Material` trait method (0.18)
- `Atmosphere: !Default` / `no method earthlike` — `ScatteringMedium` asset required (0.18); `Atmosphere` is an entity, `earthlike`→`earth` (0.19)
- `cannot find type Replace` / `on_replace` rejected — lifecycle `Replace`→`Discard` (0.19)
- `conflicting implementations of trait Component` on a resource — `#[derive(Component, Resource)]` no longer valid; `Resource: Component` (0.19)
- expected `FontSource`/`FontSize`, found `Handle<Font>`/`f32` — text migrated to parley (0.19)
- `cannot find SceneRoot`/`DynamicScene` — old scene crate is now `bevy_world_serialization`; use `WorldAssetRoot` (0.19)
- `field '.0' of InputFocus is private` — use `get()`/`set(e, FocusCause)`/`clear()` (0.19)
- `no field shadows_enabled` on a light — renamed `shadow_maps_enabled` (0.19)
- `validate_param is not a member of trait SystemParam` — merged into `get_param` returning `Result` (0.19)
- `struct takes 1 generic argument but 2 generic arguments were supplied` / `missing generics for struct Add` — lifecycle bundle generic moved into the event: `On<Add<T>>`, `On<Add<()>>` for dynamic observers (0.20)
- `cannot find type Press in this scope` + `struct takes 0 generic arguments but 1 was supplied` on `Pointer<…>` — pointer events flattened to `PointerPress`/`PointerClick`/… with a `pointer: Pointer` field (0.20)
- `no method named set_if_neq found for struct NextState` — renamed `set_if_different` (0.20); component/resource `set_if_neq` is unaffected
- `no method named and/or/nand/nor found` on a run condition — use `and_then`/`and_eager`, `or_else`/`or_eager`, … (removed in 0.20)
- `expected &T, found Result<&T, QueryEntityError>` on an `iter_many` loop — add `.matched()` (0.20)
- `expected value, found type alias Button` / `use of deprecated type alias bevy::ui::widget::Button` — `use bevy::ui_widgets::Button;` (0.20)
- `no variant or associated item named SansSerif found for enum FontSource` — generic families became `GenericFontFamily`; use `FontSource::sans_serif()` (0.20)
- shader parse error at `#import` / `#ifdef` / `#{`, or `shader_defs` ignored with a warning — shaders are WESL now; rename to `.wesl` and translate the directives (0.20)
- `ExtractComponent requires #[extract_app(...)]` — add `#[extract_app(RenderApp)]` to the derive (0.20)
- `unresolved import bevy::math::primitives` / `bevy::math::bounding` / `no Ray3d in bevy::math` — moved to `bevy::shape` (flat re-exports, 0.20)
- `error[B0002]: … conflicts with a previous system parameter` at schedule build — `&mut World` mixed with another world-accessing param (0.20)
- `cannot find type SpriteMesh` — folded into `Sprite` (0.20); the field is `alpha_mode`, imported from `bevy::sprite`
