# Proposal: Per-Thread Agent Locations

## Key Design Decisions

1. Reuse `schema::SessionId` — no new ID type. Each AgentThread already has a UUID v4 `session_id`.
2. `AgentThreadId` type alias at interface boundary (not "session_id" — overloaded term). `pub type AgentThreadId = Arc<str>` in project crate.
3. `CollaboratorId::Agent(AgentThreadId)` replaces `CollaboratorId::Agent` — per-thread identity for follow system.
4. `ReplicaId::for_agent_thread()` — deterministic per-thread replica ID (range 100+). `ReplicaId::is_agent()` helper.
5. `PlayerColors::agent_for_thread()` — per-thread cursor colors cycling through collaborator palette.
6. Follow button per-thread: each ThreadView's crosshair follows its own thread, highlights in its own color.
7. Remove root-thread-only guard. All threads (including subagents) set their own location.

## Changes By Crate

### crates/clock/src/clock.rs
- Add `AGENT_REPLICA_BASE: u64 = 100`
- Add `ReplicaId::for_agent_thread(thread_id: &AgentThreadId) -> Self` (deterministic siphash)
- Add `ReplicaId::is_agent(&self) -> bool` (checks 100..1100 range)
- Deprecate `ReplicaId::AGENT`

### crates/language/src/buffer.rs
- `set_agent_selections()` and `remove_agent_selections()`: add `replica_id: ReplicaId` param

### crates/project/src/project.rs
- `pub type AgentThreadId = Arc<str>` (with From<schema::SessionId>)
- `agent_location: Option<AgentLocation>` → `agent_locations: HashMap<AgentThreadId, AgentLocation>`
- `AgentLocation` gets `agent_thread_id: AgentThreadId` field
- `AgentLocationChanged` carries `agent_thread_id: AgentThreadId`
- New `remove_agent_location()`, `agent_location_for()`, `agent_locations()`
- `set_agent_location()` upserts/removes from HashMap, uses `ReplicaId::for_agent_thread()`
- Deprecate old `agent_location()` getter

### crates/workspace/src/workspace.rs
- `CollaboratorId::Agent` → `CollaboratorId::Agent(AgentThreadId)` 
- All match arms / follow / unfollow / follower_states updated
- `handle_agent_location_changed()` uses thread-specific CollaboratorId
- `leader_border_for_pane()` uses `agent_for_thread()` color

### crates/workspace/src/item.rs
- Any `CollaboratorId::Agent` → `CollaboratorId::Agent(_)` or bind thread ID

### crates/theme/src/styles/players.rs
- Add `agent_for_thread(thread_id: &AgentThreadId) -> PlayerColor`
- Deprecate `agent()`

### crates/editor/src/editor.rs
- `remote_selections_in_range()`: `replica_id == ReplicaId::AGENT` → `replica_id.is_agent()`
- Per-thread color via `agent_for_thread()`, per-thread name label
- Store `HashMap<ReplicaId, AgentThreadId>` on Editor for reverse mapping

### crates/editor/src/element.rs
- `CollaboratorId::Agent` → `CollaboratorId::Agent(thread_id)` with `agent_for_thread()` color

### crates/editor/src/items.rs
- `update_agent_location()` receives `AgentThreadId`, stores replica→thread mapping
- `set_leader_id()` match arm updated

### crates/agent_thread/src/thread.rs
- Remove `should_update_agent_location` root-thread guard
- Always include `agent_thread_id` in `AgentLocation`
- Turn-end: `remove_agent_location(&session_id)` instead of `set_agent_location(None)`

### crates/agent/src/thread.rs
- Remove `update_agent_location: bool` flag on ReadFileTool

### crates/agent/src/tools/edit_session.rs
- Remove `is_subagent()` guard, include `agent_thread_id`

### crates/agent/src/tools/read_file_tool.rs
- Remove `update_agent_location` flag, include `agent_thread_id`

### crates/agent/src/agent.rs
- `close_session()`: clean up agent location via `remove_agent_location()`

### crates/agent_ui/src/conversation_view/thread_view.rs
- Follow toggle: `CollaboratorId::Agent(self.agent_thread_id)`
- `.selected_icon_color()`: `agent_for_thread()` color
- All `CollaboratorId::Agent` refs updated

### crates/agent_ui/src/conversation_view.rs + agent_panel.rs
- All `CollaboratorId::Agent` refs updated to per-thread variant
