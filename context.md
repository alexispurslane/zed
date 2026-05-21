# Audit: "Agent Sessions as Tabs" Alternative Design

**Date:** 2025-07-09
**Auditor:** Automated Codebase Sweep

---

## Summary

| # | Item | Status |
|---|------|--------|
| 1 | agent_ui.rs init() — ThreadFinderProvider registration | **NOT DONE** |
| 2 | Settings AI page — LLM Providers / MCP Servers / Agent Profiles sections | **DONE** |
| 3 | Status bar integration — AgentSessionIndicator wired into workspace | **NOT DONE** |
| 4 | AgentPanel dock registration — still registered as dock panel | **DONE** (still active, not removed) |
| 5 | cmd-shift-a / ToggleFocus action — registration | **DONE** |
| 6 | ReviewBranchDiff / ResolveConflictsWithAgent — routing through AgentPanel | **DONE** |
| 7 | Sidebar thread list — ToggleWorkspaceSidebar + thread rendering | **DONE** |
| 8 | AgentSessionItem registered as SerializableItem | **PARTIALLY DONE** |
| 9 | file_finder Cargo.toml — workspace dependency | **DONE** |
| 10 | agent_ui Cargo.toml — file_finder dependency | **NOT DONE** |

---

## Detailed Findings

### 1. agent_ui.rs init() — ThreadFinderProvider registration

**Status: NOT DONE**

`ThreadFinderProvider` exists and is fully implemented in `crates/agent_ui/src/thread_finder_provider.rs` (lines 38–44+). It implements the `FinderProvider` trait from `file_finder::provider` with `section_label()`, `supports_mode()`, `recent_items()`, `search()`, `create_from_query()`, and `confirm()`.

However, the `init()` function in `crates/agent_ui/src/agent_ui.rs` (lines 265–330) does **not** register `ThreadFinderProvider` with the file finder. The `init()` function calls:
- `agent_panel::init(cx)`
- `context_server_configuration::init(...)`
- `thread_metadata_store::init(cx)`
- `inline_assistant::init(...)`
- `terminal_inline_assistant::init(...)`

But there is no call to `FileFinderDelegate::register_provider(Box::new(ThreadFinderProvider))` or any equivalent wiring. The `FileFinderDelegate::register_provider()` method exists (file_finder.rs:1013), but nothing calls it with `ThreadFinderProvider`.

**Impact:** The thread-finder integration with the unified file finder (`cmd-p`) will not work. Typing `#` in the file finder will not show agent sessions.

---

### 2. Settings AI page — LLM Providers / MCP Servers / Agent Profiles

**Status: DONE**

File: `crates/settings_ui/src/page_data.rs`

- **LLM Providers** section header at line 7598, with an `ActionLink` to "Configure LLM Providers" (line 7600) that dispatches `assistant::ToggleFocus`.
- **MCP Servers** section header at line 7626, with an `ActionLink` to "Configure MCP Servers" (line 7628) that dispatches `agent::OpenSettings`.
- **Agent Profiles** section defined in `agent_profiles_section()` at line 7652, with a `SubPageLink` pointing to `render_agent_profiles_setup_page` and json path `"agent.profiles"`.
- All three sections are composed into the AI page via `concat_sections!` at line ~7676.

---

### 3. Status bar integration — AgentSessionIndicator

**Status: NOT DONE**

`AgentSessionIndicator` is fully implemented in `crates/workspace/src/agent_session_indicator.rs` (lines 1–139). It implements `Render`, `StatusItemView`, and has the callback-based architecture described in the design doc (using `thread_count_provider` and `thread_finder_opener` callbacks to decouple from `agent_ui`).

However, it is **never wired into the workspace's status bar**. Specifically:
- No code in `crates/xenomorphic/` references `AgentSessionIndicator` at all.
- No code anywhere calls `status_bar.add_left_item()` or `status_bar.add_right_item()` with an `AgentSessionIndicator`.
- The workspace status bar initialization in `crates/workspace/src/workspace.rs` (lines 1711–1717) only adds `left_dock_buttons`, `right_dock_buttons`, and `bottom_dock_buttons`.

