# UI

## Contents
- The `Node` component — flexbox layout fields; viewport-anchored `FixedNode` (0.20)
- `Val` and helpers — `px`/`percent`/`vw`/`vh`, plus `em`/`rem` (0.20), fluent `UiRect` builders
- `UiTransform` — UI-specific 2D transform (replaces `Transform` on UI nodes)
- Visual components — `BackgroundColor`, `BorderColor`, text (`FontSource`/`FontSize`, 0.19; generic-family ctors, fallback lists, `Rem(1.)` default, 0.20); inline images and boxes (`InlineImage`/`InlineBox`, 0.20)
- Update patterns — `Text` deref, visibility toggling
- Marker pattern for HUD elements — spawn-and-update idiom
- Headless widgets (0.19: no longer experimental) — `Button`, `Slider`, `ListBox`, `TabList`, `Dialog` (0.20); events; state components
- Text input (0.19; split in 0.20) — `TextInput` + `EditableText`, `TextReadWriteMode`, `InputFocus`, `FeathersTextInput`
- Feathers (0.19: no longer experimental) — themed widget set for tooling, BSN-only in 0.20; contextual theming
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
- `border_radius` (0.18 — folded into `Node`, used to be a separate component). **(0.20)** each corner is a `CornerRadius { x: Val, y: Val }` so corners can be elliptical. `BorderRadius::all(px(8))` and `border_radius: px(8).into()` still work (`Val: Into<CornerRadius>` gives a circular corner); struct literals need `top_left: px(8).into()` or `CornerRadius::circular(px(8))`, and `CornerRadius::new(px(10), px(20))` / `[px(10), px(20)]` give an elliptical one. The `Into<CornerRadius>`-taking constructors are no longer `const`; `BorderRadius::px(..)`/`percent(..)` still are.
- `FixedNode` (0.20) — a marker (`#[require(Node)]`) that lays the node out against the target camera's viewport even when it has a UI parent, ignoring the parent's layout, scroll, transform and clip rect (CSS `position: fixed`). Handy for tooltips and modals parented to a scroll container so despawn-with-parent still works. Draw order still follows the hierarchy — pair it with `GlobalZIndex`.

```rust
children![(
    FixedNode,
    Node { position_type: PositionType::Absolute, right: px(16), bottom: px(16),
           width: px(100), height: px(100), ..default() },
    GlobalZIndex(1),
    BackgroundColor(YELLOW.into()),
)]
```

## `Val` and helpers

`Val` is the unit type for layout sizes:

- `Val::Auto` — let the layout decide.
- `Val::Px(f32)` — absolute pixels.
- `Val::Percent(f32)` — percentage of parent.
- `Val::Vw(f32)`, `Val::Vh(f32)` — viewport width/height units.
- `Val::VMin(f32)`, `Val::VMax(f32)` — min/max of viewport dimensions.
- `Val::Em(f32)` (0.20) — multiple of the node's own font size, read from its `EmSize` component (required by `Node`, default 20 px).
- `Val::Rem(f32)` (0.20) — multiple of the root font size (`RemSize` resource, default 20 px). Change the resource to scale the whole UI.

In 0.17+, helper functions accept any integer type:

```rust
use bevy::prelude::*;

px(200)        // Val::Px(200.0)
percent(50)    // Val::Percent(50.0)
vw(10)         // Val::Vw(10.0)
vh(10)         // Val::Vh(10.0)
vmin(5)        // Val::VMin(5.0)
vmax(5)        // Val::VMax(5.0)
em(1.5)        // Val::Em(1.5)   (0.20)
rem(2)         // Val::Rem(2.0)  (0.20)
auto()         // Val::Auto
```

**Pitfall (0.20): `em` is not inherited.** `EmSize` is per-entity, and bevy_ui derives it only from a `TextFont` on the *same* entity — nothing propagates it down the hierarchy. A container with no `TextFont` keeps the 20 px default no matter what its children use, and that default does not track `RemSize`. Set `EmSize(px)` on the container yourself, propagate `TextFont` with `Propagate(TextFont { .. })`, or use `rem()` for hierarchy-independent sizing. Grid tracks have `GridTrack::em(5.)` / `GridTrack::rem(10.)` too (those take `f32`).

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

