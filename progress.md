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

## Task B: Remove Plan and plan_chip from title_bar, and remove Plan-gating from agent_ui and agent — COMPLETED

### Changes made:

1. **Deleted `crates/title_bar/src/plan_chip.rs`** — Entire PlanChip component (UI chip showing subscription plan tier) deleted.

2. **`crates/title_bar/src/title_bar.rs`**:
   - Removed `mod plan_chip;`
   - Removed `use crate::plan_chip::PlanChip;`
   - Removed `use client::Plan;`
   - Removed `has_subscription_period` and `plan` variables from `render_user_menu_button()`
   - Changed organizations vector from `Vec<(Arc<Organization>, Option<Plan>)>` to `Vec<Arc<Organization>>`
   - Removed PlanChip rendering from organization entries and user menu
   - Simplified organization iteration (no plan passed through)

3. **`crates/agent_ui/src/agent_configuration.rs`**:
   - Removed `use client::Plan;`
   - Removed `use language_model::XENOMORPHIC_CLOUD_PROVIDER_ID;`
   - Removed `is_xenomorphic_provider` and `current_plan` variables
   - Removed plan-check conditional in provider header rendering
   - Deleted `render_zed_plan_info()` method entirely (displayed plan chip next to cloud provider)
   - Simplified to always show authentication check icon (no plan-gated branch)

4. **`crates/agent/src/thread.rs`**:
   - Removed `use client::UserStore;`
   - Removed `use client::Plan;`
   - Removed `use language_model_core::XENOMORPHIC_CLOUD_PROVIDER_ID;`
   - Removed `user_store: Entity<UserStore>` field from `Thread` struct
   - Removed `user_store` initialization from both `Thread::new()` and `Thread::from_db_thread()` constructors
   - Simplified `handle_completion_error()`: removed `plan` parameter, removed cloud-provider plan-gating (`auto_retry` now always proceeds to retry strategy check)
   - Fixed `_cx` unused variable warning

5. **Also fixed pre-existing compilation errors in edit_prediction_ui** (from prior refactor phases):
   - `edit_prediction_button.rs`: Removed stale `_user = None` and `zed_cloud_needs_sign_in` references, cleaned tooltip rendering
   - Fixed `_user = None` without type annotation in menu builder

### Build status: ✅ PASSING (0 errors, warnings only)

### Additional fixes during Task B:

1. **`crates/project/src/project.rs`**: Re-added `user_store: Entity<UserStore>` field and accessor.
   The field was removed by a parallel subagent but was still referenced by `editor`, `workspace`,
   and `component_preview`. Rather than rip UserStore from all callers (which would be a massive
   change touching 20+ files), kept it in Project as a stub that creates UserStore on construction.
   Removed `user_store` parameter from `in_room()` and `from_join_project_response()`.

2. **`crates/inspector_ui/src/inspector.rs`**: Removed extra `user_store` argument from `Project::local()` call.

3. **`crates/client/src/client.rs`**: Added type annotations to `Ok(Credentials { ... })` to fix inference errors.

4. **`crates/xenomorphic/src/xenomorphic_app.rs`**: Removed unused `OpenAccountSettings` import.

5. **`crates/settings_ui/src/pages/feature_flags.rs`**: Deleted (cloud feature flags page).

### Summary of all Task B changes:

| File | Change |
|------|--------|
| `title_bar/src/plan_chip.rs` | **DELETED** |
| `title_bar/src/title_bar.rs` | Removed Plan, PlanChip; simplified org list |
| `agent_ui/src/agent_configuration.rs` | Removed Plan, XENOMORPHIC_CLOUD_PROVIDER_ID, render_zed_plan_info |
| `agent/src/thread.rs` | Removed UserStore, Plan fields/params; simplified handle_completion_error |
| `edit_prediction_ui/src/edit_prediction_button.rs` | Fixed stale cloud references |
| `project/src/project.rs` | Re-added user_store field; removed from constructor params |
| `inspector_ui/src/inspector.rs` | Fixed Project::local() call |
| `client/src/client.rs` | Fixed type inference |
| `settings_ui/src/pages/feature_flags.rs` | **DELETED** |

## Task C: Remove UserStore from non-client crates

### Completed

**Removed `UserStore` from API surface of these crates/files:**

1. **`notifications/src/notification_store.rs`** — Removed `user_store` param from `init()`, `new()`, and struct. Removed cloud user fetch and contact request handling.

