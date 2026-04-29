# Assets

## Contents
- Loading — `AssetServer::load`, asset path resolution
- The handle/asset distinction — mutate handle (one entity) vs mutate asset (everyone)
- Render-component wrappers — `MeshMaterial3d`, `Mesh3d`, `Sprite::from_image`
- Reference counting — handles drop = asset unloads; the canonical pitfall
- Preload pattern — hold handles in resources to prevent unload churn
- Waiting for completion — `is_loaded_with_dependencies`, `VisitAssetDependencies` derive
- Hot reloading — `file_watcher` feature, `AssetEvent`/`AssetChanged`
- Asset-driven gameplay — RON/JSON manifests as gameplay data
- Embedded assets — `EmbeddedAssetRegistry`, `embedded://` URLs
- Web assets — `http`/`https` features, security caveat
- Custom asset types — `AssetLoader` trait, `TypePath` requirement (0.18)
- Asset processing — publish-time transforms, meta files
- Render-asset usage — GPU-only meshes, `try_*` mutation methods, auto-Aabb

The mental model: an asset of type `A` lives once in `Assets<A>` (a resource). Anything that wants to use it stores a `Handle<A>`. Handles are reference-counted; when the last handle drops, the asset is unloaded.

## Loading

```rust
fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    let handle: Handle<Image> = asset_server.load("branding/icon.png");
    commands.spawn(Sprite::from_image(handle));
}
```

`asset_server.load(path)` returns immediately with a handle. Loading happens asynchronously. The asset isn't necessarily ready when the handle comes back.

The path is relative to the `assets/` directory next to your `Cargo.toml`. Override the asset root with the `BEVY_ASSET_ROOT` environment variable or `AssetPlugin::file_path`.

Calls to `load(same_path)` are deduplicated by `AssetPath` — you get the same handle, no second load.

## The handle/asset distinction

When you "change a sprite," there are two different operations:

**Mutate the handle** (replace which asset the entity points at):

```rust
fn swap(mut sprite: Single<&mut Sprite, With<Player>>, server: Res<AssetServer>) {
    sprite.image = server.load("new_icon.png");
}
```

Only this entity changes. Other entities pointing at the original asset are unaffected.

**Mutate the asset** (change the underlying data shared by all handles):

```rust
fn fade(sprite: Single<&Sprite, With<Player>>, mut images: ResMut<Assets<Image>>) {
    if let Some(image) = images.get_mut(&sprite.image) {
        // ... edit pixels ...
    }
}
```

*Every* entity using this asset is affected — including ones in totally unrelated parts of your game. Use this deliberately, usually only for procedural content (e.g., a dynamically-rendered minimap).

## Render-component wrappers

Some components wrap a handle:

- **`MeshMaterial3d<M>`** — wraps `Handle<M>` for 3D materials.
- **`MeshMaterial2d<M>`** — same for 2D.
- **`Mesh3d`** — wraps `Handle<Mesh>` for 3D meshes.
- **`Mesh2d`** — same for 2D.
- **`Sprite`** — has an `image: Handle<Image>` field; usually constructed with `Sprite::from_image(handle)`.

A bare `Handle<StandardMaterial>` is *not* a `Component` — the wrapper is. Old code with `Query<&Handle<StandardMaterial>>` doesn't compile in 0.17+; replace with `Query<&MeshMaterial3d<StandardMaterial>>` and access the underlying handle via `.0`.

```rust
fn pulse_emissive(
    query: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
) {
    for mat in &query {
        if let Some(m) = materials.get_mut(&mat.0) {
            m.emissive = LinearRgba::new(time.elapsed_secs().sin(), 0.0, 0.0, 1.0);
        }
    }
}
```

## Reference counting

`Handle<A>` is conceptually a smart pointer to `Assets<A>` storage:

- Cloning a handle increments a counter.
- Dropping a handle decrements it.
- When the counter hits zero, Bevy unloads the asset.

This is why **dropping all handles unloads the asset, even if you're about to need it again.** A common bug:

```rust
// BAD: handle drops at end of statement, asset unloads.
asset_server.load::<Image>("enemy.png");

// Or this: spawning enemies, then despawning all of them, then trying to spawn more.
// First spawn batch loads "enemy.png" and holds it via the spawned Sprite components.
// All enemies despawn → all Sprite components drop → all handles drop → asset unloads.
// Next spawn triggers a *re-load* of "enemy.png".
```

The reload may take a frame or two during which rendering is missing/broken. Some downstream consumers panic on missing data.

## Preload pattern

Hold handles in a resource so they don't drop:

