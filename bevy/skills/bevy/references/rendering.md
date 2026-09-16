# Rendering

## Contents
- The two worlds — main world vs render world, extraction, `RenderApp`
- Render graph as systems (0.19) — `Core3d`/`Core2d` schedules, `Core3dSystems` sets, `ViewQuery`, `RenderContext`, `RenderStartup`
- Cameras — `Camera3d`/`Camera2d`, `RenderTarget` component (0.18), HDR, ordering
- Lights and shadows — `shadow_maps_enabled`/`contact_shadows_enabled` (0.19), ambient light split
- Atmosphere and sky — `Atmosphere` as an entity (0.19), `Skybox`, light probes, parallax cubemaps
- Materials — `Material` trait methods (0.18), `enable_oit` (0.20), `bevy_material` crate, `ShaderBuffer`, bindless on Metal
- Shaders are WESL (0.20) — `.wesl`, `import`, `@if`, `constants::`
- Sprites and 2D materials (0.20) — `Mesh2d` backend, `alpha_mode`, `SpriteMaterial<M>`, `ExtendedMaterial2d`
- Post-processing — `Vignette`/`LensDistortion` (0.19), the post-process split, bloom/PBR fixes, `Tonemapping::Linear` (0.20)
- Render recovery (0.19) — `RenderErrorHandler`, GPU device loss
- Skinned mesh culling (0.19) — `DynamicSkinnedMeshBounds`
- Dev/debug tools — infinite grid, diagnostics overlay, transform gizmo, text gizmos, render debug overlay, pan-orbit camera (0.20)
- Occlusion culling, Solari, mesh shaders

Most gameplay code never touches rendering internals — you spawn `Camera3d`, `Mesh3d` + `MeshMaterial3d`, and lights, and let Bevy render. This reference is for the cases where you *do* reach into rendering: custom render passes, post-processing, camera/light configuration, and dev tooling. 0.19 reworked the render-graph internals substantially (render passes are systems now), and 0.20 moved every shader to WESL and made `#[extract_app(RenderApp)]` mandatory — so custom-render code from 0.18/0.19 needs porting.

## The two worlds

Bevy renders in a separate **render world**, rebuilt each frame from the **main world** by *extraction* systems. The render world lives in a sub-app (`RenderApp`) with its own schedules. The split exists so rendering can pipeline against the next frame's simulation.

You rarely need this for gameplay. You need it when writing a custom material, a custom render pass, or a render-world resource. Gateways into the render world:

- `ExtractComponent` / `ExtractResource` — copy a component/resource into the render world each frame. **(0.20) the derives require a target sub-app:** `#[derive(Component, Clone, ExtractComponent)] #[extract_app(RenderApp)] struct Foo;` — omitting the attribute is a compile error (`ExtractComponent` accepts several labels, `ExtractResource` exactly one).
- `app.sub_app_mut(RenderApp)` — add render-world systems/resources from a plugin's `build`.
- `RenderStartup` (0.17+) — a normal startup schedule in the render world; the recommended place to initialize render resources, replacing old `Plugin::finish` patterns. (0.19: `MeshPipeline`, `MeshPipelineViewLayouts`, and similar built-in pipeline resources are created in `RenderStartup` now — order custom `RenderStartup` systems after `MeshPipelineSystems` if you depend on them.)

0.19 reworked `ExtractComponent`: removing a synced component no longer despawns the render entity (it removes the `Target` components of the `SyncComponent` supertrait instead). If you wrote a custom `ExtractComponent`, implement `SyncComponent` to declare what gets cleaned up. **(0.20)** extraction lives in the new `bevy_extract` crate and both traits are generic over the target sub-app: `impl SyncComponent<RenderApp> for T`, `impl ExtractComponent<RenderApp> for T`. Plugin registration is unchanged — `ExtractComponentPlugin::<T>::default()`, `SyncToRenderWorld`, `RenderEntity` and `TemporaryRenderEntity` from `bevy::render` are type aliases with `RenderApp` pre-filled (so spawn `TemporaryRenderEntity::default()`).

