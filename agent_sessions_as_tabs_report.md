# Report: Converting Agent Sessions/Chats into Regular Tabs

## Executive Summary

This report maps the work required to convert agent sessions/chats from being contained inside the `AgentPanel` (a dock panel with custom sidebar-based navigation) into **first-class `Item` instances** that live in regular panes alongside editors, terminals, and other workspace items. This would enable:

- Dragging agent sessions between panes
- Multiple agent sessions open simultaneously in different panes
- Standard tab behavior (click to switch, close button, drag to reorder)
- Splitting views with agent sessions
- Preview tab support
- Full serialization/restoration across workspace restarts

---

## 1. Current Architecture

### 1.1 How Agent Sessions Work Today

```
Workspace
└── Dock (Left/Right)
    └── AgentPanel (implements Panel trait)
        ├── base_view: BaseView ──► ActiveThread (ConversationView)
        │   └── server_state ──► ThreadView (actual chat UI)
        ├── retained_threads: HashMap<ThreadId, ConversationView>
        ├── draft_thread: Option<ConversationView>
        └── terminals: HashMap<TerminalId, AgentTerminal>
```

The `AgentPanel` is a **dock panel** (not a pane item). It manages its own internal navigation:
- A list of threads/terminals rendered in the panel chrome
- A single active `ConversationView` displayed in the panel body
- Custom `BaseView` enum switches between thread, terminal, and uninitialized states

### 1.2 Comparison: How Regular Items Work

```
Workspace
└── PaneGroup
    └── Pane
        ├── items: Vec<Box<dyn ItemHandle>>  ← tabs
        ├── active_item_index: usize
        └── toolbar: Toolbar
            └── items (breadcrumbs, etc.)
```

Regular items implement the `Item` trait and are managed by `Pane`, which handles:
- Tab rendering, dragging, closing
- Splitting and moving between panes
- Navigation history (back/forward)
- Autosave, dirty state, serialization
- Focus management

### 1.3 Key Files

| File | Role |
|------|------|
| `crates/workspace/src/item.rs` | `Item`, `ItemHandle`, `SerializableItem`, `FollowableItem` traits |
| `crates/workspace/src/pane.rs` | `Pane` — container for items, tab management, drag-and-drop |
| `crates/agent_ui/src/agent_panel.rs` | `AgentPanel` — current dock panel containing all thread state |
| `crates/agent_ui/src/conversation_view.rs` | `ConversationView` — the actual chat UI (not an Item today) |
| `crates/agent_ui/src/conversation_view/thread_view.rs` | `ThreadView` — renders messages, editor, etc. |
| `crates/agent_ui/src/thread_metadata_store.rs` | `ThreadMetadataStore` — persistent thread metadata |
| `crates/agent_ui/src/agent_diff.rs` | `AgentDiffPane` — **already an Item** (good reference) |
| `crates/terminal_view/src/terminal_view.rs` | `TerminalView` — **already an Item** (good reference) |

---

## 2. Required Changes

### 2.1 Make `ConversationView` Implement `Item`

**File:** `crates/agent_ui/src/conversation_view.rs`

The core change. `ConversationView` must implement:

```rust
impl Item for ConversationView {
    type Event = ConversationViewEvent;  // or reuse existing events
    
    fn tab_content_text(&self, _detail: usize, cx: &App) -> SharedString {
        // Use thread title if available, else "New Agent Thread"
        self.root_thread_view()
            .and_then(|tv| tv.read(cx).thread.read(cx).title())
            .unwrap_or_else(|| DEFAULT_THREAD_TITLE.into())
    }
    
    fn tab_icon(&self, _window: &Window, cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::XenomorphicAssistant).color(Color::Muted))
    }
    
    fn can_split(&self) -> bool {
        true
    }
    
    fn clone_on_split(
        &self,
        _workspace_id: Option<WorkspaceId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<Option<Entity<Self>>> {
        // Need to create a new ConversationView that shares or clones the thread state
        // This is the trickiest part — see §2.4
    }
    
    fn deactivated(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Pause streaming, save draft state, etc.
    }
    
    // ... other Item methods
}
```

**Key considerations:**
- `ConversationView` currently doesn't emit item events. Need to map existing events to `ItemEvent::UpdateTab`, `ItemEvent::Edit`, etc.
- Need to decide if `ItemEvent::CloseItem` should archive the thread or just close the tab
- `is_dirty()` — should return true if there's unsent draft content or pending AI response

### 2.2 Create a Wrapper `AgentSessionItem` (Alternative Approach)

Instead of making `ConversationView` itself an `Item`, create a thin wrapper struct (similar to how the test code already does `ThreadViewItem`):

**File:** New file `crates/agent_ui/src/agent_session_item.rs`

