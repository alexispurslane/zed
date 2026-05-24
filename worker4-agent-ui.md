# Worker 4: Agent & Agent UI — Per-Thread Agent Locations

## Status: ✅ COMPLETE

## Files Changed

### 1. `crates/agent_thread/src/thread.rs`
- **Removed** `From<&ResolvedLocation> for AgentLocation` impl (AgentLocation now requires `agent_thread_id` which ResolvedLocation doesn't have; conversion done inline)
- **`resolve_locations()`**: Removed `should_update_agent_location` guard. Replaced `project.agent_location()` with `project.agent_location_for(&agent_thread_id)`. Now constructs `AgentLocation` with `agent_thread_id: self.session_id.0.clone()` instead of calling `.into()`.
- **Turn completion**: Changed `if this.parent_session_id.is_none() { project.set_agent_location(None, cx) }` → `project.remove_agent_location(&agent_thread_id, cx)` (always, no guard)
- **`read_file_lines`**: Removed `should_update_agent_location` guard. Always sets agent location with `agent_thread_id`.
- **`write_text_file`**: Same pattern — removed guard, always sets location with `agent_thread_id`.

### 2. `crates/agent/src/thread.rs`
- **`add_default_tools()`**: Replaced `update_agent_location: bool` (derived from `parent_thread_id().is_none()`) with `agent_thread_id: Arc<str>` (derived from `self.id().0.clone()`). Passes `agent_thread_id` to `ReadFileTool::new()`.

### 3. `crates/agent/src/tools/edit_session.rs`
- **`set_agent_location()`**: Removed `!thread.is_subagent()` guard. Now always calls `project.set_agent_location()` with `agent_thread_id` derived from `thread.id().0.clone()`.
- Added `use std::sync::Arc;` import.

### 4. `crates/agent/src/tools/read_file_tool.rs`
- **`ReadFileTool` struct**: Changed `update_agent_location: bool` → `agent_thread_id: Arc<str>`
- **`ReadFileTool::new()`**: Same parameter change
- **Usage**: Removed `if self.update_agent_location` guard. Always sets agent location with `self.agent_thread_id.clone()`.

### 5. `crates/agent/src/agent.rs`
- **`close_session()`**: Added cleanup of agent location via `project.remove_agent_location(&agent_thread_id, cx)` before removing the session. Gets the project entity from `self.projects[&session.project_id].project`.

### 6. `crates/agent_ui/src/conversation_view/thread_view.rs`
- **All `CollaboratorId::Agent` → `CollaboratorId::Agent(self.session_id.0.clone())`**: Follows, unfollows, is_being_followed checks across 10 call sites
- **Follow toggle button**: Changed `.selected_icon_color()` from `agent().cursor` (red) to `color_for_participant(self.session_id_color_index()).cursor` (per-thread color cycling)
- **Added `session_id_color_index()`**: Deterministic hash of session ID bytes → u32 index for `color_for_participant()`
- **Added `set_session_id()` on MessageEditor**: Called during ThreadView construction

### 7. `crates/agent_ui/src/message_editor.rs`
- **Added `session_id: Option<schema::SessionId>` field** to `MessageEditor`
- **Added `set_session_id()`** setter method
- **`chat_with_follow()`**: Replaced bare `CollaboratorId::Agent` with `CollaboratorId::Agent(agent_thread_id)` using session_id

### 8. `crates/agent_ui/src/agent_panel.rs`
- **`Follow` action handler**: Replaced bare `workspace.follow(CollaboratorId::Agent, ...)` with code that gets the active thread's `session_id.0` from the active `AgentSessionItem` → `ConversationView` → `root_thread_view()` → `session_id.0.clone()`

## Key Design Choices

1. **`Arc<str>` as `AgentThreadId`**: Used `self.session_id.0.clone()` throughout, which gives `Arc<str>` — matching the to-be-defined `pub type AgentThreadId = Arc<str>` in the project crate.
2. **`color_for_participant(hash)`**: Used the existing `PlayerColors::color_for_participant(index)` method for per-thread colors rather than adding `agent_for_thread()` to `players.rs` (which is owned by another worker). The `color_for_participant` method cycles through the same collaborator color palette.
3. **No new `agent_thread_id` field on `ThreadView`**: Reused existing `self.session_id.0.clone()` instead of adding a separate field, since `SessionId(Arc<str>)` already provides the same value.
4. **`session_id` on `MessageEditor`**: Added as `Option<schema::SessionId>` since `MessageEditor` doesn't always have a session context (e.g., in tests). Set via `set_session_id()` from `ThreadView::new()`.

## Dependencies on Other Workers

These APIs are expected from other workers but don't exist yet — changes will compile once merged:

- **Project crate (Worker 1)**: `AgentLocation { agent_thread_id: Arc<str>, buffer, position }`, `agent_location_for()`, `remove_agent_location()`, updated `set_agent_location()` accepting the new struct
- **Workspace crate (Worker 2)**: `CollaboratorId::Agent(Arc<str>)` instead of `CollaboratorId::Agent`, updated `follow`/`unfollow`/`is_being_followed` signatures
