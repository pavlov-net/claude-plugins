# Communication: Event, Message, Observer

## Contents
- The split — 0.17 separated `Event` (observable) from `Message` (buffered)
- Decision matrix — when to pick each tool
- Messages — `MessageWriter`/`MessageReader`, registration, lifetime
- Events + Observers — `On<E>` parameter, `world.trigger`/`commands.trigger`
- EntityEvents — entity-targeted events, `#[event_target]`, immutability in 0.18
- Component lifecycle observers — `Add`, `Insert`, `Replace`, `Remove`, `Despawn`
- Component hooks — `#[component(on_add = ...)]` direct registration; hook vs observer
- Common patterns — hookup-on-spawn, damage batching, propagation

This is the area Bevy users get wrong most often. Three distinct tools, three distinct uses.

## The split

In 0.16 and earlier, `Event` was overloaded — the same trait covered both "fire-and-forget reactions" (handled by observers) and "buffered queues" (handled by `EventReader`/`EventWriter`). 0.17 split these:

- **`#[derive(Event)]`** is now exclusively for **observable** events: triggered explicitly, run immediately (or on command flush), routed through observers.
- **`#[derive(Message)]`** is for **buffered** communication: written to a queue, read in batches, drained over time. Uses `MessageWriter`/`MessageReader`.

A type can implement both if a use case really needs both flavors, but it's rare. Pick one.

## Decision matrix

| Question | Use |
| --- | --- |
| "When X happens, immediately do Y to a specific entity." | `EntityEvent` + observer on that entity |
| "When X happens anywhere, immediately do Y." | `Event` + global observer |
| "When component T is added/removed, do Y." | `On<Add, T>` / `On<Remove, T>` observer (or `#[component(on_add = ...)]`) |
| "Many places fire X; one place batches them." | `Message` + `MessageReader` |
| "Fire X periodically; consumers process when convenient." | `Message` |
| "Cross-frame buffering; messages may live one frame after writing." | `Message` |
| "Need backpressure or throttling on consumption." | `Message` (read in batches with `par_read`) |
| "Need event bubbling up a hierarchy." | `EntityEvent` with `#[entity_event(propagate)]` |

Heuristics for borderline cases:

- If the producer count exceeds 1 and the consumer count exceeds 1, prefer `Message`. Observers route to all matching observers, but they run synchronously and don't batch — many producers + many consumers gets expensive.
- If the action must complete before the next system runs (e.g., damage must apply before death-check), prefer an immediate observer or `world.trigger(...)` over a deferred message that won't drain until next frame.
- If the action targets one specific entity, prefer `EntityEvent` over a `Message` carrying an `Entity` field — the per-entity observer routing is faster and more expressive.

## Messages

Buffered, queued, processed in batch.

```rust
#[derive(Message)]
struct DamageDealt {
    target: Entity,
    amount: u32,
}

fn deal_damage(
    inputs: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<DamageDealt>,
    target: Single<Entity, With<Enemy>>,
) {
    if inputs.just_pressed(KeyCode::Space) {
        writer.write(DamageDealt { target: *target, amount: 10 });
    }
}

fn apply_damage(
    mut reader: MessageReader<DamageDealt>,
    mut healths: Query<&mut Health>,
) {
    for msg in reader.read() {
        if let Ok(mut h) = healths.get_mut(msg.target) {
            h.0 = h.0.saturating_sub(msg.amount);
        }
    }
}
```

Register the message with the app:

```rust
app.add_message::<DamageDealt>()
   .add_systems(Update, (deal_damage, apply_damage).chain());
```

`add_message` does two things: inserts a `Messages<DamageDealt>` resource (the queue), and adds `message_update_system` to `First`, which advances the double buffer once per frame.

Lifetime: a message is readable for one full frame after writing. Internally `Messages` keeps two buffers; each `update` call swaps them and drops the older.

If you want updates to happen at a different cadence (e.g., in `FixedUpdate` for deterministic gameplay), don't use `add_message` — manually insert `Messages::<M>::default()` and call `messages.update()` from a system in your preferred schedule.

`MessageReader::par_read()` enables parallel processing if the body is independent. `MessageMutator` is the read-and-mutate variant (mutually exclusive with other writers, like `ResMut` semantics).

