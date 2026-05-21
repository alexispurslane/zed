//! Telemetry has been removed. All macros and functions are no-ops.

/// No-op macro. Telemetry has been removed.
/// Expands to an empty block so it can be used in any expression position.
#[macro_export]
macro_rules! event {
    ($($any:tt)*) => {{}};
}

/// No-op. Telemetry has been removed.
pub fn send_event(_event: Event) {}

/// No-op. Telemetry has been removed.
pub fn init(_tx: futures::channel::mpsc::UnboundedSender<Event>) {}

pub use telemetry_events::FlexibleEvent as Event;
pub use serde_json;
