# Remove Zed Cloud Refactor - Progress (Phases 9-12)

## What I Changed

### Phase 9: Feature flags cleanup ✅
**Files modified:**
- `crates/feature_flags/src/feature_flags.rs`:
  - Removed `OnFlagsReady` struct
  - Removed `from_wire()` method from `FeatureFlagValue` trait and `PresenceFlag` impl
  - Removed `FeatureFlagViewExt<V>` trait and its `impl Context<V>` block (observe_flag, when_flag_enabled)
  - Removed `on_flags_ready()` from `FeatureFlagAppExt` trait and impl
  - Kept `update_flags(staff, flags)` as compatibility shim (just calls `set_staff`)
  - Removed `Window` from imports, removed `RefCell`/`Rc` unused imports
- `crates/feature_flags/src/store.rs`:
  - Removed `server_flags: HashMap<String, String>` field from `FeatureFlagStore`
  - Removed `server_flags_received: bool` field
  - Removed `update_server_flags()` and `server_flags_received()` methods
  - Removed server-delivered flag path from `try_flag_value()` and `resolved_key()`
  - Updated tests: removed server flag tests, `on_flags_ready` test; replaced `off_override_beats_server_flag` with `off_override_beats_staff`
  - Removed unused `collections::HashMap` import and added back `BorrowAppContext`
- `crates/feature_flags_macros/src/feature_flags_macros.rs`:
  - Removed `from_wire` impl from derived enum macro

### Phase 10: Settings UI cleanup ✅
**Files modified:**
- `crates/settings_ui/src/page_data.rs`:
  - Removed "Feature Flags" `SubPageLink` and `SectionHeader` from `developer_page()`
  - Removed `collaboration_panel_section()` function entirely (4 settings items)
  - Removed `collaboration_panel_section()` call from panels page

### Phase 11: Recent Projects cleanup ✅
**Files modified:**
- Deleted `crates/recent_projects/src/dev_container_suggest.rs` (155 lines)
- `crates/recent_projects/Cargo.toml`: Removed `db` dependency

### Other fixes needed for the changes above
- `crates/sidebar/src/sidebar.rs`: Replaced `FeatureFlagViewExt::observe_flag` with `observe_global::<FeatureFlagStore>` + `flag_value()`, removed `FeatureFlagViewExt` import
- `crates/agent_ui/src/agent_ui.rs`: Removed `on_flags_ready` callback for command palette filter
- `crates/xenomorphic/src/reliability.rs`: Removed `on_flags_ready` block that uploaded build timings to cloud
- `crates/web_search/src/web_search.rs`: Moved `WebSearchResponse` and `WebSearchResult` types from deleted `cloud_llm_client` to this crate
- `crates/web_search_providers/src/web_search_providers.rs`: Simplified to stub (removed cloud provider, `client`/`UserStore`/`LanguageModel` deps)
- `crates/web_search_providers/Cargo.toml`: Removed most deps, kept only `gpui` and `web_search`
- `crates/agent/src/tools/web_search_tool.rs`: Changed import from `cloud_llm_client::WebSearchResponse` to `web_search::WebSearchResponse`
- `crates/extension/Cargo.toml`: Added missing `strum` dependency

### Client crate on_flags_ready fix
- `crates/client/src/client.rs`: Replaced `cx.on_flags_ready(|state, _cx| { ... })` with `cx.is_staff()` direct call

## Build Status

- ✅ `feature_flags` crate compiles cleanly
- ✅ `settings_ui` crate compiles
- ✅ `recent_projects` crate compiles
- ✅ `sidebar` crate compiles
- ✅ `web_search` crate compiles
- ✅ `agent` crate compiles (standalone)
- ❌ `client` crate: 8 errors from incomplete Phase 7/8 (references to deleted `cloud_api_client`, `cloud_api_types`, `cloud_llm_client` crates in code)
- ❌ Full workspace: blocked on `client` crate fixing Phase 7/8

## What Was NOT Done (deferred)

### Phase 9: Proto/RPC cleanup
- Deferred because `client` and `project` crates still reference cloud proto message types
- The `call.proto` and `channel.proto` files still exist and generate message types
- Removing proto messages would break `client` crate further
- Should be done after Phase 8 (client refactoring) is complete

### Phase 12: Recent Projects - remote_servers.rs
- `crates/recent_projects/src/remote_servers.rs` is heavily intertwined with `dev_container`
- Deleting the whole file would remove SSH remote server management too
- Proper cleanup would require separating SSH code from dev_container code
- Only `dev_container_suggest.rs` was removed (the low-hanging fruit)

## Pre-existing Issues (from incomplete Phase 7/8)

The `client` crate has 8 compilation errors from references to deleted cloud crates:
- `use cloud_api_client::{ClientApiError, CloudApiClient, LlmApiToken}`
- `use cloud_api_types::OrganizationId`
- `use cloud_llm_client::{...}`
- `mod llm_token;` (file deleted)
- `use crate::cloud_types;` (module not declared)
- `use tokio_native_tls` (platform-specific)

These are NOT from my Phase 9-12 changes — they're from the incomplete Phase 7/8 refactoring done by a parallel subagent.