```rust
pub struct AgentSessionItem(Entity<ConversationView>);

impl Item for AgentSessionItem {
    type Event = ();
    
    fn tab_content_text(&self, _detail: usize, cx: &App) -> SharedString {
        self.0.read(cx).root_thread_view()
            .and_then(|tv| tv.read(cx).thread.read(cx).title())
            .unwrap_or_else(|| "New Agent Thread".into())
    }
    
    fn can_split(&self) -> bool { true }
    // ... etc
}
```

This is **less invasive** and may be preferable because:
- `ConversationView` already has complex state management
- Easier to control exactly what pane behavior is exposed
- The test code in `conversation_view.rs` already does exactly this (`ThreadViewItem`)

However, the wrapper approach adds indirection and requires forwarding all relevant methods.

### 2.3 Replace `AgentPanel` Internal Navigation with Pane-Based Tabs

**File:** `crates/agent_ui/src/agent_panel.rs`

The `AgentPanel` currently manages its own `base_view` state. Instead, it should:

1. **Remove `base_view`, `retained_threads`, `draft_thread` fields** — or repurpose them to track which sessions are "open" somewhere in the workspace
2. **Delegate display to the pane** — stop rendering the thread content directly; let the pane's tab bar handle switching
3. **Keep the thread list as a sidebar** — but clicking a thread opens it as a tab in the active pane, not swapping `base_view`

**Current click handler** (from sidebar/mention crease):
```rust
fn open_thread(workspace, id, name, window, cx) {
    let panel = workspace.panel::<AgentPanel>(cx)?;
    panel.update(cx, |panel, cx| {
        panel.load_agent_thread(agent, id, None, Some(name), true, "agent_panel", window, cx)
    });
}
```

**New click handler:**
```rust
fn open_thread(workspace, id, name, window, cx) {
    // Check if already open in any pane
    if let Some(existing) = find_existing_session_item(workspace, &id, cx) {
        workspace.activate_item(existing, window, cx);
        return;
    }
    
    // Create new ConversationView
    let item = cx.new(|cx| {
        ConversationView::load_thread(agent, id, None, Some(name), cx)
    });
    
    // Open as regular item in active pane
    workspace.add_item_to_active_pane(Box::new(AgentSessionItem(item)), None, true, window, cx);
}
```

### 2.4 Handle Thread Lifecycle and State Sharing

**Critical design decision:** When a `ConversationView` is split or cloned, what happens?

| Option | Pros | Cons |
|--------|------|------|
| **A. Share the same `AgentThread` entity** | Single source of truth; messages sync | Two views editing same draft message weirdness |
| **B. Clone thread state** | Independent views | Data duplication; harder to persist |
| **C. Read-only split (like editor)** | Familiar model | Agent sessions are less "document-like" |

**Recommendation:** Start with **Option A** (share the `AgentThread`), similar to how the same buffer can be open in multiple editors. The `AgentThread` is already an `Entity<AgentThread>`, so this is natural in GPUI.

The `clone_on_split` implementation:
```rust
fn clone_on_split(
    &self,
    _workspace_id: Option<WorkspaceId>,
    window: &mut Window,
    cx: &mut Context<Self>,
) -> Task<Option<Entity<Self>>> 
where Self: Sized
{
    // Create a NEW ConversationView that shares the same thread
    Task::ready(Some(cx.new(|cx| {
        ConversationView::new_with_thread(
            self.agent.clone(),
            self.connection_store.clone(),
            self.root_thread_view().unwrap().read(cx).thread.clone(),
            window, cx
        )
    })))
}
```

However, `ConversationView` currently owns subscriptions to the thread. Multiple views subscribing to the same thread may cause event handling complexity. Need to ensure events are idempotent or deduplicated.

### 2.5 Serialization (`SerializableItem`)

**File:** `crates/agent_ui/src/conversation_view.rs` (or new wrapper)

To persist open agent session tabs across workspace restarts:

```rust
impl SerializableItem for AgentSessionItem {
    fn serialized_item_kind() -> &'static str {
        "AgentSession"
    }
    
    fn serialize(
        &mut self,
        workspace: &mut Workspace,
        item_id: ItemId,
        closing: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Task<Result<()>>> {
        // Store session_id, agent_type, work_dirs
        Some(cx.background_spawn(async move {
            let kvp = KeyValueStore::global(cx);
            // Serialize thread metadata
            Ok(())
        }))
    }
    
    fn deserialize(
        project: Entity<Project>,
        workspace: WeakEntity<Workspace>,
        workspace_id: WorkspaceId,
        item_id: ItemId,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Entity<Self>>> {
        // Read back session_id and restore thread
    }
    
    fn should_serialize(&self, event: &Self::Event) -> bool {
        // Serialize on title change, thread update, etc.
        true
    }
}
```

