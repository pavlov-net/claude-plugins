# Error handling

## Contents
- When to panic — almost never; useful Clippy lints
- Result-returning systems — `Result<(), BevyError>`, `?` with `thiserror`
- Attaching context / ad-hoc errors (0.20) — `.context` / `.with_context`, `bevy_error!` / `bail!` / `ensure!`
- Configuring the global handler — presets, dev/release feature gating
- Per-call severity — `with_severity`, `map_severity`
- Recoverable failures via match / let-else / if-let
- Fallible system params — `Single<...>`, `Option<Res<T>>` for silent skip
- Piping handlers — `system.pipe(handler)` for per-system logic
- Errors in commands — `queue_handled`, `queue_silenced`
- Combinator semantics — `and`/`or` removed (0.20); `and_then`/`or_else` treat a failed param as `false`
- When to use what — decision table

Bevy distinguishes three failure tiers: panic (fatal by default — but catchable as of 0.20, see below), `Err` from a system (recoverable, routed to a global handler), and silent skip (fallible system params).

## When to panic

Almost never in application code. `unwrap`, `expect`, and `panic!` should be reserved for:

- Tests (`assert!`, `assert_eq!` panic on failure — that's the point).
- Unsafe code maintaining safety invariants.
- Genuinely unrecoverable bugs that indicate the program is in a broken state and continuing would do more harm than crashing.

If you find yourself reaching for `unwrap` because "this can't fail," ask whether the failure mode is *truly* impossible or just unlikely. Unlikely failure modes find their way to production eventually.

As of **0.20**, a panic inside a system, run condition, or command is caught, converted to a `BevyError` with `Severity::Panic` carrying the panic payload, and routed through the same `FallbackErrorHandler` rather than tearing down the whole app. The default handler resumes the unwind with the original payload (so dev behaviour is unchanged), but a logging handler now keeps a long-running process (editor, installation) alive through a panicking system. Observers have no catch of their own — an observer panic is caught by the enclosing system or command and reported against that. One-shot `world.run_system*` is not covered. A custom handler can inspect the payload with `error.take_payload()`. `unwrap`/`panic!` still signal "broken state," so the guidance above stands.

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

Bevy controls system execution, so it controls what happens when a system returns `Err`. The default global handler is `match_severity`: it dispatches on the error's `Severity`, and every `BevyError` defaults to `Severity::Panic` — so an unannotated `Err` **panics**, loud and helpful during development.

### Attach context (0.20)

`ContextExt` (in the prelude) adds anyhow-style `.context(msg)` / `.with_context(|| msg)` to any `Result<T, E: Into<BevyError>>` **and** to `Option<T>`, both yielding `Result<T, BevyError>`:

```rust
fn load_save(paths: Res<SavePaths>, active: Res<ActiveSlot>) -> Result {
    let path = paths.by_slot.get(&active.0)
        .context("active save slot has no path")?;              // Option -> Result
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    // ...
    Ok(())
}
```

One context prints as `msg: inner error`; two or more print the newest message then a `Caused by:` chain down to the root. Downcasting to the original error type still works on the `Result` path (on the `Option` path the message *is* the error). Prefer this over `unwrap`/`expect` in systems — you keep the message *and* the handler's severity policy.

### Ad-hoc errors (0.20)

`bevy_error!`, `bail!` (return `Err` early) and `ensure!` build a `BevyError` from a string literal. They are `#[macro_export]`ed at the crate root, **not** in the prelude: `use bevy::ecs::{bail, bevy_error, ensure};` (`Severity` and `Result` are in the prelude). Severity is optional and defaults to `Severity::Panic`; it goes first for `bevy_error!`/`bail!`, after the condition for `ensure!`:

```rust
ensure!(score >= 0, "score must not be negative: {}", score);
ensure!(score <= 100, Severity::Warning, "score too high: {}", score);
if val == 0 { bail!("value can't be zero"); }
```

**Pitfall:** with a lone string literal these macros do **not** call `format!`, so `bail!("bad {x}")` emits the text `bad {x}` verbatim. Always pass at least one positional argument: `bail!("bad {}", x)`.

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

- `match_severity` (**default**) — dispatch on `err.severity()`: `Ignore` drops, `Trace`…`Error` log at that level, `Panic` panics. The only preset that honours `.with_severity(...)` / `.map_severity(...)` / `.ignore()`.
- `panic` — always `panic!`, ignoring severity (it reads severity only to resume the original unwind of a caught panic).
- `error`, `warn`, `info`, `debug`, `trace` — always log at that level, ignoring severity.
- `ignore` — drop the error silently.

Pattern: keep the default in dev, log in release:

```rust
// Debug: leave the default `match_severity` so per-call severity still works.
#[cfg(not(debug_assertions))]
app.set_error_handler(warn);
```

**Pitfall:** installing any preset other than `match_severity` — including `panic` — makes `.with_severity(...)` / `.map_severity(...)` / `.ignore()` inert; every error takes the preset's action regardless of severity. `set_error_handler` also asserts if called more than once on an `App`.

**Library plugins must never call `set_error_handler`.** It's an application-level policy. A library that overrides it surprises every consumer.

(0.19 renamed the handler-of-last-resort resource `DefaultErrorHandler` → `FallbackErrorHandler` and the method `default_error_handler` → `fallback_error_handler`; the deprecated `DefaultErrorHandler` alias was removed in 0.20. `App::set_error_handler` is unchanged.)

## Per-call severity

When most errors should panic but one specific call site should warn (or vice versa), override at the `?`:

```rust
use bevy::prelude::*;

fn lookup(query: Query<&Camera>) -> Result {
    let camera = query.single().with_severity(Severity::Warning)?;
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

`Severity` variants: `Panic`, `Error`, `Warning`, `Info`, `Debug`, `Trace`, `Ignore`. (The variant is `Warning`; the global-handler preset *function* is `warn`.)

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

`Res<T>` (without `Option`) fails param validation when `T` isn't inserted, and that failure carries `Severity::Panic` — so it panics under the default `match_severity` handler but only logs, and skips the system for that tick, under `warn`. Use bare `Res<T>` when you've gated the system behind `run_if(resource_exists::<T>)` (or, 0.20, `resource_exists_and(|r: &T| …)`) so the failure is impossible; if the resource legitimately may be absent, use `Option<Res<T>>` or `If<Res<T>>` to skip silently.

If you're writing a custom `SystemParam` that may fail validation: in 0.19 the separate `validate_param` method was **removed** and folded into `get_param`, which now returns `Result<Self::Item, SystemParamValidationError>`. Do your validation there and return the error instead of the item:

```rust
unsafe impl SystemParam for MyParam<'_> {
    // ...
    unsafe fn get_param<'w, 's>(/* ... */)
        -> Result<Self::Item<'w, 's>, SystemParamValidationError>
    {
        if !is_valid(state, world) {
            return Err(SystemParamValidationError::skipped::<Self>("not ready"));
        }
        Ok(MyParam { /* ... */ })
    }
}
```

Use `SystemParamValidationError::skipped::<Self>(msg)` to silently skip the system (like `Single` with no match) or `::invalid::<Self>(msg)` to route to the error handler. The validation now happens as part of fetching the data, so `SystemState::get`/`get_mut` also return a `Result` you must `.unwrap()` or handle.

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
        |error: BevyError, context: ErrorContext| {
            error!(?error, ?context);
        },
    );
}
```

