# UI

## Contents
- The `Node` component — flexbox layout fields
- `Val` and helpers — `px`/`percent`/`vw`/`vh`, fluent `UiRect` builders
- `UiTransform` — UI-specific 2D transform (replaces `Transform` on UI nodes)
- Visual components — `BackgroundColor`, `BorderColor`, text
- Update patterns — `Text` deref, visibility toggling
- Marker pattern for HUD elements — spawn-and-update idiom
- Headless widgets (experimental) — `Button`, `Slider`, etc.; events; state components
- Feathers (experimental) — themed widget set for tooling
- Auto directional navigation (0.18) — gamepad/keyboard navigation
- Popovers and menus (0.18) — `Popover`, `MenuPopup`
- Pickable text spans (0.18) — per-glyph picking; non-text-area picking gone
- ViewportNode — render camera output into UI
- Layout idioms — centered overlay, vertical stack, horizontal toolbar
- Scroll content — `Overflow`, `ScrollPosition`, `IgnoreScroll` (0.18)
- UI gradients (0.17+) — `BackgroundGradient`, `BorderGradient`

Bevy UI is a flexbox-based layout system. UI nodes are entities with a `Node` component plus visual components (background, border, text). Layout is computed automatically each frame in `PostUpdate`.

## The `Node` component

```rust
commands.spawn((
    Node {
        position_type: PositionType::Absolute,
        left: px(10),
        top: px(10),
        width: px(200),
        height: px(20),
        padding: UiRect::all(px(4)),
        flex_direction: FlexDirection::Column,
        ..default()
    },
    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
));
```

Most flexbox properties have direct fields on `Node`:

- `position_type` — `Relative` (default, in flow) or `Absolute` (out of flow, positioned by `left/right/top/bottom`).
- `display` — `Flex` (default) or `None` (entirely removed from layout).
- `flex_direction` — `Row` (default), `Column`, `RowReverse`, `ColumnReverse`.
- `flex_wrap` — `NoWrap` (default), `Wrap`, `WrapReverse`.
- `justify_content` / `align_items` — main-axis and cross-axis alignment.
- `width`, `height`, `min_width`, `max_width`, `min_height`, `max_height`.
- `padding`, `margin`, `border` — `UiRect` of `Val`s.
- `border_radius` (0.18 — folded into `Node`, used to be a separate component).

## `Val` and helpers

`Val` is the unit type for layout sizes:

- `Val::Auto` — let the layout decide.
- `Val::Px(f32)` — absolute pixels.
- `Val::Percent(f32)` — percentage of parent.
- `Val::Vw(f32)`, `Val::Vh(f32)` — viewport width/height units.
- `Val::VMin(f32)`, `Val::VMax(f32)` — min/max of viewport dimensions.

In 0.17+, helper functions accept any integer type:

```rust
use bevy::prelude::*;

px(200)        // Val::Px(200.0)
percent(50)    // Val::Percent(50.0)
vw(10)         // Val::Vw(10.0)
vh(10)         // Val::Vh(10.0)
vmin(5)        // Val::VMin(5.0)
vmax(5)        // Val::VMax(5.0)
auto()         // Val::Auto
```

`UiRect` has fluent builders:

```rust
px(2).all()                                  // UiRect::all(px(2))
percent(20).horizontal()                      // left + right
percent(20).horizontal().with_top(px(10))     // left + right + top
vw(10).left()                                 // only the left side
```

The available side methods: `left`, `right`, `top`, `bottom`, `all`, `horizontal`, `vertical`.

## `UiTransform`

In 0.17+, UI nodes use `UiTransform` / `UiGlobalTransform` instead of `Transform` / `GlobalTransform`. UI no longer goes through general transform propagation — it has a specialized 2D propagation that's faster and avoids redundant work.

If you're tempted to put a `Transform` on a UI node, don't — use `UiTransform`. Most user code rarely touches `UiTransform` directly; layout is enough.

## Visual components

```rust
BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9))
BorderColor::all(Color::WHITE)
BorderColor { left: red, right: red, top: blue, bottom: blue }  // 0.17 per-side
```

For text:

```rust
commands.spawn((
    Text::new("Hello"),
    TextFont { font: server.load("fonts/main.ttf"), font_size: 24.0, ..default() },
    TextColor(Color::WHITE),
));
```

`Text` is the UI version. `Text2d` is the worldspace version (lives in 3D coordinates, used for damage numbers, signs, etc.).

In 0.18, `TextFont` supports font weight (`weight: FontWeight(400)`) and OpenType features:

```rust
TextFont {
    font: opentype_handle,
    font_features: FontFeatures::builder()
        .enable(FontFeatureTag::STANDARD_LIGATURES)
        .build(),
    ..default()
}
```

Strikethrough/underline are separate components (`Strikethrough`, `Underline`, `StrikethroughColor`, `UnderlineColor`).

`LineHeight` is a separate component (was a field on `TextFont` in older versions).

Drop shadows for `Text` and `Text2d`:

```rust
commands.spawn((Text2d::new("Score: 100"), Text2dShadow { /* ... */ }));
```

Text background colors:

```rust
commands.spawn((Text::new("Important"), TextBackgroundColor(Color::RED)));
```

## Update patterns

Update text:

```rust
fn update_score(mut text: Single<&mut Text, With<ScoreDisplay>>, score: Res<Score>) {
    **text = format!("Score: {}", score.0);
}
```

`Text` derefs to its inner `String`, so `**text = ...` writes the string.

Update visibility:

```rust
fn show_panel(mut node: Single<&mut Node, With<Panel>>) {
    node.display = Display::Flex;
}

fn hide_panel(mut node: Single<&mut Node, With<Panel>>) {
    node.display = Display::None;
}
```

`Display::None` removes the node from layout entirely (siblings reflow). For "make invisible but preserve layout," set `BackgroundColor::NONE` and toggle the alpha — or use `Visibility::Hidden`, which is honored by UI rendering but doesn't remove the node from layout.

## Marker pattern for HUD elements

```rust
#[derive(Component)]
struct HealthBar;

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        HealthBar,
        Node {
            position_type: PositionType::Absolute,
            left: px(10),
            top: px(10),
            width: px(200),
            height: px(20),
            ..default()
        },
        BackgroundColor(Color::srgba(0.8, 0.2, 0.2, 0.9)),
    ));
}

fn update_health_bar(
    health: Single<&Health, With<Player>>,
    mut bar: Single<&mut Node, With<HealthBar>>,
) {
    bar.width = px(health.percentage() * 200.0);
}
```

The marker component pattern lets you spawn any number of HUD elements and update each one with a focused query. `Single<...>` is great for "exactly one of this widget" cases.

## Headless widgets (experimental)

In 0.17+, behind the `experimental_bevy_ui_widgets` feature flag:

- **`Button`** — emits `Activate` events when clicked or activated by keyboard.
- **`Slider`** — `f32` value in a range; emits `ValueChange<f32>`.
- **`Scrollbar`** — scrolls a parent container.
- **`Checkbox`** — boolean, emits `ValueChange<bool>`.
- **`RadioButton`** + **`RadioGroup`** — exclusive selection.

Headless = no styling. Bevy provides the behavior (events, accessibility, keyboard navigation), you provide visual treatment.

Boolean state components used by widgets:

- **`Hovered`** — true while pointer is over.
- **`Pressed`** — true while button-like widget is held down.
- **`Checked`** — current state of toggleable widgets.
- **`InteractionDisabled`** — disable interaction (for "grayed out" states).

These are detectable via change detection (`Changed<Hovered>` etc.).

Events:

```rust
commands.spawn((
    Button,
    Node { /* style */ },
    BackgroundColor(Color::WHITE),
)).observe(|activate: On<Activate>, /* ... */| {
    info!("Button activated!");
});
```

Or globally:

```rust
commands.add_observer(|change: On<ValueChange<f32>>, /* ... */| {
    info!("Slider changed to {}", change.0);
});
```

## Feathers (experimental)

`bevy_feathers` is an opinionated, themed widget set built on top of headless widgets, intended for the future Bevy Editor. Behind the `experimental_bevy_feathers` feature flag.

Useful for tooling and inspectors. Uses for shipped games are limited — Feathers has an editor/utility aesthetic, not a general game UI aesthetic. The widgets are meant to be copied into your project as a starting point if you want to build similar widgets.