The existing `SerializedAgentPanel` logic in `agent_panel.rs` already does this for the panel — that code can be refactored to serialize individual items instead.

### 2.6 Handle the Sidebar List

The agent sessions sidebar (thread list) still needs to exist. Options:

| Approach | Description |
|----------|-------------|
| **A. Keep `AgentPanel` as a thin sidebar** | `AgentPanel` becomes just the thread list; clicking opens tabs elsewhere. Can stay as a dock panel. |
| **B. Merge with workspace sidebar** | Thread list becomes part of the workspace-level sidebar alongside the project panel |
| **C. Remove sidebar, use tab bar only** | Thread history accessed via command palette or dropdown in the tab |

**Recommendation: Option A** — keep a thin `AgentPanel` that only renders the thread list, similar to how the Terminal Panel has a tab bar. This minimizes UI disruption.

The sidebar would need to:
- Show all threads (active, retained, archived)
- Highlight which threads have open tabs
- When clicking a thread, activate the existing tab or create a new one

### 2.7 Terminal Integration

`AgentPanel` also manages agent panel terminals (`HashMap<TerminalId, AgentTerminal>`). These should also become regular `Item` instances in panes.

**Already exists:** `TerminalView` implements `Item`. So agent panel terminals can simply use `TerminalView` directly.

The question is: do we keep agent panel terminals as a special "agent terminal" concept, or are they just terminals that happen to be near agent sessions?

### 2.8 Remove or Deprecate Panel-Specific Concepts

| Current Concept | Fate |
|-----------------|------|
| `BaseView` enum | Remove — pane handles active item |
| `VisibleSurface` enum | Remove — pane handles item display |
| `AgentPanelEntryKind` | Remove if no longer needed |
| `retained_threads` hashmap | Keep track of "all known threads", but not for display |
| `draft_thread` | Every new tab starts with a draft thread |
| `zoomed` on AgentPanel | Use pane's zoom (`Pane::toggle_zoom`) |
| Panel-level serialization | Per-item serialization instead |

### 2.9 Toolbar Changes

Currently `AgentPanel::render_toolbar()` renders custom toolbar UI. When sessions are pane items, this should move to:
- Either `ConversationView::breadcrumbs()` / `breadcrumb_prefix()`
- Or a `ToolbarItemView` registered with the workspace toolbar

Reference: `AgentDiffPane` implements `breadcrumbs()` and `breadcrumb_location()`.

### 2.10 Actions and Commands

Many actions currently dispatch to `AgentPanel`. Need to redirect or make context-aware:

| Action | Current Target | New Target |
|--------|---------------|------------|
| `NewThread` | `AgentPanel::new_thread()` | Pane's new item mechanism, or `workspace.add_item()` |
| `OpenSettings` | `AgentPanel::open_configuration()` | Could open as modal, or as a tab (already an item?) |
| `ToggleOptionsMenu` | `AgentPanel` | Tab context menu or item toolbar |
| `ExpandMessageEditor` | `AgentPanel::expand_message_editor()` | Active `ConversationView` |
| `CopyThreadToClipboard` | `AgentPanel` | Active `ConversationView` |
| `OpenAgentDiff` | `AgentPanel` | Active `ConversationView`'s thread |

---

## 3. Step-by-Step Implementation Plan

### Phase 1: Foundation (Low Risk)
1. **Create `AgentSessionItem` wrapper** in new file
   - Implements `Item` with minimal behavior
   - Delegates to existing `ConversationView`
   - `tab_content_text` uses thread title
   - `can_split` returns false initially

2. **Add open-as-tab support** without removing panel
   - New command/action: `OpenThreadInTab` 
   - Opens clicked thread as `AgentSessionItem` in active pane
   - Original panel behavior stays untouched

3. **Test serialization for `AgentSessionItem`**
   - Save/restore a single tab
   - Verify thread state is preserved

### Phase 2: Full Integration
4. **Enable split/drag for `AgentSessionItem`**
   - Implement `clone_on_split` (shared thread approach)
   - Test drag-and-drop between panes

5. **Thin the `AgentPanel`**
   - Keep only the thread list sidebar
   - Remove `base_view`, `VisibleSurface` rendering
   - Thread clicks open/activate tabs instead of swapping view

6. **Migrate terminal handling**
   - Agent panel terminals become regular `TerminalView` items

### Phase 3: Polish
7. **Toolbar integration**
   - Move panel toolbar to item breadcrumbs/toolbar
   - Ensure model selector, profile selector still accessible

8. **Remove legacy code**
   - Delete `BaseView`, `VisibleSurface`, old serialization
   - Clean up `AgentPanel` to be pure sidebar

9. **Settings and configuration**
   - Update `AgentSettings` (remove `dock`, `default_width`, etc. if panel becomes optional)
   - Decide if agent panel sidebar is still dock-positionable

