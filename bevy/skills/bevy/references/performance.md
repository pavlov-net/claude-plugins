# Performance and profiling

## Contents
- Cheap optimizations — change detection, query filters, run conditions, parallel iteration
- Fixed timestep — for physics, networking, deterministic simulation
- Profiling tools — Tracy, Chrome tracing, perf flame graphs, GPU vendor tools
- Compile profiles — `release`, `dev` with package-level `opt-level = 3`
- Dev iteration speed — dynamic linking, fast linkers, Cranelift, sccache
- Compile-time analysis — `cargo build --timings`, `cargo tree -d`, `cargo bloat`, `cargo llvm-lines`
- Compile less code — feature collections (0.18), removing unused features, workspace splits
- Memory and binary size — wasm tuning with `wasm-opt`, RAM patterns
- Don't — anti-patterns for premature optimization

Don't optimize what you haven't measured. "I think this is slow" is rarely true; "the profiler says this is slow" is actionable.

## Cheap optimizations (do these first)

### Change detection

`Query<&T, Changed<T>>` skips entities whose `T` hasn't been mutated since the system last ran. For systems that touch large numbers of entities but care about a small fraction:

```rust
fn update_health_bars(query: Query<(&Health, &mut HealthBar), Changed<Health>>) {
    for (h, mut bar) in &mut query {
        bar.fill = h.percentage();
    }
}
```

Two notes:

- Performance-wise, query filters and `Ref<T>::is_changed()` are equivalent — the query iterator skips non-matching entities, but they're still fetched. The win is in the body, not the iteration.
- Mutable deref unconditionally marks changed. If your write often produces the same value, use `set_if_neq` or guard the write — otherwise downstream gates on `Changed<T>` fire constantly.

### Filter at the query, not in the loop

```rust
// Slow: fetches every entity, checks in loop.
for (health, maybe_armor) in &query {
    if let Some(armor) = maybe_armor { /* ... */ }
}

// Fast: query already filtered.
for (health, armor) in query.iter() { /* ... */ }
// where the query is Query<(&Health, &Armor)>
```

`With<T>` and `Without<T>` are essentially free — they're archetype-level filters. Use them aggressively to narrow the query rather than checking inside the loop.

### Run conditions

`.run_if(in_state(GameState::Playing))` skips the entire system when the condition is false — no dispatch cost, parallelism preserved. Returning early from the system body still pays dispatch.

For periodic tasks:

```rust
app.add_systems(Update, expensive.run_if(on_timer(Duration::from_millis(100))));
```

Note: `expensive` running once every 100ms is still blocking when it runs. If the work is heavy, split it across frames or move it to a background task — don't just throttle a frame-locking system.

### Parallel iteration

For independent work across entities:

```rust
fn process(query: Query<&Foo>) {
    query.par_iter().for_each(|foo| {
        // Independent work per entity.
    });
}
```

`par_iter_mut` for mutation. Combine with `ParallelCommands` if you need to issue commands from parallel work:

```rust
fn process(
    mut query: Query<(Entity, &Velocity)>,
    par_commands: ParallelCommands,
) {
    query.par_iter_mut().for_each(|(entity, velocity)| {
        if velocity.magnitude() > 10.0 {
            par_commands.command_scope(|mut cmds| {
                cmds.entity(entity).insert(Fast);
            });
        }
    });
}
```

`command_scope` gives each parallel iteration its own `Commands` instance.

## Fixed timestep

For physics, networking, deterministic gameplay. The fixed loop runs zero or more times per frame to maintain a target update rate independent of frame rate.

```rust
app.insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0));
app.add_systems(FixedUpdate, (apply_velocity, integrate).chain());
```

Inside `FixedUpdate`, `Res<Time>` is `Time<Fixed>` automatically; `time.delta_secs()` is the fixed timestep.

The visual gotcha: rendering happens at variable frame rate, fixed simulation at constant rate. Without interpolation, motion looks jittery (a frame happens between two fixed steps, and the rendered transform is from the previous fixed step).

The fix: track logical position separately from visual `Transform`, interpolate visual between previous and current logical based on fractional progress through the next fixed step. Put the interpolation system at `RunFixedMainLoopSystems::AfterFixedMainLoop`. See `physics_in_fixed_timestep` example, or use a community interpolation crate.

## Profiling tools

### Tracy

Best general-purpose profiler. Nanosecond resolution, low overhead, excellent UI.

Enable Bevy's tracing spans and the Tracy backend:

```sh
cargo run --release --features bevy/trace_tracy
```

Bevy automatically instruments all ECS systems. You'll see a flame graph with one row per system showing per-frame execution time.