The indicator's own documentation (line 21) states: *"These callbacks are wired up in the `xenomorphic` app crate where both `workspace` and `agent_ui` are available."* — but this wiring has not been done.

**Impact:** The status bar will not show the agent session count indicator. Users have no way to see active sessions at a glance or click to open the thread finder.

---

### 4. AgentPanel dock registration

**Status: DONE** (panel is still registered and active)

`AgentPanel` is still registered as a dock panel. The `agent_panel::init(cx)` function at `crates/agent_ui/src/agent_panel.rs` line 200 registers numerous actions on the workspace and the panel itself. The `AgentPanel` struct implements the `Panel` trait (with `toggle_action()` returning `ToggleFocus`, `activation_priority()`, etc.) and is still the primary container for agent sessions in the sidebar/dock model.

**Note:** If the design intent was to *remove* the AgentPanel dock in favor of tabs-as-items, this has NOT been done yet. The panel is fully active and all agent interactions still route through it.

---

### 5. cmd-shift-a / ToggleFocus action registration

**Status: DONE**

`ToggleFocus` (from `xenomorphic_actions::assistant::ToggleFocus`) is registered:

- In `agent_panel.rs` line 27: imported as `assistant::{FocusAgent, Toggle, ToggleFocus}`.
- `AgentPanel::toggle_focus()` handler at line 1111–1122 handles `ToggleFocus` by calling `workspace.toggle_panel_focus::<AgentPanel>(window, cx)`.
- The panel's `toggle_action()` at line 2888 returns `Box::new(ToggleFocus)`.
- It appears in the menu at `crates/xenomorphic/src/xenomorphic_app/app_menus.rs` line 44 (as "Terminal Panel" `ToggleFocus` — note: this is the terminal panel's, not the agent's).
- The vim command mapping at `crates/vim/src/command.rs` line 1785 maps `("A", "I")` to `"agent::ToggleFocus"`.

The `ToggleFocus` action is properly wired for the AgentPanel's toggle behavior.

---

### 6. ReviewBranchDiff / ResolveConflictsWithAgent — routing through AgentPanel

**Status: DONE**

In `crates/agent_ui/src/agent_panel.rs`:

- `ReviewBranchDiff` action handler at line 318: `let Some(panel) = workspace.panel::<AgentPanel>(cx) else { return; };` followed by `workspace.focus_panel::<AgentPanel>(window, cx)` at line 344.
- `ResolveConflictsWithAgent` action handler at line 363: `let Some(panel) = workspace.panel::<AgentPanel>(cx) else { return; };` followed by `workspace.focus_panel::<AgentPanel>(window, cx)` at line 371.
- `ResolveConflictedFilesWithAgent` action handler at line 391: same pattern, with `workspace.focus_panel::<AgentPanel>(window, cx)` at line 400.

All three still route through `AgentPanel`.

---

### 7. Sidebar thread list — ToggleWorkspaceSidebar + thread rendering

**Status: DONE**

File: `crates/sidebar/src/sidebar.rs`

- `ToggleWorkspaceSidebar` is imported at line 60 and used in a keybinding at line 4929.
- The sidebar has extensive thread-related rendering:
  - `ThreadEntry` struct (line 217), `ActiveThreadInfo` (line 182), `ThreadEntryWorkspace` (line 193)
  - `ListEntry::Thread` variant (line 268) with full metadata
  - Thread status rendering, notification badges, and more
  - Thread switcher integration (`ThreadSwitcher` at line 572)
  - Thread activation logic with `pending_thread_activation` (line 574)

The sidebar fully renders threads and the `ToggleWorkspaceSidebar` action is registered.

---

### 8. AgentSessionItem registered as SerializableItem

**Status: PARTIALLY DONE**

`AgentSessionItem` implements `SerializableItem` at `crates/agent_ui/src/agent_session_item.rs` line 238. The implementation includes:

