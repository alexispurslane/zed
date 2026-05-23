# Multiworkspace Sidebar Scout Report

## Executive Summary

The multiworkspace sidebar is a left-side panel that allows users to view, switch between, and manage multiple project workspaces within a single window. It is implemented primarily in two Rust files within the `workspace` crate, uses the GPUI declarative UI framework (Tailwind-like utility classes + Rust builder pattern), and has no dedicated CSS modules or external stylesheets. There are no stories or visual tests for the sidebar component specifically.

---

## 1. File Inventory

### Core Implementation

| File | Role |
|------|------|
| `crates/workspace/src/workspace_sidebar.rs` | **Primary sidebar UI component** — `WorkspaceSidebar` struct, its `Render` impl, click handlers, project group toggling |
| `crates/workspace/src/multi_workspace.rs` | **Multi-workspace controller** — `MultiWorkspace` struct, sidebar open/close/toggle logic, sidebar registration, drag-resize, `Render` impl that composes the sidebar alongside the active workspace |
| `crates/workspace/src/multi_workspace_tests.rs` | **Integration tests** for `MultiWorkspace` — project group key lifecycle, workspace activation, layout workspaces, serialization |

### Supporting Files

| File | Role |
|------|------|
| `crates/workspace/src/workspace.rs` | Re-exports `WorkspaceSidebar`, `Sidebar` trait, `SidebarSide`, etc.; contains `client_side_decorations` wrapper; restore-from-serialization logic |
| `crates/workspace/src/status_bar.rs` | Status bar includes a sidebar toggle button (when sidebar is closed) |
| `crates/workspace/src/persistence/model.rs` | `MultiWorkspaceState` struct (sidebar_open, project_groups, sidebar_state) for KVP persistence |
| `crates/workspace/src/persistence.rs` | Serialization/deserialization of sidebar state; `restore_open_sidebar` on session restore |
| `crates/xenomorphic/src/xenomorphic_app.rs` | App-level `initialize_workspace()` registers `WorkspaceSidebar` with each `MultiWorkspace` |
| `crates/settings_content/src/agent.rs` | `SidebarSide` enum (`Left` / `Right`) — configuring which side the sidebar appears on |
| `assets/keymaps/vim.json` | Only keymap with multiworkspace bindings: `]p` = `NextProject`, `[p` = `PreviousProject` |

### Key Public Exports (from `workspace` crate)

From `crates/workspace/src/workspace.rs`:
```rust
pub use multi_workspace::{
    CloseWorkspaceSidebar, DraggedSidebar, FocusWorkspaceSidebar, MoveProjectToNewWindow,
    MultiWorkspace, MultiWorkspaceEvent, NextProject, PreviousProject,
    ProjectGroup, ProjectGroupKey, SerializedProjectGroupState, Sidebar,
    SidebarEvent, SidebarHandle, SidebarRenderState, SidebarSide, ToggleWorkspaceSidebar,
};
pub use workspace_sidebar::WorkspaceSidebar;
```

---

## 2. Structural Layout (What It Renders)

### High-Level DOM Structure

The `MultiWorkspace::render()` output composes:

```
client_side_decorations(
  <root div>  (key_context, size_full, font, text_color)
    ├── <left_sidebar>   (if sidebar_open && !sidebar_on_right)
    │     ├── <AnyView from SidebarHandle>  (= WorkspaceSidebar)
    │     └── <sidebar-resize-handle>       (6px draggable column-resize handle)
    ├── <workspace container div> (flex-1, overflow-hidden)
    │     └── <Entity<Workspace>>
    ├── <right_sidebar>  (if sidebar_open && sidebar_on_right)
    │     ├── <AnyView from SidebarHandle>
    │     └── <sidebar-resize-handle>
    ├── <modal_layer>
    └── <sidebar_overlay>  (if set; positioned absolute, occluding)
  </root>
)
```

### WorkspaceSidebar Render Tree

The `WorkspaceSidebar::render()` output:

