# Sidebar Prerequisites Scout Report

This document provides detailed technical findings for implementing UI changes to the multiworkspace sidebar in Xenomorphic (a Zed fork).

---

## 1. Project Panel Background Color

### Key Files
- `crates/project_panel/src/project_panel.rs`
- `crates/theme/src/styles/colors.rs`
- `crates/theme/src/default_colors.rs`

### Background Color Used

The project panel **does not explicitly set a background color** on its root container element. Instead, it relies on the default transparency of the `h_flex()` container, and the background that shows through comes from:

1. **`cx.theme().colors().panel_background`** — This is the theme color used by panels. It's used for the custom scrollbar's horizontal track at line 7095:
   ```rust
   // project_panel.rs:7095
   scrollbars = scrollbars.with_track_along(
       ScrollAxes::Horizontal,
       cx.theme().colors().panel_background,
   );
   ```

2. **Item colors** — The `get_item_color` function (line 565) uses `panel_background` for default items:
   ```rust
   // project_panel.rs:565-575
   fn get_item_color(is_sticky: bool, cx: &App) -> ItemColors {
       let colors = cx.theme().colors();
       ItemColors {
           default: if is_sticky {
               colors.panel_overlay_background
           } else {
               colors.panel_background
           },
           // ...
       }
   }
   ```

3. **The `panel_background` theme color definition** (from `crates/theme/src/default_colors.rs`):
   ```rust
   // default_colors.rs:95 (Light theme)
   panel_background: neutral().light().step_2(),

   // default_colors.rs:248 (Dark theme)
   panel_background: neutral().dark().step_2(),
   ```

4. Similarly, **`panel_overlay_background`** is used for sticky items:
   ```rust
   // default_colors.rs:100 (Light theme)
   panel_overlay_background: neutral().light().step_2(),
   // default_colors.rs:253 (Dark theme)
   panel_overlay_background: neutral().dark().step_2(),
   ```

### How to Match for the Sidebar

To make the workspace sidebar match the project panel, you should use:
```rust
.bg(cx.theme().colors().panel_background)
```

The outline panel similarly does not set an explicit background on its root `v_flex()`, relying on the same `panel_background` appearing through. The outline panel is at `crates/outline_panel/src/outline_panel.rs:4985`.

---

## 2. Context Menu System

### Key Files
- `crates/ui/src/components/context_menu.rs` — Main `ContextMenu` and `ContextMenuEntry` types
- `crates/ui/src/components/right_click_menu.rs` — `RightClickMenu` helper for right-click menus
- `crates/project_panel/src/project_panel.rs:1019` — Full working example in `deploy_context_menu()`
- `crates/platform_title_bar/src/system_window_tabs.rs:288` — Right-click menu on window tabs

### Architecture Overview

The context menu system has two main approaches:

#### A. `ContextMenu::build()` — Static/Direct Build

Used for popover menus and programmatically-deployed context menus. Returns an `Entity<ContextMenu>`.

```rust
ContextMenu::build(window, cx, |menu, _, cx| {
    menu.action("New File", Box::new(NewFile))           // action item
        .action("New Folder", Box::new(NewDirectory))   // action item
        .separator()                                     // horizontal separator
        .when(is_local, |menu| {                         // conditional items
            menu.action("Reveal in Finder", Box::new(RevealInFileManager))
        })
        .entry("Cut", None, |window, cx| {             // entry with custom handler
            // custom handler logic
        })
        .action_disabled_when(!has_pasteable, "Paste", Box::new(Paste))  // disabled entry
        .submenu("Submenu Label", |menu, window, cx| {  // nested submenu
            menu.action("Item 1", Box::new(SomeAction))
                .action("Item 2", Box::new(AnotherAction))
        })
        .submenu_with_icon("Icon Submenu", IconName::Settings, |menu, window, cx| {
            menu.action("Item", Box::new(Action))
        })
        .context(self.focus_handle.clone())              // sets focus context
})
```

#### B. `right_click_menu()` — Declarative Right-Click Menus

This is a UI element that attaches to a child element and shows a menu on right-click:

```rust
// From system_window_tabs.rs:288
let menu = right_click_menu(ix)
    .trigger(|_, _, _| tab)   // The child element that triggers the menu
    .menu(move |window, cx| {  // Builder for the menu
        let focus_handle = cx.focus_handle();
        ContextMenu::build(window, cx, move |mut menu, _window_, _cx| {
            menu = menu.entry("Close Tab", None, move |window, cx| {
                // handler
            });
            menu = menu.entry("Close Other Tabs", None, move |window, cx| {
                // handler
            });
            menu.context(focus_handle)
        })
    });
```

#### C. `PopoverMenu<ContextMenu>` — Click-Triggered Popup Menus (like the user menu)

Used when you want a button that opens a dropdown menu:

```rust
PopoverMenu::new("user-menu")
    .trigger(trigger_button)
    .menu(move |window, cx| {
        ContextMenu::build(window, cx, |menu, _, _cx| {
            menu.action("Settings", xenomorphic_actions::OpenSettings.boxed_clone())
                .separator()
                .toggleable_entry("Classic", is_editor, IconPosition::Start, None, |window, cx| {
                    // handler
                })
        })
    })
```

### ContextMenu API Summary

| Method | Purpose |
|--------|---------|
| `.action(label, boxed_action)` | Adds an item that dispatches an action |
| `.action_disabled_when(disabled, label, boxed_action)` | Conditionally disabled action |
| `.action_checked(label, action, checked)` | Action with a checkmark |
| `.entry(label, action, handler)` | Item with custom handler |
| `.entry_with_end_slot(label, action, handler, icon, title, handler)` | Item with icon on the right |
| `.toggleable_entry(label, toggled, position, action, handler)` | Toggle/checkmark item |
| `.separator()` | Horizontal divider |
| `.header(title)` | Non-selectable header text |
| `.label(text)` | Non-selectable label |
| `.submenu(label, builder)` | Nested submenu |
| `.submenu_with_icon(label, icon, builder)` | Submenu with icon |
| `.submenu_with_colored_icon(label, icon, color, builder)` | Submenu with colored icon |
| `.custom_row(render_fn)` | Custom non-selectable row |
| `.custom_entry(render_fn, handler)` | Custom selectable row with handler |
| `.context(focus_handle)` | Set the focus context for the menu |
| `.keep_open_on_confirm(bool)` | Keep menu open after selecting an item |
| `.fixed_width(width)` | Fixed width for the menu |
| `.link(label, action)` | Link-style item |

### ContextMenuEntry Builder

```rust
ContextMenuEntry::new("Label")
    .icon(IconName::Pencil)
    .icon_color(Color::Accent)
    .icon_position(IconPosition::Start)
    .handler(|window, cx| { /* ... */ })
    .action(Box::new(SomeAction))
    .disabled(true)
```

### Full Working Example: Project Panel Context Menu

**File**: `crates/project_panel/src/project_panel.rs:1019-1119`

```rust
fn deploy_context_menu(
    &mut self,
    position: Point<Pixels>,
    entry_id: ProjectEntryId,
    window: &mut Window,
    cx: &mut Context<Self>,
) {
    // ... setup code ...
    let context_menu = ContextMenu::build(window, cx, |menu, _, cx| {
        menu.context(self.focus_handle.clone()).map(|menu| {
            if is_read_only {
                menu.when(is_dir, |menu| {
                    menu.action("Search Inside", Box::new(NewSearchInDirectory))
                })
            } else {
                menu.action("New File", Box::new(NewFile))
                    .action("New Folder", Box::new(NewDirectory))
                    .separator()
                    .when(is_local, |menu| {
                        menu.action("Reveal in Finder", Box::new(RevealInFileManager))
                    })
                    .separator()
                    .action("Cut", Box::new(Cut))
                    .action("Copy", Box::new(Copy))
                    // ...
            }
        })
    });
    // ... position and display the menu ...
}
```

The context menu is displayed by the project panel by storing it and rendering it in a deferred overlay:

```rust
// Line ~7130
.children(self.context_menu.as_ref().map(|(menu, position, _)| {
    deferred(
        anchored().position(*position).child(menu.clone()),
    )
    .with_priority(1)
}))
```

---

## 3. Title Bar / Client Side Decorations

