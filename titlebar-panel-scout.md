# Titlebar Panel Extension Scout Report

## 1. How the Left Workspace Sidebar Extends Into the Title Bar

### Key Architecture: Sidebar is a SIBLING of the workspace, not inside it

The sidebar extends into the title bar area by being rendered **outside** the Workspace view tree, as a horizontal sibling next to the entire workspace (title bar + content). Because the sidebar fills `h_full()` of the shared parent, its background naturally covers the "title bar band" on its side.

**File: `crates/workspace/src/multi_workspace.rs:2075` — `MultiWorkspace::render()`**

The `MultiWorkspace::render()` builds an `h_flex` (horizontal flex) layout:

```
h_flex (root, full window size)
├── left_sidebar (optional)     ← .h_full().w(sidebar_width).flex_shrink_0()
├── div (workspace wrapper)      ← .flex().flex_1().size_full().overflow_hidden()
│   └── Workspace entity         ← renders: [titlebar_item | workspace_content]
├── right_sidebar (optional)     ← .h_full().w(sidebar_width).flex_shrink_0()
├── modal_layer
└── sidebar_overlay
```

The sidebar container code (line ~2081–2120):

```rust
// Sidebar is built as an AnyElement:
div()
    .id("sidebar-container")
    .relative()
    .h_full()                          // ← KEY: full height of parent
    .w(sidebar_width)                  // ← sidebar width from SidebarHandle
    .flex_shrink_0()
    .child(sidebar_handle.to_any())    // ← the WorkspaceSidebar view
    .child(resize_handle)
    .into_any_element()
```

The placement into left/right (line ~2125–2129):

```rust
let (left_sidebar, right_sidebar) = if sidebar_on_right {
    (None, sidebar)
} else {
    (sidebar, None)
};
```

Then the root `h_flex` is (line ~2156–2196):

```rust
root
    .children(left_sidebar)            // ← sidebar BEFORE workspace in flex
    .child(
        div()
            .flex()
            .flex_1()
            .size_full()
            .overflow_hidden()
            .child(self.workspace().clone()),  // ← workspace AFTER sidebar
    )
    .children(right_sidebar)           // ← sidebar AFTER workspace in flex
```

**The whole thing is wrapped by `client_side_decorations()`** (line ~2199).

### Why the sidebar background fills the title bar area:

1. The sidebar `div` takes `h_full()` of the parent — the full window height.
2. The workspace `div` also takes `size_full()`, but is `flex_1` horizontally, meaning it only takes the REMAINING width after the sidebar.
3. Inside the workspace, the `titlebar_item` (PlatformTitleBar) only spans the workspace's width, NOT the sidebar's width.
4. Therefore, the sidebar's `panel_background` color covers the title bar "band" on its side naturally — the title bar simply doesn't exist over the sidebar region.

### The WorkspaceSidebar's own background

**File: `crates/workspace/src/workspace_sidebar.rs:183` — `WorkspaceSidebar::render()`**

```rust
v_flex()
    .size_full()
    .bg(cx.theme().colors().panel_background)   // ← sidebar bg color
    .track_focus(&self.focus_handle(cx))
    .child(header)                               // "Workspaces" header with title-bar-aware padding
    .child(ui::Divider::horizontal())
    // ... list of projects ...
```

The header accounts for macOS traffic light buttons (line ~192–196):

```rust
.when(cfg!(target_os = "macos"), |el| {
    el.pl(px(TRAFFIC_LIGHT_PADDING))    // ~71px left padding for traffic lights
})
.when(!cfg!(target_os = "macos"), |el| el.pl_2())
```

### `client_side_decorations()` wrapper

**File: `crates/workspace/src/workspace.rs:10484` — `client_side_decorations()`**

```rust
pub fn client_side_decorations(
    element: impl IntoElement,
    window: &mut Window,
    cx: &mut App,
    border_radius_tiling: Tiling,
) -> Stateful<Div>
```

This function wraps the entire layout in a `div#window-backdrop` that:
- Applies rounded corners (suppresses them where tiling/sidebars touch edges)
- Adds shadow/border for client-side decorations
- Handles resize-edge hit testing