Run the Tracy capture tool first to record:

```sh
./capture-release -o my_capture.tracy
```

Then start your app. Tracy auto-connects when it sees the instrumented binary.

Or run the Tracy GUI for live capture (but the GUI itself competes for graphics resources, so live capture on the same machine can skew results — prefer command-line capture for accurate measurements).

Add custom spans:

```rust
fn expensive(/* ... */) {
    let _span = info_span!("my_expensive_thing").entered();
    // ...
}
```

The `_span` guard ends when dropped, so the timing reflects the scope.

For memory profiling (significant overhead):

```sh
cargo run --release --features bevy/trace_tracy_memory
```

Tracy's "MTPC" (mean time per call) column is the most useful summary metric — sort by it to find the slowest spans.

Statistics view (per-span): histogram of execution times, mean/median/stddev, plus the actual sequence of calls for trend analysis.

Compare button: load a second trace and diff the per-span distributions. Useful for "did my change make this faster?"

### Chrome tracing

```sh
cargo run --release --features bevy/trace_chrome
```

Produces a `.json` trace; open in <https://ui.perfetto.dev>. Cross-platform, no extra tools to install. Less interactive than Tracy but good for sharing traces (just send the file).

### perf flame graphs

For "where is time *actually* spent inside this function":

```sh
RUSTFLAGS='-C force-frame-pointers=y' cargo flamegraph -c "record -g" --example my_example
```

Higher overhead than Tracy and your app runs slower. The output is a real call-graph flame graph (Tracy shows spans, which are typically system-level).

### GPU profiling