`queue_silenced` to drop the error silently:

```rust
commands.queue_silenced(/* ... */);
```

`EntityCommands` errors automatically when the target entity is despawned before the command runs — it returns an entity-doesn't-exist error to the global handler. Most of the time this is what you want; if you need to suppress it, use `queue_silenced` or check the entity's existence first.

## Combinator semantics

Run-condition combinators treat a combined condition whose params fail validation as `false` rather than propagating the error (0.18+).

**(0.20) the `and`/`or`/`nand`/`nor` methods are gone** (deprecated in 0.19, removed in 0.20, and no migration guide mentions it). Pick short-circuit or eager explicitly. `xor`/`xnor` are unchanged.

| Removed in 0.20 | Short-circuit | Eager |
| --- | --- | --- |
| `a.and(b)` | `a.and_then(b)` | `a.and_eager(b)` |
| `a.or(b)` | `a.or_else(b)` | `a.or_eager(b)` |
| `a.nand(b)` | `a.nand_then(b)` | `a.nand_eager(b)` |
| `a.nor(b)` | `a.nor_else(b)` | `a.nor_eager(b)` |

```rust
// `fails_validation` evaluates to false, `always_true` to true → the condition is true
system.run_if(fails_validation.or_else(always_true))
```

Short-circuit variants skip the second condition when the first decides the result, so a `Local`-, `MessageReader`- or change-detection-based second condition only observes what accumulated since it last ran. Use `*_eager` when both must run every evaluation (e.g. `state_changed::<A>.or_eager(state_changed::<B>)`).

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
| Production build error policy | `#[cfg(not(debug_assertions))] app.set_error_handler(warn)`; leave the `match_severity` default in debug |
| Custom error types | `thiserror` + Bevy `Result`'s blanket `From` |
| Add a human-readable message / turn `Option` into an error | `.context("…")?` / `.with_context(\|\| …)?` (0.20) |