The `Tiling` parameter (passed from `MultiWorkspace::render()`) tells it the sidebar state:

```rust
Tiling {
    left: !sidebar_on_right && self.sidebar.is_some() && self.sidebar_open(),
    right: sidebar_on_right && self.sidebar.is_some() && self.sidebar_open(),
    ..Tiling::default()
},
```

This suppresses the top-left/top-right rounding when a sidebar extends to that corner.

---

## 2. How the Right Dock Is Rendered

### The right dock is INSIDE the workspace, below the title bar

**File: `crates/workspace/src/workspace.rs:8320` — `Workspace::render()`**

The Workspace renders as a vertical flex column:

```
div (v_flex, size_full)
├── titlebar_item (PlatformTitleBar)        ← .children(self.titlebar_item.clone())
└── div (workspace content, flex_1, flex_col)
    └── div#workspace (bg: background, flex_1, border_t/b)
        └── layout (depends on BottomDockLayout)
            ├── render_dock(Left)    ← left_dock
            ├── center pane group
            ├── render_dock(Right)   ← right_dock
            └── render_dock(Bottom)  ← bottom_dock
```

For `BottomDockLayout::Full` (the default, line ~8530–8570):

```rust
div().flex().flex_col().h_full()
    .child(
        div().flex().flex_row().flex_1().overflow_hidden()
            .children(self.render_dock(DockPosition::Left, &self.left_dock, window, cx))
            .child(
                div().flex().flex_col().flex_1().overflow_hidden()
                    .child(h_flex().flex_1() /* center pane */)
            )
            .children(self.render_dock(DockPosition::Right, &self.right_dock, window, cx))
    )
    .child(div().w_full().children(self.render_dock(DockPosition::Bottom, ...)))
```

### `render_dock()` — how dock width/position is determined

**File: `crates/workspace/src/workspace.rs:7683` — `Workspace::render_dock()`**

```rust
fn render_dock(
    &self,
    position: DockPosition,
    dock: &Entity<Dock>,
    window: &mut Window,
    cx: &mut App,
) -> Option<Div>
```

Key logic:

1. If `self.zoomed_position == Some(position)`, returns `None` (the dock is hidden when something is zoomed over it).

2. Creates a container div:
```rust
let mut container = div()
    .flex()
    .overflow_hidden()
    .flex_none()                     // ← dock does NOT grow/shrink
    .child(dock.clone())
    .children(leader_border);
```

3. Sizing: if the dock is open with a visible panel:
   - **Horizontal docks (Left/Right):** Uses either flexible width or fixed pixel width:
     ```rust
     if let Some(grow) = flex_grow {
         // Flexible: uses flex_grow/flex_shrink/flex_basis
         style.flex_grow = Some(grow);
         style.flex_shrink = Some(1.0);
         style.flex_basis = Some(relative(0.).into());
     } else {
         // Fixed: uses stored size or panel's default_size
         let size = size_state
             .and_then(|state| state.size)
             .unwrap_or_else(|| panel.default_size(window, cx));
         container = container.w(size);
     }
     if let Some(min) = min_size {
         container = container.min_w(min);
     }
     ```
   - **Vertical dock (Bottom):** Uses height:
     ```rust
     let size = size_state.and_then(|state| state.size)
         .unwrap_or_else(|| panel.default_size(window, cx));
     container = container.h(size);
     ```

4. When dock is closed, the container still renders (to keep focus handles mounted) but has no explicit size set, so it takes zero space.

### Dock background color

**File: `crates/workspace/src/dock.rs:1027` — `Dock::render()` (in `impl Render for Dock`)**