## Render graph as systems (0.19)

The headline rendering change in 0.19: **the `RenderGraph` API is gone. Render passes are ordinary ECS systems** that run in the `Core3d` / `Core2d` schedules in the render world. This lets custom rendering use familiar Bevy ordering instead of a bespoke node/label/edge API.

Before (0.18) you implemented `ViewNode`, derived a `RenderLabel`, and wired edges:

```rust
// 0.18 — the old way
impl ViewNode for MyNode {
    type ViewQuery = (&'static ExtractedCamera, &'static ViewTarget);
    fn run<'w>(&self, _graph: &mut RenderGraphContext, render_context: &mut RenderContext<'w>,
               (camera, target): QueryItem<'w, Self::ViewQuery>, world: &'w World)
        -> Result<(), NodeRunError> { /* ... */ }
}
render_app
    .add_render_graph_node::<ViewNodeRunner<MyNode>>(Core3d, MyLabel)
    .add_render_graph_edges(Core3d, (Node3d::Foo, MyLabel, Node3d::Bar));
```

After (0.19) it's a system with two new system params:

```rust
// 0.19 — a render pass is just a system
fn my_render_pass(
    view: ViewQuery<(&ExtractedCamera, &ViewTarget)>,  // fetches data for the current view
    mut ctx: RenderContext,                            // command encoder, render device
) {
    let (camera, target) = view.into_inner();
    let encoder = ctx.command_encoder();
    // ... encode rendering commands ...
}

impl Plugin for MyRenderPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app.add_systems(Core3d, my_render_pass.after(main_opaque_pass_3d)
            .in_set(Core3dSystems::MainPass));
    }
}
```

Key pieces:

- **`ViewQuery<D>`** — a `SystemParam` that queries `D` for the *current view* entity. `view.into_inner()` unwraps the item. Replaces `ViewNode::ViewQuery`.
- **`RenderContext`** — now a `SystemParam` (was a `&mut` argument); provides the command encoder and render device.
- **`Core3dSystems` / `Core2dSystems`** — coarse ordering sets, chained: `Prepass` → `MainPass` → `EarlyPostProcess` → `PostProcess`. (0.19 split the old `PostProcess` into `EarlyPostProcess` + `PostProcess`, and 2D gained a `Prepass`.) Order against these sets, or `.before`/`.after` the actual built-in pass systems (e.g. `main_opaque_pass_3d`) rather than `Node3d::*` labels.
- **`RenderGraph` schedule** still exists as the top-level schedule for non-camera rendering (`bevy::render::renderer::RenderGraph`).
- **Depth in custom passes (0.20):** `ViewDepthTexture` → `ViewDepthStencilTexture` and `DepthAttachment` → `DepthStencilAttachment`; `get_attachment(StoreOp::Store)` is unchanged, the public `texture` field became `texture()`, and `ViewPrepassTextures::depth_view()` is now `depth_only_view()`.

Fullscreen post-process materials moved from `run_in`/`run_after`/`run_before` to a single `FullscreenMaterial::schedule_configs(...)` that returns `ScheduleConfigs` you configure with `.in_set(...)`/`.before(...)`.

## Cameras

Spawn a camera as an entity:

```rust
commands.spawn(Camera3d::default());          // 3D
commands.spawn(Camera2d);                      // 2D
```

