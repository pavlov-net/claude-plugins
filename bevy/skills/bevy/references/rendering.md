# Rendering

## Contents
- The two worlds — main world vs render world, extraction, `RenderApp`
- Render graph as systems (0.19) — `Core3d`/`Core2d` schedules, `Core3dSystems` sets, `ViewQuery`, `RenderContext`, `RenderStartup`
- Cameras — `Camera3d`/`Camera2d`, `RenderTarget` component (0.18), HDR, ordering
- Lights and shadows — `shadow_maps_enabled`/`contact_shadows_enabled` (0.19), ambient light split
- Atmosphere and sky — `Atmosphere` as an entity (0.19), `Skybox`, light probes, parallax cubemaps
- Materials — `Material` trait methods (0.18), `bevy_material` crate, bindless on Metal
- Post-processing — `Vignette`/`LensDistortion` (0.19), the post-process split, bloom/PBR fixes
- Render recovery (0.19) — `RenderErrorHandler`, GPU device loss
- Skinned mesh culling (0.19) — `DynamicSkinnedMeshBounds`
- Dev/debug tools — infinite grid, diagnostics overlay, transform gizmo, text gizmos
- Occlusion culling, Solari

Most gameplay code never touches rendering internals — you spawn `Camera3d`, `Mesh3d` + `MeshMaterial3d`, and lights, and let Bevy render. This reference is for the cases where you *do* reach into rendering: custom render passes, post-processing, camera/light configuration, and dev tooling. 0.19 reworked the render-graph internals substantially (render passes are systems now), so custom-render code from 0.18 needs porting.

## The two worlds

Bevy renders in a separate **render world**, rebuilt each frame from the **main world** by *extraction* systems. The render world lives in a sub-app (`RenderApp`) with its own schedules. The split exists so rendering can pipeline against the next frame's simulation.

You rarely need this for gameplay. You need it when writing a custom material, a custom render pass, or a render-world resource. Gateways into the render world:

- `ExtractComponent` / `ExtractResource` — copy a component/resource into the render world each frame.
- `app.sub_app_mut(RenderApp)` — add render-world systems/resources from a plugin's `build`.
- `RenderStartup` (0.17+) — a normal startup schedule in the render world; the recommended place to initialize render resources, replacing old `Plugin::finish` patterns. (0.19: `MeshPipeline`, `MeshPipelineViewLayouts`, and similar built-in pipeline resources are created in `RenderStartup` now — order custom `RenderStartup` systems after `MeshPipelineSystems` if you depend on them.)

0.19 reworked `ExtractComponent`: removing a synced component no longer despawns the render entity (it removes the `Target` components of the new `SyncComponent` subtrait instead). If you wrote a custom `ExtractComponent`, implement `SyncComponent` to declare what gets cleaned up.

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

Fullscreen post-process materials moved from `run_in`/`run_after`/`run_before` to a single `FullscreenMaterial::schedule_configs(...)` that returns `ScheduleConfigs` you configure with `.in_set(...)`/`.before(...)`.

## Cameras

Spawn a camera as an entity:

```rust
commands.spawn(Camera3d::default());          // 3D
commands.spawn(Camera2d);                      // 2D
```

- **`RenderTarget` is its own component (0.18).** To render to a texture, spawn `RenderTarget::Image(image_handle.into())` alongside `Camera3d` rather than setting `Camera { target: ... }`. `RenderTarget::Window(..)` is the default.
- **`Hdr` is a component (moved to `bevy_camera` in 0.19).** Add `Hdr` to a camera for an HDR render target; it's a camera property, not a view property. Internally `ExtractedView::hdr` moved to `ExtractedCamera::hdr`, and `ViewTarget::is_hdr`/`TextureFormat::bevy_default()` are deprecated in favor of sourcing the format from `ExtractedView::target_format`.
- **Camera `order` controls draw sequence** when multiple cameras render to the same target (UI camera on top of game camera, etc.).
- **`Viewport`** restricts a camera to a sub-rect of its target (split-screen).

`ScreenSpaceTransmission` (0.19) pulled the screen-space transmission knobs off `Camera3d` into their own `bevy_pbr` component (`steps`, `quality`).

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