For text (0.19 shape — note `font` and `font_size` changed types):

```rust
commands.spawn((
    Text::new("Hello"),
    TextFont {
        font: server.load("fonts/main.ttf").into(),  // FontSource, not Handle<Font>
        font_size: FontSize::Px(24.0),                // FontSize enum, not f32
        ..default()
    },
    TextColor(Color::WHITE),
));
```

`Text` is the UI version. `Text2d` is the worldspace version (lives in 3D coordinates, used for damage numbers, signs, etc.).

### Text changes in 0.19 (parley migration)

0.19 swapped the text backend from `cosmic-text` to `parley`. Most of it is invisible, but two `TextFont` fields changed type:

- **`font` is now a `FontSource`.** Variants: `Handle(Handle<Font>)` (`asset_server.load(...).into()` converts for you), `Family(SmolStr)` (exact family name — `FontSource::family("FiraMono")`), and **(0.20)** `Families(SmolStr)` for a CSS-style fallback list (`FontSource::families("Arial, 'Noto Sans', sans-serif")`), `List(Vec<FontSource>)` for an ordered chain of any sources (`FontSource::list([handle.into(), FontSource::sans_serif()])`), and `Generic(GenericFontFamily)` for semantic categories — built with `FontSource::sans_serif()`, `serif()`, `monospace()`, `cursive()`, `fantasy()`, `system_ui()`, `emoji()`, `math()`, …, since the 0.19 unit variants (`FontSource::SansSerif`, …) are **gone**. Note `"Fira Sans".into()` now yields `Families` (parsed as a CSS list), so use `FontSource::family(..)` for a literal name. Enable the `system_font_discovery` feature to resolve installed system fonts by family name (needs `fontconfig` on Linux). Override the generic-family defaults via the `FontCx` resource (`set_generic_family(GenericFontFamily::Monospace, "JetBrains Mono")`).
- **`font_size` is now a `FontSize` enum.** `FontSize::Px(24.0)` is the unchanged behavior; `Vw`, `Vh`, `VMin`, `VMax` are viewport-relative, and `Rem(1.5)` scales with the `RemSize` resource (one knob to resize all relative text). **(0.20) `TextFont::default()` is `FontSize::Rem(1.)`, not `Px(20.)`** — same 20 px by default, but every `TextFont` left at the default (UI `Text` and `Text2d` alike) now follows `RemSize`. Pin a fixed size with `font_size: FontSize::Px(20.)`. (`FontSize::default()` itself is still `Px(20.)`.) `TextFont::from_font_size(FontSize::Px(24.0))` and `.with_font(handle)` / `.with_family("…")` are convenience constructors.

Variable-font fields on `TextFont`: `weight: FontWeight` (now a named-constant API, `FontWeight::BOLD` = 700, any value 1–1000; the field existed in 0.18 as `FontWeight(400)`), plus the genuinely-new `width: FontWidth` (`ULTRA_CONDENSED`…`ULTRA_EXPANDED`) and `style: FontStyle` (`Normal`/`Italic`/`Oblique`):

```rust
TextFont {
    font: FontSource::sans_serif(),
    weight: FontWeight::BOLD,
    style: FontStyle::Italic,
    ..default()
}
```

OpenType features and variation axes are still available via the `font_features: FontFeatures` and `font_variations: FontVariations` fields. `LetterSpacing` is a new component (enum, `Px`/`Rem`, follows the same pattern as `LineHeight`; negative values tighten). `Font::try_from_bytes` → `Font::from_bytes(bytes)` (no longer returns `Result`; a later 0.19 change also dropped the family-name argument — loaded font assets auto-register their embedded family name plus an internal alias).

`TextLayout` constructors dropped the `new_with_` prefix: `TextLayout::justify(...)`, `TextLayout::linebreak(...)`, `TextLayout::no_wrap()`.

Strikethrough/underline are separate components (`Strikethrough`, `Underline`, `StrikethroughColor`, `UnderlineColor`). `LineHeight` is a separate component (was a field on `TextFont` in older versions).

Drop shadows for `Text` and `Text2d`:

```rust
commands.spawn((Text2d::new("Score: 100"), Text2dShadow { /* ... */ }));
```