```
v_flex().size_full().track_focus(...)
  ├── header (h_flex, w_full, px_2, py_1, justify_between, items_center)
  │     ├── Label("Workspaces") [Small, Semibold]
  │     └── IconButton("close-sidebar", IconName::ThreadsSidebarLeftOpen) [Small, tooltip="Close Workspace Sidebar"]
  │
  ├── Divider::horizontal()
  │
  └── [conditional on has_groups]
        ├── "No projects open"  (if no groups)
        │     └── Label("No projects open") [Small, Color::Muted]
        │
        └── div#workspace-sidebar-list (flex_1, overflow_y_scroll)
              └── v_flex (w_full)
                    └── [for each ProjectGroup]
                          └── v_flex (w_full)
                                ├── group_header ListItem
                                │     .spacing(Dense)
                                │     .toggle_state(is_active)
                                │     └── h_flex (gap_1, items_center)
                                │           ├── div#expand-toggle (cursor_pointer, on_click → toggle expand/collapse)
                                │           │     └── Icon(chevron_icon) [Small]  ← ChevronDown if expanded, ChevronRight if collapsed
                                │           ├── Icon(IconName::Folder) [Small]
                                │           └── Label(project_name) [Small, single_line, truncate]
                                │     .on_click → toggle_project_group (same as chevron)
                                │
                                ├── [if is_expanded] workspace entry ListItems (one per workspace in group)
                                │     .spacing(Dense)
                                │     .toggle_state(is_active_workspace)
                                │     └── h_flex (gap_1, items_center, pl_3)
                                │           ├── Icon(IconName::File) [Small, color: Default|Muted]
                                │           └── Label(display_name) [Small, single_line, truncate, color: Default|Muted]
                                │     .on_click → activate that workspace
                                │
                                └── [if is_expanded] "New Workspace" ListItem
                                      .spacing(Dense)
                                      └── h_flex (gap_1, items_center, pl_3)
                                            ├── Icon(IconName::Plus) [Small, Color::Muted]
                                            └── Label("New Workspace") [Small, single_line, Color::Muted]
                                      .on_click → add_layout_workspace (if project already loaded) or open_project
```

### Visual Elements Summary

| Element | Icon | Label | Interactive? |
|---------|------|-------|-------------|
| Sidebar header | — | "Workspaces" (semibold) | Close button (× → `ThreadsSidebarLeftOpen` icon) |
| Project group header | `ChevronDown`/`ChevronRight` + `Folder` | Project name (from root path basename) | Yes — click toggles expand/collapse |
| Workspace entry | `File` | Workspace root name (disambiguated with index if duplicates) | Yes — click activates workspace |
| New Workspace button | `Plus` | "New Workspace" | Yes — click adds layout workspace or opens project |
| Empty state | — | "No projects open" | No |

---

## 3. Functional Behavior

### Workspace Listing

- Workspaces are organized into **project groups** (`ProjectGroup`), keyed by `ProjectGroupKey` (which encapsulates host + root path list from `PathList`).
- `MultiWorkspace::derived_project_groups()` derives groups from `project_groups: Vec<ProjectGroupState>` + `retained_workspaces: Vec<Entity<Workspace>>`.
- Each workspace's `project_group_key(cx)` determines which group it belongs to.
- Groups appear in the order they exist in `self.project_groups` (most-recently-added first).

### Workspace Selection

- Clicking a workspace entry calls `multi_workspace.activate(workspace, None, window, inner_cx)`.
- `activate()`:
  1. Retains the old active workspace (so it persists across switches).
  2. Registers the new workspace if it wasn't already retained.
  3. Sets `self.active_workspace = workspace`.
  4. Syncs sidebar focus handle to the new workspace.
  5. Emits `MultiWorkspaceEvent::ActiveWorkspaceChanged`.
  6. Serializes state and focuses the active workspace.

### Workspace Switching (Keyboard)