```rust
div()
    .id("dock-panel")
    .key_context(dispatch_context)
    .track_focus(&self.focus_handle(cx))
    .focus_follows_mouse(self.focus_follows_mouse, cx)
    .flex()
    .bg(cx.theme().colors().panel_background)       // ← dock bg color
    .border_color(cx.theme().colors().border)
    .overflow_hidden()
    .map(|this| match self.position().axis() {
        Axis::Horizontal => this.w_full().h_full().flex_row(),
        Axis::Vertical => this.h_full().w_full().flex_col(),
    })
    .map(|this| match self.position() {
        DockPosition::Left => this.border_r_1(),
        DockPosition::Right => this.border_l_1(),
        DockPosition::Bottom => this.border_t_1(),
    })
```

So the dock uses **`panel_background`** — the same color as the workspace sidebar. This is a darker/elevated color compared to `title_bar_background`.

---

## 3. The Dock Struct and Its Visible State

### File: `crates/workspace/src/dock.rs`

#### `Dock` struct (line ~110):

```rust
pub struct Dock {
    position: DockPosition,
    panel_entries: Vec<PanelEntry>,
    workspace: WeakEntity<Workspace>,
    is_open: bool,
    active_panel_index: Option<usize>,
    focus_handle: FocusHandle,
    focus_follows_mouse: FocusFollowsMouse,
    pub(crate) serialized_dock: Option<DockData>,
    zoom_layer_open: bool,
    modal_layer: Entity<ModalLayer>,
    _subscriptions: [Subscription; 2],
}
```

#### Key query methods:

| Method | Signature | Location | Description |
|--------|-----------|----------|-------------|
| `is_open` | `pub fn is_open(&self) -> bool` | line ~462 | Returns `self.is_open` |
| `active_panel` | `pub fn active_panel(&self) -> Option<&Arc<dyn PanelHandle>>` | line ~790 | Returns the active panel's `PanelHandle` (based on `active_panel_index`) |
| `active_panel_index` | `pub fn active_panel_index(&self) -> Option<usize>` | line ~487 | Returns the index of the active panel |
| `visible_panel` | `pub fn visible_panel(&self) -> Option<&Arc<dyn PanelHandle>>` | line ~783 | Returns active panel only if `is_open` is true |
| `position` | `pub fn position(&self) -> DockPosition` | line ~458 | Returns the dock's position (Left/Right/Bottom) |
| `stored_active_panel_size` | `pub fn stored_active_panel_size(&self, window: &Window, cx: &App) -> Option<Pixels>` | line ~697 | Gets pixel size of active panel |
| `stored_panel_size_state` | `pub fn stored_panel_size_state(&self, panel: &dyn PanelHandle) -> Option<PanelSizeState>` | line ~683 | Gets the full `PanelSizeState` (size + flex) for a panel |
| `has_agent_panel` | `pub fn has_agent_panel(&self, cx: &App) -> bool` | line ~812 | Whether any panel in this dock is an agent panel |
| `panels_len` | `pub fn panels_len(&self) -> usize` | line ~622 | Number of panel entries |

#### `PanelSizeState` (line ~163):

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PanelSizeState {
    pub size: Option<Pixels>,
    pub flex: Option<f32>,
}
```

#### Accessing the right dock from Workspace:

```rust
// File: workspace.rs:2154
pub fn right_dock(&self) -> &Entity<Dock>        // line 2154
pub fn left_dock(&self) -> &Entity<Dock>         // line 2131
pub fn bottom_dock(&self) -> &Entity<Dock>       // line 2135
pub fn all_docks(&self) -> [&Entity<Dock>; 3]    // line 2158
```

#### Typical usage pattern:

```rust
let right_dock = workspace.right_dock().read(cx);
if right_dock.is_open() {
    if let Some(active_panel) = right_dock.active_panel() {
        let name = active_panel.persistent_name();
        let size = right_dock.stored_active_panel_size(window, cx);
        // ...
    }
}
```

---

## 4. Panel Names/Titles

### The Panel trait has NO dedicated `title()` or `display_name()` method

**File: `crates/workspace/src/dock.rs:36`**

The `Panel` trait defines:

```rust
pub trait Panel: Focusable + EventEmitter<PanelEvent> + Render + Sized {
    fn persistent_name() -> &'static str;     // ← static class method, e.g. "Project Panel"
    fn panel_key() -> &'static str;           // ← static class method, e.g. "project-panel"
    fn icon_tooltip(&self, window: &Window, cx: &App) -> Option<&'static str>;  // ← instance, e.g. "Project Panel"
    // ...
}
```

### The `PanelHandle` trait exposes these via instance methods:

**File: `crates/workspace/src/dock.rs:98`**

```rust
pub trait PanelHandle: Send + Sync {
    fn persistent_name(&self) -> &'static str;          // line 100
    fn panel_key(&self) -> &'static str;                // line 101
    fn icon_tooltip(&self, window: &Window, cx: &App) -> Option<&'static str>;  // line 119
    // ...
}
```

### Best candidate for "display title": `icon_tooltip()`

The `icon_tooltip()` returns a human-readable label like "Project Panel", "Git Panel", etc. This is the closest thing to a "panel title" in the current API.

However, note that `icon_tooltip()` returns `Option<&'static str>` — it may be `None` or conditional:

