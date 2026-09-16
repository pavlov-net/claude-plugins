# Scheduling, states, and time

## Contents
- The frame — schedule order: `First` → `PreUpdate` → `StateTransition` → `FixedMain` → `Update` → `PostUpdate` → `Last`
- Where to put systems — picking the right schedule for input/logic/animation/cleanup
- System ordering — sets, `.chain()`, `.before`/`.after`, weak ordering (0.20)
- Run conditions — skip-the-system semantics, common conditions
- Fallible system params — `Single<...>`, `Option<Res<T>>` for silent skip
- Returning Result from systems — `BevyError`, severity, global handler
- States — top-level, sub-states, computed states, `OnEnter`/`OnExit`, `DespawnOnExit`
- Time — `Real`, `Virtual`, `Fixed`; fixed timestep + interpolation; timers; `DelayedCommands`

## The frame

Each frame, Bevy runs the `Main` schedule, which runs these schedules in order:

1. **`First`** — runs at the very start of the frame. Bevy uses this for time-system update, message-buffer update.
2. **`PreUpdate`** — application code that prepares state for `Update`. Input gathering, clock advancement, asset state updates.
3. **`StateTransition`** — applies queued state changes (`OnExit` → `OnTransition` → `OnEnter`).
4. **`RunFixedMainLoop`** — runs the fixed-time loop zero or more times (see Time below). Each iteration: `FixedFirst` → `FixedPreUpdate` → `FixedUpdate` → `FixedPostUpdate` → `FixedLast`.
5. **`Update`** — application logic (the main game tick).
6. **`PostUpdate`** — application code that consumes `Update` output. Animation, transform propagation, render extraction.
7. **`Last`** — final logic before the frame ends.

Plus startup-only schedules that run once before the game loop begins:

- **`PreStartup`** — library setup that must precede application setup.
- **`Startup`** — application setup.
- **`PostStartup`** — application cleanup/finalization of startup.

`StateTransition` also runs once at app startup, *before* `PreStartup` (startup order: `StateTransition` → `PreStartup` → `Startup` → `PostStartup`), so the initial state's `OnEnter` systems run before any startup schedule. To run something ahead of the initial `OnEnter`, insert a custom startup schedule with `MainScheduleOrder::insert_startup_before(StateTransition, MySchedule)`.

## Where to put systems

- **`Startup`** — one-time spawning, asset preloading, resource initialization that needs `Commands`.
- **`OnEnter(State::X)`** — setup tied to entering a specific state (spawn the level, show the menu). Despawn happens on exit; pair with `DespawnOnExit(State::X)` on entities that should not survive the state.
- **`OnExit(State::X)`** — cleanup leaving a state.
- **`Update`** — gameplay logic. ~95% of your systems live here.
- **`PreUpdate`** — input processing, clock ticks, anything that updates "current state" before `Update` reads it.
- **`PostUpdate`** — animation drivers, LOD swaps, derived-component updates that read what `Update` just wrote.
- **`FixedUpdate`** — physics, networking, anything that needs deterministic timestep.
- **`Last`** — diagnostics, save game, anything that needs to see the final state of the frame. (0.20) For save-on-quit, put the system in `bevy::app::OnAppExitSystems`, which `WindowPlugin` orders after the window-close exit systems. The set carries no run condition of its own: `app.add_systems(Last, save_on_quit.in_set(OnAppExitSystems).run_if(on_message::<AppExit>));`.

Don't put input/clock work in `Update` — it'll be one frame late for systems running before yours. Don't put animation in `Update` — it'll either flicker or fight transform propagation. The pre/update/post split exists for a reason; respect it.

## System ordering

Default: systems within a schedule run in parallel. Bevy orders them automatically based on the data they access — non-conflicting systems can run simultaneously, conflicting systems are serialized.

When you need explicit order, the canonical pattern is system sets:

```rust
#[derive(SystemSet, Clone, Eq, PartialEq, Hash, Debug)]
enum AppSet { Input, Logic, Visual }

app.configure_sets(Update, (AppSet::Input, AppSet::Logic, AppSet::Visual).chain());
app.add_systems(Update, gather_input.in_set(AppSet::Input));
app.add_systems(Update, update_state.in_set(AppSet::Logic));
app.add_systems(Update, update_hud.in_set(AppSet::Visual));
```

For tight local sequences inside a single `add_systems` call, `.chain()` is fine:

```rust
app.add_systems(Update, (write_message, read_message).chain());
```

For inter-system explicit edges (less preferred — fragile), `.before(other)` / `.after(other)`:

```rust
app.add_systems(Update, my_system.after(some_dependency));
```

Prefer named system sets. They survive refactors that move or rename functions.

