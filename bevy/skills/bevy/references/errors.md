# Error handling

## Contents
- When to panic — almost never; useful Clippy lints
- Result-returning systems — `Result<(), BevyError>`, `?` with `thiserror`
- Configuring the global handler — presets, dev/release feature gating
- Per-call severity — `with_severity`, `map_severity`
- Recoverable failures via match / let-else / if-let
- Fallible system params — `Single<...>`, `Option<Res<T>>` for silent skip
- Piping handlers — `system.pipe(handler)` for per-system logic
- Errors in commands — `queue_handled`, `queue_silenced`
- Combinator semantics (0.18) — `or`/`and` no longer propagate errors
- When to use what — decision table

Bevy distinguishes three failure tiers: panic (truly unrecoverable), `Err` from a system (recoverable, routed to a global handler), and silent skip (fallible system params).

## When to panic

Almost never in application code. `unwrap`, `expect`, and `panic!` should be reserved for:

- Tests (`assert!`, `assert_eq!` panic on failure — that's the point).
- Unsafe code maintaining safety invariants.
- Genuinely unrecoverable bugs that indicate the program is in a broken state and continuing would do more harm than crashing.

If you find yourself reaching for `unwrap` because "this can't fail," ask whether the failure mode is *truly* impossible or just unlikely. Unlikely failure modes find their way to production eventually.

Useful Clippy lints to enforce this:

```toml
[workspace.lints.clippy]
unwrap_used = "warn"
expect_used = "warn"
indexing_slicing = "warn"
panic = "warn"
todo = "warn"
```

## Result-returning systems

Bevy's prelude defines `Result` as `Result<(), BevyError>`. Systems can return it:

```rust
fn camera_log(query: Query<&Camera>) -> Result {
    let camera = query.single()?;
    info!(?camera);
    Ok(())
}
```

`BevyError` has a blanket `From` for any `std::error::Error`. So `?` works on most error types, including custom ones built with `thiserror`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum SaveError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

fn save_game(/* ... */) -> Result {
    let data = serialize_world()?;
    std::fs::write("save.dat", data)?;  // io::Error → BevyError via From
    Ok(())
}
```

Bevy controls system execution, so it controls what happens when a system returns `Err`. The default global handler **panics** on `Err` — loud and helpful during development.

## Configuring the global handler

For production builds, you usually want non-panicking behavior:

```rust
use bevy::ecs::error::warn;

fn main() {
    let mut app = App::new();
    app.set_error_handler(warn);  // log at warn level instead of panicking
    app.add_plugins(DefaultPlugins).run();
}
```

Available presets:

- `panic` (default) — `panic!` on any `Err`. Loud crashes.
- `error` — log at error level.
- `warn` — log at warn level.
- `info`, `debug`, `trace` — log at lower levels.
- `ignore` — drop the error silently.

Pattern: feature-flag the choice between dev (panic) and release (warn):

```rust
#[cfg(debug_assertions)]
app.set_error_handler(panic);
#[cfg(not(debug_assertions))]
app.set_error_handler(warn);
```

**Library plugins must never call `set_error_handler`.** It's an application-level policy. A library that overrides it surprises every consumer.

## Per-call severity

When most errors should panic but one specific call site should warn (or vice versa), override at the `?`:

```rust
use bevy::prelude::*;

fn lookup(query: Query<&Camera>) -> Result {
    let camera = query.single().with_severity(Severity::Warn)?;
    info!(?camera);
    Ok(())
}
```

`with_severity` applies one severity to all error variants. `map_severity` varies by variant:

```rust
fn lookup(query: Query<&Camera>) -> Result {
    let camera = query.single().map_severity(|e| match e {
        QuerySingleError::NoEntities(_) => Severity::Ignore,    // expected when no camera yet
        QuerySingleError::MultipleEntities(_) => Severity::Error,  // a real bug
    })?;
    info!(?camera);
    Ok(())
}
```

`Severity` variants: `Panic`, `Error`, `Warn`, `Info`, `Debug`, `Trace`, `Ignore`.

## Recoverable failures via match / let-else

For errors you want to handle locally (not propagate to the global handler):

```rust
fn update(query: Query<&Camera>, mut commands: Commands) {
    match query.single() {
        Ok(camera) => info_once!(?camera),
        Err(QuerySingleError::NoEntities(_)) => {
            commands.spawn(Camera2d);  // spawn one if missing
        }
        Err(QuerySingleError::MultipleEntities(_)) => {
            warn!("multiple cameras found");
        }
    }
}
```

`let-else` for the common single-arm case:

```rust
fn update(query: Query<&Camera>, mut commands: Commands) {
    let Ok(camera) = query.single() else {
        commands.spawn(Camera2d);
        return;
    };
    let Some(target_info) = &camera.computed.target_info else {
        return;
    };
    info!(?target_info);
}
```

`if-let` with `&&` chains:

```rust
let computed = if let Ok(camera) = query.single()
    && let Some(info) = &camera.computed.target_info
{
    info
} else if query.count() == 0 {
    commands.spawn(Camera2d);
    return;
} else {
    return;
};
```

## Fallible system params

For "this resource may not exist yet" or "the player may not exist," fallible system params skip the system entirely:

- **`Single<T, F>`** — succeeds when exactly one entity matches; system skipped otherwise.
- **`Option<Res<T>>`** / **`Option<ResMut<T>>`** — succeeds with `None` if the resource is absent; the system runs and you handle absence in the body.

`Single` is the right tool for "no player yet, that's fine":

```rust
fn move_player(player: Single<&mut Transform, With<Player>>) {
    player.translation.x += 1.0;
}
```

Zero or 2+ matching entities → the system is silently skipped. No panic, no `Err`, no log.

`Option<Res<T>>` is the right tool for resources that may load late:

```rust
fn use_assets(assets: Option<Res<EnemyAssets>>) {
    let Some(assets) = assets else { return };
    // ...
}
```

`Res<T>` (without `Option`) panics if `T` isn't inserted — useful when you've gated the system behind a `run_if(resource_exists::<T>)` so the panic is genuinely impossible.

If you're writing a custom `SystemParam` that may fail validation, implement `validate_param`:

```rust
impl SystemParam for MyParam {
    fn validate_param(/* ... */) -> Result<(), SystemParamValidationError> {
        // Return Err to skip the system silently, or build a panic message.
    }
}
```

## Piping handlers

When you want a custom handler for one specific system without overriding the global handler:

```rust
app.add_systems(Update, update.pipe(handle_error));

fn update(query: Query<&Camera>) -> Result {
    let camera = query.single()?;
    info!(?camera);
    Ok(())
}

fn handle_error(In(input): In<Result>) {
    let Err(err) = input else { return };
    info_once!(?err);
}
```

The piped handler takes `In<Result<T, E>>` (where `T`/`E` match the upstream system's return type). Bevy treats the pipe as a single combined system from the scheduler's perspective.

You can pipe through more specific error types:

```rust
fn update(query: Query<&Camera>) -> Result<(), QuerySingleError> {
    let camera = query.single()?;
    info!(?camera);
    Ok(())
}

fn handle_error(In(input): In<Result<(), QuerySingleError>>) {
    if let Err(e) = input {
        match e {
            QuerySingleError::NoEntities(_) => { /* ... */ }
            QuerySingleError::MultipleEntities(_) => { /* ... */ }
        }
    }
}
```

## Errors in commands

Commands can fail too — the entity might be despawned before the command runs, the world might not have the expected resources, etc.

Default behavior: command errors go to the global handler.

`queue_handled` for explicit handling:

```rust
fn save(mut commands: Commands) {
    commands.queue_handled(
        |world: &mut World| -> Result {
            world.get_resource::<SomeData>().ok_or("not inserted")?;
            // ...
            Ok(())
        },
        |error: BevyError, ctx: ErrorContext| {
            error!(?error, ?context = ctx);
        },
    );
}
```

`queue_silenced` to drop the error silently:

```rust
commands.queue_silenced(/* ... */);
```

`EntityCommands` errors automatically when the target entity is despawned before the command runs — it returns an entity-doesn't-exist error to the global handler. Most of the time this is what you want; if you need to suppress it, use `queue_silenced` or check the entity's existence first.

## Combinator semantics (0.18)

System combinators (`a.or(b)`, `a.and(b)`, `a.xor(b)`, etc.) used to propagate errors. In 0.18, they treat a failed combined system as `false`:

```rust
// 0.17
fails_validation.or(always_true)  // returned Err if fails_validation failed

// 0.18
fails_validation.or(always_true)  // fails_validation = false, always_true = true → returns true
```

This is more useful for run conditions where "the param isn't available, skip it" should mean "this condition is false," not "the entire system fails."

## When to use what

| Situation | Tool |
| --- | --- |
| Truly impossible failure (or a bug) | `unwrap` |
| Test assertions | `assert!`, `assert_eq!`, `unwrap` |
| Resource may not be loaded yet | `Option<Res<T>>` |
| Entity may not exist yet | `Single<...>` |
| Recoverable failure with global default behavior | Return `Result` from the system |
| Recoverable failure with per-call severity | `.with_severity(...)?` or `.map_severity(...)?` |
| Recoverable failure with custom logic | `match` / `let-else` / `if-let` in the system body |
| Recoverable failure with custom handler for one system | `system.pipe(handler)` |
| Production build error policy | `app.set_error_handler(warn)` (gated by feature flag) |
| Custom error types | `thiserror` + Bevy `Result`'s blanket `From` |
