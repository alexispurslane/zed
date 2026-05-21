//! Telemetry has been removed. This module provides stubs for the public API
//! so that dependent crates continue to compile without changes.

use gpui::App;
use std::sync::Arc;
use telemetry_events::AssistantEventData;

pub struct Telemetry;

impl Telemetry {
    pub fn new(
        _clock: Arc<dyn clock::SystemClock>,
        _client: Arc<http_client::HttpClientWithUrl>,
        _cx: &mut App,
    ) -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn start(
        self: &Arc<Self>,
        _system_id: Option<String>,
        _installation_id: Option<String>,
        _session_id: String,
        _cx: &App,
    ) {
    }

    pub fn metrics_enabled(self: &Arc<Self>) -> bool {
        false
    }

    pub fn diagnostics_enabled(self: &Arc<Self>) -> bool {
        false
    }

    pub fn set_authenticated_user_info(
        self: &Arc<Self>,
        _metrics_id: Option<String>,
        _is_staff: bool,
    ) {
    }

    pub fn report_assistant_event(self: &Arc<Self>, _event: AssistantEventData) {}

    pub fn log_edit_event(self: &Arc<Self>, _environment: &'static str, _is_via_ssh: bool) {}

    pub fn report_discovered_project_type_events(
        self: &Arc<Self>,
        _worktree_id: worktree::WorktreeId,
        _updated_entries_set: &worktree::UpdatedEntriesSet,
    ) {
    }

    pub fn has_checksum_seed(&self) -> bool {
        false
    }

    pub fn metrics_id(self: &Arc<Self>) -> Option<Arc<str>> {
        None
    }

    pub fn system_id(self: &Arc<Self>) -> Option<Arc<str>> {
        None
    }

    pub fn installation_id(self: &Arc<Self>) -> Option<Arc<str>> {
        None
    }

    pub fn is_staff(self: &Arc<Self>) -> Option<bool> {
        None
    }

    pub fn flush_events(self: &Arc<Self>) -> gpui::Task<()> {
        gpui::Task::ready(())
    }
}

pub fn os_name() -> String {
    #[cfg(target_os = "macos")]
    {
        "macOS".to_string()
    }
    #[cfg(target_os = "linux")]
    {
        format!("Linux {}", gpui::guess_compositor())
    }
    #[cfg(target_os = "freebsd")]
    {
        format!("FreeBSD {}", gpui::guess_compositor())
    }
    #[cfg(target_os = "windows")]
    {
        "Windows".to_string()
    }
}

pub fn os_version() -> String {
    "unknown".to_string()
}

pub static MINIDUMP_ENDPOINT: std::sync::LazyLock<Option<String>> =
    std::sync::LazyLock::new(|| None);