- **`Material` trait methods (0.18):** prepass/shadow opt-outs are `fn enable_prepass() -> bool` / `fn enable_shadows() -> bool` on the `Material` impl, not `MaterialPlugin` fields.
- **`bevy_material` crate (0.19):** material machinery (`AlphaMode`, `OpaqueRendererMethod`, `MaterialProperties`, pipeline descriptor types, …) was extracted out of `bevy_pbr`/`bevy_render` into a new `bevy_material` crate. Many items re-export, but `AlphaMode` and a few others need import-path updates.
- **Partial bindless on Metal (0.19):** `StandardMaterial` and other texture-only materials now get bindless rendering on Mac/iOS (big perf win on Apple GPUs). No code changes needed.
- **`ShaderStorageBuffer` → `ShaderBuffer`** (0.19), and `bevy_shader` dropped some superfluous getter methods.

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

Both live in `bevy::post_process::effect_stack`. `Vignette` darkens the periphery (animate `intensity` for damage-pulse / horror effects). `LensDistortion` warps spatially — positive `intensity` is barrel (fisheye/speed), negative is pincushion (impairment). Existing effects (`Bloom`, `Tonemapping`, `Fxaa`, etc.) are unchanged; note 0.19 corrected bloom's luma calculation to linear space, so high-saturation scenes may show *less* bloom than before — bump `Bloom::intensity` if a scene now looks dim. Order custom effects against the `EarlyPostProcess` / `PostProcess` sets (see the render-graph section above).

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

`DeviceLost` (driver crashes, thermal shutdown, hardware disconnect) is the case most games want to recover. Test recovery carefully — repeated failures can cause flickering, a photosensitive-epilepsy risk. Without a configured handler, validation errors are ignored and everything else sends `AppExit` for a graceful shutdown.

## Skinned mesh culling (0.19)

Animated characters used to vanish mid-animation because culling used the skeleton's *rest* pose. 0.19 computes bounds from actual joint positions each frame. For glTF-loaded skinned meshes this is automatic. For hand-built skinned meshes, call `mesh.generate_skinned_mesh_bounds()?` and add `DynamicSkinnedMeshBounds` to the entity. Opt out (or back to the old behavior) via `GltfPlugin::skinned_mesh_bounds_policy` (`GltfSkinnedMeshBoundsPolicy::BindPose` / `NoFrustumCulling`). Morph-target/vertex-shader-driven motion still needs a permissive manual bounding box.

## Dev/debug tools

These live behind `bevy_dev_tools` / the `dev` feature collection (and gizmos in `bevy_gizmos`). New in 0.19:

- **Infinite grid** — an editor-style ground grid drawn as a fullscreen shader (no aliasing at the horizon). `app.add_plugins(InfiniteGridPlugin)`, then `commands.spawn(InfiniteGrid)`. Tune via `InfiniteGridSettings` on the grid entity or a camera. Path: `bevy::dev_tools::infinite_grid`.
- **Diagnostics overlay** — in-game diagnostics without hand-rolled UI. `DiagnosticsOverlayPlugin`, then `commands.spawn(DiagnosticsOverlay::fps())` or `DiagnosticsOverlay::mesh_and_standard_material()`, or build a custom one from `DiagnosticPath`s. Path: `bevy::dev_tools::diagnostics_overlay`.
- **Transform gizmo** — click-and-drag translate/rotate/scale handles for a level editor. `app.add_plugins(TransformGizmoPlugin)`, mark the camera with `TransformGizmoCamera` and editable entities with `TransformGizmoFocus`. It's deliberately not wired to input (you bring your own); configure via the `TransformGizmoSettings` resource (`mode: TransformGizmoMode`, snapping, screen scaling). Path: `bevy::gizmos::transform_gizmo`.
- **Text gizmos** — zero-setup world-space debug text with a built-in ASCII stroke font: `gizmos.text(isometry, "label", font_size, anchor, color)` (and `text_2d`, `text_sections`). For dev tools only — use `Text2d` for real in-game labels.

Existing gizmos (`Gizmos::line`, `sphere`, `arrow`, AABB gizmos, etc.) are unchanged.

## Occlusion culling and Solari

- **Occlusion culling is no longer experimental (0.19):** `bevy::render::experimental::occlusion_culling` → `bevy::render::occlusion_culling`.
- **Solari** (Bevy's realtime path-traced renderer, still experimental) gained mirror/non-metallic fixes, performance, and much better temporal stability in 0.19. Niche; see Bevy's Solari blog posts.