Text background colors:

```rust
commands.spawn((Text::new("Important"), TextBackgroundColor(Color::RED)));
```

### Inline images and boxes (0.20)

`InlineImage` is a text child, like `TextSpan`, that flows an image with the text. It is in the prelude, `#[require]`s `InlineBox`, and is sized from the image asset once it loads (until then it is a zero-size out-of-flow box that displaces nothing). UI `Text` only — `Text2d` supports the underlying `InlineBox` but has no built-in image drawing.

```rust
commands.spawn((
    Text::new("Press "),
    children![
        InlineImage { image: asset_server.load("ui/key_a.png"), ..default() },
        TextSpan::new(" to jump"),
    ],
));
```

For custom content use `InlineBox` directly (`use bevy::text::{InlineBox, InlineBoxKind};` — not in the prelude): `InlineBox { kind: InlineBoxKind::InFlow, size: Vec2::splat(24.) }` reserves space in logical pixels; read the placed rectangles back from `TextLayoutInfo::inline_boxes` (`Vec<(Entity, InlineBoxKind, Rect)>`, physical pixels relative to the layout's top-left) and draw them yourself. `InlineBox::default()` is `OutOfFlow` with zero size, so it gets a position but takes no space. Inline boxes are leaves — `TextSpan` children under one are ignored — and, like `TextSpan`, must be descendants of a root `Text`/`Text2d`.

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

`Display::None` removes the node from layout entirely (siblings reflow). For "make invisible but preserve layout," set `BackgroundColor(Color::NONE)` (the same value as `BackgroundColor::DEFAULT`) or toggle the colour's alpha — or use `Visibility::Hidden`, which is honored by UI rendering but doesn't remove the node from layout.

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

## Headless widgets

As of 0.19 these are **no longer experimental**: the feature was renamed `experimental_bevy_ui_widgets` → `bevy_ui_widgets` and folded into the `ui` collection (and thus default features), and the `UiWidgetsPlugins` plugin group is now part of `DefaultPlugins` (so is `InputDispatchPlugin`) — remove any manual `add_plugins(UiWidgetsPlugins)` if you have `DefaultPlugins`. The widgets:

- **`Button`** (`bevy::ui_widgets::Button` — **import it explicitly**) — emits `Activate` when clicked or activated by keyboard. **(0.20)** `bevy::prelude::Button` is the deprecated `bevy_ui` marker, which never fires `Activate`; an explicit `use bevy::ui_widgets::Button;` shadows the prelude glob. `ui_widgets::Button` requires only `AccessibilityNode`, so add `Node` and `Hovered::default()` yourself. `ActivateOnPress` fires on pointer-down instead of release.
- **`Slider`** — `f32` value in a range; emits `ValueChange<f32>`.
- **`Scrollbar`** — scrolls a parent container. (0.19 dropped the `Core` prefix: `CoreScrollbarThumb` → `ScrollbarThumb`, `CoreScrollbarDragState` → `ScrollbarDragState`, `CoreSliderDragState` → `SliderDragState`.)
- **`Checkbox`** — boolean, emits `ValueChange<bool>`.
- **`RadioButton`** + **`RadioGroup`** — exclusive selection.
- **`ListBox`** + **`ListItem`** — exclusive selection from a list; emits `ValueChange<Entity>`. (0.20) adds the `SetSelected { entity, row }` event for programmatic selection.
- **`MenuButton`/`MenuItem`/`MenuPopup`**, **`Popover`**, **`ScrollArea`** — menus, floating placement, and wheel/trackpad scrolling on an `overflow: scroll` node.
- **`TabList` + `Tab` (0.20)** — headless tab strip. `TabList { orientation, activation }` requires `SelectedTab(Option<Entity>)`, which *owns* the selection: click, Enter/Space, or (with `TabActivation::Automatic`) arrow-key focus moves emit `ValueChange<Option<Entity>>` as a *request* — apply it yourself or attach `on(tablist_self_update)`. `Tab` requires `Selectable` and a roving `TabIndex(-1)`, and gets the derived `bevy::ui::Selected` marker but **not** `Hovered`. `TabNavigationPlugin` is still not in `DefaultPlugins` — add it plus an ancestor `TabGroup` for Tab/Shift+Tab into the strip.
- **`Dialog` / `ModalDialog` (0.20)** — `Dialog` is a movable floating window (`DialogDragHandle` marks the drag region; `DialogPlugin` manages z-order). `ModalDialog` requires `Dialog` and traps focus, and expects you to spawn a full-screen ancestor carrying `ModalDialogBarrier`. Nothing closes itself: barrier click or Escape triggers the propagating `RequestClose { source }` entity event — observe it and despawn `close.event_target()`. Feathers ships themed `FeathersDialog`/`FeathersFloatingDialog`.

Headless = no styling. Bevy provides the behavior (events, accessibility, keyboard navigation), you provide visual treatment. The widget set is still immature and will see breaking changes, but it's stable enough for general use now.

State components used by widgets:

- **`Hovered(bool)`** — `bevy::picking::hover::Hovered`, **not** in the prelude. It is `#[component(immutable)]` and the picking backend keeps it current by re-inserting it, and it is **opt-in**: `ui_widgets::Button` does not require it, so spawn `Hovered::default()` yourself or `&Hovered` queries match nothing. Read `hovered.get()`, filter `Changed<Hovered>`, or observe `On<Insert<Hovered>>`.
- **`Pressed`**, **`Checked`**, **`Checkable`**, **`InteractionDisabled`** — `bevy::ui`, **unit markers** (present = true), inserted and removed by the widget. There is no `.0`: query with `Has<Pressed>` / `With<Checked>`. `Changed<Pressed>` only matches on insertion, never on removal — poll each frame, or observe `On<Add<Pressed>>` / `On<Remove<Pressed>>`.
- **`Selected`** — derived marker set from `SelectedTab` / `ListBox` selection.

```rust
use bevy::{picking::hover::Hovered, prelude::*, ui::Pressed, ui_widgets::Button};

fn style_buttons(mut q: Query<(&Hovered, Has<Pressed>, &mut BackgroundColor), With<Button>>) {
    for (hovered, pressed, mut bg) in &mut q {
        *bg = match (hovered.get(), pressed) {
            (_, true) => PRESSED,
            (true, false) => HOVERED,
            _ => NORMAL,
        }.into();
    }
}
```

`bevy::ui::Interaction` is deprecated in 0.20 — replace `Query<&Interaction, Changed<Interaction>>` and `match Interaction::{Pressed, Hovered, None}` with the query above, or react to `On<Activate>`.

Events:

```rust
use bevy::{picking::hover::Hovered, prelude::*, ui_widgets::{Activate, Button}};

commands.spawn((
    Button,
    Hovered::default(),
    Node { /* style */ },
    BackgroundColor(Color::WHITE),
)).observe(|activate: On<Activate>, /* ... */| {
    info!("Button activated!");
});
```

Or globally:

```rust
commands.add_observer(|change: On<ValueChange<f32>>, /* ... */| {
    // `source` is the emitting widget; `is_final` is false while dragging, true on release.
    info!("Slider {} changed to {} (final: {})", change.source, change.value, change.is_final);
});
```

## Text input (0.19; split in 0.20)

A text field is **two components in 0.20**. `EditableText` (`bevy::text`) is the **state**: value, cursor, selection, `viewport`, `max_characters`, `visible_width`, `visible_lines`, `allow_newlines`. `TextInput` (`bevy::ui_widgets`, a unit struct that `#[require]`s `EditableText`) is the **widget**: keyboard editing, cursor navigation (arrows, Home/End, word-level with Ctrl/Alt), selection (Shift+arrows, click-drag, double/triple-click), backspace/delete, OS clipboard (with the `system_clipboard` feature) or in-app buffer, unicode-aware navigation, bidirectional text, IME for CJK, multiline + scrolling, and per-character filtering via `EditableTextFilter`.

In 0.19 those observers matched any `EditableText`; in 0.20 every one of them filters `With<TextInput>`, so **an entity with only `EditableText` renders and takes focus but ignores all input.** Neither type is in the prelude. The plugin is `TextInputPlugin` (was `EditableTextInputPlugin`), already in `UiWidgetsPlugins`.

```rust
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui_widgets::TextInput;

commands.spawn((
    Node { width: px(200), border: px(2).all(), padding: px(8).all(), ..default() },
    BorderColor::from(Color::WHITE),
    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
    TextInput,                                        // behavior (0.20)
    EditableText { allow_newlines: false, ..default() },   // state
    TextFont { font_size: FontSize::Px(24.0), ..default() },
    TextCursorStyle::default(),
    TabIndex(0),   // click-to-focus needs a TabIndex; add TabNavigationPlugin for Tab-key focus
));
```

**Read-only / display-only** is the separate `bevy::text::TextReadWriteMode` component (required by `EditableText`, default `Editable`), not a `TextInput` field: `ReadOnly` keeps cursor movement, selection and copy but drops destructive edits; `Static` is display-only and also ignores pointer press/drag.

**Scrolling (0.20):** the `TextScroll` component and `scroll_editable_text` system are gone. Scroll state lives in `editable.viewport` (a `TextViewport { offset: Vec2, size: Vec2 }`): read it to drive a custom scrollbar, and scroll with `editable.queue_edit(TextEdit::ScrollBy(delta))` / `ScrollTo` / `ScrollByLines`, or by writing `viewport.offset`. `viewport.size` is synced to the node's content box — don't set it. Cursor reveal is automatic; `cursor_margin` (default `Vec2::splat(0.2)`, a fraction of the viewport) tunes the inset.

**Escape blurs and bubbles (0.20).** Escape in a focused field collapses the selection, clears `InputFocus` on the press, and *keeps propagating* `FocusedInput<KeyboardInput>` to ancestors and the window (0.19 consumed it), so a dialog-cancel observer on an ancestor fires on the same press. For two-step behaviour, test the original target — `InputFocus` is already empty and `focused_entity` is rewritten at each hop:

```rust
fn on_escape(input: On<FocusedInput<KeyboardInput>>, fields: Query<(), With<TextInput>>) {
    if !matches!(input.input.logical_key, Key::Escape) || !input.input.state.is_pressed() { return; }
    if fields.contains(input.original_event_target()) { return; } // this press blurred a field
    // cancel / close / navigate back ...
}
```

**`FocusCause` gained `Auto` (0.20)**, set when the `AutoFocus` component grants focus — add the arm to any exhaustive `match`, and note that a handler which special-cased `Navigated` no longer matches auto-focus.

`EditableText` only accepts input while its entity is focused, via the `InputFocus` resource. **`InputFocus` fields are private in 0.19** — use `input_focus.get()`, `input_focus.set(entity, FocusCause::Navigated)`, `input_focus.clear()` (the `.0` field access is gone). The `TextEditChange` event fires on the entity *after* edits are applied; read the value with `editable.value()` (returns a string-like `SplitString`), reset with `editable.clear()`, cap length with `max_characters`, opt into select-all-on-focus with the `SelectAllOnFocus` component.

```rust
fn on_submit(
    input_focus: Res<InputFocus>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut inputs: Query<&mut EditableText>,
) {
    if keyboard.just_pressed(KeyCode::Enter)
        && let Some(entity) = input_focus.get()
        && let Ok(mut input) = inputs.get_mut(entity)
    {
        println!("Submitted: {}", input.value());
        input.clear();
    }
}
```

Use `TextInput` + `EditableText` directly when you need full control over appearance (player-name fields, chat boxes, search bars). Use `FeathersTextInput` (below) when you want a polished, themed input out of the box.

## Feathers

As of 0.19, Feathers is **no longer experimental**: the feature was renamed `experimental_bevy_feathers` → `bevy_feathers`, and the core plugin `FeathersPlugin` → `FeathersCorePlugin` (the `FeathersPlugins` group bundles it with `TabNavigationPlugin`). `bevy_feathers` is an opinionated, themed widget set built on the headless widgets, intended for the Bevy editor.

Useful for tooling and inspectors. Uses for shipped games are limited — Feathers has an editor/utility aesthetic, not a general game UI aesthetic.

0.19 grew the widget set considerably: `FeathersTextInput`, number input, dropdown menu + divider, disclosure toggle, icon/label primitives, pane/subpane/group decorators, and a Feathers-themed scrollbar and list view (the themed counterparts to the headless `Scrollbar`) — plus a `feathers_gallery` example. **(0.20) Feathers is BSN-only**: the 0.19-deprecated `*_bundle` spawn fns and `ButtonBundleProps` are removed. Themed labels come from the new `bevy::feathers::display::caption("…")` scene fn (which emits `Text` + `ThemedText`) — write `@caption(..)` rather than spelling out `Text(..) ThemedText`. A Feathers checkbox in BSN, with its caption and change observer in one declaration:

```rust
use bevy::feathers::{controls::FeathersCheckbox, display::caption};

bsn! {
    @FeathersCheckbox {
        @caption: bsn! { @caption("Enable shadows") }
    }
    MyCheckbox
    on(|change: On<ValueChange<bool>>, mut config: ResMut<ShadowConfig>| {
        config.enabled = change.value;
    })
}
```

See `references/bsn.md` for the BSN syntax these widgets use.

**0.20 additions.** `FeathersSelect` — a dropdown whose `@options` prop takes a `Box<dyn SceneList>` of `@FeathersListRow` scenes (`list_rows_from_strings(["One", "Two"], Some(0))` builds them and tags each with `OptionIndex`), capped by `@max_visible` (default 8); it emits `ValueChange<Entity>` with the row entity. `FeathersLazyMenu { popup: Arc<dyn Fn() -> Box<dyn Scene> + Send + Sync> }` spawns its popup scene on open and despawns it on close, with `FeathersMenuToolButton` as the trigger. `FeathersColorInput` (swatch button that opens a picker), `FeathersColorSwatchGrid` (emits `ValueChange<Color>`), `FeathersColorWheel`. Dialogs: `FeathersDialog` (modal; its root node *is* the barrier, and you attach the despawn observer) and `FeathersFloatingDialog` (movable, self-despawns on `RequestClose`), composed from `@FeathersDialogHeader` / `@FeathersDialogClose` / `@FeathersDialogBody` / `@FeathersDialogFooter`.

**Number input (0.20).** The value is a component you insert, not an event: `commands.entity(e).insert(NumberInputValue::F32(v))` — `UpdateNumberInput` is gone. `NumberInputValue` (`F32`/`F64`/`I32`/`I64`) is `#[require]`d by `FeathersNumberInput`, so it can also be given inline in `bsn!`. Tune with optional components: `SoftLimit(NumberInputRange::F32(0.0..=10.0))` (draws a slider bar and bounds dragging), `HardLimit(..)` (clamps), `NumberInputPrecision(2)`, `NumberInputStep(1.0)` (the release note calls it `Step`), `NumberInputWrap::Wrap`, `NumberInputUnits`. It emits a plain `ValueChange<f32>`/`<f64>`/`<i32>`/`<i64>` matching the `NumberInputValue` variant, not `ValueChange<NumberInputValue>`; re-insert `NumberInputValue` on `change.event_target()` to move the display.

```rust
// bsn!: @FeathersNumberInput NumberInputValue::F32(1.0) SoftLimit(NumberInputRange::F32(0.0..=10.0)) NumberInputStep(1.0)
```

**Contextual theming (0.20).** `ThemeProps` is no longer a flat token → color map but `{ token_assignments, semantic_base, semantic_overrides }`, so the same widget can render differently on a window, a floating panel or a popup. `UiTheme(create_dark_theme())` still just works; a custom theme built from the old flat map ports with `ThemeProps::new_non_contextual(map)`. Context comes from a `ThemeContext(SurfaceLevel)` component on the themed entity (absent means `SurfaceLevel::Base`), and **it does not propagate on its own**, whatever its doc comment says — to theme a subtree, put `Propagate::<ThemeContext>(ThemeContext(SurfaceLevel::Floating))` on the container (`FeathersCorePlugin` registers `HierarchyPropagatePlugin::<ThemeContext>` for you).

**Cursor types moved (0.20).** `EntityCursor`, `DefaultCursor`, `OverrideCursor` and `CursorIconPlugin` moved from `bevy::feathers::cursor` to `bevy::picking::cursor`, so per-entity hover cursors no longer need Feathers — but `CursorIconPlugin` is not in `DefaultPickingPlugins`, so add it yourself.

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
        (TextSpan::new("here"), observe(|_: On<PointerClick>| {
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