| Panel | `persistent_name()` | `icon_tooltip()` | File:Line |
|-------|---------------------|-------------------|-----------|
| ProjectPanel | `"Project Panel"` | `Some("Project Panel")` | project_panel.rs:7261–7270 |
| GitPanel | `"GitPanel"` | `Some("Git Panel")` | git_panel.rs:6096–6127 |
| TerminalPanel | `"TerminalPanel"` | `Some("Terminal Panel")` | terminal_panel.rs:1632–1651 |
| OutlinePanel | `"Outline Panel"` | `Some("Outline Panel")` | outline_panel.rs:4875–4915 |
| DebugPanel | `"DebugPanel"` | `Some("Debug Panel")` (conditional on settings) | debugger_panel.rs:1539–1592 |

### If you need a guaranteed non-None label

Use `persistent_name()` as the fallback — it returns `&'static str` (never `None`). The values are stable identifiers but sometimes lack spaces (e.g., `"GitPanel"` vs `"Git Panel"`).

### Recommended approach for getting the active right-dock panel title:

```rust
let right_dock = workspace.right_dock().read(cx);
if right_dock.is_open() {
    if let Some(panel) = right_dock.active_panel() {
        let title: &str = panel
            .icon_tooltip(window, cx)
            .unwrap_or(panel.persistent_name());
    }
}
```

---

## 5. PlatformTitleBar Right-Side Window Controls

### File: `crates/platform_title_bar/src/platform_title_bar.rs:175` — `PlatformTitleBar::render()`

#### How sidebar affects window control visibility (line ~270–276):

```rust
let show_right_controls = !(sidebar.open && sidebar.side == SidebarSide::Right);
```

When the sidebar is open on the right side, `show_right_controls` becomes `false`, and the right window controls are **hidden entirely**. The sidebar then provides its own title-bar-level area (since the sidebar is a sibling of the workspace, it naturally covers the title bar area).

**For the dock panels, we do NOT want this behavior** — we want to keep window controls visible and just add a tinted background section with the panel title.

#### Right-side window controls dimensions (Linux)

**File: `crates/platform_title_bar/src/platforms/platform_linux.rs` — `LinuxWindowControls`**

The container for right-side Linux window controls:

```rust
h_flex()
    .id(self.id)
    .when(!button_elements.is_empty(), |el| {
        el.gap_3()       // 0.75rem gap between buttons (~12px at 16px/rem)
            .px_3()      // 0.75rem horizontal padding on each side (~12px each)
            .children(button_elements)
    })
```

Each button (`WindowControl`, same file):

```rust
h_flex()
    .w_5()              // 1.25rem = 20px
    .h_5()              // 1.25rem = 20px
```

**Estimated total width for 3 buttons (minimize/maximize/close):**

| Component | Calculation | Width |
|-----------|------------|-------|
| Left padding | `px_3` | ~12px |
| Button 1 (Minimize) | `w_5` | ~20px |
| Gap 1 | `gap_3` | ~12px |
| Button 2 (Maximize) | `w_5` | ~20px |
| Gap 2 | `gap_3` | ~12px |
| Button 3 (Close) | `w_5` | ~20px |
| Right padding | `px_3` | ~12px |
| **Total** | | **~108px** |

