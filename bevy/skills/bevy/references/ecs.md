# ECS data design

## Contents
- Entities — opaque IDs, spawn/despawn semantics
- Components — flavors (marker, newtype, named, enum), design rules
- Required components — `#[require(...)]`, replaces bundles for "always together" composition
- Queries — read/mut, multi-component, filters, optional, single-entity, multi-entity
- Change detection — `Changed<T>`, `Added<T>`, `Spawned`, `Ref<T>`, `set_if_neq`
- Resources — `Res`/`ResMut`, init patterns, optional, resources-as-components (0.19), vs singleton entities
- Local — per-system state
- Relationships — `ChildOf`/`Children`, custom relationships, naming convention
- Custom QueryData and SystemParam — derive macros for repeated patterns
- Disabling entities — `Disabled` component, default query filters

The mental model: Bevy's world is a sparse database. Entities are row IDs. Components are columns. Archetypes are tables — Bevy automatically groups entities with the same set of components into the same archetype, packing components into dense arrays for cache-friendly iteration.

## Entities

`Entity` is an opaque ID. Treat it as a stable handle to a row, not as a value with semantics. Spawn:

```rust
let e = commands.spawn((Player, Health(100), Transform::default())).id();
```

Despawn:

```rust
commands.entity(e).despawn();
```

Spawn and despawn are deferred — they happen when the command queue flushes (end of the schedule, or at an explicit sync point inserted by the scheduler). Within a single system, you cannot spawn an entity and then immediately query for it.

## Components

A component is any `#[derive(Component)]` Rust type. Several flavors:

```rust
// Marker (zero-sized): for filtering
#[derive(Component)]
struct Player;

// Newtype: for primitives that need ECS identity
#[derive(Component)]
struct Health(u32);

// Field-named struct: when invariants bind fields
#[derive(Component)]
struct LifeBar { current: u32, max: u32 }

// Enum: for mutually exclusive states
#[derive(Component)]
enum Allegiance { Friendly, Hostile }
```

Design rules:

- **Small and focused.** Separate `Health`, `Armor`, `Speed`. Bundle into one struct only when you need to maintain an invariant (e.g., `current ≤ max`) or when the fields are always read together by methods.
- **Marker components are nearly free.** Use them aggressively for filtering: `Burning`, `Frozen`, `NeedsHookup`, `Inventoried`. Adding/removing them is the cleanest way to switch behavior.
- **Methods belong on components for pure projections.** `Health::is_alive(&self) -> bool` is fine. `Health::deal_damage(&mut self, src: Entity, world: &World)` is not — that's system territory.
- **Components store IDs, not pointers.** If component A needs to reference component B on another entity, store an `Entity` (or a relationship — see below). Don't try to hold a `&B` or `Box<B>`.

## Required components

In 0.17+, declare "this component depends on these others":

```rust
#[derive(Component)]
#[require(Transform, Visibility)]
struct Bullet;
```

When you `commands.spawn(Bullet)`, Bevy inserts `Transform::default()` and `Visibility::default()` for free. Override the default:

```rust
#[derive(Component)]
#[require(Health(100), Speed(5.0))]
struct Combatant;
```

This effectively replaces the bundle pattern for "things that always go together." Tuple bundles still work for ad-hoc spawn calls — they just shouldn't be your durable composition unit.

Manual implementations of `Component` can declare requirements programmatically; the macro is the easy path.

## Queries

Read-only:

```rust
fn print_health(query: Query<&Health>) {
    for h in &query { println!("{}", h.0); }
}
```

Mutable:

```rust
fn poison(mut query: Query<&mut Health, With<Poisoned>>) {
    for mut h in &mut query { h.0 = h.0.saturating_sub(1); }
}
```

Multi-component (tuple in `D`):

```rust
Query<(&Transform, &mut Velocity, &Health)>
```

Filters (tuple in `F`):

```rust
Query<&Transform, (With<Player>, Without<Disabled>)>
Query<&Health, Or<(With<Burning>, With<Poisoned>)>>
```

Optional:

```rust
Query<(&Name, Option<&Health>)>          // Some(...) or None
Query<AnyOf<(&Health, &Mana)>>           // multiple Options bundled
Query<Has<Burning>>                       // bool
```

Single-entity queries:

```rust
fn move_player(player: Single<&mut Transform, With<Player>>) {
    // Skipped if zero or 2+ matches; no error to handle.
    player.translation.x += 1.0;
}
```

