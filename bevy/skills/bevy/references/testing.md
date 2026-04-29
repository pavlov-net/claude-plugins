# Testing

## Contents
- Pure method tests — fastest, no `World`
- Raw `World` tests — for setup helpers and component-presence checks
- `World::run_system_once` — test individual systems in isolation
- `Schedule` for system ordering — multi-system interaction tests
- `App::update()` for plugin tests — closest to "running the game"
- Mocking input — write input messages directly; prefer action-layer tests
- Headless setup — feature flag for `DefaultPlugins`-disabled CI
- Visual regression tests — when to invest, when to skip
- Doc tests — better for libraries than for games
- Architecture for testability — keep gameplay decoupled from rendering
- What not to test — heuristics for skipping

Games are notoriously hard to test. Don't let that stop you from testing what *can* be tested cheaply. The cost/benefit lens: spend testing effort proportional to how often the code changes and how subtle its bugs are.

The testing ladder, cheapest to most expensive:

1. Pure method tests (no `World`).
2. Raw `World` for setup helpers.
3. `World::run_system_once` for individual systems.
4. `Schedule` for system ordering.
5. `App::update()` for plugin-level integration.
6. Visual regression / smoke tests (rare).

Use the cheapest tier that gives you confidence.

## Pure method tests

If the logic is on a method, test the method directly. No `World`, no ECS:

```rust
#[derive(Component)]
struct Health {
    current: u32,
    max: u32,
}

impl Health {
    fn heal(&mut self, amount: u32) {
        self.current = (self.current + amount).min(self.max);
    }

    fn is_alive(&self) -> bool {
        self.current > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healing_clamps_to_max() {
        let mut h = Health { current: 90, max: 100 };
        h.heal(50);
        assert_eq!(h.current, 100);
    }

    #[test]
    fn zero_health_is_dead() {
        let h = Health { current: 0, max: 100 };
        assert!(!h.is_alive());
    }
}
```

These run in milliseconds, never flake, and have zero dependency on Bevy plumbing. Always your first choice.

## Raw `World` tests

For setup helpers and simple "does X end up in the world" checks:

```rust
fn spawn_enemy(world: &mut World, hp: u32) -> Entity {
    world.spawn((Enemy, Health::new(hp))).id()
}

#[test]
fn enemies_spawn_at_full_hp() {
    let mut world = World::new();
    let goblin = spawn_enemy(&mut world, 20);
    let dragon = spawn_enemy(&mut world, 500);

    assert_eq!(world.get::<Health>(goblin).unwrap().current, 20);
    assert_eq!(world.get::<Health>(dragon).unwrap().current, 500);
}
```

Useful `World` methods:

- `world.spawn(bundle).id()` — spawn and get the entity ID.
- `world.get::<T>(entity)` / `world.get_mut::<T>(entity)` — read/write a component.
- `world.resource::<T>()` / `world.resource_mut::<T>()` — read/write a resource.
- `world.query::<&T>()` — get a query (then `.iter(&world)` or `.single(&world)`).
- `world.write_message(msg)` — write a message.
- `world.trigger(event)` — trigger an observer event.
- `world.contains_resource::<T>()` — check for a resource without panicking.

Direct world tests work well for testing helper functions that take `&mut World`. They get awkward when the borrow checker fights you across multiple component reads/writes — that's the cue to escalate to `run_system_once`.

## `World::run_system_once`

Run a system once against a constructed world:

```rust
fn apply_poison(
    mut query: Query<&mut Health, With<Poisoned>>,
    strength: Res<PoisonStrength>,
) {
    for mut h in &mut query {
        h.current = h.current.saturating_sub(strength.0);
    }
}

#[test]
fn poison_only_hurts_poisoned() {
    let mut world = World::new();
    world.insert_resource(PoisonStrength(5));
    let poisoned = world.spawn((Health::new(100), Poisoned)).id();
    let healthy = world.spawn(Health::new(100)).id();

    world.run_system_once(apply_poison).unwrap();

    assert_eq!(world.get::<Health>(poisoned).unwrap().current, 95);
    assert_eq!(world.get::<Health>(healthy).unwrap().current, 100);
}
```