Variable at runtime: depends on `rem_size`. At 1rem = 16px: **108px**. Some systems may have 1, 2, or 3 buttons depending on the desktop environment's `gtk-decoration-layout` setting.

#### macOS traffic light buttons

On macOS, there are no right-side controls (traffic lights are on the left). The left-side traffic light area is ~71–78px (file: `crates/ui/src/utils/constants.rs`):

```rust
#[cfg(macos_sdk_26)]
pub const TRAFFIC_LIGHT_PADDING: f32 = 78.;

#[cfg(not(macos_sdk_26))]
pub const TRAFFIC_LIGHT_PADDING: f32 = 71.;
```

#### Right-side controls: Mac = None, Linux = 108px variable, Windows = Server-side

The `render_right_window_controls()` function (platform_title_bar.rs:60) returns `None` on Mac. On Linux with client-side decorations, it returns the `LinuxWindowControls` element. On Windows, decorations are server-side.

---

## Summary: How to Make the Right Dock Extend Into the Title Bar

### The core challenge

The workspace sidebar extends into the title bar because it's a **sibling** of the entire workspace in `MultiWorkspace::render()`. The right dock, however, is **inside** the workspace and below the title bar.

### Two possible approaches

#### Approach A: Move right dock rendering outside the workspace (like the sidebar)

**Pros:** Matches the existing sidebar pattern exactly.
**Cons:** Requires major refactoring of the `MultiWorkspace`/`Workspace` render pipeline. The dock interactions (resize, toggle, focus) are tightly coupled to `Workspace`. Would need to either duplicate or expose dock state to `MultiWorkspace`.

**Steps:**
1. In `MultiWorkspace::render()`, read the right dock's `is_open`, `active_panel`, and `stored_active_panel_size` from the workspace.
2. Render a wrapper div alongside the workspace (like `right_sidebar`) with `h_full().w(dock_width)` and `bg(panel_background)`.
3. Inside that wrapper, render the right_dock entity.
4. Remove right dock rendering from `Workspace::render()` when accessed from `MultiWorkspace`.
5. Pass `Tiling { right: right_dock_open, .. }` to `client_side_decorations` just like the sidebar.

#### Approach B: Add a tinted panel-title section in the title bar itself

**Pros:** Minimal structural changes. Dock stays inside workspace. Just the title bar gets an overlay div.
**Cons:** Need to carefully align the titlebar section with the dock width. The title bar and workspace are in different containers, so width alignment depends on matching CSS/layout properties.

**Steps:**
1. In `PlatformTitleBar::render()` (or `TitleBar::render()`), detect when the right dock is open (read from `Workspace` → `right_dock` → `is_open()`).
2. When the right dock is open, render a tinted background section on the right side of the title bar, with:
   - Width matching the right dock's width (from `stored_active_panel_size`).
   - Background color = `panel_background` (matching the dock).
   - The panel title text (from `active_panel().icon_tooltip()` or `persistent_name()`).
   - **Keep window controls visible** (unlike the sidebar which hides them). The tinted section should sit behind/around the window control buttons.
3. Account for the right window controls width (~108px on Linux, 0 on Mac) so the panel title text doesn't overlap them.

#### Recommended: Approach B

Approach B avoids the major refactoring of Approach A. The title bar already knows about sidebar state (`sidebar_render_state`), so adding dock state follows an established pattern. The key measurements you'll need:

- **Title bar height:** `platform_title_bar_height(window)` → `1.75 * rem_size` (minimum 34px)
- **Right dock width:** `workspace.right_dock().read(cx).stored_active_panel_size(window, cx)` → `Option<Pixels>`
- **Right window controls width:** ~108px on Linux (3 buttons × 20px + 2 gaps × 12px + 2 padding × 12px), 0 on Mac
- **Panel background color:** `cx.theme().colors().panel_background`
- **Panel title:** `right_dock.active_panel().icon_tooltip(window, cx).unwrap_or(persistent_name())`
