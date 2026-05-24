#![allow(missing_docs)]

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use gpui::Hsla;
use serde::Deserialize;

use crate::{amber, blue, jade, lime, orange, pink, purple, red};

/// Uniquely identifies an agent thread. Derived from the thread's session ID.
/// This is a Copy type (u64 hash) so it can be used in Copy derive chains.
/// TODO: Replace with `project::AgentThreadId` once that crate exports it.
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct AgentThreadId(pub u64);

impl AgentThreadId {
    /// Create from a session ID string (typically from schema::SessionId).
    /// Uses a deterministic hash so the same session ID always produces the same AgentThreadId.
    pub fn from_session_id(session_id: &str) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        session_id.hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq)]
pub struct PlayerColor {
    pub cursor: Hsla,
    pub background: Hsla,
    pub selection: Hsla,
}

/// A collection of colors that are used to color players in the editor.
///
/// The first color is always the local player's color, usually a blue.
///
/// The rest of the default colors crisscross back and forth on the
/// color wheel so that the colors are as distinct as possible.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PlayerColors(pub Vec<PlayerColor>);

impl Default for PlayerColors {
    /// Don't use this!
    /// We have to have a default to be `[refineable::Refinable]`.
    /// TODO "Find a way to not need this for Refinable"
    fn default() -> Self {
        Self::dark()
    }
}

impl PlayerColors {
    pub fn dark() -> Self {
        Self(vec![
            PlayerColor {
                cursor: blue().dark().step_9(),
                background: blue().dark().step_5(),
                selection: blue().dark().step_3(),
            },
            PlayerColor {
                cursor: orange().dark().step_9(),
                background: orange().dark().step_5(),
                selection: orange().dark().step_3(),
            },
            PlayerColor {
                cursor: pink().dark().step_9(),
                background: pink().dark().step_5(),
                selection: pink().dark().step_3(),
            },
            PlayerColor {
                cursor: lime().dark().step_9(),
                background: lime().dark().step_5(),
                selection: lime().dark().step_3(),
            },
            PlayerColor {
                cursor: purple().dark().step_9(),
                background: purple().dark().step_5(),
                selection: purple().dark().step_3(),
            },
            PlayerColor {
                cursor: amber().dark().step_9(),
                background: amber().dark().step_5(),
                selection: amber().dark().step_3(),
            },
            PlayerColor {
                cursor: jade().dark().step_9(),
                background: jade().dark().step_5(),
                selection: jade().dark().step_3(),
            },
            PlayerColor {
                cursor: red().dark().step_9(),
                background: red().dark().step_5(),
                selection: red().dark().step_3(),
            },
        ])
    }

    pub fn light() -> Self {
        Self(vec![
            PlayerColor {
                cursor: blue().light().step_9(),
                background: blue().light().step_4(),
                selection: blue().light().step_3(),
            },
            PlayerColor {
                cursor: orange().light().step_9(),
                background: orange().light().step_4(),
                selection: orange().light().step_3(),
            },
            PlayerColor {
                cursor: pink().light().step_9(),
                background: pink().light().step_4(),
                selection: pink().light().step_3(),
            },
            PlayerColor {
                cursor: lime().light().step_9(),
                background: lime().light().step_4(),
                selection: lime().light().step_3(),
            },
            PlayerColor {
                cursor: purple().light().step_9(),
                background: purple().light().step_4(),
                selection: purple().light().step_3(),
            },
            PlayerColor {
                cursor: amber().light().step_9(),
                background: amber().light().step_4(),
                selection: amber().light().step_3(),
            },
            PlayerColor {
                cursor: jade().light().step_9(),
                background: jade().light().step_4(),
                selection: jade().light().step_3(),
            },
            PlayerColor {
                cursor: red().light().step_9(),
                background: red().light().step_4(),
                selection: red().light().step_3(),
            },
        ])
    }
}

impl PlayerColors {
    pub fn local(&self) -> PlayerColor {
        *self.0.first().unwrap()
    }

    #[deprecated(note = "Use agent_for_thread() instead for per-thread agent colors")]
    pub fn agent(&self) -> PlayerColor {
        *self.0.last().unwrap()
    }

    /// Color for a specific agent thread. Deterministic from the thread ID —
    /// the same thread always gets the same color across restarts.
    /// Uses indices 1..len-1 from the palette (skipping local=0 and the last
    /// color which was the old single-agent color), cycling through
    /// orange, pink, lime, purple, amber, jade.
    pub fn agent_for_thread(&self, thread_id: &AgentThreadId) -> PlayerColor {
        let index = thread_id.0 as usize;
        let color_count = self.0.len().saturating_sub(2);
        let color_index = if color_count > 0 {
            (index % color_count) + 1
        } else {
            0
        };
        self.0[color_index]
    }

    pub fn absent(&self) -> PlayerColor {
        *self.0.last().unwrap()
    }

    pub fn read_only(&self) -> PlayerColor {
        let local = self.local();
        PlayerColor {
            cursor: local.cursor.grayscale(),
            background: local.background.grayscale(),
            selection: local.selection.grayscale(),
        }
    }

    pub fn color_for_participant(&self, participant_index: u32) -> PlayerColor {
        let len = self.0.len() - 1;
        self.0[(participant_index as usize % len) + 1]
    }
}
