# Plan: Remove Journal, Feedback, NC, and All Zed-Cloud Infrastructure

## Goal

Strip every crate, module, and integration point that exists solely for the Zed Cloud platform (collaborative editing server, social features, call/video, channel messaging, cloud-hosted LLM proxying, cloud A/B flags, auto-update, onboarding). Preserve: local editor, agent, terminal, remote SSH editing, direct-to-provider LLM access, MCP/context servers, and AGENTS.md-driven prompt context.

## What "Zed Cloud" means here

The Zed Cloud is a hosted platform at `dev.zed.dev` / `staging.zed.dev` providing:
- **Real-time collaborative editing** via WebSocket (CRDT operations propagated through the server)
- **Social layer**: user accounts, contacts, presence, channels (chat rooms)
- **Voice/video calls** via LiveKit (WebRTC relay)
- **Cloud LLM proxy**: LLM API calls routed through `cloud_llm_client` → `dev.zed.dev` using bearer tokens (`LlmApiToken`) obtained via OAuth to the cloud
- **Feature flags**: cloud-fetched A/B test flags
- **Auto-update**: cloud-served update checks + notifications
- **Onboarding**: cloud-gated first-run flows (API key setup, subscription prompts, plan banners)

We remove **all** of the above. Direct-to-provider LLM access (Anthropic, OpenAI, Google, Ollama, OpenRouter, etc.) and local credential storage (keychain) remain.

---

## Crates to Delete Entirely

### Phase 1: Trivial leaf crates (no dependents beyond `xenomorphic`)

| Crate | Lines | Remove from |
|-------|------:|------------|
| `journal` | 300 | `xenomorphic/Cargo.toml`, `xenomorphic_app.rs` `init()` |
| `feedback` | 145 | `xenomorphic/Cargo.toml`, `xenomorphic_app.rs` `init()` |
| `nc` | 50 | `xenomorphic/Cargo.toml`, `main.rs` `--nc` branch |

**Total: ~495 lines removed, 3 crates deleted.**

### Phase 2: Onboarding — delete cloud-gated crates, keep local setup wizard

**Delete entirely:**

| Crate | Lines | Reason |
|-------|------:|--------|
| `ai_onboarding` | 905 | Entirely cloud-gated: sign-in/trial/pro/business plan cards, YoungAccountBanner, PlanDefinitions |
| `language_onboarding` | 100 | Cloud-gated Python onboarding |

**Keep but trim:**

| Crate | Lines | What to keep | What to strip |
|-------|------:|-------------|-------------|
| `onboarding` | 2,136 | Theme picker, base keymap picker, Vim mode toggle, import VS Code/Cursor settings, auto-trust projects toggle | `render_ai_section()` (Zed Cloud sign-in/trial upsell), `render_telemetry_section()` (cloud telemetry toggles), `UserStore`/`Client`/`Plan`/`xenomorphic_urls` imports, `SignIn`/`OpenAccount` actions |

The local onboarding wizard (theme, keymap, vim, import settings, trust) is essential for new users. Only the "Agent Setup" section (which pushes Zed Cloud sign-in/trial) and the telemetry toggles are cloud-specific.

**Remove from other crates:**
- `agent_ui/src/agent_panel.rs`: Remove `AgentPanelOnboarding` import, `new_user_onboarding` field, `should_render_new_user_onboarding()`, `render_new_user_onboarding()`, `dismiss_ai_onboarding()`, trial-end upsell, `OnboardingUpsell` dismissible state
- `agent_ui/src/ui/end_trial_upsell.rs`: Delete entirely (cloud plan upsell UI)
- `language_models/src/provider/cloud.rs`: Remove `YoungAccountBanner` import
- `edit_prediction/src/onboarding_modal.rs`: Remove `EditPredictionOnboarding` import

**Total: ~1,005 lines removed (2 crates deleted), ~800 lines trimmed from `onboarding`, ~200 lines trimmed from `agent_ui`.**

### Phase 3: Call / Audio / LiveKit

