# Phase 7: Remove Cloud LLM Proxy Infrastructure - Progress Report

## Completed Work

### 1. Deleted 4 cloud crates
- `crates/cloud_api_client` (475 lines)
- `crates/cloud_api_types` (485 lines)
- `crates/cloud_llm_client` (425 lines)
- `crates/language_models_cloud` (982 lines)
- **Total: ~2,367 lines removed, 4 crates deleted**

### 2. Removed from root Cargo.toml
- Removed all 4 from workspace `members` list
- Removed all 4 from `[workspace.dependencies]`

### 3. Removed from dependent Cargo.toml files (15+ crates)
- `language_models`, `agent`, `agent_ui`, `client`, `edit_prediction`, `edit_prediction_ui`, `edit_prediction_cli`, `extension`, `extension_cli`, `extension_host`, `extensions_ui`, `language_model_core`, `title_bar`, `web_search_providers`, `web_search`

### 4. `client` crate - Major refactoring
- **Created `crates/client/src/cloud_types.rs`** — local stubs for types previously from cloud crates:
  - `Plan`, `OrganizationId`, `Organization`, `OrganizationConfiguration`, `PlanInfo`, `KnownOrUnknown`
  - `GetAuthenticatedUserResponse`, `AuthenticatedUser`, `SubscriptionPeriod`, `Timestamp`
  - `MessageToClient`, `UsageLimit`, `UsageData`, `CurrentUsage`, `LlmApiToken`
  - HTTP header constants: `EDIT_PREDICTIONS_USAGE_AMOUNT_HEADER_NAME`, etc.
- **Deleted `crates/client/src/llm_token.rs`** (117 lines)
- **Updated `crates/client/src/client.rs`**:
  - Replaced `mod llm_token` → `mod cloud_types`
  - Replaced `pub use llm_token::*` → `pub use cloud_types::*`
  - Removed all `cloud_api_client`/`cloud_api_types` imports
  - Removed `cloud_client: Arc<CloudApiClient>` field
  - Removed `message_to_client_handlers: Mutex<Vec<MessageToClientHandler>>` field
  - Removed `MessageToClientHandler` type alias
  - Removed `CloudApiClient::new()`, `cloud_client()` method
  - Removed `validate_credentials()` method
  - Removed `connect_to_cloud()` method
  - Removed `acquire_llm_token()`, `refresh_llm_token()`, `clear_and_refresh_llm_token()` methods
  - Removed `add_message_to_client_handler()`, `handle_message_to_client()` methods
  - Simplified `sign_in_with_optional_connect()` (removed is_staff/collab logic)
  - Simplified credential validation (assumes stored credentials are valid)
- **Updated `crates/client/src/user.rs`**:
  - Replaced cloud crate imports → `crate::cloud_types::*`
  - Stubbed `_maintain_current_user` (no more `cloud_client().get_authenticated_user()`)
  - Stubbed `handle_message_to_client`
  - Removed `add_message_to_client_handler` registration
- **Updated `crates/client/src/test.rs`**:
  - Replaced cloud crate imports → `crate::cloud_types::*`
- **Build: `cargo check -p client` passes (warnings only)**

### 5. `language_models` crate cleanup
- **Deleted `crates/language_models/src/provider/cloud.rs`** (778 lines)
- Removed `pub mod cloud;` from `provider.rs`
- Updated `language_models.rs`:
  - Removed `CloudLanguageModelProvider` import and registration
  - Removed `UserStore` parameter from `init()`
  - Simplified `update_environment_fallback_model()` (removed cloud preference logic)
- Updated `settings.rs`:
  - Removed `ZedDotDevSettings` and `zed_dot_dev` field

### 6. `language_model_core` fix
- Fixed `CompletionRequestStatus::Queued` field type (u32 → usize cast)

## Remaining Work (for continued Phase 7)

### `edit_prediction` crate (~17 errors)
Deep dependency on `cloud_llm_client` for:
- `predict_edits_v3::{RawCompletionRequest, RawCompletionResponse}`
- `EditPredictionRejectReason`
- `LlmApiToken` and token acquisition
- `NeedsLlmTokenRefresh` trait
- `global_llm_token()` function

These need either:
1. Relocating the prediction protocol types to a local module, OR
2. Removing cloud-mediated edit prediction entirely (keep only direct-to-provider prediction)

### `extension_host` crate (~4 errors)
- `cloud_api_types` imports for extension manifest types
- Need to relocate `SchemaKind` and related types locally

### Other crates with cloud references
- `agent`, `agent_ui`: `cloud_api_types::Plan` references
- `title_bar`: `plan_chip`, `cloud_api_types` references
- `web_search_providers`: `CloudWebSearchProvider`
- `web_search`: `cloud_llm_client` dependency

## Build Status
- ✅ `cargo check -p client` — passes (warnings only)
- ❌ `cargo check` — errors in `edit_prediction` and `extension_host` (cloud crate references)