- `serialized_item_kind()` returns `"AgentSessionItem"` (line 240)
- `cleanup()` returns `Ok(())` — no workspace-scoped cleanup (line 247)
- `serialize()` stores `thread_id` and optional `session_id` in a HashMap (line 271), but the data is never actually written — the map is assigned but unused (`let _ = map;` at line 291)
- `deserialize()` at line 255 returns an error: `"AgentSessionItem deserialization not yet implemented"` — this is explicitly a placeholder per the Phase 4 note
- `should_serialize()` at line 308 handles `AgentSessionItemEvent::UpdateTab`

**Critically:** `register_serializable_item::<AgentSessionItem>(cx)` is **never called** anywhere in the codebase. Other items like `Editor`, `TerminalView`, `Onboarding`, etc. all call `register_serializable_item` during their init, but `AgentSessionItem` is missing this registration call. Without it, the workspace serialization system will not recognize `AgentSessionItem` as a restorable item type.

**Gaps:**
1. No `register_serializable_item::<AgentSessionItem>(cx)` call
2. `serialize()` is a no-op (data is discarded with `let _ = map`)
3. `deserialize()` explicitly returns an error

---

### 9. file_finder Cargo.toml — workspace dependency

**Status: DONE**

File: `crates/file_finder/Cargo.toml`

Line 35: `workspace.workspace = true` — the `workspace` crate is listed as a dependency. This is needed because `FinderProvider::confirm()` takes a `&mut Workspace` parameter and the file finder needs to interact with workspace types.

Additionally, `file_finder` has the `provider` module (`crates/file_finder/src/provider.rs`) which defines the `FinderProvider` trait, `ProviderMatch`, `ProviderMatchData`, `SearchMode`, etc.

---

### 10. agent_ui Cargo.toml — file_finder dependency

**Status: NOT DONE**

File: `crates/agent_ui/Cargo.toml`

The `agent_ui` crate does **NOT** list `file_finder` as a dependency. The `[dependencies]` section includes many crates (agent_thread, editor, workspace, etc.) but `file_finder` is absent.

This is required because `crates/agent_ui/src/thread_finder_provider.rs` line 35 does `pub use file_finder::provider::{FinderProvider, ProviderMatch, ProviderMatchData, SearchMode};` — this import will fail at compile time without the dependency.

Currently, `thread_finder_provider.rs` is declared as a module in `agent_ui.rs` (`pub mod thread_finder_provider;` at line 10), meaning the crate will fail to compile as-is.

---

## Risk Assessment

### Compilation Blockers
- **Item 10 (missing `file_finder` dep)** will cause a compile error when `thread_finder_provider.rs` is compiled, since it imports from `file_finder::provider`.

### Functional Gaps
- **Item 1 (ThreadFinderProvider not registered):** Even if the code compiles, the file finder won't show thread results because `register_provider` is never called.
- **Item 3 (AgentSessionIndicator not wired):** The status bar indicator exists but is dead code — no instantiation or wiring.
- **Item 8 (AgentSessionItem not registered):** The `SerializableItem` impl exists but won't be recognized by the workspace restoration system.

### Design Alignment
- **Item 4 (AgentPanel still active):** If the "Sessions as Tabs" design intends to deprecate the AgentPanel in favor of tab-based sessions, the panel is still fully operational and all routing goes through it. The design transition has not begun.
- **Item 6 (ReviewBranchDiff still through AgentPanel):** These actions still focus the AgentPanel rather than opening an `AgentSessionItem` tab, which would be the expected behavior under the new design.

---

## Recommended Next Steps

1. Add `file_finder.workspace = true` to `crates/agent_ui/Cargo.toml`
2. In `agent_ui::init()`, register `ThreadFinderProvider` with the file finder delegate (likely via `cx.observe_new` on workspace, then accessing the file finder to call `register_provider`)
3. Wire `AgentSessionIndicator` into the workspace status bar in the `xenomorphic` app crate init, providing the `thread_count_provider` and `thread_finder_opener` callbacks
4. Call `workspace::register_serializable_item::<AgentSessionItem>(cx)` in `agent_ui::init()`
5. Implement `AgentSessionItem::serialize()` and `deserialize()` properly
6. If the design calls for removing AgentPanel, create a migration plan — currently all agent interactions still route through it