- `ToggleWorkspaceSidebar` — toggles sidebar open/closed.
- `FocusWorkspaceSidebar` — focuses the sidebar (or defocuses back to the previous focus if already focused).
- `CloseWorkspaceSidebar` — closes the sidebar only (no toggle).
- `NextProject` / `PreviousProject` — cycles through project groups via `Sidebar::cycle_project()`.
- `MoveProjectToNewWindow` — moves current project group to a new native window (only available when ≥2 groups).
- `NewWorkspaceLayout` — creates a new workspace tab sharing the same Project entity.
- Vim-only bindings: `]p` and `[p` for NextProject/PreviousProject.
- **No default keybindings** exist for `ToggleWorkspaceSidebar`, `FocusWorkspaceSidebar`, or `CloseWorkspaceSidebar` in the default keymaps.

### Adding Workspaces

- **"New Workspace" button** in each expanded project group:
  - If a workspace already exists for that group → calls `add_layout_workspace()` (creates a new `Workspace` sharing the same `Project` entity).
  - If no workspace loaded → calls `open_project()` (creates or reuses a workspace for those paths).
- `add_layout_workspace()`:
  1. Retains the old active workspace.
  2. Creates a new `Workspace` sharing the active workspace's `Project`.
  3. Registers, retains, and activates the new workspace.
  4. Both workspaces persist in the sidebar.

### Removing Workspaces

- `close_workspace(&workspace)` — removes a specific workspace:
  1. Finds a fallback workspace (prefers already-loaded neighbor in the same group, then neighboring groups, then creates an empty workspace).
  2. Calls `prepare_to_close()` on each workspace being removed (save prompts).
  3. Calls `detach_workspace()` which removes from retained list, clears session state, and emits `WorkspaceRemoved`.
- `remove_project_group(&group_key)` — removes all workspaces in a group.
- `open_project_group_in_new_window()` — serializes, removes from this window, opens in a new native window.

### Sidebar Open/Close/Toggle

- `toggle_sidebar()` — if open → close; if closed → save previous focus, open, focus sidebar.
- `close_sidebar()` — fires telemetry event, clears focus handles from workspaces, restores previous focus.
- `open_sidebar()` — fires telemetry, retains workspaces, syncs focus handle to all workspaces.
- `restore_open_sidebar()` — same as `open_sidebar()` but without telemetry (session restore).
- `focus_sidebar()` — toggles focus between sidebar and previous element without closing.

### Sidebar Resize

- A 6px invisible resize handle (`SIDEBAR_RESIZE_HANDLE_SIZE = px(6.0)`) is positioned absolutely at the sidebar's inner edge.
- Drag-moving adjusts the sidebar width.
- Double-click resets the sidebar width to default.
- Width is clamped: `MIN_WORKSPACE_SIDEBAR_WIDTH = px(192.)` to `MAX_WORKSPACE_SIDEBAR_WIDTH = px(400.)`.
- Default width: `DEFAULT_WORKSPACE_SIDEBAR_WIDTH = px(240.)`.

### Status Bar Integration

When the sidebar is closed, the status bar renders a toggle button:
- Left sidebar → button appears on the left side of the status bar.
- Right sidebar → button appears on the right side.
- Uses `IconName::ThreadsSidebarLeftClosed` or `IconName::ThreadsSidebarRightClosed`.
- On click → `multi_workspace.toggle_sidebar()`.

---

## 4. Styling / CSS Approach

### Framework

The codebase uses **GPUI**, a custom Rust UI framework with a **Tailwind-like builder API**. There are no CSS files, CSS modules, or styled-components. All styling is done through Rust method chains on element builders.

Example patterns:
```rust
// From WorkspaceSidebar::render()
v_flex()
    .size_full()
    .track_focus(&self.focus_handle(cx))
    .child(header)
    .child(ui::Divider::horizontal())

h_flex()
    .w_full()
    .px_2()
    .py_1()
    .justify_between()
    .items_center()
```

### Current Styling Details

