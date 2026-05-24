# Worker 2: Workspace & Theme — Completed

## Summary

Successfully updated the CollaboratorId enum, AgentThreadId type, and agent_for_thread() color method across the entire codebase. All changes compile cleanly.

## Files Modified (10 files)

### crates/theme/src/styles/players.rs
- **Added `AgentThreadId`** struct: `#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)] pub struct AgentThreadId(pub u64)`
- **Added `AgentThreadId::from_session_id()`**: deterministic u64 hash from session ID string
- **Added `PlayerColors::agent_for_thread(&self, thread_id: &AgentThreadId) -> PlayerColor`**: cycles through palette indices 1..len-1 (orange, pink, lime, purple, amber, jade) based on thread ID hash
- **Deprecated `agent()`** with `#[deprecated(note = "Use agent_for_thread() instead for per-thread agent colors")]`

### crates/workspace/src/workspace.rs
- **`CollaboratorId` enum changed**: `Agent,` → `Agent(AgentThreadId),` (tuple variant, still `Copy`)
- **Added `pub use theme::AgentThreadId;`** re-export
- **`handle_agent_location_changed()`**: Uses `CollaboratorId::Agent(thread_id)` for per-thread follower state key. Currently uses placeholder thread ID until the project crate's `AgentLocationChanged` event carries `agent_thread_id`
- **`leader_border_for_pane()`**: Uses `cx.theme().players().agent_for_thread(&thread_id).cursor` for per-thread border color
- **`active_item_for_agent()`**: Signature changed to `(thread_id: AgentThreadId)`
- All 11 `CollaboratorId::Agent` references updated to tuple variant with thread ID

### crates/workspace/src/item.rs
- `CollaboratorId::Agent` → `CollaboratorId::Agent(_)` in match arm

### crates/workspace/src/pane_group.rs
- `CollaboratorId::Agent` → `CollaboratorId::Agent(thread_id)` in match
- Leader decoration color uses `agent_for_thread()`

### crates/clock/src/clock.rs
- **Added `AGENT_REPLICA_BASE: u16 = 100`**
- **Added `ReplicaId::for_agent_thread_hash(thread_hash: u64)`**: deterministic mapping from hash to replica ID in range 100-1099
- **Added `ReplicaId::is_agent_thread(self) -> bool`**: checks if replica ID is in agent thread range

### crates/editor/src/editor.rs
- `replica_id == ReplicaId::AGENT` → `replica_id.is_agent_thread()`
- Derives `AgentThreadId` from replica ID for `RemoteSelection` (placeholder — needs reverse mapping from Project)
- Cursor color uses `agent_for_thread()`

### crates/editor/src/element.rs
- `CollaboratorId::Agent` → `CollaboratorId::Agent(thread_id)` in match
- Selection styling uses `agent_for_thread()`

### crates/agent_ui/src/conversation_view/thread_view.rs
- **Added `agent_thread_id()` helper**: `AgentThreadId::from_session_id(self.session_id.0.as_ref())`
- All 9 `CollaboratorId::Agent` → `CollaboratorId::Agent(self.agent_thread_id())` or `this.agent_thread_id()`
- Follow toggle `selected_icon_color` uses `agent_for_thread()` — per-thread glow color
- Fixed async closure borrow issues by pre-computing `agent_thread_id` before `cx.spawn()`

### crates/agent_ui/src/agent_panel.rs
- `Follow` action uses `CollaboratorId::Agent(AgentThreadId::from_session_id("default"))` (placeholder)

### crates/agent_ui/src/message_editor.rs
- `ChatWithFollow` action uses placeholder thread ID (TODO: needs session context)

## Design Decision: AgentThreadId as Copy u64

Initially attempted `AgentThreadId = Arc<str>` (direct session ID alias), but this required removing `Copy` from `CollaboratorId` and `ViewId`, causing ~70+ compilation errors across 15+ crates. The `Copy` derive chain (`CollaboratorId` → `ViewId` → `FollowerState` keys, `.copied()` iterators, pattern match derefs) is deeply embedded.

Solution: `AgentThreadId(pub u64)` — a `Copy` newtype wrapping a deterministic hash of the session ID string. Same session ID always maps to same `AgentThreadId`. The theme crate owns this type and workspace re-exports it.

## Remaining TODOs (for other workers or follow-up)
1. **Project crate**: `AgentLocation` needs `agent_thread_id: AgentThreadId` field, `AgentLocationChanged` event carries it, `agent_locations: HashMap<AgentThreadId, AgentLocation>`, `set_agent_location()` uses `ReplicaId::for_agent_thread_hash()`
2. **Buffer crate**: `set_agent_selections()` / `remove_agent_selections()` need `replica_id: ReplicaId` parameter
3. **Agent thread crate**: Remove root-thread-only guard, always set location with agent_thread_id, per-thread turn-end cleanup
4. **Editor reverse mapping**: Need `HashMap<ReplicaId, AgentThreadId>` to resolve per-thread colors from buffer replica IDs
5. **agent_panel.rs / message_editor.rs**: Replace placeholder thread IDs with actual session context