2. **`project/src/project.rs`** — Removed `user_store` field, `UserStore` import, and `user_store()` accessor. Updated `Project::local()`, `Project::remote()`, `Project::in_room()`, `from_join_project_response()` to not take `user_store`. Updated ALL callers across ~20 files.

3. **`edit_prediction/src/edit_prediction.rs`** — Removed `user_store` field from `EditPredictionStore`, removed from `new()` and `global()`. Replaced `user_store.plan()` checks with no-ops. Replaced `user_store.current_organization()` with `None`. Removed cloud usage tracking and data collection org checks.

4. **`edit_prediction/src/xenomorphic_edit_prediction_delegate.rs`** — Removed `user_store` parameter from `new()`. Removed cloud plan-gating check in `refresh()`.

5. **`edit_prediction/src/onboarding_modal.rs`** — Removed `UserStore` import and `_user_store` parameter from `toggle()`.

6. **`edit_prediction/src/capture_example.rs`** — Removed `UserStore` import and creation.

7. **`edit_prediction/src/edit_prediction_tests.rs`** — Removed `UserStore` import and creation. Updated test to not check cloud org configuration for data collection.

8. **`edit_prediction_ui/src/edit_prediction_button.rs`** — Removed `user_store` field, import, and constructor param. Replaced `account_too_young()` and `has_overdue_invoices()` checks with `false`. Replaced `current_organization_configuration()` check with no-op.

9. **`edit_prediction_ui/src/edit_prediction_context_view.rs`** — Removed `UserStore` import and `user_store` parameter.

10. **`edit_prediction_cli/src/headless.rs`** — Removed `user_store` field from `EpAppState` and creation in `init()`.

11. **`eval_cli/src/headless.rs`** — Removed `user_store` field from `AgentCliAppState` and creation in `init()`.

12. **`component_preview/src/component_preview.rs`** — Removed `user_store` field, import, and constructor param. Updated all callers.

13. **`component_preview/examples/component_preview.rs`** — Updated to not pass `user_store` to `ComponentPreview::new()`.

14. **`agent_ui/src/conversation_view/thread_view.rs`** — Removed `project.user_store()` calls. Replaced cloud org configuration checks with no-ops.

15. **`editor/src/editor.rs`** — Replaced `user_store().participant_indices()` and `participant_names()` with empty implementations.

16. **`editor/src/git.rs`** — Removed `project.user_store()` usage for avatar URI.

17. **`client/src/cloud_types.rs`** — Added `RefreshLlmTokenListener::register_global()` method that doesn't require `UserStore`.

18. **`xenomorphic/src/xenomorphic_app/edit_prediction_registry.rs`** — Full rewrite: removed `user_store` parameter from `init()`, `assign_edit_prediction_providers()`, `assign_edit_prediction_provider()`. Removed `user_store` subscription for cloud user events. Removed cloud org configuration check.

19. **`xenomorphic/src/main.rs`** — Updated `notifications::init()` and `edit_prediction_registry::init()` calls.

20. **`xenomorphic/src/xenomorphic_app.rs`** — Updated `notifications::init()`, `edit_prediction_registry::init()`, `language_models::init()`, `web_search_providers::init()`, and `EditPredictionButton::new()` calls.

### Updated ~30+ files for Project::local/remote signature change

Removed `user_store` argument from all callers of `Project::local()` and `Project::remote()` across: workspace, multi_workspace, agent_ui, settings_ui, recent_projects, edit_prediction_cli, component_preview, project_benchmarks, terminal_view, git_ui, xenomorphic visual test runner.

### Remaining UserStore references (expected deliberate holds)

- `workspace::AppState.user_store` — still holds UserStore, accessible via `workspace.user_store()`. Many crates still use this. Full removal requires a separate pass.
- `title_bar::TitleBar.user_store` — still used for plan chip. Already partially cleaned by Task B.
- **Test fixtures** — `inline_assistant`, `sidebar_tests`, `remote_editing_tests`, `agent/tests`, `visual_test_runner`, `project_benchmarks` still create UserStore for test setup. This is fine since UserStore still exists in the client crate.
- `xenomorphic/src/main.rs` — creates UserStore during app initialization. Expected since AppState still needs it.

### Build Status
- `cargo check` ✅ passes (0 errors)
- `cargo test --no-run` ✅ passes (0 errors)