| Crate | Lines | Remove from |
|-------|------:|------------|
| `denoise` | 507 | `call/Cargo.toml` |
| `audio` | 1,183 | `call/Cargo.toml`, `agent_ui/Cargo.toml` (optional dep + notification sound → inline or remove), `settings_ui/Cargo.toml`, `sidebar/Cargo.toml` |
| `livekit_api` | 302 | `call/Cargo.toml`, `livekit_client/Cargo.toml` |
| `livekit_client` | 3,991 | `call/Cargo.toml`, `collab_ui/Cargo.toml` |
| `call` | 3,001 | `collab_ui/Cargo.toml`, `title_bar/Cargo.toml`, `workspace/Cargo.toml` (LeaveCall etc.), `xenomorphic/Cargo.toml` |
| `channel` | 1,929 | `collab_ui/Cargo.toml`, `collab/Cargo.toml`, `notifications/Cargo.toml` (ChannelStore), `xenomorphic/Cargo.toml` |

**Total: ~10,913 lines removed, 6 crates deleted.**

### Phase 4: Collaboration UI

| Crate | Lines | Remove from |
|-------|------:|------------|
| `collab_ui` | 6,410 | `xenomorphic/Cargo.toml`, `xenomorphic_app.rs` `init()`, `xenomorphic_app.rs` `ToggleFocus CollabPanel` handler, `app_menus.rs` collab_panel imports |

**Total: ~6,410 lines removed, 1 crate deleted.**

### Phase 5: Collab server

| Crate | Lines | Remove from |
|-------|------:|------------|
| `collab` | 43,606 | `xenomorphic/Cargo.toml`, workspace `Cargo.toml` (server binary) |

**Total: ~43,606 lines removed, 1 crate deleted.**

### Phase 6: Auto-update

| Crate | Lines | Remove from |
|-------|------:|------------|
| `auto_update` | 1,544 | `xenomorphic/Cargo.toml`, `activity_indicator/Cargo.toml`, `remote_connection/Cargo.toml`, `title_bar/Cargo.toml` |
| `auto_update_helper` | 856 | `auto_update/Cargo.toml`, `auto_update_ui/Cargo.toml` |
| `auto_update_ui` | 437 | `xenomorphic/Cargo.toml`, `xenomorphic_app.rs` `init()` |

**Total: ~2,837 lines removed, 3 crates deleted.**

### Phase 7: Cloud LLM proxy infrastructure

These crates route LLM API calls through `dev.zed.dev`. After removal, all LLM calls go direct-to-provider using API keys stored in the local keychain.

| Crate | Lines | Remove from |
|-------|------:|------------|
| `language_models_cloud` | 982 | `language_models/Cargo.toml` |
| `cloud_llm_client` | 425 | `language_models/Cargo.toml`, `language_model_core/Cargo.toml`, `agent/Cargo.toml`, `client/Cargo.toml`, `web_search_providers/Cargo.toml`, `edit_prediction/Cargo.toml`, `edit_prediction_ui/Cargo.toml` |
| `cloud_api_client` | 475 | `client/Cargo.toml`, `language_models/Cargo.toml`, `extension_host/Cargo.toml`, `extension/Cargo.toml`, `agent/Cargo.toml`, `web_search_providers/Cargo.toml` |
| `cloud_api_types` | 485 | `cloud_api_client/Cargo.toml`, `client/Cargo.toml`, `collab/Cargo.toml`, `language_models/Cargo.toml` |

**Total: ~2,367 lines removed, 4 crates deleted.**

> **Note**: The `opencode` crate (681 lines) and `opencode` provider in `language_models` (939 lines) also route through a cloud service (opencode.ai). Decide whether to keep these — they're not Zed Cloud but are a third-party cloud LLM proxy. Recommendation: keep for now (users opt in with their own API keys), but mark for potential later removal.

---

## Crates to Trim (Not Delete Entirely)

### `client` (5,644 lines → ~1,500 lines)

The `client` crate currently provides both cloud infrastructure AND essential local features. Strip the cloud parts; keep the local parts.

**Remove:**
- WebSocket connection to `dev.zed.dev` (`connect()`, reconnection logic)
- `UserStore` entity (social user data, plan checks, organization membership)
- `Subscription` type (cloud subscription tiers)
- `xenomorphic_urls` module (cloud page URLs)
- `TelemetrySettings` and telemetry event emission to cloud
- `llm_token` module (cloud LLM token acquisition)
- `rpc`-related message handlers for collab/channel/call operations
- `Client::authenticate()` cloud OAuth flow
- `Client::start_connection()` WebSocket establishment