**Weak ordering (0.20).** `.chain_weak()` (and `.before_weak(set)` / `.after_weak(set)`) requests the same ordering as `.chain()`/`.before`/`.after`, but the scheduler keeps an edge only between systems whose tracked ECS accesses actually conflict — run conditions on the system and on its sets count toward that access. A system that produces deferred effects (`Commands`) and any exclusive system always count as conflicting, so sync points survive; and a conflicting pair reachable only through a non-conflicting middle system stays ordered. A strict `.before`/`.after`/`.chain()` on the same pair always wins.

Use it on wide pipeline-style set chains where `.chain()` leaves threads idle. Keep `.chain()` when systems communicate through anything the scheduler can't see: channels, atomics, interior mutability behind `Res<T>`, statics, thread-locals. (`NonSend<T>`/`NonSendMut<T>` *are* tracked.)

**Pitfall:** Bevy's own `PostUpdate` `UiSystems` chain, the `Core2d`/`Core3d` pass sets, and the render-world `RenderSystems` / `RenderGraphSystems` / `ExtractSchedule` sets are now weakly ordered, so a system dropped into one of them can see stale state from a set it thought had finished. See `references/pitfalls.md`.

**Shake out accidental ordering (0.20).** With the opt-in `debug` cargo feature (also pulled in by the `dev` collection), `ScheduleBuildSettings` gains `shuffle_seed: Option<u64>`. See `references/testing.md` for the recipe.

## Run conditions

Run conditions skip a system entirely when they return `false` — the system isn't dispatched, no parallelism cost, no execution overhead beyond the condition check.

```rust
app.add_systems(Update, update_enemies.run_if(in_state(GameState::Playing)));
```

Bevy ships many common conditions (search docs.rs for `common_conditions`):