Includes (0.18): buttons, sliders, checkboxes, menu buttons, radio buttons, color pickers (`ColorPlane`), text input (in progress), virtual keyboard.

## Auto directional navigation (0.18)

For gamepad/keyboard UI navigation:

```rust
commands.spawn((
    Button,
    Node { /* ... */ },
    AutoDirectionalNavigation::default(),
));
```

Bevy auto-computes neighbors based on spatial position — pressing right on a gamepad navigates to the nearest button to the right. No more manual `DirectionalNavigationMap::add_edge` for every pair.

Configure with the `AutoNavigationConfig` resource:

```rust
app.insert_resource(AutoNavigationConfig {
    min_alignment_factor: 0.0,
    max_search_distance: Some(500.0),
    prefer_aligned: true,
});
```

Manual edges still take precedence over auto-generated ones, so you can override specific connections (e.g., screen-edge wraparound) while leaving the rest automatic.

## Popovers and menus (0.18)

`Popover` is a component for absolutely-positioned popups that auto-position relative to an anchor:

```rust
commands.spawn((
    Popover { /* placement preferences */ },
    Node { position_type: PositionType::Absolute, ..default() },
));
```

Inspired by the JS `floating-ui` library — handles flipping placement when the popup would go off-screen, etc.

`MenuPopup` builds on `Popover` to provide dropdown menus with keyboard navigation.

## Pickable text spans (0.18)

Individual text sections are pickable:

```rust
commands.spawn((
    Text::new(""),
    children![
        TextSpan::new("Click "),
        (TextSpan::new("here"), observe(|_: On<Pointer<Click>>| {
            info!("Hyperlink clicked!");
        })),
        TextSpan::new(" to continue"),
    ],
));
```

In 0.18, the *non-text* areas of `Text` nodes are no longer pickable. To recreate the 0.17 behavior, wrap the `Text` in a parent `Node` and put the picking observer on the parent.

## ViewportNode

Render a camera output into a UI node:

```rust
commands.spawn(ViewportNode::new(camera_entity));
```

The referenced camera's `RenderTarget` must be a `RenderTarget::Image`. Useful for picture-in-picture, mini-maps, or in-game monitors.

If `bevy_ui_picking_backend` (renamed `ui_picking` in 0.18) is enabled, you can pick through the viewport into the rendered scene.

## Layout idioms

Centered overlay:

```rust
Node {
    position_type: PositionType::Absolute,
    left: percent(50),
    top: percent(50),
    margin: UiRect {
        left: px(-150),  // half of width
        top: px(-100),   // half of height
        ..default()
    },
    width: px(300),
    height: px(200),
    ..default()
}
```

Or use `align_self: AlignSelf::Center` and `justify_self: JustifySelf::Center` if the parent is a flex container.

Vertical stack:

```rust
Node {
    flex_direction: FlexDirection::Column,
    row_gap: px(8),
    padding: UiRect::all(px(8)),
    ..default()
}
```

Horizontal toolbar:

```rust
Node {
    flex_direction: FlexDirection::Row,
    column_gap: px(4),
    align_items: AlignItems::Center,
    padding: UiRect::all(px(4)),
    ..default()
}
```

## Scroll content

For scrollable lists, set `overflow` and `ScrollPosition`:

```rust
commands.spawn((
    Node {
        overflow: Overflow::scroll_y(),
        ..default()
    },
    ScrollPosition::default(),
));
```

In 0.18, `IgnoreScroll` lets specific child elements ignore the parent's scroll on a specific axis — useful for sticky headers in scroll containers.

## UI gradients (0.17+)

```rust
commands.spawn((
    Node { width: px(200), height: px(20), ..default() },
    BackgroundGradient::from(LinearGradient {
        angle: 0.0,
        stops: vec![
            ColorStop::new(Color::WHITE, percent(0)),
            ColorStop::new(Color::BLACK, percent(100)),
        ],
        ..default()
    }),
));
```

Variants: `Linear`, `Conic`, `Radial`. Each takes color stops and an interpolation color space (default `Oklab`). `BorderGradient` does the same for borders.