### Key Files
- `crates/workspace/src/workspace.rs:10471` — `client_side_decorations()` function
- `crates/title_bar/src/title_bar.rs:144` — `TitleBar::render()`
- `crates/platform_title_bar/src/platform_title_bar.rs` — `PlatformTitleBar::render()`
- `crates/workspace/src/multi_workspace.rs:2044` — `MultiWorkspace::render()`

### How It Works

The call chain is:

1. **`MultiWorkspace::render()`** (multi_workspace.rs:2044) — The root view for each window. It calls `client_side_decorations()` wrapping the entire layout:
   ```rust
   client_side_decorations(
       root.key_context(workspace_key_context)
           .relative()
           .size_full()
           .font(ui_font)
           .text_color(text_color)
           .on_action(cx.listener(Self::close_window))
           // ... action handlers ...
           .children(left_sidebar)        // ← workspace sidebar (left)
           .child(
               div().flex().flex_1().size_full().overflow_hidden()
                   .child(self.workspace().clone())
           )
           .children(right_sidebar)       // ← workspace sidebar (right)
           .child(self.workspace().read(cx).modal_layer.clone())
           // ... overlay ...
       ,
       window,
       cx,
       Tiling {
           left: !sidebar_on_right && self.sidebar.is_some() && self.sidebar_open(),
           right: sidebar_on_right && self.sidebar.is_some() && self.sidebar_open(),
           ..Tiling::default()
       },
   )
   ```

2. **`client_side_decorations()`** (workspace.rs:10471) — Wraps the element with:
   - An outer `div()` with `id("window-backdrop")`, transparent black background
   - If client-side decorations: rounded corners, shadows, borders
   - If server-side decorations: no extra decoration
   - Handles window resize edges for CSD

3. **Title bar is per-workspace** — The `TitleBar` is set as the workspace's `titlebar_item` (title_bar.rs:63-66):
   ```rust
   let item = cx.new(|cx| TitleBar::new("title-bar", workspace, multi_workspace, window, cx));
   workspace.set_titlebar_item(item.into(), window, cx);
   ```

4. **`PlatformTitleBar::render()`** (platform_title_bar.rs:186-327) renders the actual title bar structure:
   ```rust
   h_flex()
       .window_control_area(WindowControlArea::Drag)
       .w_full()
       .h(height)
       // ... mouse handlers for dragging ...
       // Left side: traffic light or left window controls
       .map(|this| {
           let show_left_controls = !(sidebar.open && sidebar.side == SidebarSide::Left);
           if window.is_fullscreen() {
               this.pl_2()
           } else if self.platform_style == PlatformStyle::Mac && show_left_controls {
               this.pl(px(TRAFFIC_LIGHT_PADDING))
           } else if let Some(controls) = show_left_controls.then(|| { ... }).flatten() {
               this.child(controls)
           } else {
               this.pl_2()
           }
       })
       .bg(titlebar_color)
       .content_stretch()
       .child(
           div().id(self.id.clone()).flex().flex_row()
               .items_center().justify_between().overflow_x_hidden().w_full()
               .children(children)  // ← Left children + right children from TitleBar
       )
       .when(!window.is_fullscreen(), |title_bar| {
           let show_right_controls = !(sidebar.open && sidebar.side == SidebarSide::Right);
           title_bar.children(
               show_right_controls
                   .then(|| render_right_window_controls(button_layout, close_action, window))
                   .flatten(),
           )
       })
   ```

   **Key insight for adding a toggle button**: The window control buttons (close, minimize, maximize) are rendered by `render_right_window_controls()` on the right side of the `PlatformTitleBar`. When the sidebar is open, `show_right_controls` becomes `false` when `sidebar.side == SidebarSide::Right`, and `show_left_controls` becomes `false` when `sidebar.side == SidebarSide::Left`. This means the window controls are **hidden** when the sidebar is on that side — the sidebar panel itself is touching the edge.

   You would add a toggle button in the `PlatformTitleBar` rendering, or in the `TitleBar::render()` where it pushes children into the `PlatformTitleBar`. The multi_workspace is already available in `PlatformTitleBar` via `self.multi_workspace`.

5. **`TitleBar::render()`** (title_bar.rs:144) pushes children into `PlatformTitleBar`:
   - Left children: application menu, project host, project name, branch info
   - Right children: connection status, update version, user menu button