When CPU profiling shows the GPU is the bottleneck (frames are taking suspiciously long, render-thread spans look long but GPU work isn't visible), use vendor tools:

- NVIDIA: Nsight Graphics
- AMD: Radeon GPU Profiler
- Intel: Graphics Frame Analyzer
- Apple: Xcode's Metal debugger

These show GPU command timing — what shaders ran, how long each pass took, memory bandwidth usage.

Tracy's `RenderQueue` row shows coarse GPU timings if you enable `trace_tracy`. Useful for "is the GPU getting saturated?" but not detailed enough for shader-level optimization. Add custom GPU spans with `RenderDiagnosticsPlugin`.

GPU clocks vary frame-to-frame. Don't trust individual frame times — use the MTPC or median.

RenderDoc is a great *debugging* tool but not a profiler. Don't use it for performance work.

## Compile profiles

Release tuning:

```toml
[profile.release]
opt-level = 3                # 'z' or 's' for binary size on wasm/mobile
lto = "fat"                  # Link-time optimization (slow link, faster binary)
codegen-units = 1            # Less parallelism, more optimization
strip = "debuginfo"          # Strip debug symbols, keep symbol table for profilers
```

For wasm/mobile, optimize for binary size:

```toml
[profile.release]
opt-level = 'z'
lto = "fat"
codegen-units = 1
strip = true                 # Strip everything for smallest size
```

Keep dev compile times reasonable while running dependencies at full optimization:

```toml
[profile.dev]
opt-level = 1                # Optimize your code mildly (helps if Bevy is unoptimized)

[profile.dev.package."*"]
opt-level = 3                # Full optimization for dependencies (Bevy etc.)
```

This is the canonical Bevy dev profile. Your code recompiles fast (light optimization), Bevy is precompiled with full optimization (no per-recompile cost since Bevy doesn't change), the resulting binary runs at near-release speed.

## Dev iteration speed

### Dynamic linking

Single biggest dev-time win: build Bevy as a dynamic library so your code doesn't relink it every change.

```sh
cargo run --features bevy/dynamic_linking
```

Or set up a feature flag for it:

```toml
[features]
fast-compile = ["bevy/dynamic_linking"]
```

Then `cargo run --features fast-compile`.

Don't ship `dynamic_linking` — it requires shipping `libbevy_dylib` alongside the executable, prevents some optimizations, and inflates binary size.

### Linker

The linker is often the slowest part of incremental builds. Use a fast one:

- Linux: Rust 1.90+ uses `lld` by default on `x86_64-unknown-linux-gnu`. Otherwise install via your package manager.
- macOS: `lld` works; `mold` does too.
- Windows: `lld-link` works.

For even faster link times on Linux, install `mold`:

```sh
sudo apt install mold clang
```

Then in `.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

### Cranelift

Faster codegen than LLVM (~30%), nightly only:

```sh
rustup component add rustc-codegen-cranelift-preview --toolchain nightly
```

`.cargo/config.toml`:

```toml
[unstable]
codegen-backend = true

[profile.dev]
codegen-backend = "cranelift"

[profile.dev.package."*"]
codegen-backend = "llvm"
```

Cranelift is fast to compile but the binary is slower than LLVM-compiled. Use for `cargo run` during dev; switch to LLVM for benchmarking. Wasm builds don't work with Cranelift yet.

### sccache

Caches `rustc` outputs across multiple build directories or feature combinations. Mostly useful in CI / Docker scenarios; minimal benefit for a single local workflow.

```sh
cargo install sccache
```

`.cargo/config.toml`:

```toml
[build]
rustc-wrapper = "sccache"
```

## Compile-time analysis

### `cargo build --timings`

Generates an HTML report showing how long each crate took to compile. Save it (in `target/cargo-timings/`), open in browser. The big spikes are your bottlenecks.

For a clean timing baseline, run `cargo clean` first.

### `cargo tree -d`

Lists duplicate crate versions in your tree. Cargo will compile each version separately, bloating compile time and binary size.

```sh
cargo tree -d
```

The fix is usually upstream — politely ask maintainers to bump shared deps. As a last resort, `[patch.crates-io]` to force one version (with the caveat that nothing was tested against your patched version).

### `cargo bloat`

Shows the largest functions in your final binary:

```sh
cargo install cargo-bloat
cargo bloat --release
```

Generic functions monomorphized many times often dominate. `cargo bloat --release --crates` aggregates by crate.

### `cargo llvm-lines`

Counts LLVM IR lines per function — reveals which generics produce the most code. Useful when binary size is a concern (especially for wasm):

```sh
cargo install cargo-llvm-lines
cargo llvm-lines --release
```

## Compile less code

### Cargo feature collections

In 0.18, top-level collections replace hand-listing dozens of features:

```toml
bevy = { version = "0.18", default-features = false, features = ["3d", "ui"] }
```

Available collections: `2d`, `3d`, `ui`, `audio`, `dev`. Mid-level: `2d_api`, `3d_api`, `default_app`, `default_platform`.

`dev` should not be enabled in release builds — it pulls in `bevy_dev_tools`, fast-compile features, etc.

### Removing unused features

Two approaches:

1. Start with `default-features = false` and add features one at a time as compile errors demand them. Slowest path to a working build, fastest compile times.
2. Start with default features, get a working build, then disable features one at a time and see what breaks. Faster path to a working build, harder to find optimal feature set.

Bevy's feature flow: `bevy_ecs` (etc.) defines features → `bevy_internal` re-exposes them → `bevy` mirrors them → feature collections bundle them. `cargo tree -f {p} {f}` shows what's enabled.

If a dependency is silently enabling a Bevy feature you don't need, that's an upstream bug worth reporting.

### Workspace splits

Cargo parallelizes compilation at the crate level. Splitting your project into multiple crates lets:

- Cargo skip recompiling crates you didn't touch (incremental builds become much faster).
- The compiler localize generic-type instantiation to fewer crates.

A common shape: a binary crate (your app's `main.rs`) + a library crate (your gameplay code) + further library crates for stable subsystems (AI, UI, content). The binary recompiles instantly when you only change library code.

This compounds with `cargo build --timings` — split out the crates that show up as bottlenecks.

## Memory and binary size

### Binary size

Look at the final binary in `target/release/`. Don't confuse with the `target/` directory size, which is mostly cached compilation artifacts.

For wasm, every byte matters (page load time):

- Use `opt-level = 'z'` and `panic = "abort"`.
- Run `wasm-opt` after `wasm-bindgen`:

```sh
wasm-opt -Oz --strip-debug -o output.wasm input.wasm
```

`wasm-opt -O` for speed, `-Oz`/`-Os` for size, `-O -ol 100 -s 100` for both.

### RAM

Bevy is ECS-architectured for cache-friendly iteration, but data layout still matters. Components packed densely (`#[derive(Component)]` on a small struct) iterate fast. `Vec<Entity>` inside a component breaks density and is slow.

For large entity counts, profile with Tracy memory tracking enabled to find allocation hot spots. Common sources:

- `String` fields on components that should be `&'static str` or interned.
- `Vec<...>` fields that grow per frame and should be reused (use `Local<Vec<...>>` and clear between calls).
- Asset clones — `Handle<T>::clone` is cheap (just a refcount) but cloning the *contents* (e.g., `Image::clone`) is not.

## Don't

- Don't pre-optimize. Profile first.
- Don't optimize compile time at the cost of runtime performance for your users (e.g., shipping `dynamic_linking`).
- Don't optimize before you have a representative scene. A test with 100 entities is uninformative for a game with 10,000.
- Don't `cargo clean` unless you suspect corruption — Bevy from-scratch is multi-minute. Trust incremental.