| Component | Styling |
|-----------|---------|
| **Sidebar container** | `v_flex().size_full().track_focus(...)` — full height, full width, no explicit background (inherits theme default) |
| **Header bar** | `h_flex().w_full().px_2().py_1().justify_between().items_center()` — horizontal flex, padded, space-between |
| **Close button** | `IconButton` with `IconSize::Small` |
| **Divider** | `ui::Divider::horizontal()` — standard themed divider |
| **Project group list** | `div().id("workspace-sidebar-list").flex_1().overflow_y_scroll()` — scrollable, takes remaining space |
| **Project group header** | `ListItem` with `ListItemSpacing::Dense`, `toggle_state(is_active)` — themed selected/hover states |
| **Workspace entries** | `ListItem` with `ListItemSpacing::Dense`, `toggle_state(is_active_workspace)` — themed selection states |
| **Icons** | `IconSize::Small` (14px rems) for all sidebar icons |
| **Labels** | `LabelSize::Small` for all text; `.single_line().truncate()` for overflow |
| **Workspace entry text color** | `Color::Default` when active, `Color::Muted` when inactive |
| **New Workspace text color** | `Color::Muted` |
| **Empty state text** | `Color::Muted` |

### ListItem Styling (from `crates/ui/src/components/list/list_item.rs`)

The `ListItem` component provides:
- Hover: `ghost_element_hover` background
- Active: `ghost_element_active` background
- Selected: `ghost_element_selected` background
- Dense spacing: default (no extra vertical padding)
- Rounded corners when outlined
- Cursor pointer when clickable

### Theme Colors

The sidebar does not set any explicit background color — it inherits the window's default theme background. Key theme color tokens used:
- `cx.theme().colors().text` — root text color
- `cx.theme().colors().ghost_element_hover` — list item hover
- `cx.theme().colors().ghost_element_active` — list item active/pressed
- `cx.theme().colors().ghost_element_selected` — selected list item
- `Color::Muted` — de-emphasized text (inactive workspaces, placeholder labels)
- `Color::Default` — standard text (active workspaces)

---

## 5. Known Issues, TODOs, FIXMEs

### In Sidebar Code

- `crates/workspace/src/workspace_sidebar.rs:34` — `#[allow(dead_code)]` on the `fs` field of `WorkspaceSidebar`. The `fs: Arc<dyn Fs>` field is stored but never used — likely leftover from planned or removed functionality (e.g., file system operations from the sidebar).

### No TODO/FIXME comments

There are **no TODO, FIXME, HACK, or XXX comments** in either `workspace_sidebar.rs` or `multi_workspace.rs`.

### Design Notes

- `agent_sessions_alternative_design.md:762` mentions a planned "Phase 5: Remove Sidebar" step that would delete the `ToggleWorkspaceSidebar` thread list UI. This appears to be an older design doc and may not reflect current plans.
- The `CollapseProjectGroup` and `ExpandProjectGroup` actions are defined in `workspace_sidebar.rs` but have **no keybindings** and **no handlers** registered — they are currently unused.
- No default keybindings for `ToggleWorkspaceSidebar`, `FocusWorkspaceSidebar`, or `CloseWorkspaceSidebar` exist in the default macOS/Linux/Windows keymaps. Only the vim keymap has `]p`/`[p` for project cycling.

---

## 6. Design Tokens, Theme, and Shared UI Components

### Theme System

- The project uses a custom theme system via `crates/theme`. Theme colors are accessed via `cx.theme().colors()`.
- No sidebar-specific theme tokens exist (no `sidebar_background`, etc.).
- The sidebar inherits the parent container's appearance.

### Shared UI Components Used by the Sidebar