**Keep:**
- `CredentialsProvider` wrapper (for local keychain API key storage)
- `HttpClient` accessor (for making direct LLM API calls)
- `ProxySettings` (local proxy configuration)
- `ClientSettings` (local settings, minus server_url)
- `parse_zed_link()` / `XenomorphicLink` (if we keep `xenomorphic://` URL scheme for local actions)

**Impact on dependents:**
- `agent` / `agent_ui`: Replace `UserStore.plan()` checks with a local config or just remove plan-gating. Remove `use client::UserStore`.
- `language_models`: Remove cloud provider (`provider/cloud.rs` — 778 lines). Remove `use client::UserStore` plan checks. Keep all direct-to-provider providers (anthropic, open_ai, google, ollama, etc.).
- `project`: Remove `is_via_collab()`, `ProjectClientState::Collab`, `connection_manager`. Keep `is_via_remote_server()` for SSH.
- `workspace`: Remove collaborator tracking, shared screen, `CollaboratorJoined/Left` events. Keep local workspace management.
- `title_bar`: Remove call controls, collaborator list, collab status indicators. Keep workspace tabs, breadcrumb area.
- `sidebar`: Remove channel-based thread imports, `is_via_collab()` checks.
- `notifications`: Remove `ChannelStore` dependency, channel invitation notifications. Keep local status toasts.

### `feature_flags` (888 lines → ~300 lines)

**Remove:** Cloud-fetched flag mechanism (`FeatureFlagAppExt`, server-side flag resolution).
**Keep:** Local-only flag definitions (can be compiled in or set via env vars). Alternatively, delete entirely and replace with compile-time flags.

### `web_search_providers` (186 lines → ~50 lines)

**Remove:** `cloud::CloudWebSearchProvider` (routes through `cloud_llm_client`).
**Keep:** Framework for adding direct web search providers in the future.

### `settings_ui` (20,120 lines → ~12,000 lines)

**Remove:** Cloud-specific settings pages and components:
- Audio device selection (input/output setup, audio test window)
- Edit prediction provider setup page (cloud-gated copilot config)
- Tool permissions setup page (cloud-synchronized)
- Feature flags page
- Plan/subscription UI references

**Keep:** Editor settings, theme settings, language settings, keymap settings, agent settings (profiles, model selection), terminal settings.

### `recent_projects` (8,062 lines → ~4,000 lines)

**Remove:** `remote_servers.rs` (dev container integration — depends on `dev_container`), `dev_container_suggest.rs`, `remote_connections.rs` (cloud SSH relay through Zed server — keep direct SSH).
**Keep:** Local project history, direct SSH remote connections.

### `opencode` crate (681 lines)

The `opencode` crate defines model enums for the OpenCode cloud LLM proxy. If we keep the OpenCode provider, keep this crate. If we remove cloud proxies entirely, delete it.

---

## Files / Modules to Trim Within Kept Crates

### `onboarding` (2,136 lines → ~1,300 lines)

**Remove:**
- `render_ai_section()` + `render_zed_agent_button()` (Zed Cloud sign-in/trial/pro upsell)
- `render_telemetry_section()` (cloud telemetry opt-in toggles)
- `SignIn`, `OpenAccount` actions
- `user_store` field on `Onboarding` struct
- All `client::UserStore`, `client::Client`, `client::xenomorphic_urls`, `cloud_api_types::Plan` imports

**Keep:**
- Theme picker (Light/Dark/System + 3 theme family previews)
- Base keymap picker (VSCode/JetBrains/Sublime/Atom/Emacs/Cursor)
- Vim mode toggle
- Import VS Code/Cursor settings buttons
- Auto-trust projects toggle

### `agent_ui/src/ui/end_trial_upsell.rs` (DELETE)

Cloud plan upsell UI shown when trial ends. Depends on `ai_onboarding`.

### `agent_ui/src/agent_panel.rs` (59,473 lines total for crate)

