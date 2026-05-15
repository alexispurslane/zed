//! Agent thread types — the core entity for agent conversation threads.
//!
//! This crate replaces the former `acp_thread` and `agent-client-protocol::schema`
//! dependencies with locally-defined types.

mod connection;
mod diff;
mod mention;
mod terminal;
mod thread;
pub mod schema;

pub use connection::*;
pub use diff::*;
pub use mention::*;
pub use terminal::*;
pub use thread::*;