### Where to Add a Sidebar Toggle Button

The best place would be in `PlatformTitleBar::render()`, specifically in the right-side controls area. You could add a toggle button before the close button, or as part of the title bar children. The `PlatformTitleBar` already has access to `self.multi_workspace` and knows the `sidebar` render state.

---

## 4. Available Icon Names

**File**: `crates/icons/src/icons.rs`

The complete `IconName` enum:

```
AiAnthropic, AiBedrock, AiClaude, AiDeepSeek, AiEdit, AiGemini, AiGoogle,
AiLmStudio, AiMistral, AiOllama, AiOpenAi, AiOpenAiCompat, AiOpenCode,
AiOpenRouter, AiVercel, AiXAi, AiXenomorphic,

Archive, ArrowCircle, ArrowDown, ArrowDown10, ArrowDownRight, ArrowLeft,
ArrowRight, ArrowRightLeft, ArrowUp, ArrowUpRight, AtSign, Attach, AudioOff,
AudioOn, Backspace, Bell, BellDot, BellOff, BellRing, Binary, Blocks,
Bookmark, BoltFilled, BoltOutlined, Book, BookCopy, Box, BoxOpen,
CaseSensitive, Chat, Check, CheckDouble, ChevronDown, ChevronDownUp,
ChevronLeft, ChevronRight, ChevronUp, ChevronUpDown, Circle, CircleHelp,
Clock, Close, CloudDownload, Code, Command, Control, Copilot,
CopilotDisabled, CopilotError, CopilotInit, Copy, CountdownTimer,
Crosshair, CursorIBeam, Dash, DatabaseZap, Debug, DebugBreakpoint,
DebugContinue, DebugDetach, DebugDisabledBreakpoint, DebugDisabledLogBreakpoint,
DebugIgnoreBreakpoints, DebugLogBreakpoint, DebugPause, DebugStepInto,
DebugStepOut, DebugStepOver, Diff, DiffSplit, DiffSplitAuto, DiffUnified,
Disconnected, Download, EditorAtom, EditorCursor, EditorEmacs, EditorJetBrains,
EditorSublime, EditorVsCode, Ellipsis, Envelope, Eraser, Escape, Exit,
ExpandDown, ExpandUp, ExpandVertical, Eye, EyeOff, FastForward, FastForwardOff,
File, FileCode, FileDiff, FileDoc, FileGeneric, FileGit, FileLock,
FileMarkdown, FileRust, FileTextFilled, FileTextOutlined, FileToml, FileTree,
Filter, Flame, Folder, FolderOpen, FolderOpenAdd, FolderSearch, Font,
FontSize, FontWeight, ForwardArrow, ForwardArrowUp, GenericClose,
GenericMaximize, GenericMinimize, GenericRestore, GitBranch, GitBranchPlus,
GitCommit, GitGraph, GitMergeConflict, GitWorktree, Github, Hash,
HistoryRerun, Image, Inception, Indicator, Info, Json, Keyboard, Library,
LineHeight, Link, Linux, ListCollapse, ListFilter, ListTodo, ListTree,
ListX, LoadCircle, LocationEdit, LockOutlined, MagnifyingGlass, Maximize,
MaximizeAlt, Menu, MenuAltTemp, Mic, MicMute, Minimize, NewThread, Notepad,
OpenFolder, Option, PageDown, PageUp, Paperclip, Pencil, PencilUnavailable,
Person, Pin, PlayFilled, PlayOutlined, Plus, Power, Public, PullRequest,
QueueMessage, Quote, Reader, RefreshTitle, Regex, ReplNeutral, Replace,
ReplaceAll, ReplaceNext, ReplyArrowRight, Rerun, Return, RotateCcw,
RotateCw, Scissors, Screen, SelectAll, Send, Server, Settings, Shift,
SignalHigh, SignalLow, SignalMedium, Slash, Sliders, Space, Sparkle,
Split, SplitAlt, SquareDot, SquareMinus, SquarePlus, Star, StarFilled,
Stop, Tab, Terminal, TerminalAlt, TextSnippet, ThinkingMode, ThinkingModeOff,
Thread, ThreadFromSummary, ThreadsSidebarLeftClosed, ThreadsSidebarLeftOpen,
ThreadsSidebarRightClosed, ThreadsSidebarRightOpen, ThumbsDown, ThumbsUp,
TodoComplete, TodoPending, TodoProgress, ToolCopy, ToolDeleteFile,
ToolDiagnostics, ToolFolder, ToolHammer, ToolNotification, ToolPencil,
ToolSearch, ToolTerminal, ToolThink, ToolWeb, Trash, Triangle,
TriangleRight, Undo, Unpin, UserCheck, UserGroup, UserRoundPen, Warning,
WholeWord, XCircle, XCircleFilled,

XenomorphicAgent, XenomorphicAgentTwo, XenomorphicAssistant,
XenomorphicPredict, XenomorphicPredictDisabled, XenomorphicPredictDown,
XenomorphicPredictError, XenomorphicPredictUp, XenomorphicSrcCustom,
XenomorphicSrcExtension
```

