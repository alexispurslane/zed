# Phase 4: Delete Call/Audio/LiveKit/Channel Crates - COMPLETED ✅

## Crates Deleted (6 crates, ~10,913 lines removed)

| Crate | Lines | Status |
|-------|------:|--------|
| `denoise` | 507 | ✅ Deleted |
| `audio` | 1,183 | ✅ Deleted |
| `livekit_api` | 302 | ✅ Deleted |
| `livekit_client` | 3,991 | ✅ Deleted |
| `call` | 3,001 | ✅ Deleted |
| `channel` | 1,929 | ✅ Deleted |

## Cargo.toml Changes

- **Root `Cargo.toml`**: Removed 6 workspace members, removed workspace dependency entries for `audio`, `call`, `channel`, `livekit_api`, `livekit_client`, `libwebrtc`, `webrtc-sys`, removed `[patch.crates-io]` entries for livekit-rust-sdks/libwebrtc/webrtc-sys
- **`crates/xenomorphic/Cargo.toml`**: Removed `audio`, `call`, `channel` deps; removed `features = ["audio"]` from `agent_ui`; removed `call` from dev-dependencies
- **`crates/title_bar/Cargo.toml`**: Removed `call`, `channel`, `livekit_client` deps; removed `call` from test-support and dev-deps; removed `screen-capture` gpui feature
- **`crates/git_ui/Cargo.toml`**: Removed `call` dep
- **`crates/notifications/Cargo.toml`**: Removed `channel` dep and test-support feature
- **`crates/settings_ui/Cargo.toml`**: Removed `audio`, `cpal`, `rodio` deps
- **`crates/file_finder/Cargo.toml`**: Removed `channel` dep
- **`crates/agent_ui/Cargo.toml`**: Removed `audio` feature and optional dep
- **`crates/sidebar/Cargo.toml`**: Removed `features = ["audio"]` from `agent_ui` dep

## Source Code Changes

### `crates/xenomorphic/` (3 files)
- **`xenomorphic_app.rs`**: Removed `audio::init()`, `channel::init()`, `call::init()` calls
- **`main.rs`**: Removed same init calls
- **`visual_test_runner.rs`**: Removed `audio::init()`, `call::init()` calls
- **`xenomorphic_app/visual_tests.rs`**: Removed `audio::init()` call

### `crates/title_bar/` (3 files deleted, 1 heavily modified)
- **Deleted `collab.rs`** (722 lines): All call/channel/livekit UI (toggle_screen_sharing, toggle_mute, toggle_deafen, render_collaborator_list, render_call_controls)
- **`title_bar.rs`**: 
  - Removed `call::ActiveCall`, `cloud_api_types::Plan` direct imports (kept Plan via cloud_api_types re-add)
  - Changed `actions!(collab, [...])` to `actions!(title_bar, [...])`, removed call-related actions (SimulateUpdateAvailable)
  - Removed `screen_share_popover_handle` and `_diagnostics_subscription` fields from `TitleBar` struct
  - Removed `ActiveCall::global(cx)` subscriptions in `new()`
  - Removed `window_activation_changed` ActiveCall tracking
  - Removed `active_call_changed()`, `observe_diagnostics()`, `share_project()`, `unshare_project()` methods
  - Removed `render_collaborator_list()` and `render_call_controls()` calls from render
  - Simplified `render_project_host()` to remove collab host display
  - Removed `toggle_update_simulation()` method
  - Kept `PlanChip` in user menu (cloud_api_types not yet removed)

### `crates/git_ui/`
- **`git_panel.rs`**: Stubbed `potential_co_authors()` to return `Vec::default()`, removed `local_committer()` method, replaced `ActiveCall`/room references in render with `has_co_authors = false`

### `crates/notifications/`
- **`notification_store.rs`**: Removed `channel::ChannelStore` import and field, removed `ChannelInvitation` handling in `respond_to_notification()` and `add_notifications()`, removed `ChannelId` import

### `crates/settings_ui/` (2 files deleted, 2 modified)
- **Deleted `pages/audio_input_output_setup.rs`**: Audio device selection UI
- **Deleted `pages/audio_test_window.rs`**: Audio test window
- **`pages.rs`**: Removed audio module declarations and re-exports
- **`page_data.rs`**: Removed `open_audio_test_window` import, emptied `collaboration_page()`, removed `calls_section()`, `audio_settings()`, removed `AudioInputDeviceName`/`AudioOutputDeviceName` imports and DEFAULT_AUDIO constants
- **`settings_ui.rs`**: Removed audio renderer registrations

### `crates/file_finder/`
- **`file_finder.rs`**: 
  - Removed `channel::ChannelStore` and `client::ChannelId` imports
  - Removed `channel_store` field from `FileFinderDelegate`
  - Removed `Match::Channel` enum variant
  - Removed all channel matching logic (~55 lines)
  - Removed `OpenChannelNotesById` import and dispatch
  - Removed `Match::Channel` rendering (hash icon, channel name display)
  - Fixed `file_icon` from match expression to direct `maybe!()` macro

### `crates/agent_ui/`
- **`conversation_view.rs`**: Removed `audio` feature-gated imports and `play_notification_sound()` method, removed `#[cfg(feature = "audio")]` guard on notification call

### `crates/workspace/` (1 file deleted, 1 heavily modified)
- **Deleted `shared_screen.rs`**: Shared screen viewing during calls
- **`workspace.rs`**:
  - Removed `pub mod shared_screen` and `pub use SharedScreen`
  - Removed `open_shared_screen()` method and all calls
  - Removed `shared_screen_for_peer()` method
  - Removed all `open_shared_screen` invocations in auto-watch code
  - Removed call-related close prompt ("Do you want to leave the current call?")
  - Removed call-related actions from `actions!(collab, [...])`: Mute, Deafen, LeaveCall, ShareProject, ScreenShare, CopyRoomId
  - Removed `create_shared_screen` from `AnyActiveCall` trait
  - Kept `AnyActiveCall` trait, `GlobalAnyActiveCall`, `RemoteCollaborator`, `ParticipantLocation`, `ActiveCallEvent` types (they're part of public API, no one implements them now so they're effectively dead code)

### `crates/edit_prediction/`
- **`onboarding_modal.rs`**: Added missing `Entity` import from gpui

## Build Status
- `cargo check` passes ✅ with only warnings (unused variables, unused imports from prior phases)
- No compilation errors

## Notes
- The `AnyActiveCall` trait and related types in `workspace` are now dead code (no implementers), but kept to minimize cascading changes. They can be removed in Phase 8 (client crate stripping).
- The `collaboration_page` in settings_ui is now empty (no items). It could be removed entirely in a later cleanup.
- The `livekit` workspace dependency is still present since it's used transitively by the crate graph. The patch entries were removed.