| Component | Source | Usage |
|-----------|--------|-------|
| `ListItem` | `crates/ui/src/components/list/list_item.rs` | Project group headers, workspace entries, new workspace button |
| `Label` | `crates/ui/src/components/label.rs` | All text labels (header, project names, workspace names, empty state) |
| `Icon` | `crates/ui/src/components/icon.rs` | ChevronDown/Right, Folder, File, Plus icons |
| `IconButton` | `crates/ui/src/components/button/icon_button.rs` | Close sidebar button |
| `Tooltip` | `crates/ui/src/components/tooltip.rs` | Action tooltips on close button |
| `Divider` | `crates/ui/src/components/divider.rs` | Horizontal divider below header |
| `h_flex` / `v_flex` | `crates/ui/src/components/stack.rs` | Layout primitives (flex containers) |
| `Color` enum | `crates/ui/src/styles/color.rs` | Semantic color tokens (Default, Muted, etc.) |
| `IconSize` enum | `crates/ui/src/components/icon.rs` | Small (14px) for all sidebar icons |
| `LabelSize` enum | `crates/ui/src/components/label.rs` | Small for all sidebar labels |
| `ListItemSpacing` enum | `crates/ui/src/components/list/list_item.rs` | Dense spacing for compact layout |
| `SidebarSide` enum | `crates/settings_content/src/agent.rs` | Configures sidebar position (Left/Right) |

### Icon Names Used

| Icon | Context |
|------|---------|
| `IconName::ChevronDown` | Expanded project group |
| `IconName::ChevronRight` | Collapsed project group |
| `IconName::Folder` | Project group header |
| `IconName::File` | Workspace entry |
| `IconName::Plus` | "New Workspace" button |
| `IconName::ThreadsSidebarLeftOpen` | Close sidebar button (in sidebar header) |
| `IconName::ThreadsSidebarLeftClosed` | Toggle sidebar button (in status bar, left side) |
| `IconName::ThreadsSidebarRightClosed` | Toggle sidebar button (in status bar, right side) |

---

## 7. Data Flow & State Model

### Key Data Structures

```rust
// In MultiWorkspace:
project_groups: Vec<ProjectGroupState>,  // Ordered list of project group metadata
retained_workspaces: Vec<Entity<Workspace>>,  // All persistent workspaces
active_workspace: Entity<Workspace>,  // Currently displayed workspace
sidebar: Option<Box<dyn SidebarHandle>>,  // The sidebar (WorkspaceSidebar)
sidebar_open: bool,  // Whether sidebar is visible
sidebar_overlay: Option<AnyView>,  // Optional overlay rendered on top

// ProjectGroupState:
pub struct ProjectGroupState {
    pub key: ProjectGroupKey,      // host + PathList
    pub expanded: bool,            // UI expand/collapse state
    pub last_active_workspace: Option<WeakEntity<Workspace>>,
}

// ProjectGroup (derived for rendering):
pub struct ProjectGroup {
    pub key: ProjectGroupKey,
    pub workspaces: Vec<Entity<Workspace>>,
    pub expanded: bool,
}
```

### Persistence

