# Phase 7: Remove Cloud LLM Proxy Infrastructure - Progress

## Completed
- Deleted crate directories: `cloud_api_client`, `cloud_api_types`, `cloud_llm_client`, `language_models_cloud` (4 crates)
- Removed all 4 from root `Cargo.toml` workspace members and workspace.dependencies
- Removed cloud deps from 15+ dependent Cargo.toml files
- Created `crates/client/src/cloud_types.rs` with stub types (Plan, OrganizationId, LlmApiToken, etc.)
- Removed `crates/client/src/llm_token.rs`
- Updated `crates/client/src/client.rs`:
  - Replaced `mod llm_token` with `mod cloud_types`
  - Removed cloud API imports (cloud_api_client, cloud_api_types)
  - Removed `cloud_client` field, `MessageToClientHandler` type, `message_to_client_handlers` field
  - Removed `CloudApiClient` initialization
  - Removed `cloud_client()` accessor
  - Removed `validate_credentials()` method
  - Removed `connect_to_cloud()` method
  - Removed `acquire_llm_token()`, `refresh_llm_token()`, `clear_and_refresh_llm_token()` methods
  - Removed `add_message_to_client_handler()`, `handle_message_to_client()` methods
  - Simplified `sign_in_with_optional_connect()` to remove is_staff collab logic
  - Simplified credential validation (assumes stored credentials are valid)
  - Removed `cloud_client.set_credentials()` and `cloud_client.clear_credentials()` calls
- Updated `crates/client/src/user.rs`:
  - Replaced cloud crate imports with `crate::cloud_types::*`
  - Stubbed `_maintain_current_user` cloud client calls
  - Stubbed `handle_message_to_client`
  - Removed `add_message_to_client_handler` call
- Updated `crates/client/src/test.rs`:
  - Replaced cloud crate imports with `crate::cloud_types::*`
- Updated `crates/language_models/src/provider.rs`:
  - Removed `pub mod cloud;`
- Deleted `crates/language_models/src/provider/cloud.rs` (778 lines)
- Updated `crates/language_models/src/language_models.rs`:
  - Removed CloudLanguageModelProvider import and registration
  - Removed UserStore parameter from init()
  - Simplified update_environment_fallback_model()
- Updated `crates/language_models/src/settings.rs`:
  - Removed ZedDotDevSettings and cloud provider settings
- Updated `crates/language_model_core/src/language_model_core.rs`:
  - Fixed CompletionRequestStatus position type (u32 → usize cast)

## Still Needs Work (for remaining dependent crates)
- `edit_prediction` - deep cloud_llm_client dependency for predict_edits_v3
- `extension_host` - cloud_api_types dependency
- `agent` / `agent_ui` - cloud_api_types references
- `title_bar` - cloud_api_types, plan_chip references
- `web_search_providers` - cloud search provider
- `web_search` - cloud_llm_client dependency

## Build Status
- `cargo check -p client` passes (warnings only)
- `cargo check` has errors in `edit_prediction`, `extension_host` (cloud crate references)