### Recommendations for Workspace Icons

- **Project group** already uses `IconName::Folder` — consider keeping it or using `FolderOpen` for expanded groups.
- **Workspace/layout tabs** currently use `IconName::File` — this is reasonable but could also be:
  - `IconName::Window` → Not available
  - `IconName::Screen` → Could represent a workspace/layout
  - `IconName::Tab` → Could represent a tab-like workspace layout
  - `IconName::Split` → Could represent a split workspace
  - `IconName::ThreadsSidebarLeftClosed` / `ThreadsSidebarLeftOpen` → Already used for the sidebar close/open toggle button
- **Sidebar toggle button** would use something like `ThreadsSidebarLeftClosed` / `ThreadsSidebarLeftOpen` (already used in `workspace_sidebar.rs` for the close button).

---

## 5. Workspace Rename

### Key Files
- `crates/workspace/src/workspace.rs:1341` — `Workspace` struct definition
- `crates/workspace/src/workspace_sidebar.rs` — Current sidebar implementation
- `crates/git_ui/src/git_ui.rs:316` — `RenameBranchModal` pattern

### Findings

**Workspaces do NOT have names.** The `Workspace` struct has no `name` or `display_name` field. Instead, the sidebar derives display names from the project's root paths:

```rust
// workspace_sidebar.rs:125-133
fn project_name(paths: &PathList) -> SharedString {
    let joined = paths
        .ordered_paths()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .collect::<Vec<_>>()
        .join(", ");
    if joined.is_empty() {
        "Untitled".into()
    } else {
        joined.into()
    }
}
```

Each workspace entry in the sidebar derives its name similarly from `root_paths`:

```rust
// workspace_sidebar.rs:233-242
let root_paths = workspace.read(cx).root_paths(cx);
let base_name: SharedString = if root_paths.is_empty() {
    "Empty".into()
} else {
    root_paths
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .collect::<Vec<_>>()
        .join(", ")
        .into()
};
```

**If you want to add workspace names**, you would need to:
1. Add a `name: Option<String>` field to the `Workspace` struct
2. Persist it through the existing serialization system
3. Fall back to the path-derived name when `name` is `None`

### Rename Modal Pattern

The `RenameBranchModal` in `git_ui/src/git_ui.rs:316` provides a complete pattern for a rename/edit modal:

```rust
struct RenameBranchModal {
    current_branch: SharedString,
    editor: Entity<Editor>,  // Uses Editor::single_line() for the input
    repo: Entity<Repository>,
}

impl RenameBranchModal {
    fn new(
        current_branch: String,
        repo: Entity<Repository>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_text(current_branch.clone(), window, cx);
            editor
        });
        Self { current_branch: current_branch.into(), editor, repo }
    }

    fn cancel(&mut self, _: &Cancel, _window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }

    fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        let new_name = self.editor.read(cx).text(cx);
        if new_name.is_empty() || new_name == self.current_branch.as_ref() {
            cx.emit(DismissEvent);
            return;
        }
        // ... do the rename ...
        cx.emit(DismissEvent);
    }
}

impl ModalView for RenameBranchModal {}
impl EventEmitter<DismissEvent> for RenameBranchModal {}
impl Focusable for RenameBranchModal {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.editor.focus_handle(cx)
    }
}

impl Render for RenameBranchModal {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context("RenameBranchModal")
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::confirm))
            .elevation_2(cx)
            .w(rems(34.))
            .child(
                h_flex().px_3().pt_2().pb_1().w_full().gap_1p5()
                    .child(Icon::new(IconName::GitBranch).size(IconSize::XSmall))
                    .child(Headline::new(format!("Rename Branch ({})", self.current_branch))
                        .size(HeadlineSize::XSmall)),
            )
            .child(div().px_3().pb_3().w_full().child(self.editor.clone()))
    }
}
```