## Events + Observers

Immediate, synchronous, observer-routed.

```rust
#[derive(Event)]
struct PlayerScored { points: u32 }

// Register a global observer (no entity target).
fn setup(mut commands: Commands) {
    commands.add_observer(|score: On<PlayerScored>, mut total: ResMut<TotalScore>| {
        total.0 += score.points;
    });
}

// Trigger somewhere.
fn detect_goal(mut commands: Commands /* ... */) {
    commands.trigger(PlayerScored { points: 10 });
}
```

Notes:

- The first parameter of the observer is `On<E>`, not `Trigger<E>` (renamed in 0.17). The variable name should match the event semantics — `score: On<PlayerScored>` reads naturally.
- Observers can take any system params after the `On<E>` (Queries, Resources, Commands, MessageWriter, etc.) — same as a regular system.
- `world.trigger(E { .. })` runs the event *immediately*; the observer handler executes before `trigger` returns.
- `commands.trigger(E { .. })` defers the trigger to the next command flush. Inside an observer, `commands.trigger(...)` adds to the queue and runs at flush time, recursively if needed.
- Observers can `.run_if(...)` like systems. Multiple `.run_if` calls AND together.

Events default to a `GlobalTrigger` (untargeted). They cannot be triggered with an entity target unless you derive `EntityEvent` instead.

## EntityEvents

Targeted at a specific entity. The entity is *part of* the event:

```rust
#[derive(EntityEvent)]
struct Hit {
    entity: Entity,    // event_target — implicit if the field is named `entity`
    damage: u32,
}

// Or, when you need different naming:
#[derive(EntityEvent)]
struct Attack {
    #[event_target]
    attacker: Entity,
    victim: Entity,
    weapon: WeaponKind,
}
```

Trigger:

```rust
commands.trigger(Hit { entity: target, damage: 10 });
```

Observe globally (runs for every triggered Hit):

```rust
world.add_observer(|hit: On<Hit>| {
    println!("Entity {} took {} damage", hit.entity, hit.damage);
});
```

Observe on a specific entity (runs only when that entity is the target):

```rust
commands.entity(player).observe(|hit: On<Hit>, mut hp: Query<&mut Health>| {
    if let Ok(mut h) = hp.get_mut(hit.entity) {
        h.0 = h.0.saturating_sub(hit.damage);
    }
});
```

Per-entity observers are spawned as observer entities themselves, with an `Observer` component attached to the watched entity. They despawn when the watched entity despawns.

In 0.18, `EntityEvent` is **immutable by default** — you can't mutate the target after constructing the event. Mutation moved to a separate `SetEntityEventTarget` trait, which is auto-impl'd only for propagated events.

### Propagation

By default, `EntityEvent` doesn't propagate. To opt in:

```rust
#[derive(EntityEvent)]
#[entity_event(propagate)]
struct Click {
    entity: Entity,
}
```

Propagation walks the `ChildOf` relationship by default — if you click a child, the click bubbles up to its ancestors. Stop propagation explicitly:

```rust
world.add_observer(|mut click: On<Click>| {
    if SOME_CONDITION {
        click.propagate(false);
    }
});
```

Auto-propagate (every observer continues bubbling unless explicitly stopped):

```rust
#[derive(EntityEvent)]
#[entity_event(propagate, auto_propagate)]
struct Click { entity: Entity }
```

Use a different relationship for propagation:

```rust
#[derive(EntityEvent)]
#[entity_event(propagate = &'static MyRelationship)]
struct Custom { entity: Entity }
```

This is how UI events propagate — clicks bubble from leaf widgets up to their parents until something handles them.

`On::original_event_target()` returns the entity the event was *first* triggered on, even after propagation has bubbled it up several levels. Useful when ancestors need to know which descendant initiated the event.

## Component lifecycle observers

Five lifecycle events, observable on any component:

- **`Add`** — fires when a component is added to an entity that didn't already have it.
- **`Insert`** — fires when a component is added, regardless of whether it was already there. (`Insert` is a superset of `Add`.)
- **`Replace`** — fires when a component is removed *or* replaced by a new value of the same type.
- **`Remove`** — fires when a component is removed and not replaced. Runs *before* the component is actually removed.
- **`Despawn`** — fires for each component on an entity when the entity is despawned.

