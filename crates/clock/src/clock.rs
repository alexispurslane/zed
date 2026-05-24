mod system_clock;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    cmp::{self, Ordering},
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
};

pub use system_clock::*;

/// A unique identifier for each distributed node.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ReplicaId(u16);

impl ReplicaId {
    /// The local replica
    pub const LOCAL: ReplicaId = ReplicaId(0);
    /// The remote replica of the connected remote server.
    pub const REMOTE_SERVER: ReplicaId = ReplicaId(1);
    /// The agent's unique identifier.
    ///
    /// Deprecated: use `for_agent_thread()` for per-agent-thread replica IDs.
    /// This constant remains for backward compatibility during the migration.
    #[deprecated(note = "Use ReplicaId::for_agent_thread() for per-thread agent replica IDs")]
    pub const AGENT: ReplicaId = ReplicaId(2);
    /// A local branch.
    pub const LOCAL_BRANCH: ReplicaId = ReplicaId(3);
    /// The first collaborative replica ID, any replica equal or greater than this is a collaborative replica.
    pub const FIRST_COLLAB_ID: ReplicaId = ReplicaId(8);

    /// The base value for agent thread replica IDs.
    /// Agent thread replicas occupy the range [AGENT_REPLICA_BASE, AGENT_REPLICA_BASE + AGENT_REPLICA_RANGE).
    pub const AGENT_REPLICA_BASE: u16 = 100;
    /// The number of replica IDs reserved for agent threads.
    pub const AGENT_REPLICA_RANGE: u16 = 1000;

    pub fn new(id: u16) -> Self {
        ReplicaId(id)
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }

    pub fn is_remote(self) -> bool {
        self == ReplicaId::REMOTE_SERVER || self >= ReplicaId::FIRST_COLLAB_ID
    }

    /// Returns a deterministic replica ID for an agent thread, derived from the
    /// thread's ID string. The same thread ID always maps to the same replica ID.
    pub fn for_agent_thread(thread_id: &str) -> Self {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        thread_id.hash(&mut hasher);
        let hash = hasher.finish();
        let offset = (hash % (Self::AGENT_REPLICA_RANGE as u64)) as u16;
        ReplicaId(Self::AGENT_REPLICA_BASE + offset)
    }

    /// Returns true if this replica ID belongs to any agent thread.
    pub fn is_agent(self) -> bool {
        self.0 >= Self::AGENT_REPLICA_BASE
            && self.0 < Self::AGENT_REPLICA_BASE + Self::AGENT_REPLICA_RANGE
    }
}

impl fmt::Debug for ReplicaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == ReplicaId::LOCAL {
            write!(f, "<local>")
        } else if *self == ReplicaId::REMOTE_SERVER {
            write!(f, "<remote>")
        } else if *self == ReplicaId(2) {
            write!(f, "<agent-legacy>")
        } else if self.is_agent() {
            write!(f, "<agent-thread:{}>", self.0)
        } else if *self == ReplicaId::LOCAL_BRANCH {
            write!(f, "<branch>")
        } else {
            write!(f, "{}", self.0)
        }
    }
}

/// A [Lamport sequence number](https://en.wikipedia.org/wiki/Lamport_timestamp).
pub type Seq = u32;

/// A [Lamport timestamp](https://en.wikipedia.org/wiki/Lamport_timestamp),
/// used to determine the ordering of events in the editor.