- **`RenderTarget` is its own component (0.18).** To render to a texture, spawn `RenderTarget::Image(image_handle.into())` alongside `Camera3d` rather than setting `Camera { target: ... }`. `RenderTarget::Window(..)` is the default.
- **`Hdr` is a component (moved to `bevy_camera` in 0.19).** Add `Hdr` to a camera for an HDR render target; it's a camera property, not a view property. Internally `ExtractedView::hdr` moved to `ExtractedCamera::hdr`; `ViewTarget::is_hdr` was **removed** (use `ExtractedCamera::hdr`), and `TextureFormat::bevy_default()` / the `BevyDefault` trait are **gone in 0.20** — read the format from `ExtractedView::target_format` in render-world code, or name it explicitly (`TextureFormat::Rgba8UnormSrgb`, what `bevy_default()` returned) when creating a render-to-texture image.
- **Camera `order` controls draw sequence** when multiple cameras render to the same target (UI camera on top of game camera, etc.). With the default `blend_state: None`, the bottom camera replaces the target and every camera above it alpha-blends over it — (0.20) including stacks that mix `Hdr`, so an explicit `CameraOutputMode::Write { blend_state: Some(BlendState::ALPHA_BLENDING), .. }` is no longer needed to make that work. An explicit `blend_state` still overrides (`Some(BlendState::REPLACE)` forces overwrite); neither `CameraOutputMode` nor `BlendState` is in the prelude.
- **`Viewport`** restricts a camera to a sub-rect of its target (split-screen).

`ScreenSpaceTransmission` (0.19) holds the screen-space transmission knobs (`steps`, default 1; `quality`) as a `bevy_pbr` component. **(0.20) it is opt-in:** `Camera3d` no longer requires it, so spawn `(Camera3d::default(), ScreenSpaceTransmission::default())` (import from `bevy::pbr`) or `StandardMaterial::specular_transmission` renders with no refraction.

## Lights and shadows

```rust
commands.spawn((
    PointLight {
        intensity: 1500.0,
        shadow_maps_enabled: true,      // 0.19: was shadows_enabled
        contact_shadows_enabled: false, // 0.19: new screen-space contact shadows
        ..default()
    },
    Transform::from_xyz(4.0, 8.0, 4.0),
));
```

- **0.19 renamed `shadows_enabled` → `shadow_maps_enabled`** on `PointLight`, `DirectionalLight`, and `SpotLight`, because those lights now *also* support **contact shadows** via the new `contact_shadows_enabled` field (the old name only ever configured shadow maps). Screen-space contact shadows need a `ContactShadows` component on the camera.
- **Ambient light (0.18 split):** `GlobalAmbientLight` is the world-default resource; `AmbientLight` is a per-camera component override. The old "`AmbientLight` as a resource" API is gone.
- **Light gizmos** moved from `bevy_gizmos` to `bevy_light` in 0.19 (`ShowLightGizmo`, `LightGizmoConfigGroup`).

## Atmosphere and sky

**`Atmosphere` is a standalone entity in 0.19** (moved to `bevy_light`), not a camera component. The nearest atmosphere is chosen per camera; the camera opts into rendering it with `AtmosphereSettings`:

```rust
use bevy::light::{atmosphere::ScatteringMedium, Atmosphere};
use bevy::pbr::AtmosphereSettings;

fn setup(mut commands: Commands, mut media: ResMut<Assets<ScatteringMedium>>) {
    commands.spawn(Atmosphere::earth(media.add(ScatteringMedium::earth(256, 256))));
    commands.spawn((Camera3d::default(), AtmosphereSettings::default()));
}
```