To show a modal, the codebase uses `workspace.toggle_modal(window, cx, || RenameBranchModal::new(...))`.

Key traits needed:
- `impl ModalView for YourModal {}`
- `impl EventEmitter<DismissEvent> for YourModal {}`
- `impl Focusable for YourModal { fn focus_handle(...) }`
- `impl Render for YourModal { fn render(...) }`

---

## 6. Moving Workspaces to Other Windows

### Key Files
- `crates/workspace/src/multi_workspace.rs:45` — `MoveProjectToNewWindow` action definition
- `crates/workspace/src/multi_workspace.rs:1054` — `open_project_group_in_new_window()` implementation
- `crates/workspace/src/multi_workspace.rs:2159` — Action handler
- `crates/gpui/src/app.rs:1092` — `App::windows()` method
- `crates/gpui/src/window.rs:5620` — `AnyWindowHandle::downcast::<T>()` method

### How MoveProjectToNewWindow Works

The action is defined at `multi_workspace.rs:45`:
```rust
actions!(
    multi_workspace,
    [
        ToggleWorkspaceSidebar,
        CloseWorkspaceSidebar,
        FocusWorkspaceSidebar,
        NextProject,
        PreviousProject,
        MoveProjectToNewWindow,  // ← this one
        NewWorkspaceLayout,
    ]
);
```

The action handler at `multi_workspace.rs:2159`:
```rust
.when(self.project_group_keys().len() >= 2, |el| {
    el.on_action(cx.listener(
        |this: &mut Self, _: &MoveProjectToNewWindow, window, cx| {
            let key = this.project_group_key_for_workspace(this.workspace(), cx);
            this.open_project_group_in_new_window(&key, window, cx)
                .detach_and_log_err(cx);
        },
    ))
})
```

**Note**: The `MoveProjectToNewWindow` action is currently only registered when there are 2+ project groups. It moves the **entire project group** (all workspaces for that project) to a new window.

The implementation at `multi_workspace.rs:1054`:
```rust
pub fn open_project_group_in_new_window(
    &mut self,
    key: &ProjectGroupKey,
    window: &mut Window,
    cx: &mut Context<Self>,
) -> Task<Result<()>> {
    let paths: Vec<PathBuf> = key.path_list().ordered_paths().cloned().collect();
    if paths.is_empty() {
        return Task::ready(Ok(()));
    }

    let app_state = self.workspace().read(cx).app_state().clone();

    // Serialize all workspaces in the group before moving
    let workspaces: Vec<_> = self.workspaces_for_project_group(key, cx).unwrap_or_default();
    let mut serialization_tasks = Vec::new();
    for workspace in &workspaces {
        serialization_tasks.push(workspace.update(cx, |workspace, inner_cx| {
            workspace.flush_serialization(window, inner_cx)
        }));
    }

    // Remove from current window
    let remove_task = self.remove_project_group(key, window, cx);

    // Spawn new window
    cx.spawn(async move |_this, cx| {
        futures::future::join_all(serialization_tasks).await;
        let removed = remove_task.await?;
        if !removed { return Ok(()); }

        cx.update(|cx| {
            Workspace::new_local(paths, app_state, None, None, None, OpenMode::NewWindow, cx)
        }).await?;

        Ok(())
    })
}
```

### Enumerating Other Open Windows / MultiWorkspace Instances

To show a submenu listing other open windows, you need to enumerate all `App` windows and find their `MultiWorkspace` roots.

**`App::windows()`** (gpui/src/app.rs:1092):
```rust
pub fn windows(&self) -> Vec<AnyWindowHandle> {
    self.windows
        .keys()
        .flat_map(|window_id| self.window_handles.get(&window_id).copied())
        .collect()
}
```