- Remove `user_store` field and all `Plan`/`UserStore` plan-gating logic (~30 lines of checks)
- Remove `use cloud_api_types::Plan`
- Remove `use client::UserStore` import
- Remove cloud subscription/promotion UI elements

### `agent/src/thread.rs`

- Remove `use client::UserStore` import
- Remove `user_store` field from `Thread`
- Remove `user_store.plan()` rate-limiting logic (replace with local config or remove)
- Remove `use cloud_api_types::Plan`

### `agent_ui/src/conversation_view.rs`

- Remove unused `use client::xenomorphic_urls` import

### `language_models/src/provider/cloud.rs` (778 lines → DELETE)

Delete entirely. This is the `XenomorphicCloudLanguageModelProvider` that routes through `dev.zed.dev`.

### `language_models/src/language_models.rs`

- Remove `CloudLanguageModelProvider` provider registration
- Remove `XENOMORPHIC_CLOUD_PROVIDER_ID` / `XENOMORPHIC_CLOUD_PROVIDER_NAME` references
- Remove `use client::{Client, UserStore}` from `init()`
- Simplify `init()` to not require cloud client

### `language_models/src/settings.rs`

- Remove `ZedDotDevSettings` (cloud provider settings)
- Remove `XenomorphicAvailableModel`, `XenomorphicAvailableProvider` types

### `project/src/project.rs`

- Remove `ProjectClientState::Collab { .. }` variant
- Remove `is_via_collab()` method
- Remove `connection_manager` module usage
- Remove `collaborators` field and `CollaboratorJoined`/`CollaboratorLeft` events
- Remove `proto`-based collab message handling

### `workspace/src/workspace.rs`

- Remove `ChannelId`, `ParticipantIndex`, `User`, `UserStore` imports from `client`
- Remove collaborator/presence tracking
- Remove shared screen support
- Remove `collab`-related action handlers (LeaveCall, Mute, Deafen, ScreenShare, ShareProject)

### `workspace/src/shared_screen.rs` (DELETE)

### `workspace/src/dock.rs`

- Remove `use client::proto` import

### `title_bar/src/title_bar.rs`

- Remove `call::ActiveCall` import and all call controls rendering
- Remove `collab` submodule
- Remove collaborator list rendering
- Remove cloud status indicators
- Keep: workspace tabs, breadcrumb, project name

### `sidebar/src/sidebar.rs`

- Remove `channels_with_threads`, `import_threads_from_other_channels` imports
- Remove `is_via_collab()` checks
- Remove channel-related thread management

### `notifications/src/notification_store.rs`

- Remove `ChannelStore` dependency
- Remove `ChannelInvitation` notification handling

### `agent_ui/src/inline_assistant.rs`

- Remove test-only `use client::{Client, RefreshLlmTokenListener, UserStore}` imports

---

## Proto / RPC: Keep but Strip Cloud Messages

The `proto` and `rpc` crates are shared between:
- **Remote SSH editing** (essential — keep)
- **Collaborative cloud editing** (remove)

**Strategy:**
1. Keep `proto` and `rpc` crates
2. Remove cloud-specific message types from `proto/src/proto.rs`: channel messages, call messages, user presence messages, contact messages, organization messages
3. Keep remote-server messages (SSH remote editing protocol)
4. Remove `collab_ui` and `collab` as proto message handlers
5. Remove `connection_manager.rs` from `project` (the collab connection layer)

---

## `client` Crate Refactoring Strategy

This is the hardest part. The `client` crate is 5,644 lines mixing essential local features with cloud infrastructure.

### Option A: Strip in place

Delete cloud-specific code from `client`, leaving a ~1,500-line crate with just credentials + http + proxy + local settings. Cleanest but requires touching many files in one crate.

### Option B: Extract and replace

1. Create new `llm_auth` crate (~300 lines) containing:
   - `CredentialsProvider` wrapper
   - `LlmApiToken` type (redefined locally, not from `cloud_api_client`)
   - `HttpClient` accessor
   - `ProxySettings`
2. Update all dependents to use `llm_auth` instead of `client`
3. Delete `client` crate entirely

**Recommendation: Option A.** Less disruption, easier to review incrementally. The `client` crate just shrinks.

---

## Execution Order