---

## 4. Key Technical Challenges

### 4.1 Thread Lifecycle and Pane Lifecycle Mismatch

Currently, threads live in `AgentPanel::retained_threads` (kept alive even when not visible). Pane items get dropped when closed. **Decision needed:**
- Should closing a tab archive the thread? (User can reopen from sidebar)
- Should closing a tab persist the thread to `ThreadMetadataStore`?
- Should threads without tabs auto-archive after N minutes? (MaxIdleRetainedThreads logic)

### 4.2 Draft Thread Semantics

Currently there's exactly one `draft_thread` per `AgentPanel`. With tabs, every new tab could be a draft:
- User clicks "New Thread" → opens draft tab
- User sends message → draft becomes persisted thread 
- Tab title updates from "New Agent Thread" to actual title

This is actually **more intuitive** than today's draft model.

### 4.3 Focus and Keyboard Handling

`ConversationView::focus_handle()` delegates to the active `ThreadView`. When tabs switch, pane manages focus. Need to verify `ThreadView`'s focus handle works correctly when parented by pane instead of panel.

### 4.4 Multiple Views of Same Thread

If split creates two views of the same thread, both show the same messages but may have different scroll positions and draft states. The `message_editor` is inside `ThreadView`, so each split view could have a different draft. This is acceptable.

### 4.5 Agent Selection Per Thread

Currently `AgentPanel::selected_agent` applies to the next new thread. With tabs, each thread has its own agent. The agent selection UI (model selector) needs to exist per-tab.

---

## 5. Reference Implementations

### 5.1 `AgentDiffPane` — Already an Item

`crates/agent_ui/src/agent_diff.rs:505` already implements `Item` for a related agent UI. Study:
- `tab_content()` — custom label with thread title
- `can_split()` / `clone_on_split()`
- `breadcrumbs()` integration
- `navigate()` delegation to editor

### 5.2 `TerminalView` — Item with Custom Tab

`crates/terminal_view/src/terminal_view.rs:1325` shows:
- Rich tab content with icons and actions
- `handle_drop()` for drag-and-drop support
- `clone_on_split()` with terminal cloning

### 5.3 `ThreadViewItem` — Existing Test Wrapper

`crates/agent_ui/src/conversation_view.rs:3524` already has a minimal `Item` wrapper for testing. This can be the basis for the production `AgentSessionItem`.

---

## 6. Files to Modify

| File | Changes |
|------|---------|
| `crates/agent_ui/src/conversation_view.rs` | Add `Item` impl (or create wrapper); emit `ItemEvent`s |
| `crates/agent_ui/src/agent_panel.rs` | Thin to sidebar-only; remove `BaseView`; dispatch to panes |
| `crates/agent_ui/src/agent_ui.rs` | Export new `AgentSessionItem` |
| `crates/workspace/src/item.rs` | May need no changes — the trait is already general enough |
| **New:** `crates/agent_ui/src/agent_session_item.rs` | Wrapper `Item` implementation |
| `crates/agent_ui/src/threads_archive_view.rs` | Click handler opens tabs instead of panel |
| `crates/agent_ui/src/ui/mention_crease.rs` | `open_thread()` opens tab instead of panel |
| `crates/agent_ui/src/message_editor.rs` | Ensure works when parented by pane (may need no change) |

---

## 7. Summary Table: What Changes

| Feature | Today | After |
|--------|-------|-------|
| **Display container** | `AgentPanel` (dock panel) | `Pane` (center workspace) |
| **Navigation** | Custom sidebar list inside panel | Standard tabs + sidebar list |
| **Multiple sessions visible** | No (one at a time in panel) | Yes (one per tab/pane) |
| **Drag between panes** | No | Yes (via `DraggedTab`) |
| **Split** | No | Yes (`clone_on_split`) |
| **Close** | No (panel stays open) | Yes (standard close tab) |
| **Serialization** | Panel-level KVP | Per-item `SerializableItem` |
| **Preview tabs** | No | Yes |
| **Zoom** | Panel zoom | Pane zoom |
| **Dirty indicator** | No | Possible (draft = dirty) |

---

## 8. Estimated Effort

| Phase | Scope | Complexity |
|-------|-------|-----------|
| Phase 1 | `AgentSessionItem` wrapper, open-as-tab | Medium |
| Phase 2 | Full drag/split, thin panel | High |
| Phase 3 | Toolbar, polish, cleanup | Medium |

**Biggest risk:** Thread lifecycle and state sharing between split views. The `AgentThread` entity model should handle this, but shared mutable state across views needs careful event handling.

**Second biggest risk:** The existing `ConversationView` has `focus_handle` delegation to `ThreadView`. Pane focus management depends on stable focus handles. Need to ensure tab switching and focus work correctly.