```rust
#[derive(Resource)]
struct EnemyAssets {
    sprite: Handle<Image>,
    sound: Handle<AudioSource>,
}

fn load_enemy_assets(server: Res<AssetServer>, mut commands: Commands) {
    commands.insert_resource(EnemyAssets {
        sprite: server.load("enemy.png"),
        sound: server.load("hit.ogg"),
    });
}

App::new().add_systems(Startup, load_enemy_assets);
```

Now spawning an enemy clones the handle:

```rust
fn spawn_enemy(assets: Res<EnemyAssets>, mut commands: Commands) {
    commands.spawn((Enemy, Sprite::from_image(assets.sprite.clone())));
}
```

Cloning is cheap (it's just bumping a refcount). The asset stays loaded as long as `EnemyAssets` lives.

The cost: `EnemyAssets` keeps the asset alive even when no enemies exist. For most games, the memory tradeoff is worth the gameplay simplicity. Only optimize this if profiling shows asset RAM is a real problem.

## Waiting for completion

`asset_server.load(path)` returns immediately, but the asset may not be ready. To gate gameplay on assets being loaded, check completion:

```rust
fn wait_for_assets(
    assets: Res<EnemyAssets>,
    server: Res<AssetServer>,
    mut next: ResMut<NextState<AssetState>>,
) {
    if server.is_loaded_with_dependencies(&assets.sprite)
        && server.is_loaded_with_dependencies(&assets.sound)
    {
        next.set(AssetState::Ready);
    }
}
```

Use `is_loaded_with_dependencies` (recursive — also checks dependencies of dependencies) for most cases. `is_loaded_with_direct_dependencies` only checks one level. `is_loaded` only checks the asset itself.

For multi-asset structs, derive `VisitAssetDependencies`:

```rust
#[derive(Resource, VisitAssetDependencies)]
struct EnemyAssets {
    #[dependency]
    sprite: Handle<Image>,
    #[dependency]
    sound: Handle<AudioSource>,
    #[dependency]
    sub: SubAssets,
}

#[derive(VisitAssetDependencies)]
struct SubAssets {
    #[dependency]
    extra: Handle<Image>,
}
```

Now `asset_server.are_dependencies_loaded(&enemy_assets)` reports the combined state. Adding new fields requires only the `#[dependency]` annotation — no manual `is_loaded` to keep updated.

The canonical loading screen pattern: define an `AssetState` (or similar) state, run `wait_for_assets` only in the loading state, transition to `Ready` when loading finishes, gate gameplay systems behind `in_state(AssetState::Ready)`.

## Hot reloading

Enable the `file_watcher` feature:

```toml
bevy = { version = "0.18", features = ["file_watcher"] }
```

Or run with the flag:

```sh
cargo run --features bevy/file_watcher
```

Now changes to asset files are picked up automatically — `Assets<T>` is updated with the new data, and existing handles silently point at the new version. No restart needed.

To react to reloads:

```rust
fn on_image_changed(mut events: MessageReader<AssetEvent<Image>>) {
    for event in events.read() {
        if let AssetEvent::Modified { id } = event {
            // ...
        }
    }
}
```

Or use the `AssetChanged<T>` query filter:

```rust
fn handle_changed_images(query: Query<&Sprite, AssetChanged<Image>>) {
    // Sprites whose image asset changed since last run.
}
```

`AssetEvent` variants: `Added`, `Modified`, `Removed`, `LoadedWithDependencies`, `Unused`.

For embedded assets, also enable `embedded_watcher` to hot-reload from the embedded source.

Don't ship `file_watcher` in production builds — it adds runtime overhead and is a vector for asset-spoofing if your binary is on disk where users can replace assets.

## Asset-driven gameplay

Once you can hot-reload assets, RON or JSON files can serve as gameplay data:

```ron
(
    items: {
        "sword": (name: "Iron Sword", damage: 10, weight: 2.0),
        "shield": (name: "Round Shield", defense: 5, weight: 5.0),
    },
)
```

Define an asset type, register a loader (typically using `serde`):

```rust
#[derive(Asset, TypePath, Deserialize)]
struct ItemManifest {
    items: HashMap<String, Item>,
}

#[derive(Deserialize)]
struct Item { name: String, damage: u32, /* ... */ }
```

Implement `AssetLoader` for the type or use `bevy_common_assets` (an ecosystem crate) for RON/JSON/etc.

Load it like any asset, hold the handle in a resource, look up data via the handle:

```rust
fn use_item(manifest: Res<Assets<ItemManifest>>, handle: Res<ItemManifestHandle>) {
    if let Some(m) = manifest.get(&handle.0) {
        // m.items["sword"].damage
    }
}
```

Hot reload tweaks the file → asset reloads → next frame your game uses new values. Excellent for tuning numbers, especially in games with lots of structured data (RPGs, factory builders, ARPGs).

This isn't right for every game. Walking simulators don't have data to tune. Action games may need real script (Lua, Dioxus) for genuinely complex behavior. But for tuning-heavy gameplay, asset-driven configuration is a powerful pattern.

## Embedded assets

Assets compiled into the binary. Useful for fonts, default icons, small models — anything you don't want users tampering with.

```rust
use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use std::path::PathBuf;

const AVATAR_GLB: &[u8] = include_bytes!("../assets/avatar.glb");

fn build(app: &mut App) {
    let registry = app.world().resource::<EmbeddedAssetRegistry>();
    let path = PathBuf::from("my_crate/avatar.glb");
    registry.insert_asset(path.clone(), &path, AVATAR_GLB);
}
```

Then load with the `embedded://` URL scheme:

```rust
let handle: Handle<Scene> = asset_server.load("embedded://my_crate/avatar.glb#Scene0");
```

`#Scene0` is a glTF subasset path — same syntax as for filesystem glTFs.

Combine with `embedded_watcher` for hot reload from the source files during development, falling back to embedded bytes in release builds.

## Web assets

The `http`/`https` features let `asset_server.load("https://example.com/icon.png")` work over the network:

```toml
bevy = { version = "0.18", features = ["http", "https"] }
```

On native, this uses the `ureq` crate. On wasm, it uses the browser's fetch API.

Add the `web_asset_cache` feature to cache fetched assets to disk locally.

**Security note**: don't take asset URLs from untrusted user input. A malicious URL can DoS your game (huge downloads), and any vulnerability in an asset loader becomes a remote code execution vector. Whitelist the hosts you load from.

## Custom asset types

Implement `AssetLoader` for your type:

```rust
#[derive(Asset, TypePath)]
struct MyManifest { /* fields */ }

#[derive(TypePath)]
struct MyManifestLoader;

impl AssetLoader for MyManifestLoader {
    type Asset = MyManifest;
    type Settings = ();
    type Error = MyError;

    async fn load(&self, reader: &mut dyn Reader, _: &(), _: &mut LoadContext<'_>) 
        -> Result<MyManifest, MyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let manifest = ron::de::from_bytes(&bytes)?;
        Ok(manifest)
    }

    fn extensions(&self) -> &[&str] { &["manifest.ron"] }
}

App::new()
    .init_asset::<MyManifest>()
    .init_asset_loader::<MyManifestLoader>();
```

In 0.18, `AssetLoader`/`AssetTransformer`/`AssetSaver`/`Process` all require `TypePath` — derive it on the loader struct.

For 0.18, `LoadContext::path()` returns `AssetPath` (used to return a `Path`). If you need the underlying path, `load_context.path().path()` works, but using `AssetPath` is preferred — it supports custom asset sources cleanly.

## Asset processing

Bevy supports processing assets at "publish time" (build time, or first run) into a more optimal format:

```rust
fn build(app: &mut App) {
    app.register_asset_processor::<LoadTransformAndSave<TextLoader, TextTransformer, TextSaver>>(/* ... */);
}
```

Configure per-asset processing via meta files (`my_asset.png.meta`):

```ron
(
    meta_format_version: "1.0",
    asset: Process(
        // 0.18 supports the short form too:
        processor: "LoadTransformAndSave<TextLoader, TextTransformer, TextSaver>",
        settings: ( /* ... */ ),
    ),
)
```

Processed assets are cached. The processor runs once and the result is reused for subsequent loads. Useful for expensive transforms — image resizing, mesh optimization, audio compression.

## Render-asset usage

Some assets (meshes, images) are uploaded to GPU memory and may have their CPU-side data discarded:

```rust
let mesh = Mesh::new(/* ... */).with_render_asset_usages(RenderAssetUsages::RENDER_WORLD);
```

For these meshes, `mesh.insert_attribute(...)` panics in 0.18 (the data has been extracted to the render world and is no longer accessible CPU-side).

Use the `try_*` variants if there's any chance the asset is render-only:

```rust
mesh.try_insert_attribute(Mesh::ATTRIBUTE_POSITION, positions)?;
```

Or set `RenderAssetUsages::all()` (default) to keep CPU data alongside the GPU upload — costs RAM but lets you read/mutate after the asset has been extracted.

In 0.18, `Aabb` for meshes/sprites updates automatically when you mutate the underlying mesh. Drop any old `entity.remove::<Aabb>()` workarounds. Use `NoAutoAabb` to opt out (e.g., for procedural meshes where you want manual control).