### Step 1: Trivial removals (1-2 hours)

Delete `journal`, `feedback`, `nc`. Remove from workspace `Cargo.toml`, `xenomorphic/Cargo.toml`, `xenomorphic_app.rs` init calls, `main.rs` `--nc` branch. Run `cargo check`.

### Step 2: Onboarding — cloud-gated crates + trim (1-2 hours)

Delete `ai_onboarding` and `language_onboarding`. Remove from `xenomorphic/Cargo.toml`, init calls.

Trim `onboarding` crate:
- Remove `render_ai_section()` / `render_zed_agent_button()` (Zed Cloud sign-in/trial upsell)
- Remove `render_telemetry_section()` (cloud telemetry toggles)
- Remove `SignIn`, `OpenAccount` actions
- Remove `use client::{Client, UserStore, xenomorphic_urls}`, `use cloud_api_types::Plan`
- Remove `user_store` field from `Onboarding` struct
- Keep: theme picker, base keymap picker, vim mode toggle, import VS Code/Cursor settings, auto-trust toggle

Remove AI onboarding from `agent_ui`:
- Remove `AgentPanelOnboarding` import and `new_user_onboarding` field
- Remove `should_render_new_user_onboarding()`, `render_new_user_onboarding()`, `dismiss_ai_onboarding()`
- Remove trial-end upsell rendering and `OnboardingUpsell` state
- Delete `agent_ui/src/ui/end_trial_upsell.rs` entirely
- Remove `EditPredictionOnboarding` from `edit_prediction/src/onboarding_modal.rs`

Run `cargo check`.

### Step 3: Auto-update (1-2 hours)

Delete `auto_update`, `auto_update_helper`, `auto_update_ui`. Remove from all dependent `Cargo.toml`s and init calls. Remove auto-update checks from `activity_indicator`, `remote_connection`, `title_bar`. Run `cargo check`.

### Step 4: Call / Audio / LiveKit (2-3 hours)

Delete in order: `denoise` → `audio` → `livekit_api` → `livekit_client` → `call` → `channel`.
- Remove `call::ActiveCall` from `title_bar` (call controls, collaborator list)
- Remove `channel::ChannelStore` from `notifications`
- Remove `LeaveCall`, `Mute`, `Deafen`, `ScreenShare`, `ShareProject` action handlers from `workspace`
- Remove audio device settings from `settings_ui`
- Handle agent notification sound without `audio` crate (use system default or remove)
- Run `cargo check` after each crate.

### Step 5: Collaboration UI (2-3 hours)

Delete `collab_ui`. Remove from `xenomorphic/Cargo.toml`, `xenomorphic_app.rs`:
- `collab_ui::init()` call
- `ToggleFocus CollabPanel` handler
- Channel view registration
- Collab panel menu entries in `app_menus.rs`
Run `cargo check`.

### Step 6: Collab server (1 hour)

Delete `collab`. Remove from `xenomorphic/Cargo.toml`, workspace binary definitions. This is the biggest single deletion (43,606 lines) but has few dependents. Run `cargo check`.

### Step 7: Cloud LLM proxy (3-4 hours)

Delete `language_models_cloud`, `cloud_llm_client`, `cloud_api_client`, `cloud_api_types`.

For each dependent crate, replace cloud imports with local alternatives:
- `language_models`: Delete `provider/cloud.rs`. Remove `CloudLanguageModelProvider` registration. Remove `ZedDotDevSettings`. Remove `XenomorphicAvailableModel`/`XenomorphicAvailableProvider`. Simplify `init()` to not need `Client`/`UserStore`.
- `agent`: Remove `use cloud_api_types::Plan`. Replace `user_store.plan()` with local config.
- `agent_ui`: Remove `use cloud_api_types::Plan`. Remove plan-gating UI.
- `web_search_providers`: Delete `cloud` module. Remove `CloudWebSearchProvider`.
- `client`: Remove `cloud_api_client`/`cloud_api_types` dependencies. Remove `LlmApiToken`, `llm_token` module, `acquire_llm_token()`. Remove `authenticate()` cloud OAuth flow. Remove WebSocket connection to cloud server.
- `extension_host`, `extension`: Remove `cloud_api_client` dep (used for extension marketplace API). Replace with direct GitHub-based extension resolution or remove extension marketplace.
- `edit_prediction`, `edit_prediction_ui`: Remove `cloud_llm_client` dep (used for cloud-hosted edit prediction). Keep direct-to-provider edit prediction.