**Usage pattern from tests** (recent_projects/src/remote_connections.rs:593-597):
```rust
let windows = cx.update(|cx| cx.windows().len());
// ...
cx.update(|cx| cx.windows()[0].downcast::<MultiWorkspace>().unwrap());
```

**To enumerate other MultiWorkspace instances from within a MultiWorkspace**:

```rust
// Inside a method with access to cx: &mut App
let current_window_id = self.window_id; // Already stored in MultiWorkspace

let other_windows: Vec<AnyWindowHandle> = cx.windows()
    .into_iter()
    .filter(|handle| handle.window_id() != current_window_id)
    .collect();

// For each other window, try to get its root MultiWorkspace
for handle in other_windows {
    if let Some(multi_workspace_handle) = handle.downcast::<MultiWorkspace>() {
        // Now you can read the MultiWorkspace to get its project groups
        let project_groups = multi_workspace_handle.read(cx).project_groups(cx);
        // Use this to build the submenu entries
    }
}
```

**Displaying window info**: You'll want to show meaningful labels for each window. You can use:
- The project group names (derived from root paths) from each window's `MultiWorkspace`
- `window.window_title()` from `AnyWindowHandle` for the window title
- `Workspace::project_group_key(cx)` for each workspace in a project group

### Important Considerations for "Move to Window" Submenu

1. Each `MultiWorkspace` is per-window, so you need to go through `cx.windows()` to find other windows.
2. The `AnyWindowHandle::downcast::<MultiWorkspace>()` method only works if you have `use workspace::MultiWorkspace` imported.
3. Moving a project group to another existing window would require:
   - Removing the project group from the current `MultiWorkspace`
   - Adding it to the target `MultiWorkspace` in the other window
   - This is more complex than `open_project_group_in_new_window()` which just creates a brand new window
4. A simpler approach might be to offer both:
   - "Move to New Window" (existing functionality)
   - "Move to [Window Name]" for each other open window

---

## Summary of All Relevant File Paths

| Item | File | Key Line(s) |
|------|------|-------------|
| Panel background color | `crates/theme/src/default_colors.rs` | 95, 248 |
| Panel background color API | `crates/theme/src/styles/colors.rs` | 132 |
| Project panel render | `crates/project_panel/src/project_panel.rs` | 6536 |
| Project panel item colors | `crates/project_panel/src/project_panel.rs` | 565-575 |
| ContextMenu struct | `crates/ui/src/components/context_menu.rs` | 211 |
| ContextMenu API | `crates/ui/src/components/context_menu.rs` | 266+ |
| ContextMenuEntry | `crates/ui/src/components/context_menu.rs` | 82 |
| Right-click menu helper | `crates/ui/src/components/right_click_menu.rs` | 67 |
| Project panel context menu example | `crates/project_panel/src/project_panel.rs` | 1019-1119 |
| Window tab right-click menu example | `crates/platform_title_bar/src/system_window_tabs.rs` | 288 |
| client_side_decorations | `crates/workspace/src/workspace.rs` | 10471 |
| TitleBar render | `crates/title_bar/src/title_bar.rs` | 144 |
| PlatformTitleBar render | `crates/platform_title_bar/src/platform_title_bar.rs` | 186-327 |
| MultiWorkspace render | `crates/workspace/src/multi_workspace.rs` | 2044 |
| IconName enum | `crates/icons/src/icons.rs` | 10 |
| Workspace struct | `crates/workspace/src/workspace.rs` | 1341 |
| WorkspaceSidebar | `crates/workspace/src/workspace_sidebar.rs` | 30 |
| RenameBranchModal pattern | `crates/git_ui/src/git_ui.rs` | 316-416 |
| MoveProjectToNewWindow action | `crates/workspace/src/multi_workspace.rs` | 45 |
| open_project_group_in_new_window | `crates/workspace/src/multi_workspace.rs` | 1054 |
| App::windows() | `crates/gpui/src/app.rs` | 1092 |
| AnyWindowHandle::downcast | `crates/gpui/src/window.rs` | 5620 |
| Window::root() | `crates/gpui/src/window.rs` | 1742 |
| PopoverMenu usage examples | `crates/title_bar/src/title_bar.rs` | 995 |
