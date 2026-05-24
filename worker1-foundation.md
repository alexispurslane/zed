# Worker 1: Foundation Layer — COMPLETED ✅

## Changes Made

### crates/clock/src/clock.rs
- Added `std::hash::{Hash, Hasher}` and `std::sync::Arc` imports
- Added `ReplicaId::AGENT_REPLICA_BASE: u64 = 100` (public const)
- Added `ReplicaId::AGENT_REPLICA_COUNT: u64 = 1000` (private const)
- Added `ReplicaId::for_agent_thread(thread_id: &Arc<str>) -> Self` — deterministic hash mapping into range 100..1100
- Added `ReplicaId::is_agent(&self) -> bool` — checks if replica ID is in agent thread range
- Deprecated `ReplicaId::AGENT` with `#[deprecated]` attribute
- Updated `Debug` impl to recognize agent thread replica IDs (`<agent-thread:N>`)

### crates/language/src/buffer.rs
- `set_agent_selections()` — added `replica_id: ReplicaId` as second parameter, uses it instead of hardcoded `ReplicaId::AGENT`
- `remove_agent_selections()` — added `replica_id: ReplicaId` parameter, now directly removes from `remote_selections` TreeMap instead of calling `set_agent_selections` with empty selections

### crates/project/src/project.rs
- Added `pub type AgentThreadId = Arc<str>` with doc comment explaining it's the interface-boundary alias for agent thread IDs
- Changed `agent_location: Option<AgentLocation>` → `agent_locations: HashMap<AgentThreadId, AgentLocation>`
- Added `agent_thread_id: AgentThreadId` as first field of `AgentLocation` struct
- Changed `Event::AgentLocationChanged` to carry `AgentThreadId` (`AgentLocationChanged(AgentThreadId)`)
- Changed `AgentLocationChanged` from unit struct to newtype carrying `AgentThreadId`
- Rewrote `set_agent_location()` to upsert into HashMap, compute per-thread ReplicaId, and handle buffer changes
- Added `remove_agent_location()` — removes from HashMap and cleans up buffer selections
- Added `agent_location_for()` — get a specific thread's location
- Added `agent_locations()` — get all agent locations
- Deprecated old `agent_location()` getter
- Updated all three Project constructors to use `HashMap::default()`

## Compilation Status
- ✅ `clock` compiles (deprecation warning for AGENT constant — expected)
- ✅ `language` compiles
- ✅ `project` compiles (deprecation warnings from downstream consumers — expected)