- `in_state(S)` / `not(in_state(S))`
- `state_changed::<S>` / `state_exists::<S>`
- `resource_exists::<R>` / `resource_changed::<R>`
- `resource_exists_and(|r: &R| ...)` (0.20) — false if `R` is missing, else runs the predicate on `&R`; replaces the `resource_exists::<R>.and_then(...)` two-step
- `on_message::<M>` (skip unless any messages of type `M` were written; it's a plain system fn — no parentheses)
- `on_timer(Duration::from_secs(N))`
- `input_just_pressed(KeyCode::Space)`

Compose with AND (chain `.run_if`) or boolean ops:

```rust
app.add_systems(Update, my_system
    .run_if(in_state(GameState::Playing).and_then(resource_exists::<Player>)));
```

`.run_if(condition_a).run_if(condition_b)` is AND-only and evaluates *every* condition — equivalent to `a.and_eager(b)`, not `a.and_then(b)`. **(0.20)** the bare `and`/`or`/`nand`/`nor` methods are removed in favour of a short-circuiting form and an eager twin apiece (`and_then`/`and_eager`, and so on — `references/errors.md` has the table; `xor`/`xnor` always evaluate both). Use `_then`/`_else` when the second condition would fail validation on its own (`resource_exists::<R>.and_then(resource_equals(R(0)))`); prefer `_eager` when it carries change-detection, `Local` or `MessageReader` state, since short-circuiting means it only sees what changed since the last time it actually ran. To negate, wrap with the free function `not(condition)` from `common_conditions` — `not` is a function, not a trait method.

Run conditions can be applied to entire system sets:

```rust
app.configure_sets(Update, AppSet::Combat.run_if(in_state(GameState::Playing)));
```

Useful when many systems share a gating condition.

## Fallible system params

Some system params (like `Single<...>`) can fail. By default, Bevy panics; you can configure a fallible param to skip the system instead.

`Single<T, F>` succeeds only when exactly one entity matches the query. Otherwise the system is silently skipped:

```rust
fn move_player(player: Single<&mut Transform, With<Player>>) {
    player.translation.x += 1.0;
}
```

If there's no player (zero matches) or somehow multiple players, the system doesn't run, no error, no panic. This is what you want for "the player might not exist yet (asset still loading) and that's fine."

**0.19 performance note.** Under the multithreaded executor, param validation is no longer a cheap pre-pass — a task is now spawned even for a system that ends up skipped. For most systems that's a net win, but a system that is *frequently* skipped (commonly a `Single` that usually has no match) now pays task-spawn overhead each tick. If such a system profiles as hot, gate it with a cheap run condition (e.g. `.run_if(in_state(...))` or `.run_if(resource_exists::<T>)`) to recover the old no-dispatch skip. Correctness of the `Single` guidance above is unchanged — this is tuning only.

For "the resource might not exist," wrap in `Option`:

```rust
fn read_settings(settings: Option<Res<AudioSettings>>) {
    let Some(settings) = settings else { return };
    // ...
}
```

The system runs but you handle absence in the body. (`Res<T>` panics if `T` isn't inserted.)

## Returning Result from systems

Recoverable failures can be returned to a global error handler:

```rust
fn camera_log(query: Query<&Camera>) -> Result {
    let camera = query.single()?;
    info!(?camera);
    Ok(())
}
```

Return type `Result` is `Result<(), BevyError>`. Bevy's `BevyError` has a blanket `From` for any `Error` impl, so `?` works on most error types.

The default handler is `match_severity`, and errors default to `Severity::Panic`, so an unannotated `Err` panics. Configure with `app.set_error_handler(warn)` for a warn-level log instead. Other presets: `error`, `info`, `debug`, `trace`, `ignore`. **(0.20)** panics in systems, run conditions and commands are routed through the same handler — see `references/errors.md`.

Per-error-call severity:

```rust
let camera = query.single().with_severity(Severity::Debug)?;
```

Or by error variant:

```rust
let camera = query.single().map_severity(|e| match e {
    QuerySingleError::NoEntities(_) => Severity::Ignore,
    QuerySingleError::MultipleEntities(_) => Severity::Error,
})?;
```

Library plugins must never call `set_error_handler` — that's an application policy decision.

## States

Define a state machine:

```rust
#[derive(States, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum AppState {
    #[default]
    Loading,
    MainMenu,
    InGame,
    GameOver,
}
```

Register:

```rust
app.init_state::<AppState>();
```

Transition:

```rust
fn start_game(mut next: ResMut<NextState<AppState>>) {
    next.set(AppState::InGame);
}
```

The transition isn't immediate. `NextState` is queued and applied when the `StateTransition` schedule runs (after `PreUpdate`, before `FixedMain` and `Update`). Systems running later in the same frame still see the old state.

In 0.18, `next_state.set(X)` re-fires `OnEnter`/`OnExit` even if the state was already `X`. **In 0.19 this consistency was extended to `DespawnOnEnter`/`DespawnOnExit`** — they now also trigger on same-state transitions (a bug where they didn't was fixed). If you want the old "skip if equal" behavior (no transition schedules, no scoped despawns):

```rust
next_state.set_if_different(AppState::InGame);   // (0.20) was set_if_neq
```

Renamed in 0.20 so it stops colliding with the unrelated `Mut::set_if_neq` (which is unchanged); no alias ships, so `NextState::set_if_neq` is a hard compile error. Command form: `commands.set_state_if_different(X)`.

Setup/teardown:

```rust
app.add_systems(OnEnter(AppState::InGame), spawn_world);
app.add_systems(OnExit(AppState::InGame), save_progress);
```

For "exactly when transitioning from A to B":

```rust
app.add_systems(OnTransition { from: AppState::Loading, to: AppState::MainMenu }, fade_in);
```

Read the current state in any system:

```rust
fn debug_state(state: Res<State<AppState>>) {
    info!("Current: {:?}", *state.get());
}
```

Skip systems based on state — the most common usage:

```rust
app.add_systems(Update, gameplay.run_if(in_state(AppState::InGame)));
```

### Despawn-on-exit components

Tie an entity's lifetime to a state:

```rust
commands.spawn((Hud, DespawnOnExit(AppState::InGame)));
```

When the state exits, the entity (and its `linked_spawn` children) despawns automatically. No manual cleanup system. Variants:

- `DespawnOnEnter(State::X)` — despawn when entering.
- `DespawnWhen(condition)` — despawn when an arbitrary state predicate matches.

This pattern eliminates a huge class of "I forgot to clean up" bugs. Use it for menus, HUD elements, level entities — anything tied to a specific state. (For the 0.19 same-state-transition behavior, see the `set_if_different` note above.)

### Sub-states

States that only exist while another state is active:

```rust
#[derive(SubStates, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[source(AppState = AppState::InGame)]
pub enum InGameSubState {
    #[default]
    Playing,
    Paused,
    Cutscene,
}

app.add_sub_state::<InGameSubState>();
```

The sub-state only exists while `AppState::InGame` is active. Transitioning out of `InGame` removes the sub-state and runs its `OnExit`. Re-entering `InGame` restores the default sub-state and runs its `OnEnter`.

### Computed states

States derived from other states with no setter:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InGame;

impl ComputedStates for InGame {
    type SourceStates = AppState;
    fn compute(sources: AppState) -> Option<Self> {
        match sources {
            AppState::InGame | AppState::GameOver => Some(InGame),
            _ => None,
        }
    }
}

app.add_computed_state::<InGame>();
```

`compute` returns `Some(value)` to indicate the state is active, `None` to indicate it doesn't exist. Computed states automatically appear/disappear when their source states change.

Use computed states when you have "is the game in any of these states" logic that gets repeated:

```rust
app.add_systems(Update, (update_hud, draw_score).run_if(in_state(InGame)));
```

If you later add `AppState::Cutscene` and want the HUD active there too, only `compute` changes — every consumer keeps working.

Computed states are read-only — you can't set them via `NextState`. Use sub-states if you need a settable derived state.

## Time

Three flavors:

- **`Time<Real>`** — wall clock. Ignores pause and time scaling. Use for UI animations that shouldn't pause when the game does.
- **`Time<Virtual>`** — in-game time. Pausable, scalable. Use for gameplay that should pause when the player pauses.
- **`Time<Fixed>`** — fixed-timestep clock. Advances in discrete steps inside the fixed loop.

`Res<Time>` (no generic) automatically picks the right flavor for the current schedule:

- In `Main` (and its children: `Update`, `PostUpdate`, etc.), `Time` aliases `Time<Virtual>`.
- In `FixedMain` (and `FixedUpdate` etc.), `Time` aliases `Time<Fixed>`.

So in 90% of code, `Res<Time>` "just works":

```rust
fn move_player(time: Res<Time>, mut q: Query<&mut Transform, With<Player>>) {
    for mut t in &mut q {
        t.translation.x += SPEED * time.delta_secs();
    }
}
```

For specific flavors, use the explicit generic:

```rust
fn ui_animation(time: Res<Time<Real>>) { /* doesn't pause */ }
fn pause_world(mut time: ResMut<Time<Virtual>>) { time.pause(); }
fn slow_motion(mut time: ResMut<Time<Virtual>>) { time.set_relative_speed(0.5); }
```

`time.delta()` returns a `Duration`; `time.delta_secs()` returns `f32`; `time.elapsed_secs()` is total elapsed time since startup.

### Fixed timestep

For physics, networking, deterministic simulation. The fixed loop runs zero or more times per frame: each frame, virtual time advances and adds to a budget; while the budget exceeds the fixed timestep, the fixed loop runs once and decrements the budget.

Set the fixed timestep:

```rust
app.insert_resource(Time::<Fixed>::from_seconds(1.0 / 60.0));
```

Put deterministic systems in `FixedUpdate`:

```rust
app.add_systems(FixedUpdate, (apply_velocity, integrate_physics).chain());
```

Inside `FixedUpdate`, `Res<Time>` is `Time<Fixed>` automatically. `time.delta_secs()` returns the fixed timestep, not the variable frame delta.

Bevy supports a single global fixed timestep. For "every 5 seconds, do X" behavior, don't change the fixed timestep — use a `Timer` or the `on_timer` run condition.

### Visual interpolation

A fixed timestep means rendering may happen between fixed updates, which causes visible jitter unless you interpolate visual transforms between fixed steps. The pattern:

1. Maintain a separate "logical position" component (your physics owns this).
2. Each frame after fixed updates run, interpolate the visual `Transform` between previous and current logical positions based on the fractional progress through the next fixed step.

Put the interpolation system at `RunFixedMainLoopSystems::AfterFixedMainLoop`. See the `physics_in_fixed_timestep` example.

There are also community crates that handle this for you (search "bevy interpolation").

### Timers

`Timer` is a small struct with a duration and elapsed time. It does not tick itself.

```rust
#[derive(Component)]
struct Cooldown {
    timer: Timer,
}

fn tick_cooldowns(time: Res<Time>, mut q: Query<&mut Cooldown>) {
    for mut c in &mut q {
        c.timer.tick(time.delta());
    }
}

fn check_cooldown(q: Query<&Cooldown>) {
    for c in &q {
        if c.timer.is_finished() { /* ... */ }
    }
}
```

Modes: `TimerMode::Once` (finishes once) and `TimerMode::Repeating` (auto-resets when finished).

`timer.just_finished()` is true on the tick where it crossed the finish line — useful for "fire once when the timer completes":

```rust
fn auto_attack(time: Res<Time>, mut q: Query<(&mut AttackTimer, &Damage)>) {
    for (mut t, dmg) in &mut q {
        if t.0.tick(time.delta()).just_finished() {
            // Fire the attack
        }
    }
}
```

For "run a system every N seconds":

```rust
app.add_systems(Update, slow_thing.run_if(on_timer(Duration::from_secs(5))));
```

`on_timer` uses `Res<Time>` internally, so it adapts to virtual/fixed automatically.

`Stopwatch` is the related "elapsed time, no duration" type for things that need to count up but not finish.

### DelayedCommands (0.19)

For "do this thing in 2 seconds":

```rust
fn schedule_explosion(mut commands: Commands, target: Entity) {
    commands.delayed().secs(2.0).entity(target).insert(Exploding);
}
```

`DelayedCommands` is a wrapper over `Commands` that buffers operations with a delay. Bevy ticks the delay buffer in `PreUpdate` automatically. Cleaner than spawning a one-shot timer entity for fire-and-forget delayed actions.

Use `.duration(Duration::from_millis(500))` for sub-second precision, `.secs(N)` for whole seconds.
