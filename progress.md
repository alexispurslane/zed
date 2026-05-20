# Phase 2 Progress: Delete onboarding cloud-gated crates and trim onboarding

## Status: COMPLETE (our targeted packages compile; full workspace has pre-existing errors from other phases)

## Part A: Deleted entire crates ✅

- Deleted `crates/ai_onboarding/` directory
- Deleted `crates/language_onboarding/` directory
- Removed both from workspace `Cargo.toml` members list and workspace dependency table
- Removed `language_onboarding.workspace = true` from `crates/xenomorphic/Cargo.toml`
- Removed `use language_onboarding::BasedPyrightBanner;` from `crates/xenomorphic/src/xenomorphic_app.rs`
- Removed BasedPyrightBanner toolbar item creation from `xenomorphic_app.rs`

## Part B: Trimmed the onboarding crate ✅

### `crates/onboarding/src/onboarding.rs`:
- Removed `use client::{Client, UserStore, xenomorphic_urls};`
- Removed `use cloud_api_types::Plan;`
- Removed `notifications::NotifyResultExt as _` import (unused after changes)
- Removed `SignIn` and `OpenAccount` actions from the actions! macro
- Removed `user_store` field from `Onboarding` struct
- Removed `user_store` field from `clone_on_split()` method
- Removed `handle_sign_in()` method
- Removed `handle_open_account()` method
- Removed `.on_action(cx.listener(Self::handle_sign_in))` and `.on_action(Self::handle_open_account)` from Render impl
- Removed cloud status/plan checking logic from `Onboarding::new()`
- Simplified `render_page()` to call `crate::basics_page::render_basics_page(cx)` (no longer takes `user_store` param)
- Kept: theme picker, base keymap picker, vim mode toggle, import VS Code/Cursor settings, auto-trust

### `crates/onboarding/src/basics_page.rs`:
- Removed `render_ai_section()` function entirely
- Removed `render_zed_agent_button()` function entirely
- Removed `render_telemetry_section()` function entirely
- Removed `use client::{Client, TelemetrySettings, UserStore, xenomorphic_urls};`
- Removed `use cloud_api_types::Plan;`
- Removed `use ui::{AgentSetupButton, ...Animation, AnimationExt, ...pulsating_between};`
- Updated `render_basics_page()` signature: removed `user_store` parameter
- Updated `render_basics_page()` body: removed calls to `render_ai_section()` and `render_telemetry_section()`
- Kept: `render_theme_section`, `render_base_keymap_section`, `render_import_settings_section`, `render_vim_mode_switch`, `render_worktree_auto_trust_switch`

### `crates/onboarding/Cargo.toml`:
- Removed `client.workspace = true`
- Removed `cloud_api_types.workspace = true`
- Removed `collections.workspace = true`
- Kept `telemetry.workspace = true` (still used for telemetry events)

## Part C: Removed AI onboarding from agent_ui ✅

### `crates/agent_ui/Cargo.toml`:
- Removed `ai_onboarding.workspace = true`

### `crates/agent_ui/src/agent_panel.rs`:
- Removed `use ai_onboarding::AgentPanelOnboarding;`
- Removed `use client::UserStore;`
- Removed `use cloud_api_types::Plan;`
- Removed `use db::kvp::{Dismissable, ...}` → changed to `use db::kvp::KeyValueStore;`
- Removed `new_user_onboarding: Entity<AgentPanelOnboarding>` field from struct
- Removed `new_user_onboarding_upsell_dismissed: AtomicBool` field from struct
- Removed `user_store: Entity<UserStore>` field from struct
- Removed local `user_store` and `client` variables from `new()`
- Removed `OnboardingUpsell::set_dismissed(false, cx)` from `ResetOnboarding` action handler
- Removed `ResetTrialUpsell` and `ResetTrialEndUpsell` action registrations
- Removed `dismiss_ai_onboarding()` method
- Removed `should_render_new_user_onboarding()` method
- Removed `render_new_user_onboarding()` method
- Removed `should_render_trial_end_upsell()` method
- Removed `render_trial_end_upsell()` method
- Removed calls to `render_new_user_onboarding()` and `render_trial_end_upsell()` from Render impl
- Removed `OnboardingUpsell` struct and its `Dismissable` impl
- Removed `TrialEndUpsell` struct and its `Dismissable` impl
- Removed `ui::EndTrialUpsell` import
- Removed `ResetTrialEndUpsell, ResetTrialUpsell` from crate imports
- Removed `atomic::{AtomicBool, Ordering}` import

### `crates/agent_ui/src/ui.rs`:
- Removed `mod end_trial_upsell;` and `pub use end_trial_upsell::*;`

### `crates/agent_ui/src/ui/end_trial_upsell.rs`:
- Deleted entirely

### `crates/agent_ui/src/agent_ui.rs`:
- Removed `ResetTrialUpsell` and `ResetTrialEndUpsell` action definitions

## Other fixes:

### `crates/language_models/src/provider/cloud.rs`:
- Replaced `use ai_onboarding::YoungAccountBanner;` with a comment
- Replaced `this.child(YoungAccountBanner)` with `this.child(div())` (placeholder; cloud.rs will be fully deleted in Phase 7)

## Build verification:
- `cargo check -p onboarding -p agent_ui -p language_models` passes cleanly
- Full workspace `cargo check -p xenomorphic` has errors from other phases (git_ui references `call` crate, edit_prediction has UserStore references) — these are pre-existing from Phase 1 or will be addressed in later phases
