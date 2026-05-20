# Progress

## Status
In Progress

## Tasks

## Files Changed

## Notes

## Task A: Remove xenomorphic_urls — COMPLETED

### Changes made:

1. **Deleted `crates/client/src/xenomorphic_urls.rs`** — 70 lines removed
2. **Removed `pub mod xenomorphic_urls;` from `crates/client/src/client.rs`**
3. **`crates/xenomorphic/src/xenomorphic_app.rs`**: Removed `use client::xenomorphic_urls;` and removed the `OpenAccountSettings` handler that opened cloud account URL
4. **`crates/title_bar/src/title_bar.rs`**: Removed `xenomorphic_urls` from import; replaced cloud account link custom_entry with a simple disabled menu item showing the user name; simplified organizations vector from tuples to just `Arc<Organization>`
5. **`crates/edit_prediction_ui/src/edit_prediction_button.rs`**: Removed `xenomorphic_urls` from import; replaced `xenomorphic_urls::edit_prediction_docs(cx)` with hardcoded `"https://xenomorphic.dev/docs/ai/edit-prediction"`; replaced all `xenomorphic_urls::account_url(cx)` calls with no-op handlers (these were cloud upsell links)
6. **`crates/agent_ui/src/conversation_view.rs`**: Removed `use client::xenomorphic_urls;`
7. **`crates/agent_ui/src/conversation_view/thread_view.rs`**: Replaced `client::xenomorphic_urls::shared_agent_thread_url(&session_id)` with inline `format!("xenomorphic://agent/shared/{}", session_id)` (local URL scheme); replaced `xenomorphic_urls::upgrade_to_xenomorphic_pro_url(cx)` with hardcoded `"https://xenomorphic.dev/account/upgrade"`

### Also fixed pre-existing compilation errors:
- **`crates/edit_prediction/src/edit_prediction.rs`**: Replaced `self.user_store.read(cx).current_organization()` with `None` (cloud org removed)
- **`crates/edit_prediction/src/xeta.rs`**: Replaced 3x `self.user_store` references with `None` for organization_id and removed usage tracking
- **`crates/xenomorphic/src/main.rs`**: Removed extra `user_store` argument from `edit_prediction_registry::init()` call

### Build status: ✅ PASSING (0 errors, warnings only)
