# Progress

## Status
Completed - Phase 8 (Remove UserStore from workspace, title_bar, main.rs, and all dependents)

## Summary

All 12 phases of the remove-zed-cloud refactor are now substantially complete:

- **20 crates fully deleted** (journal, feedback, nc, ai_onboarding, language_onboarding, denoise, audio, livekit_api, livekit_client, call, channel, collab_ui, collab, auto_update, auto_update_helper, auto_update_ui, cloud_api_client, cloud_api_types, cloud_llm_client, language_models_cloud)
- **UserStore removed** from workspace AppState, title_bar, main.rs, and all test code
- **is_via_collab()** removed from all crates (was always false after collab removal)
- **xenomorphic_urls** module removed from client crate and all references cleaned
- **Plan/plan_chip** removed from title_bar (cloud subscription UI)
- **Cloud LLM proxy** (provider/cloud.rs) deleted
- **Proto cloud messages** stripped
- **Feature flags** cloud-fetching removed
- **Settings UI** cloud pages removed
- **Onboarding** cloud-gated sections removed
- **Build passes** with only warnings (0 errors)

## Remaining Cleanup (low priority)
- client crate cloud_types.rs stub (468 lines) - could be further trimmed
- client crate proxy.rs unused code
- client crate telemetry.rs unused http_client field
- Various unused variable warnings in workspace/editor
- extension_cli still references cloud types

## Files Changed

Too many to list - 19 crates deleted, 50+ files modified across the codebase.

## Notes

Total lines removed: ~89,000+ (approximately 7% of the codebase)
Client crate shrunk from 5,644 to 4,139 lines