Returns a `Result` — `unwrap` in tests, since the system either runs or it doesn't (missing resource etc. produces a clear test failure).

`run_system_once_with(my_system, input_value)` for systems that take `In<T>` input.

`Commands` queued during the system are flushed automatically before `run_system_once` returns.

**Caveat**: the system is created fresh on each call. `Local<T>` resets every time. `Added<T>`/`Changed<T>` filters won't behave the way they would in a real schedule (everything looks "newly added"). For change-detection tests, escalate to a `Schedule`.

## `Schedule` for system ordering

When the behavior depends on the *interaction* between systems:

```rust
fn regenerate(mut query: Query<&mut Health>) {
    for mut h in &mut query {
        h.heal(1);
    }
}

#[test]
fn poison_runs_before_regen() {
    let mut world = World::new();
    world.insert_resource(PoisonStrength(5));
    world.spawn((Health::new(100), Poisoned));

    let mut schedule = Schedule::default();
    schedule.add_systems((apply_poison, regenerate).chain());
    schedule.run(&mut world);

    let h = world.query::<&Health>().single(&world);
    // poison: 100 - 5 = 95, then regen: 95 + 1 = 96
    assert_eq!(h.current, 96);
}
```

`schedule.run(&mut world)` runs the schedule once. Loop calls if you need multiple ticks.

This is the right tier for change-detection tests, run-condition tests, and anything that depends on system ordering.

Schedule-level tests are more brittle than `run_system_once` tests because they have more moving parts. Save them for when the *interaction* is the thing under test.

## `App::update()` for plugin tests

The closest you can get to "running the game" without actually running it:

```rust
#[test]
fn combat_plugin_applies_poison() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, CombatPlugin));

    let entity = app.world_mut().spawn((Health::new(100), Poisoned)).id();
    app.update();

    // CombatPlugin registered PoisonStrength(5) and the apply_poison system
    let h = app.world().get::<Health>(entity).unwrap();
    assert_eq!(h.current, 95);
}
```

`app.update()` runs one full frame: `Startup` (on first call), then `PreUpdate` → `StateTransition` → `FixedUpdate` → `Update` → `PostUpdate` → `Last`.

`MinimalPlugins` is the lightweight default — schedules, time, but no rendering or windowing. Use it for headless tests. Add your specific plugins on top.

This tier verifies that your plugin wires everything up correctly: resources are initialized, observers are registered, schedules contain the right systems.

The cost: tests fail when irrelevant code in your plugin changes. A refactor that splits one system into two passes, or moves a system from `Update` to `PostUpdate`, may break tests that didn't actually depend on that detail. Use sparingly, on critical paths.

### Loop-with-cap pattern

For "this should eventually happen" tests, loop `update()` calls but cap the iteration count:

```rust
#[test]
fn poisoned_creature_eventually_dies() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, CombatPlugin));
    let e = app.world_mut().spawn((Health::new(30), Poisoned)).id();

    // 30 hp at 5 dmg/tick = 6 ticks; cap at 20 for margin
    for _ in 0..20 {
        app.update();
        if app.world().get::<Health>(e).unwrap().current == 0 {
            return;
        }
    }
    panic!("creature did not die within 20 ticks");
}
```

Without the cap, a bug turns the test into an infinite loop. Pick a cap with margin over the expected case but not so large that hangs waste minutes of your time.

## Mocking input

Bevy's input runs through messages. To simulate an input, write the message:

```rust
#[test]
fn space_triggers_jump() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, JumpPlugin));

    // Simulate a Space key press by writing the input message.
    use bevy::input::keyboard::{KeyboardInput, KeyCode};
    use bevy::input::ButtonState;
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Space,
        logical_key: bevy::input::keyboard::Key::Space,
        state: ButtonState::Pressed,
        repeat: false,
        window: Entity::PLACEHOLDER,
        text: None,
    });

    app.update();

    let player = app.world().query_filtered::<&Velocity, With<Player>>().single(app.world());
    assert!(player.0.y > 0.0);
}
```

Better: test at the action layer, not the input layer. Mock a `JumpEvent` directly rather than synthesizing keyboard input. The further from raw input, the more robust the test.