Ordering: `Add` runs before `Insert`. `Replace` runs before `Remove`. `Despawn` runs last.

Observe via the `On<Lifecycle, Component>` form:

```rust
world.add_observer(|add: On<Add, Player>| {
    info!("Player spawned: {}", add.entity);
});
```

This is the canonical replacement for "poll for `Added<Player>` in `Update` and do hookup work" — observers fire immediately on the lifecycle event, you don't pay the per-frame poll cost, and you have full system-param access in the handler.

## Component hooks

Same machinery, but registered *on the component type* rather than added by a plugin:

```rust
#[derive(Component)]
#[component(on_add = log_player_spawn)]
struct Player;

fn log_player_spawn(mut world: DeferredWorld, ctx: HookContext) {
    let name = world.get::<Name>(ctx.entity).map(|n| n.0.clone());
    info!("Player {:?} spawned", name);
}
```

Or inline as a closure/path:

```rust
#[derive(Component)]
#[component(on_add = log_add("added"))]
#[component(on_remove = log_add("removed"))]
struct Tracked;

fn log_add(action: &'static str) -> impl Fn(DeferredWorld, HookContext) {
    move |_, ctx| info!("{} on {}", action, ctx.entity)
}
```

Or have a method on the type:

```rust
#[derive(Component)]
#[component(on_add)]
struct AutoInit;

impl AutoInit {
    fn on_add(world: DeferredWorld, ctx: HookContext) { /* ... */ }
}
```

### Hook vs observer

Both can react to lifecycle events. Use a **hook** when:

- The reaction is a fundamental property of the component, not a behavior some plugin opts into.
- You want it to fire even if no plugin/observer is registered.
- You need it to run before any user code in the schedule.

Use an **observer** when:

- The reaction is plugin-specific and not always wanted.
- You need the full power of a `SystemParam` (more flexibility than `DeferredWorld + HookContext`).
- You need to attach/detach the reaction at runtime (e.g., debug observers that turn on with a feature flag).

Hooks are registered exactly once per type. Observers can be added/removed dynamically and stack.

## Common patterns

**Hookup-on-spawn** (the canonical observer use case):

```rust
#[derive(Component)]
struct NeedsHookup;

commands.add_observer(|add: On<Add, NeedsHookup>, mut commands: Commands /* deps */| {
    // Do hookup work using full system params.
    commands.entity(add.entity).remove::<NeedsHookup>();
});
```

The observer fires *once*, immediately after `NeedsHookup` is added — no per-frame polling. The pattern of inserting a marker + observing its `Add` is more idiomatic than inserting and then checking `Added<...>` in `Update`.

**Damage batching** (canonical message use case):

```rust
#[derive(Message)]
struct Damage { target: Entity, amount: u32 }

// Many systems write damage messages.
fn ranged_attack(/* ... */, mut writer: MessageWriter<Damage>) { /* ... */ }
fn melee_attack(/* ... */, mut writer: MessageWriter<Damage>) { /* ... */ }
fn hazard_tick(/* ... */, mut writer: MessageWriter<Damage>) { /* ... */ }

// One system applies them all.
fn apply_damage(mut reader: MessageReader<Damage>, mut healths: Query<&mut Health>) {
    for msg in reader.read() {
        if let Ok(mut h) = healths.get_mut(msg.target) {
            h.0 = h.0.saturating_sub(msg.amount);
        }
    }
}
```

**One-shot UI animation completion** (entity event with global observer):

```rust
#[derive(EntityEvent)]
struct AnimationFinished { entity: Entity, clip: AnimationClipHandle }

// Per-entity: react to this specific avatar's animation completing.
commands.entity(avatar).observe(|done: On<AnimationFinished>, /* ... */| { /* ... */ });

// Global: react to *any* animation completing (useful for analytics).
commands.add_observer(|done: On<AnimationFinished>| { /* ... */ });
```

**Custom event triggers**: the `Event::Trigger` associated type is the extension point. The default triggers (`GlobalTrigger`, `EntityTrigger`, `PropagateEntityTrigger`, `EntityComponentsTrigger`) cover almost everything. Implement `Trigger<E>` for an exotic case (e.g., events that fan out to all entities matching some predicate). Rare in application code; common in framework code.
