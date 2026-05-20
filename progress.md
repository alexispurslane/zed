# Remove Zed Cloud Refactor - Progress

## Completed Phases

### Phase 1: Trivial removals (journal, feedback, nc) ✅
- Deleted `journal`, `feedback`, `nc` crates

### Phase 2: Onboarding ✅
- Deleted `ai_onboarding`, `language_onboarding` crates
- Trimmed `onboarding` crate (removed AI section, telemetry section)
- Removed AI onboarding from `agent_ui`

### Phase 3: Auto-update ✅
- Deleted `auto_update`, `auto_update_helper`, `auto_update_ui` crates

### Phase 4: Call / Audio / LiveKit ✅
- Deleted `denoise`, `audio`, `livekit_api`, `livekit_client`, `call`, `channel` crates

### Phase 5: Collaboration UI ✅
- Deleted `collab_ui` crate

### Phase 6: Collab server ✅
- Deleted `collab` crate

### Phase 9: Feature flags cleanup (partial) ✅
- Removed `update_server_flags()`, `server_flags_received`, `on_flags_ready()`, `FeatureFlagViewExt`, `OnFlagsReady`, `from_wire` from feature_flags crate
- Kept `update_flags(staff, _flags)` as a compatibility shim for tests (just sets staff)
- Feature flag resolution now only uses: `enabled_for_all` → user overrides → staff defaults
- Server-delivered flag path removed from `try_flag_value()` and `resolved_key()`

### Phase 10: Settings UI cleanup ✅
- Removed "Feature Flags" sub-page link from Developer settings page
- Removed "Collaboration Panel" section from Panels settings page
- Kept Edit Prediction Provider Setup page (local providers, not cloud)
- Kept Tool Permissions Setup page (local, not cloud-synced)

### Phase 11: Recent Projects cleanup (partial) ✅
- Deleted `dev_container_suggest.rs`
- Removed `dev_container_suggest` module from `recent_projects.rs`
- Removed `db` dependency from `recent_projects/Cargo.toml`

## In Progress / Blocked

### Phase 7: Cloud LLM proxy (incomplete from earlier)
- Deleted `language_models_cloud`, `cloud_llm_client`, `cloud_api_client`, `cloud_api_types` crates
- BUT: many crates still reference these types (edit_prediction, agent, agent_ui, extension_host, etc.)
- `edit_prediction` crate has ~10 `use cloud_llm_client` statements that need replacement
- `web_search_providers/cloud.rs` deleted but cloud.rs references remained → fixed
- `WebSearchResponse` type moved from `cloud_llm_client` to `web_search` crate

### Phase 8: Client crate refactoring (incomplete from earlier)
- A WIP commit (d3cde46905) partially stripped the client crate
- Deleted `user.rs`, `llm_token.rs` from client crate
- Moved some types inline to `client.rs` (Plan, Organization, LlmApiToken stubs)
- Still references `cloud_api_client::CloudApiClient` with stub implementations
- project, title_bar, agent, agent_ui, edit_prediction, extension_host crates all have unresolved cloud type references
- **57 errors in `project` crate alone** from cloud type references

### Phase 9: Proto/RPC cleanup (deferred)
- Cannot safely remove proto messages until client/project crates no longer reference them
- Blocked on Phase 8 completion

## Build Status
- `feature_flags` crate: ✅ Compiles
- `client` crate: ✅ Compiles (with warnings)
- `settings_ui`: ✅ Compiles
- `recent_projects`: ✅ Compiles
- `sidebar`: ✅ Compiles
- Full workspace: ❌ 13+ errors across `project`, `edit_prediction`, `agent`, `agent_ui`, `extension_host`, `extensions_ui`, `title_bar` crates
  - All errors are from incomplete Phase 7/8 — references to deleted `cloud_api_client`, `cloud_api_types`, `cloud_llm_client` crates

## Summary of Changes Made (Phases 9-12)

### feature_flags crate
- `src/feature_flags.rs`: Removed `OnFlagsReady`, `from_wire()`, `FeatureFlagViewExt` trait, `on_flags_ready()` method, `update_flags()` → kept as compatibility shim
- `src/store.rs`: Removed `server_flags` field, `server_flags_received` field, `update_server_flags()` method, `server_flags_received()` method; removed server flag path from `try_flag_value()` and `resolved_key()`; updated tests
- `src/flags.rs`: No changes
- `src/settings.rs`: No changes
- `feature_flags_macros/src/feature_flags_macros.rs`: Removed `from_wire` from generated derive code

### settings_ui crate
- `src/page_data.rs`: Removed Feature Flags SubPageLink and SectionHeader from developer page; removed `collaboration_panel_section()` from panels page
- `src/pages/feature_flags.rs`: Still exists but no longer referenced from navigation
- `src/pages/edit_prediction_provider_setup.rs`: Kept (local providers)
- `src/pages/tool_permissions_setup.rs`: Kept (local permissions)

### recent_projects crate
- Deleted `src/dev_container_suggest.rs`
- `src/recent_projects.rs`: Removed `mod dev_container_suggest` and `suggest_on_worktree_updated` call
- `Cargo.toml`: Removed `db` dependency

### Other fixes
- `crates/sidebar/src/sidebar.rs`: Replaced `FeatureFlagViewExt::observe_flag` with `observe_global::<FeatureFlagStore>`
- `crates/agent_ui/src/agent_ui.rs`: Removed `on_flags_ready` callback
- `crates/xenomorphic/src/reliability.rs`: Removed `on_flags_ready` cloud timing upload
- `crates/web_search/src/web_search.rs`: Moved `WebSearchResponse` type from deleted `cloud_llm_client` to local
- `crates/web_search_providers/src/web_search_providers.rs`: Simplified to stub (cloud provider removed)
- `crates/agent/src/tools/web_search_tool.rs`: Changed import from `cloud_llm_client::WebSearchResponse` to `web_search::WebSearchResponse`
