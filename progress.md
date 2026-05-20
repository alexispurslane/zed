# Progress: Remove Zed Cloud Infrastructure

## Status
**Phases 1-7 COMPLETE. Build passes with only warnings.**

## Completed Phases

### Phase 1: Trivial removals ✅
- Deleted: journal, feedback, nc (3 crates, ~495 lines)
- Removed from workspace Cargo.toml, xenomorphic/Cargo.toml, main.rs, xenomorphic_app.rs

### Phase 2: Onboarding ✅
- Deleted: ai_onboarding, language_onboarding (2 crates, ~1005 lines)
- Trimmed onboarding crate: removed render_ai_section(), render_telemetry_section(), cloud imports
- Removed AI onboarding from agent_ui

### Phase 3: Auto-update ✅
- Deleted: auto_update, auto_update_helper, auto_update_ui (3 crates, ~2837 lines)

### Phase 4: Call/Audio/LiveKit ✅
- Deleted: denoise, audio, livekit_api, livekit_client, call, channel (6 crates, ~10913 lines)

### Phase 5: Collaboration UI ✅
- Deleted: collab_ui (1 crate, ~6410 lines)

### Phase 6: Collab server ✅
- Deleted: collab (1 crate, ~43606 lines)

### Phase 7: Cloud LLM proxy ✅
- Deleted: cloud_api_client, cloud_api_types, cloud_llm_client, language_models_cloud (4 crates, ~2367 lines)
- Stripped client crate: removed CloudApiClient, cloud_client field, llm_token module, LLM token methods
- Rewrote user.rs: removed contacts/social/cloud-user-fetch, kept local org/plan infrastructure
- Added cloud_types.rs module: local stubs for Plan, OrganizationId, LlmApiToken, EditPredictionRejectReason, predict_edits_v3 types, RefreshLlmTokenListener
- Fixed all downstream crates: edit_prediction, extension, extension_host, extensions_ui, title_bar, sidebar, workspace, agent_ui
- Removed channel.proto, stripped cloud messages from proto
- Stubbed cloud_client() calls in agent_ui (submit_agent_feedback → no-op)

## Remaining Phases

### Phase 8: Strip client crate (partially done)
- UserStore still has some cloud code (update_authenticated_user)
- WebSocket connection code still present
- sign_in_with_optional_connect still has cloud connection logic
- xenomorphic_urls.rs still exists

### Phase 9: Proto/RPC cleanup
- Remove cloud-specific message types from proto
- Remove collab message handlers

### Phase 10: Feature flags cleanup ✅ (mostly done)
- Cloud-fetching already removed in earlier phase

### Phase 11: Settings UI cleanup ✅ (partially done)
- Audio device pages removed

### Phase 12: Recent Projects cleanup
- Remove dev_container dependency
- Remove cloud SSH relay connections

## Summary
- **20 crates deleted** (~68,633 lines removed from deleted crates)
- **4 cloud infrastructure crates deleted** in Phase 7 (~2,367 lines)
- **Build compiles successfully** with only warnings
- Total lines removed from codebase: ~70,000+

## Key Architectural Decisions
1. **cloud_types.rs**: Preserved cloud type stubs locally in the client crate so downstream code compiles without the cloud crates. Types like Plan, OrganizationId, LlmApiToken are kept as local definitions.
2. **RefreshLlmTokenListener**: Kept as a no-op stub for API compatibility - many callers do `RefreshLlmTokenListener::register(client, user_store, cx)` 
3. **Edit prediction types**: predict_edits_v3 types (RawCompletionRequest, etc.) moved to cloud_types.rs as stubs. Direct-to-provider edit prediction still works; cloud-hosted prediction paths return errors or are no-ops.
4. **UserStore**: Stripped to local-only functionality. Cloud user fetch removed (was via cloud_client().get_authenticated_user()). Contacts, social features removed. Organization/plan info still available via update_authenticated_user() for local/test use.