## Headless setup

Most CI runners don't have GPUs. To run any rendering-touching code, either avoid rendering plugins (use `MinimalPlugins`) or set up a software renderer.

Gate `DefaultPlugins` behind a feature:

```rust
let mut app = App::new();

#[cfg(not(feature = "headless"))]
app.add_plugins(DefaultPlugins);

#[cfg(feature = "headless")]
app.add_plugins(
    DefaultPlugins.build()
        .disable::<AudioPlugin>()
        .disable::<UiRenderPlugin>()
);

app.run();
```

For tests, prefer `MinimalPlugins` over a feature-flagged `DefaultPlugins` — fewer plugins means fewer ways to fail.

If you need a real GPU in CI:

- Linux: install `mesa-vulkan-drivers` (provides Lavapipe — software Vulkan), wrap with `xvfb-run` for the virtual display.
- Windows: WARP is built in (software DX12).
- macOS: GitHub macOS runners actually have GPUs.

Force a software backend with `WGPU_BACKEND=vulkan` or `WGPU_ADAPTER_NAME=llvmpipe`.

For deterministic rendering in CI tests, lock the frame time and seeds:

```rust
app.add_plugins(DefaultPlugins.set(TimePlugin {
    /* fixed frame time */
}));
```

Use `bevy_ci_testing` (with the `bevy_ci_testing` feature) for scripted test runs that capture screenshots at specific frames and exit cleanly.

## Visual regression tests

The full visual-test setup is fiddly:

1. Capture screenshots at deterministic points.
2. Compare pixel-by-pixel against a baseline.
3. Allow some pixel deviation (anti-aliasing differs across platforms).
4. Compare per-platform, not cross-platform.

For most teams this is overkill. Smoke testing (does it run for 100 frames without crashing?) catches more regressions than you'd expect with much less infrastructure. Visual regression makes sense for projects where rendering correctness is the primary feature (rendering libraries, shader-heavy effects).

If you need it, study Bevy's own CI workflows in `.github/workflows/` and use [Pixel Eagle](https://pixel-eagle.com) or `nv-flip-rs`.

## Doc tests

In published libraries, doc tests double as usage examples and as canaries for breaking changes:

```rust
/// A health value with a maximum.
///
/// # Example
///
/// ```
/// use my_crate::Health;
/// let mut h = Health::new(100);
/// h.heal(20);
/// assert_eq!(h.current(), 100);
/// ```
struct Health { /* ... */ }
```

Doc tests run with `cargo test --doc`. They're better than unit tests for *teaching* API usage. They're worse than unit tests for *verifying correctness* — IDE support is weaker, the surrounding context (module imports, helper functions) is missing.

For game projects, doc tests are usually overkill. They're valuable for libraries you publish.

## Architecture for testability

The big win: **keep gameplay logic decoupled from rendering**. Rendering can't be tested cheaply; gameplay logic can. If your damage calculation depends on `Camera::computed.target_info`, it's coupled to rendering and you can't unit-test it without GPU.

- Game state (who has how much HP) → component or resource → testable.
- Game logic (damage formula, AI decisions) → systems → testable with `run_system_once`.
- Visualization (how the HP bar looks) → separate plugin, separate concern → not unit-tested.

Splitting pure logic into a non-Bevy crate (`cargo test -p my_logic` doesn't link Bevy) makes this easier — both ergonomically (no Bevy-graph compile time on every test) and architecturally (you literally *can't* depend on rendering from inside the pure-logic crate).

## What not to test

Don't test what the compiler already proves. Don't test trivial getters. Don't test the framework itself (Bevy already tests `Query::iter`).

Do test:

- Algorithms with edge cases (numeric overflow, off-by-one, empty input).
- Invariants that aren't enforceable by types (life ≤ max).
- Bug fixes (a regression test for every bug you fix is a good policy — they're cheap and catch the bug coming back).
- Anything you've debugged twice for the same reason.
- Subtle state machines, especially ones with concurrent transitions.

Skip:

- Code that's so simple a typo would be obvious.
- Code that changes daily (test maintenance overwhelms test value).
- Anything that requires a GPU and you don't have a cheap GPU testing setup.