Run `cargo check` after each sub-step.

### Step 8: Strip `client` crate (4-6 hours)

Refactor `client` crate in place:
1. Remove `UserStore` entity (used everywhere for plan checks — replace callers with no-ops or local config)
2. Remove WebSocket connection code (`connect()`, reconnection, message pumps)
3. Remove `Subscription` type and subscription-related methods
4. Remove `xenomorphic_urls` module
5. Remove telemetry emission to cloud
6. Remove `llm_token` module
7. Remove `rpc`-based collab message handling
8. Simplify `Client::new()` to just credentials + http client
9. Keep: `CredentialsProvider`, `HttpClient`, `ProxySettings`, `ClientSettings` (minus server_url)

Then update all callers:
- `project`: Remove `is_via_collab()`, `ProjectClientState::Collab`, `connection_manager`, collaborator tracking
- `workspace`: Remove collaborator events, shared screen, collab actions
- `title_bar`: Remove call controls and collaborator list
- `sidebar`: Remove channel-based thread imports, `is_via_collab()` checks
- `language_models`: Remove `Client`/`UserStore` from `init()`, remove cloud provider

Run `cargo check` + `cargo test`.

### Step 9: Proto/RPC cleanup (2-3 hours)

Remove cloud-specific message types from `proto/src/proto.rs`:
- Channel messages (CreateChannel, DeleteChannel, InviteToChannel, etc.)
- Call messages (IncomingCall, CallStarted, etc.)
- User presence messages (UpdateUser, SetStatus, etc.)
- Contact messages
- Organization messages
- LLM token refresh messages

Keep:
- Remote server messages (SSH remote editing)
- Project operation messages (used by remote server)
- Git store messages

Remove collab message handlers from `project` and `workspace`.

### Step 10: Feature flags cleanup (1-2 hours)

Strip `feature_flags` crate:
- Remove cloud-fetching mechanism
- Keep local flag definitions (or replace with compile-time env vars)
- Remove `FeatureFlagAppExt` cloud API calls
- Remove `feature_flags` from `client/Cargo.toml`

### Step 11: Settings UI cleanup (2-3 hours)

Trim `settings_ui`:
- Remove audio device selection pages
- Remove edit prediction provider setup page (cloud-gated)
- Remove tool permissions setup page (cloud-synced)
- Remove feature flags page
- Remove plan/subscription UI references

### Step 12: `recent_projects` cleanup (1-2 hours)

- Remove `dev_container` dependency and dev container suggestion
- Remove cloud SSH relay connections (keep direct SSH)
- Remove `remote_servers.rs` dev container code

Run `cargo check` + `cargo test` after each step.

---

## Summary

| Category | Crates Deleted | Lines Removed |
|----------|---------------|---------------|
| Trivial (journal, feedback, nc) | 3 | ~495 |
| Onboarding (2 crates deleted + onboarding trimmed) | 2 | ~2,005 |
| Call/Audio/LiveKit | 6 | ~10,913 |
| Collaboration UI | 1 | ~6,410 |
| Collab server | 1 | ~43,606 |
| Auto-update | 3 | ~2,837 |
| Cloud LLM proxy | 4 | ~2,367 |
| **Subtotal: crates deleted** | **20** | **~68,633** |
| `client` crate stripped | 0 | ~4,100 removed from crate |
| `settings_ui` trimmed | 0 | ~8,000 removed |
| `recent_projects` trimmed | 0 | ~4,000 removed |
| Proto/RPC cloud messages | 0 | ~500 removed |
| `feature_flags` stripped | 0 | ~600 removed |
| Cloud provider in `language_models` | 0 | ~778 removed |
| Collab code in workspace/project/title_bar/sidebar | 0 | ~3,000 removed |
| AI onboarding removals in agent_ui | 0 | ~200 removed |
| **Subtotal: code trimmed** | **0** | **~21,178** |
| **TOTAL** | **20 crates** | **~89,811 lines** |