Use `Single<...>` for "there is exactly one of these" cases — the system silently skips when the count is wrong, which is usually what you want for optional gameplay objects (the player may have died and not respawned yet).

When you need to handle the wrong-count cases:

```rust
fn handle(query: Query<&Player>) -> Result {
    let player = query.single()?;
    // ...
    Ok(())
}
```

`single()` returns `Result<_, QuerySingleError>` with `NoEntities` and `MultipleEntities` variants if you need to branch on the cause.

By specific entity:

```rust
fn act_on(target: Res<TargetEntity>, query: Query<&Health>) {
    if let Ok(h) = query.get(target.0) { /* ... */ }
}
```

Multi-entity, mutable (handles the borrow checker for you):

```rust
fn collide(events: MessageReader<Collision>, mut q: Query<&mut Transform>) {
    for e in events.read() {
        if let Ok([mut a, mut b]) = q.get_many_mut([e.a, e.b]) {
            // mutate both
        }
    }
}
```

Combinations (good for gravity, pairwise interactions):

```rust
fn gravity(mut q: Query<&mut Transform, With<HasMass>>) {
    let mut combos = q.iter_combinations_mut();
    while let Some([a, b]) = combos.fetch_next() {
        // pair logic
    }
}
```

In 0.18, you can also access multiple distinct components on a single entity safely with `entity.get_components_mut::<(&mut A, &mut B)>()`.

For CPU-heavy bulk updates over many entities, 0.19 adds **contiguous iteration** — `query.contiguous_iter()` / `contiguous_iter_mut()` hand you whole table column slices so LLVM can auto-vectorize (SIMD). They return `Err(QueryNotDenseError)` when the query isn't dense (sparse-set components, or `Changed`/`Added` filters break contiguity). See `references/performance.md`.

