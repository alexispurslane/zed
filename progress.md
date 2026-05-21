# Progress: Stopping AgentPanel from appearing as a dock panel

## Changes Made

### `crates/xenomorphic/src/xenomorphic_app.rs`
- Removed `initialize_agent_panel()` call from workspace setup (replaced with comment)
- Removed `register_sidebar()` call for the agent sidebar
- Replaced `agent_ui::AgentPanel::toggle_focus/focus/toggle` action registrations with inline closures using `xenomorphic_actions::assistant::ToggleFocus/FocusAgent/Toggle`
- Added `open_agent_session_or_focus()` helper function that opens a new `AgentSessionItem` tab (or focuses existing one)
- Marked `initialize_agent_panel()` as `#[allow(dead_code)]`

### `crates/xenomorphic/src/main.rs`
- Changed `OpenRequestKind::AgentPanel` handler to create `AgentSessionItem` tab via `thread_finder_provider::create_conversation_view()` instead of `workspace.focus_panel::<AgentPanel>()`
- Changed `OpenRequestKind::SharedAgentThread` handler to use `ThreadStore::global(cx)` instead of `workspace.panel::<AgentPanel>(cx)`, and open the thread as an `AgentSessionItem` tab
- Replaced `use agent_ui::AgentPanel` with `use agent_ui::AgentSessionItem`

### `crates/agent_ui/src/conversation_view.rs`
- Added public `thread_id()` and `root_session_id()` accessor methods (fields remain pub(crate))

### `crates/sidebar/src/sidebar.rs`
- Added `AgentPanel` and `AgentSessionItem` imports
- Fixed dead code in `dump_single_workspace` that referenced old AgentPanel state
- Removed duplicate code block from agent panel dump

### `crates/xenomorphic/src/xenomorphic_app.rs` (additional)
- Made `ensure_agent_panel_for_workspace()` return `Task::ready(Ok(()))` — no-op

## Build Status
✅ `xenomorphic` binary compiles and links successfully
✅ `agent_ui` compiles successfully
✅ `sidebar` compiles successfully (dead code, not instantiated)
✅ `file_finder` compiles successfully