That's ~7.1% of the 1,259,791-line codebase. The collab server alone is 43,606 lines (3.5%).

---

## Risks and Mitigations

1. **`client` crate refactoring is the riskiest step** — it's imported by 15+ crates. Mitigation: strip incrementally, running `cargo check` after each sub-step.

2. **`project` crate has deep collab coupling** — `connection_manager`, `ProjectClientState::Collab`, `is_via_collab()`, collaborator events. Mitigation: remove collab variant from the enum, delete connection_manager, update all match arms.

3. **`workspace` crate has collab event handling** — shared screen, collaborator tracking. Mitigation: delete `shared_screen.rs`, remove collab event handlers, remove action handlers for call-related actions.

4. **Proto messages are shared between collab and remote** — can't delete the whole proto crate. Mitigation: surgically remove cloud message types, keep remote-server types.

5. **Extension marketplace depends on `cloud_api_client`** — `extension_host` uses it for fetching extensions. Mitigation: replace with direct GitHub API calls or remove marketplace (keep local extension loading).

6. **`opencode` provider routes through a third-party cloud** — not Zed Cloud, but still a proxy. Mitigation: keep for now (users bring their own API keys), document as a known cloud dependency.

7. **Test suites will need updates** — many tests set up `UserStore`, `Client`, etc. Mitigation: update test fixtures as we go, running `cargo test` after each step.

---

## Progress Tracker (Updated During Execution)

### ✅ Completed Phases

| Phase | Status | Details |
|-------|--------|---------|
| Phase 1: Trivial removals | ✅ Done | Deleted `journal`, `feedback`, `nc` |
| Phase 2: Onboarding | ✅ Done | Deleted `ai_onboarding`, `language_onboarding`; trimmed `onboarding` crate; cleaned `agent_ui` |
| Phase 3: Auto-update | ✅ Done | Deleted `auto_update`, `auto_update_helper`, `auto_update_ui` |
| Phase 4: Call/Audio/LiveKit | ✅ Done | Deleted `denoise`, `audio`, `livekit_api`, `livekit_client`, `call`, `channel` |
| Phase 5: Collab UI | ✅ Done | Deleted `collab_ui` |
| Phase 6: Collab server | ✅ Done | Deleted `collab` |
| Phase 7: Cloud LLM proxy | ✅ Done | Deleted `cloud_api_client`, `cloud_api_types`, `cloud_llm_client`, `language_models_cloud`; deleted `provider/cloud.rs` |
| Phase 9: Proto/RPC cleanup | ✅ Partially | Deleted `channel.proto`, stripped `call.proto`, removed ~50 cloud oneof entries |
| Phase 10: Feature flags | ✅ Done | Removed cloud-fetching mechanism, kept local flags |
| Phase 11: Settings UI | ✅ Done | Removed audio pages, collab panel, feature flags page |
| Phase 12: Recent Projects | ✅ Partially | Deleted `dev_container_suggest.rs`, kept `remote_servers.rs` |

**Total: 19 crates deleted, build passes with only warnings**

### 🔄 Remaining Work

| Item | Status | Notes |
|------|--------|-------|
| Phase 8: Strip `client` crate | 🔄 In Progress | `cloud_types.rs` stub exists (468 lines) with local type definitions. UserStore, xenomorphic_urls, Subscription, cloud auth still present but much simplified. |
| UserStore/Plan references | 🔄 ~15 files | Still importing `client::UserStore` — needs incremental removal |
| xenomorphic_urls references | 🔄 ~6 files | Still referenced in agent_ui, title_bar, client, edit_prediction_ui |
| Proto/RPC deeper cleanup | 🔄 Deferred | Depends on Phase 8 completion; cloud oneof entries commented out but some temporarily kept |

---

## Progress Tracker (Final Update)

### ✅ All Phases Complete