`Atmosphere::earthlike` → `earth`; fields `bottom_radius`/`top_radius` → `inner_radius`/`outer_radius`; `scene_units_to_m` removed (use the `Atmosphere` entity's `Transform` scale, inversely — `scale: 0.001` for the old `scene_units_to_m: 1000.0`). A default `Transform` positions the planet so the horizon lines up with the camera.

**`Skybox` moved to `bevy_light` and its `image` is now `Option<Handle<Image>>` (0.19)** — `Skybox { image: Some(handle), brightness: 1000.0, ..default() }`. A `Skybox` with `image: None` draws nothing (handy as a placeholder).

**Light probes / reflections:** `LightProbe` + `EnvironmentMapLight` give image-based reflections. 0.19 added **parallax-corrected cubemaps** (on by default for light probes, using the probe's bounding box). To disable correction on a probe, set its `ParallaxCorrection::None`. The white-furnace fixes (0.19) make image-based lighting on metallic/rough materials more physically correct.

## Materials

The render-component wrappers are the API: `MeshMaterial3d<StandardMaterial>` / `MeshMaterial2d<M>` on a `Mesh3d` / `Mesh2d` entity (a bare `Handle<StandardMaterial>` is not a component — see `references/assets.md`).

- **`Material` trait methods:** prepass/shadow opt-outs are `fn enable_prepass() -> bool` / `fn enable_shadows() -> bool` (0.18), and `fn enable_oit() -> bool` (0.20, also on `MaterialExtension`) opts a material out of order-independent transparency. All default to `true`; `enable_oit` is a no-op unless the camera carries `OrderIndependentTransparencySettings` and the alpha mode is `Blend`, `Premultiplied` or `Add` (0.20 widened it from `Blend`-only). OIT-aware custom shaders now gate on `@if(MATERIAL_OIT_ENABLED)` (per material; `OIT_ENABLED` is the per-view def), import `bevy_core_pipeline::oit::draw::oit_draw`, and pass **premultiplied** colour: `oit_draw(in.position, vec4(c.rgb * c.a, c.a))`.
- **`bevy_material` crate (0.19):** material machinery (`AlphaMode`, `OpaqueRendererMethod`, `MaterialProperties`, pipeline descriptor types, …) was extracted out of `bevy_pbr`/`bevy_render` into a new `bevy_material` crate. Many items re-export, but `AlphaMode` and a few others need import-path updates.
- **Partial bindless on Metal (0.19):** `StandardMaterial` and other texture-only materials now get bindless rendering on Mac/iOS (big perf win on Apple GPUs). No code changes needed.
- **`ShaderStorageBuffer` → `ShaderBuffer`** (0.19), and **(0.20) it is typed, not bytes:** `ShaderBuffer::new(vec_of_t, RenderAssetUsages::default())` where `T: bytemuck::NoUninit` (`#[repr(C)]`, pad for WGSL yourself; Bevy doesn't re-export bytemuck). `set_data(x)` is gone — `buffer.clear(); buffer.extend([x])`, and `extend` *appends*, so skipping `clear()` grows the buffer every frame. `references/api-cheatsheet.md` maps the renamed fields and methods.
- **`ExtendedMaterial2d<B, E>` (0.20):** the 2D twin of `ExtendedMaterial`. Implement `MaterialExtension2d` (`vertex_shader`/`fragment_shader`/`depth_bias`/`alpha_mode` all return `Option`, where `None` keeps the base's value; `specialize` returns `Result` and runs *after* the base's), register `Material2dPlugin::<ExtendedMaterial2d<ColorMaterial, MyExt>>::default()`, and spawn `(Mesh2d(mesh), MeshMaterial2d(materials.add(ExtendedMaterial2d { base, extension })))`. Base and extension share one bind group, so put extension bindings at `#[uniform(20)]` and up.
- **`Material2d::specialize` gained a leading `pipeline: &Material2dPipeline` argument (0.20)**, matching 3D. `Material2dPipeline` is no longer generic; import it from `bevy::sprite_render`. `Material2dKey<Self>` is unchanged (no migration guide covers this one).

## Shaders are WESL (0.20)

Bevy's shader dialect is now [WESL](https://wesl-lang.dev); naga_oil's `#import`/`#ifdef`/`#{DEF}` preprocessor is gone.

- **File extension `.wesl`.** The loader accepts `spv`, `wgsl`, `wesl`. A `.wgsl` file is *plain* WGSL: it cannot import, cannot be imported, and `shader_defs` you push in `specialize` are **silently ignored**. Anything that imports Bevy modules or uses defs must be `.wesl` — and the Rust-side path string has to change too.
- **Imports first, ending with `;`.** `import bevy_pbr::render::forward_io::VertexOutput;` — before any declaration or `enable`. Brace groups work: `import bevy_pbr::render::{pbr_fragment::pbr_input_from_standard_material, pbr_functions::alpha_discard};`.
- **Module = crate + file path.** `bevy_pbr::forward_io` → `bevy_pbr::render::forward_io`; `bevy_pbr::prepass_utils` → `bevy_pbr::prepass::utils`; `bevy_sprite::mesh2d_vertex_output` → `bevy_sprite_render::mesh2d::vertex_output`; `bevy_ui::ui_vertex_output` → `bevy_ui_render::ui_vertex_output`; `bevy_core_pipeline::fullscreen_vertex_shader` is unchanged. `#define_import_path` is gone: `embedded://my_crate/a/b.wesl` imports as `my_crate::a::b`, and a shader in your `assets/` folder is `super::util` from a sibling (`package::` is WESL's absolute origin).
- **Conditionals are attributes.** `@if(DEF)` / `@elif(DEF)` / `@else` attach to a declaration, struct field, function parameter, import, statement, or a `{ }` block. Bool shader defs set a flag; `Int`/`UInt` defs set the flag *and* are readable as `constants::NAME`.
- **The Rust side is unchanged** apart from the path: `Material::fragment_shader() -> ShaderRef` still takes `"shaders/custom_material.wesl".into()`, and `descriptor.fragment.as_mut().unwrap().shader_defs.push("IS_RED".into())` in `specialize` still works.
- **GLSL is gone**, as are the `shader_format_glsl` / `shader_format_wesl` features (WESL is always on). `shader_format_spirv` is unchanged. `Shader::from_wgsl` still exists for plain WGSL with no directives; use `Shader::from_wesl` for inline source that uses `import`, `@if` or `constants::`.

Before (0.19, `custom_material.wgsl`):

```wgsl
#import bevy_pbr::forward_io::VertexOutput
#import "shaders/custom_material_import.wgsl"::COLOR_MULTIPLIER

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material_color: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
#ifdef IS_RED
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
#else
    return material_color * COLOR_MULTIPLIER;
#endif
}
```

After (0.20, `custom_material.wesl`):

```wgsl
import bevy_pbr::render::forward_io::VertexOutput;
import super::custom_material_import::COLOR_MULTIPLIER;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material_color: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    @if(IS_RED)
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    @else
    return material_color * COLOR_MULTIPLIER;
}
```

```rust
const SHADER_ASSET_PATH: &str = "shaders/custom_material.wesl"; // was .wgsl

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef { SHADER_ASSET_PATH.into() }
}
```

**Symptom:** a shader that fails to parse at `#import` / `#ifdef` / `#{`, or a `.wgsl` material whose shader defs silently do nothing. Some 0.20 migration guides (`material_oit_changes.md`, `shader_octahedral_moved.md`) still show naga_oil syntax — follow the shipped `.wesl` example shaders instead.

## Sprites and 2D materials (0.20)

`Sprite` now renders through the `Mesh2d` + `Material2d` pipeline: `SpriteMeshPlugin` (part of `DefaultPlugins`) inserts `Mesh2d` (a shared quad) and `MeshMaterial2d<SpriteMeshMaterial>` on every entity that gains a `Sprite`, in `PostUpdate`.

- **Filter mesh-only queries with `Without<Sprite>`** — `Query<&mut Transform, With<Mesh2d>>` now also matches sprites (the engine's own `Mesh2d` AABB systems do this). Never hand-set `Mesh2d` or a second `MeshMaterial2d` on a `Sprite` entity: the quad overwrites yours, and a second material draws twice.
- **`Sprite::alpha_mode`** — `SpriteAlphaMode::Blend` (default, the 0.19 behaviour), `Opaque`, or `Mask(f32)`. `Opaque`/`Mask` route to the binned `Opaque2d`/`AlphaMask2d` phases instead of the sorted transparent pass. `Opaque` forces alpha to 1.0, so use `Mask(0.5)` for cut-out art. Not in the prelude: `use bevy::sprite::SpriteAlphaMode;`.
- **Same-Z draw order may differ from 0.19.** The sort key is still `translation.z`, but the tie-break comes from a different iteration order. Give overlapping sprites distinct Z — same-Z order was never part of the contract.
- **Sprites share materials by value.** Each distinct `Sprite` + `Anchor` value maps to a cached `SpriteMeshMaterial`; animating `color`/`custom_size`/`rect`/`flip_*`/atlas index every frame on N sprites allocates up to N materials per frame (old ones are evicted, so it churns rather than leaks). For per-sprite animated parameters use a `SpriteMaterial<M>` extension and mutate the `M` asset instead.
- **Custom sprite shaders** — extend the built-in sprite material rather than hand-rolling a `Mesh2d` + `Material2d`:

```rust
use bevy::{prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef};
// AsBindGroup and ShaderRef are not in the prelude.

#[derive(AsBindGroup, Asset, Reflect, Clone)]
struct Dissolve {
    #[uniform(20)]   // the sprite owns bindings 0–2 (0 and 10 when bindless); start at 20
    amount: f32,
}

impl MaterialExtension2d for Dissolve {
    fn fragment_shader() -> Option<ShaderRef> { Some("shaders/dissolve.wesl".into()) }
}

app.add_plugins(SpriteMaterialPlugin::<Dissolve>::default());

commands.spawn((
    Sprite::from_image(image),
    SpriteMaterial(materials.add(Dissolve { amount: 0.0 })),
));
```

```wgsl
import bevy_sprite_render::{mesh2d::vertex_output::VertexOutput, sprite_mesh::functions as sprite_fn};

struct Dissolve { amount: f32 }
@group(constants::MATERIAL_BIND_GROUP) @binding(20) var<uniform> material: Dissolve;

@fragment fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let c = sprite_fn::sample_sprite_texture(in.uv, in.instance_index); // flip/atlas/slice applied
    return sprite_fn::get_final_color(c, in.instance_index);            // tint + alpha cutoff
}
```

`sample_final_color(uv, instance_index)` does both steps. The `Sprite`'s image, `color`, atlas, flip, image mode and `Anchor` still apply (it is `ExtendedMaterial2d<SpriteMeshMaterial, _>` under the hood); removing `SpriteMaterial<M>` restores the plain sprite material. Animate by mutating the asset (`materials.get_mut(&handle.0)`), not by re-inserting the component. Watch the name collision: 0.19's internal `SpriteMaterial`/`SpriteMaterialPlugin` are now `SpriteMeshMaterial`/`SpriteMeshMaterialPlugin`.

`Text2d` still uses the legacy sprite extractor.

## Post-processing

Add post-process effects as components on the camera. 0.19 added two:

```rust
commands.spawn((
    Camera3d::default(),
    Vignette { intensity: 1.0, radius: 0.75, smoothness: 5.0, roundness: 1.0,
               center: Vec2::splat(0.5), edge_compensation: 1.0, color: Color::BLACK },
    LensDistortion { intensity: 0.5, scale: 1.0, multiplier: Vec2::ONE,
                     center: Vec2::splat(0.5), edge_curvature: 0.0 },
));
```

Both live in `bevy::post_process::effect_stack`. `Vignette` darkens the periphery (animate `intensity` for damage-pulse / horror effects). `LensDistortion` warps spatially — positive `intensity` is barrel (fisheye/speed), negative is pincushion (impairment). `Bloom`, `Fxaa` and friends are unchanged; note 0.19 corrected bloom's luma calculation to linear space, so high-saturation scenes may show *less* bloom than before — bump `Bloom::intensity` if a scene now looks dim. Order custom effects against the `EarlyPostProcess` / `PostProcess` sets (see the render-graph section above).

**`Tonemapping` changed in 0.20.** `Tonemapping::None` is now a full passthrough — `ColorGrading`, exposure, the negative-channel clamp and `DebandDither` no longer apply under it (Bevy warns if you pair it with `DebandDither::Enabled` or non-default `ColorGrading`). Use the new `Tonemapping::Linear` for "no tone curve, keep grading/dither/clamp". `Camera2d` now defaults to `Linear` (was `None`); `Camera3d` still defaults to `TonyMcMapface`, so an explicit `Tonemapping::None` on a `Camera3d` renders differently than in 0.19. The types moved to `bevy::render::view`, so reflected type paths in scene files and BRP keys are now `bevy_render::view::Tonemapping` / `DebandDither` (Rust imports still work via re-export).

## Render recovery (0.19)

GPU errors (driver crash, out-of-memory, device loss) previously hung or crashed the app with no recovery path — a real problem for long-lived apps (installations, VR). 0.19 surfaces them as typed errors via a `RenderErrorHandler` resource:

```rust
use bevy::render::error_handler::{ErrorType, RenderErrorHandler, RenderErrorPolicy};

app.insert_resource(RenderErrorHandler(|error, _main, _render| match error.ty {
    ErrorType::DeviceLost => RenderErrorPolicy::Recover(default()), // reinit renderer, keep running
    ErrorType::OutOfMemory => RenderErrorPolicy::StopRendering,     // halt rendering, app alive
    ErrorType::Validation => RenderErrorPolicy::Ignore,
    ErrorType::Internal => panic!(),                                // a bug
}));
```

`DeviceLost` (driver crashes, thermal shutdown, hardware disconnect) is the case most games want to recover. Test recovery carefully — repeated failures can cause flickering, a photosensitive-epilepsy risk. Without a configured handler, **every** render error (validation included) is logged and sends `AppExit::error()`, then stops rendering — deliberately overzealous, because an unfixed OOM or validation error repeats every frame. Return `RenderErrorPolicy::Ignore` for `Validation` in your own handler to keep running past those.

## Skinned mesh culling (0.19)

Animated characters used to vanish mid-animation because culling used the skeleton's *rest* pose. 0.19 computes bounds from actual joint positions each frame. For glTF-loaded skinned meshes this is automatic. For hand-built skinned meshes, call `mesh.generate_skinned_mesh_bounds()?` and add `DynamicSkinnedMeshBounds` to the entity. Opt out (or back to the old behavior) via `GltfPlugin::skinned_mesh_bounds_policy` (`GltfSkinnedMeshBoundsPolicy::BindPose` / `NoFrustumCulling`). Morph-target/vertex-shader-driven motion still needs a permissive manual bounding box.

## Dev/debug tools

These live behind `bevy_dev_tools` / the `dev` feature collection (and gizmos in `bevy_gizmos`). **(0.20) the render-dependent ones (overlays, infinite grid, screenshots, render debug) moved behind the new `render_dev_tools` feature**, which `dev` includes — `bevy_dev_tools` alone now builds render-free. New in 0.19:

- **Infinite grid** — an editor-style ground grid drawn as a fullscreen shader (no aliasing at the horizon). `app.add_plugins(InfiniteGridPlugin)`, then `commands.spawn(InfiniteGrid)`. Tune via `InfiniteGridSettings` on the grid entity or a camera. Path: `bevy::dev_tools::infinite_grid`.
- **Diagnostics overlay** — in-game diagnostics without hand-rolled UI. `DiagnosticsOverlayPlugin`, then `commands.spawn(DiagnosticsOverlay::fps())` or `DiagnosticsOverlay::mesh_and_standard_material()`, or build a custom one from `DiagnosticPath`s. Path: `bevy::dev_tools::diagnostics_overlay`.
- **Transform gizmo** — click-and-drag translate/rotate/scale handles for a level editor. `app.add_plugins(TransformGizmoPlugin)`, mark the camera with `TransformGizmoCamera` and editable entities with `TransformGizmoFocus`. It's deliberately not wired to input (you bring your own); configure via the `TransformGizmoSettings` resource (`mode: TransformGizmoMode`, snapping, screen scaling). Path: `bevy::gizmos::transform_gizmo`.
- **Text gizmos** — zero-setup world-space debug text with a built-in ASCII stroke font: `gizmos.text(isometry, "label", font_size, anchor, color)` (and `text_2d`, `text_sections`). For dev tools only — use `Text2d` for real in-game labels.
- **Render debug overlay** — draws depth, normals, motion vectors, deferred G-buffer channels or depth-pyramid mips over the frame. The plugin has been in `DefaultPlugins` since 0.19 (now under `render_dev_tools`), but **(0.20) its F1/F2 hotkeys are off by default**: `app.insert_resource(RenderDebugOverlayKeybindings { enable_keybindings: true, ..default() })` (not on `RenderDebugOverlay`, whatever the migration guide says). Turn the overlay on directly with the `GlobalRenderDebugOverlay { enabled: true, mode: RenderDebugMode::Normal, opacity: 1.0 }` resource, or a per-camera `RenderDebugOverlay` component. Each mode needs the matching prepass on the camera (`DepthPrepass`, `NormalPrepass`, `MotionVectorPrepass`, `DeferredPrepass`; `DepthPyramid` also needs `OcclusionCulling`) — with no prepass, F1 does nothing. Path: `bevy::dev_tools::render_debug`.
- **Pan-orbit camera (0.20)** — `bevy_editor_cam` upstreamed into `bevy::camera_controller::pan_orbit_camera` behind the off-by-default `pan_orbit_camera` feature (a sibling of `free_camera`/`pan_camera`). `app.add_plugins((MeshPickingPlugin, DefaultPanOrbitCameraPlugins))`, then `commands.spawn((Camera3d::default(), PanOrbitCamera::default()))`. It orbits/zooms around the picked point under the cursor, so a picking backend is required. **Pitfall:** it doesn't move out of the box — `DefaultPanOrbitCameraPlugins` ships no input plugin, whatever its doc comment says. Write one that calls `PanOrbitCamera::{start_orbit, start_pan, start_zoom, send_screenspace_input, send_zoom_input, end_move}`, or copy `examples/camera/pan_orbit_camera_custom_input_plugin.rs`.

Existing gizmos (`Gizmos::line`, `sphere`, `arrow`, AABB gizmos, etc.) are unchanged.

## Occlusion culling, Solari, mesh shaders

- **Occlusion culling is no longer experimental (0.19):** `bevy::render::experimental::occlusion_culling` → `bevy::render::occlusion_culling`.
- **Solari** (Bevy's realtime path-traced renderer, still experimental, feature `bevy_solari`) gained mirror/non-metallic fixes, performance and temporal stability in 0.19; 0.20 (wgpu 30) also runs on Apple Silicon, but the only denoiser is DLSS Ray Reconstruction (NVIDIA RTX + Vulkan). Shape: `add_plugins(SolariPlugins)`; the camera gets `SolariLighting::default()` (which auto-requires `Hdr` and the depth/motion-vector/deferred prepasses) plus `Msaa::Off` and `CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING)`; meshes get `RaytracingMesh3d(handle)` alongside `Mesh3d` + `MeshMaterial3d<StandardMaterial>` (tangents required — `.with_generated_tangents()`). Still niche.
- **Mesh shaders (0.20, experimental, native-only):** task/mesh pipelines go through the normal `PipelineCache` via `queue_mesh_pipeline(MeshPipelineDescriptor { .. })` (`bevy::material::descriptor`), drawn with `pass.draw_mesh_tasks(x, y, z)`. Needs `WgpuFeatures::EXPERIMENTAL_MESH_SHADER | PASSTHROUGH_SHADERS` on `RenderPlugin`'s `WgpuSettings`; no `Material` integration yet.