0.19 also generalized queries to support reading from **multiple entities per item** (e.g. a component on the entity's parent). The fallout for *generic* code: iteration methods (`into_iter`, `single_mut`, `iter_combinations_mut`, …) now need a `D: IterQueryData` bound, and `transmute`/`join`/`sort` need `SingleEntityQueryData`. Concrete query types satisfy these automatically; only generic functions over `D: QueryData` need to add the bound (or iterate non-iterable data with `iter_mut().fetch_next()`).

## Change detection

`Changed<T>` skips iteration entries where the component hasn't been mutably accessed since the last time *this* system ran:

```rust
fn react_to_health(query: Query<(Entity, &Health), Changed<Health>>) {
    for (e, h) in &query { /* only changed entities */ }
}
```

`Added<T>` is the subset where the component was just added (or re-added — re-insertion of an existing component counts as added).

`Spawned` (0.17+) filters entities spawned since the last system run:

```rust
fn debug_spawns(query: Query<Entity, Spawned>) { /* ... */ }
```

`SpawnDetails` provides spawn-tick info per entity:

```rust
Query<(Entity, SpawnDetails)>  // .is_spawned(), .spawn_tick(), .spawned_by()
```

For *all* entities with optional change-checking, use `Ref<T>`:

```rust
fn audit(query: Query<(Entity, Ref<Health>)>) {
    for (e, h) in &query {
        if h.is_changed() { /* ... */ }
    }
}
```

In 0.19, `Ref<T>` is `Copy + Clone`. That means `r.clone()` returns another `Ref<T>`, *not* a cloned inner `T` — to clone the underlying value use `r.as_ref().clone()` (or `r.deref().clone()`).

Mutable access (`Query<&mut T>` or `ResMut<T>`) returns a `Mut<T>` smart pointer. Mutable deref unconditionally marks changed, even if you write the same value back. To avoid spurious change-detection signals:

```rust
// Bad: marks changed every frame even if value is identical.
*health = Health(new_value);

// Good: only marks changed if PartialEq says they differ.
health.set_if_neq(Health(new_value));
```

Resources support the same: `Res<T>::is_changed()`, `ResMut<T>::set_if_neq(...)`.

Removal is detected differently — components no longer exist, so:

```rust
fn handle_removed(mut removed: RemovedComponents<Health>) {
    for entity in removed.read() { /* ... */ }
}
```

But `RemovedComponents` can miss removals when used in `FixedUpdate`. Prefer an `On<Remove, T>` observer or a `#[component(on_remove = ...)]` hook for reliable removal handling — those also give you access to the *value* before it's gone, which `RemovedComponents` cannot.

## Resources

For singleton, world-level data:

```rust
#[derive(Resource)]
struct AudioSettings { music: f32, effects: f32 }
```

Initialize at app build:

```rust
app.insert_resource(AudioSettings { music: 0.7, effects: 0.6 });
// or
app.init_resource::<AudioSettings>();  // requires Default or FromWorld
```

Insert/remove dynamically inside systems:

```rust
fn open_menu(mut commands: Commands) {
    commands.insert_resource(MenuState::default());
}

fn close_menu(mut commands: Commands) {
    commands.remove_resource::<MenuState>();
}
```

Access:

```rust
fn read(settings: Res<AudioSettings>) { /* ... */ }
fn write(mut settings: ResMut<AudioSettings>) { settings.music = 0.5; }
```

If a resource may not exist:

```rust
fn maybe_read(settings: Option<Res<AudioSettings>>) {
    let Some(settings) = settings else { return };
    /* ... */
}
```

`Res<T>` panics if `T` isn't inserted. `Option<Res<T>>` lets you handle the absence gracefully.

In 0.18, resources require `'static` lifetime — `#[derive(Resource)] struct Foo<'a>` no longer compiles.

### Resources are components (0.19)

In 0.19 `Resource` became a subtrait of `Component`, and resources are stored as components on singleton entities. The `#[derive(Resource)]` macro now *also* generates the `Component` impl, which has concrete consequences:

- **Don't co-derive.** `#[derive(Component, Resource)]` produces two `Component` impls and won't compile. If you genuinely need both a component flavor and a resource flavor, make them two types:

  ```rust
  // 0.18
  #[derive(Component, Resource)]
  struct Config { /* ... */ }

  // 0.19 — split them
  #[derive(Component)]
  struct ConfigComponent { /* ... */ }
  #[derive(Resource)]
  struct ConfigResource { /* ... */ }
  ```

- **Reflection uses `ReflectComponent`.** `#[reflect(Resource)]` is now a marker only; `ReflectComponent` carries the real machinery. Code that drives resources through reflection (BRP, `bevy_world_serialization`) should reach for `ReflectComponent`. You no longer need `#[derive(MapEntities)]` on a resource — components map entities by default, so `#[derive(Resource)] struct Foo(#[entities] Entity)` is enough.

- **Resources can have hooks, observers, and relationships.** The capabilities that used to be component-only now work on resources: `#[component(on_add = ...)]` on a resource type, `world.add_observer(|_: On<Add, MyResource>, ...|)`, immutability via `#[component(immutable)]`, even relationships pointing at the resource's entity (`world.resource_entity::<R>()` gets it). This narrows the old "resource vs singleton entity" gap considerably.

- **Broad queries now see resources.** Queries that match *all* entities — `Query<Entity>`, `Query<EntityMut>`, `Query<EntityRef>`, `Query<Option<&T>>` — now also match resource entities, which can conflict with `Res`/`ResMut` in the same system. Exclude resource entities with `Without<IsResource>` (the `IsResource` marker is on every resource entity) or `Without<MySpecificResource>`. The same applies to non-send data and `NonSend<T>`.

- **Non-send "resources" are now non-send "data."** Since `Send` resources are components, the `!Send` variants split off: `init_non_send`/`insert_non_send` (the `*_non_send_resource` forms are deprecated), `World::non_send`/`non_send_mut`, etc.

- **Immutable resources affect generic bounds.** `ResMut<R>`, `World::resource_mut::<R>`, and friends now require `R: Resource<Mutability = Mutable>`. Generic code over an arbitrary `R: Resource` that needs `ResMut` must add the bound:

  ```rust
  fn bump<R: Resource<Mutability = Mutable>>(mut r: ResMut<R>) { /* ... */ }
  ```

### Resource vs singleton entity

Resource when the data is truly singular and won't be queried as part of a larger collection (audio settings, world clock, score). Singleton entity (queried via `Single<...>` or `Query<&T, With<Marker>>`) when:

- The data might one day grow to a small collection (one player → split-screen, one camera → multi-camera).
- It needs to be rendered/simulated alongside other entities (player avatar — needs `Transform`, `Visibility`, `Mesh3d` etc.).
- It needs lifecycle hooks (`On<Add, ...>` etc., which are entity-scoped).

When in doubt, use a singleton entity — it's easier to scale up to a collection later than to scale a resource down.

## Local

`Local<T>` is per-system state, persisted across runs of that system:

```rust
fn frame_counter(mut count: Local<u32>) {
    *count += 1;
    println!("frame {}", *count);
}
```

Used internally by `MessageReader` (to track which messages each system has read) and by run conditions like `on_timer` (to track elapsed time).

When the default value isn't useful, wrap in `Option`:

```rust
fn lazy_init(mut state: Local<Option<HeavyThing>>) {
    let state = state.get_or_insert_with(HeavyThing::new);
    /* ... */
}
```

## Relationships

`ChildOf`/`Children` is the parent-child relationship. Other relationships use the same machinery:

```rust
#[derive(Component)]
#[relationship(relationship_target = Contents)]
struct ContainedBy(Entity);

#[derive(Component, Default)]
#[relationship_target(relationship = ContainedBy, linked_spawn)]
struct Contents(Vec<Entity>);
```

The naming convention is unambiguous: name the component on the *holder* side from the holder's perspective. `ContainedBy` means "this entity is contained by another," not "this entity contains things." `ChildOf` means "this entity is a child of another."

`linked_spawn` means despawning the relationship-target entity (the parent / container) automatically despawns the relationship entities (children / contents).

By default Bevy rejects a relationship that points at its own entity (it logs a warning and removes it) — sensible for structural, traversed relationships like `ChildOf`. For purely semantic relationships where self-reference is valid (`Likes(self)`, `Healing(self)`), opt in with `allow_self_referential` (0.19):

```rust
#[derive(Component)]
#[relationship(relationship_target = PeopleILike, allow_self_referential)]
pub struct LikedBy(pub Entity);

#[derive(Component, Default)]
#[relationship_target(relationship = LikedBy)]
pub struct PeopleILike(Vec<Entity>);
```

Spawn a hierarchy:

```rust
commands.spawn((
    Vehicle,
    children![
        (Wheel, Color::Black),
        (Wheel, Color::Black),
        (Wheel, Color::Black),
        (Wheel, Color::Black),
    ],
));
```

The `children!` macro supports up to ~1400 children per call (Rust recursion limit). For more, use `Children::spawn(SpawnIter(...))`.

Query relationships:

```rust
Query<&Children>           // parent's-eye view
Query<&ChildOf>            // child's-eye view (only children, not roots)
Query<(&Color, &Children)> // joined: parent components + their children list
```

Traverse:

```rust
fn descendants(query: Query<&Children>, root: Entity) {
    for entity in query.iter_descendants(root) { /* ... */ }
}

fn ancestors(query: Query<&ChildOf>, leaf: Entity) {
    for entity in query.iter_ancestors(leaf) { /* ... */ }
}
```

For custom relationships, equivalent helpers come from `add_related`/`Contents::spawn` etc.

`detach_*` API in 0.18: `entity.detach_all_children()`, `entity.detach_child(c)`, `entity.detach_all_related::<R>()`. The detach methods only sever the relationship — they don't despawn anything.

## Custom QueryData and SystemParam

For repeated multi-component access patterns, define a `QueryData`:

```rust
#[derive(QueryData)]
#[query_data(mutable)]
struct CombatantData<'w> {
    entity: Entity,
    health: &'w mut Health,
    armor: &'w Armor,
    transform: &'w Transform,
}

fn combat(mut q: Query<CombatantData>) {
    for c in &mut q {
        // c.entity, c.health, c.armor, c.transform
    }
}
```

You can implement methods on `CombatantData` and on the read-only/mutable item types it generates.

For repeated patterns that span multiple queries, resources, or messages, use `SystemParam`:

```rust
#[derive(SystemParam)]
struct Combat<'w, 's> {
    combatants: Query<'w, 's, CombatantData>,
    rng: ResMut<'w, GameRng>,
    damage_msgs: MessageWriter<'w, DamageDealt>,
}

fn combat_tick(mut combat: Combat) {
    // combat.combatants, combat.rng, combat.damage_msgs
}
```

Composability: `SystemParam` is great for encapsulating complex logic with multiple inputs. `QueryData` is more flexible because it composes inside larger queries; `SystemParam` is opaque from outside.

## Disabling entities

Sometimes you want to hide an entity from queries without despawning it. Add the `Disabled` component:

```rust
commands.entity(e).insert(Disabled);
```

`Disabled` entities are excluded from queries by default (`bevy_ecs` adds it to a `DefaultQueryFilters` set). Override with `With<Disabled>`/`Has<Disabled>`/`Allow<Disabled>` when you specifically want them.

You can define your own disable-style markers and register them as default query filters — useful when you want to disambiguate "disabled because killed" from "disabled because off-screen."