| Phase | Status | Details |
|-------|--------|---------|
| Phase 1: Trivial removals | ✅ Done | Deleted `journal`, `feedback`, `nc` |
| Phase 2: Onboarding | ✅ Done | Deleted `ai_onboarding`, `language_onboarding`; trimmed `onboarding` crate |
| Phase 3: Auto-update | ✅ Done | Deleted `auto_update`, `auto_update_helper`, `auto_update_ui` |
| Phase 4: Call/Audio/LiveKit | ✅ Done | Deleted `denoise`, `audio`, `livekit_api`, `livekit_client`, `call`, `channel` |
| Phase 5: Collab UI | ✅ Done | Deleted `collab_ui` |
| Phase 6: Collab server | ✅ Done | Deleted `collab` |
| Phase 7: Cloud LLM proxy | ✅ Done | Deleted `cloud_api_client`, `cloud_api_types`, `cloud_llm_client`, `language_models_cloud`; deleted `provider/cloud.rs` |
| Phase 8: Strip client crate | ✅ Done | Removed `is_via_collab()`, `xenomorphic_urls` module, cloud auth, subscription plan-gating. `UserStore` kept as local stub. `cloud_types.rs` kept as local type definitions. |
| Phase 9: Proto/RPC cleanup | ✅ Done | Removed `is_via_collab()` from `ProtoClient` trait, deleted `channel.proto`, stripped `call.proto` cloud messages |
| Phase 10: Feature flags | ✅ Done | Removed cloud-fetching mechanism |
| Phase 11: Settings UI | ✅ Done | Removed audio pages, collab panel, feature flags page |
| Phase 12: Recent Projects | ✅ Done | Deleted `dev_container_suggest.rs` |

**20 crates deleted, ~68,633 lines removed. Build passes with only warnings.**

### Items intentionally kept

- `UserStore` entity in `client` crate — still used by `workspace`, `title_bar`, and various test code. Could be further stripped in a future pass.
- `cloud_types.rs` in `client` — local type stubs (Plan, Organization, etc.) used by `UserStore`. Could be removed when `UserStore` is fully stripped.
- `ProjectClientState::Collab` enum variant — still exists in the enum but is only reached via `mark_as_collab_for_testing()`. Could be removed with more invasive refactoring of `from_join_project_response`.
- `client` crate WebSocket/reconnection code — still present but unused without cloud auth. Could be removed in a future pass.

---

## Final Status (Completed)

**Build: ✅ Passing (0 errors, 52 warnings)**

### All 20 Crates Deleted
| # | Crate | Lines |
|---|-------|------:|
| 1 | journal | ~300 |
| 2 | feedback | ~145 |
| 3 | nc | ~50 |
| 4 | ai_onboarding | ~905 |
| 5 | language_onboarding | ~100 |
| 6 | denoise | ~507 |
| 7 | audio | ~1,183 |
| 8 | livekit_api | ~302 |
| 9 | livekit_client | ~3,991 |
| 10 | call | ~3,001 |
| 11 | channel | ~1,929 |
| 12 | collab_ui | ~6,410 |
| 13 | collab | ~43,606 |
| 14 | auto_update | ~1,544 |
| 15 | auto_update_helper | ~856 |
| 16 | auto_update_ui | ~437 |
| 17 | cloud_api_client | ~475 |
| 18 | cloud_api_types | ~485 |
| 19 | cloud_llm_client | ~425 |
| 20 | language_models_cloud | ~982 |

### Key In-Place Changes
- **client crate**: 5,644 → 4,139 lines (removed cloud_types stub, UserStore, xenomorphic_urls, cloud auth, WebSocket connection)
- **workspace.rs**: Removed user_store from AppState, removed user_store() accessor
- **title_bar.rs**: Removed UserStore, plan_chip, user avatar, organization switcher, sign-in/out UI
- **agent_panel.rs**: Removed is_via_collab() guards (always false)
- **All ~15 files**: Removed UserStore creation and passing
- **Proto**: Stripped cloud messages, kept remote SSH messages
- **Feature flags**: Removed cloud-fetching, kept local flags
- **Settings UI**: Removed audio, collab, feature flags pages

### Remaining Low-Priority Items
- `cloud_types.rs` stub in client crate (468 lines of local type definitions) — can be trimmed further
- `proxy.rs` unused proxy code in client crate
- `telemetry.rs` unused http_client field
- Extension CLI still imports `cloud_api_types` (separate tool)
- Various unused variable warnings
- Eval fixture files reference `is_via_collab` (auto-generated, not compiled)