- **MultiWorkspaceState** (`persistence/model.rs`): serialized to KVP (key-value store) per window.
  - `active_workspace_id`
  - `sidebar_open`
  - `project_groups: Vec<SerializedProjectGroup>`
  - `sidebar_state: Option<String>` (opaque JSON blob from sidebar's `serialized_state()`)
- **WorkspaceSidebar**: serializes its width (`f32`) as JSON; restores from it on session restore.

### Event Flow

```
User clicks workspace entry
  → WorkspaceSidebar::on_click handler
    → MultiWorkspace::activate(workspace, None, window, cx)
      → Retains old workspace, registers new if needed
      → Sets active_workspace
      → Emits MultiWorkspaceEvent::ActiveWorkspaceChanged
      → Serializes state
      → cx.notify() (triggers re-render)

User clicks project group header
  → WorkspaceSidebar::toggle_project_group()
    → MultiWorkspace::group_state_by_key_mut(key).expanded = !expanded
    → Emits SidebarEvent::SerializeNeeded
    → cx.notify()

User clicks close sidebar button
  → IconButton on_click → MultiWorkspace::close_sidebar_action()
    → MultiWorkspace::close_sidebar()
      → Emits telemetry
      → sidebar_open = false
      → Restores previous focus
      → Serializes
      → cx.notify()

User drags resize handle
  → MultiWorkspace::on_drag_move handler
    → Calculates new width based on mouse position and sidebar side
    → sidebar.set_width(Some(new_width), cx)
    → WorkspaceSidebar clamps to [192, 400]px
```

---

## 8. Test Coverage

The test file `crates/workspace/src/multi_workspace_tests.rs` covers:

| Test | What It Tests |
|------|---------------|
| `test_project_group_keys_initial` | One project group key created on MultiWorkspace init |
| `test_project_group_keys_add_workspace` | Adding a workspace with different root adds a new key |
| `test_open_new_window_does_not_open_sidebar_on_existing_window` | New-window open doesn't open sidebar on existing window |
| `test_open_directory_in_empty_workspace_does_not_open_sidebar` | Opening a directory in a blank workspace doesn't open sidebar |
| `test_project_group_keys_duplicate_not_added` | Same root path doesn't duplicate project group keys |
| `test_adding_worktree_updates_project_group_key` | Adding a worktree triggers key change and group update |
| `test_find_or_create_local_workspace_reuses_active_workspace` | Reopening same path reuses active workspace |
| `test_find_or_create_workspace_uses_project_group_key_when_paths_are_missing` | Missing worktree paths fall back to project group key |
| `test_find_or_create_local_workspace_reuses_active_workspace_after_sidebar_open` | Sidebar open retains workspace, subsequent open reuses it |
| `test_close_workspace_prefers_already_loaded_neighboring_workspace` | Closing workspace prefers loaded neighbor over creating new |
| `test_remote_project_root_dir_changes_update_groups` | Remote worktree updates change project group keys |
| `test_open_project_retains_existing_workspaces` | Opening a project retains previous workspaces |
| `test_add_layout_workspace` | Layout workspace shares same Project entity |
| `test_close_layout_workspace_falls_back_to_sibling` | Closing layout workspace falls back to sibling |
| `test_layout_workspaces_track_independent_active_entries` | Layout workspaces track different active entries independently |
| `test_set_active_entry_deduplicates` | Setting same active entry is a no-op |

**Notable gap:** There are no tests for the `WorkspaceSidebar` UI component itself (its `Render` impl, click handlers, etc.). All tests exercise the `MultiWorkspace` logic layer.

---

## 9. Key Code Snippets

### Sidebar Trait Definition
`crates/workspace/src/multi_workspace.rs:92-115`
```rust
pub trait Sidebar: Focusable + Render + EventEmitter<SidebarEvent> + Sized {
    fn width(&self, cx: &App) -> Pixels;
    fn set_width(&mut self, width: Option<Pixels>, cx: &mut Context<Self>);
    fn has_notifications(&self, cx: &App) -> bool;
    fn side(&self, _cx: &App) -> SidebarSide;
    fn cycle_project(&mut self, _forward: bool, _window: &mut Window, _cx: &mut Context<Self>) {}
    fn serialized_state(&self, _cx: &App) -> Option<String> { None }
    fn restore_serialized_state(&mut self, _state: &str, _window: &mut Window, _cx: &mut Context<Self>) {}
}
```

### WorkspaceSidebar Instantiation
`crates/xenomorphic/src/xenomorphic_app.rs:450-453`
```rust
let sidebar = cx.new(|cx| {
    WorkspaceSidebar::new(multi_workspace_handle, fs, window, cx)
});
_multi_workspace.register_sidebar(sidebar, cx);
```

### Sidebar Resize Constants
`crates/workspace/src/workspace_sidebar.rs:27-29`
```rust
const DEFAULT_WORKSPACE_SIDEBAR_WIDTH: Pixels = px(240.);
const MIN_WORKSPACE_SIDEBAR_WIDTH: Pixels = px(192.);
const MAX_WORKSPACE_SIDEBAR_WIDTH: Pixels = px(400.);
```

### Workspace Disambiguation Logic
`crates/workspace/src/workspace_sidebar.rs:236-247`
```rust
let disambiguation_index = if workspaces.iter().any(|ws| {
    ws.read(cx).project().entity_id() == project.entity_id()
        && ws != workspace
}) {
    Some(index + 1)
} else {
    None
};
```

---

*Report generated by multiworkspace sidebar scout. All file paths relative to repository root.*
