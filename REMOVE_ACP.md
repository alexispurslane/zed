# Removing ACP Support Entirely

## Status

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Create `agent_thread` crate scaffold | ✅ Done — compiles cleanly |
| 1 | Non-code cleanup (docs, markdown, comments) | ✅ Done |
| 2 | Remove `acp_tools` crate | ✅ Done |
| 3 | Remove ACP-specific UI features | ✅ Done (3a Registry, 3b Import, 3c Debug Logs) |
| 4 | Rename keymap context entries | ✅ Done |
| 5 | Remove `agent_servers::acp` module | ✅ Done |
| 6 | Remove ACP features from `agent_ui` | ✅ Done |
| 7 | Remove ACP features from `sidebar` | ✅ Done |
| 8 | **Big-bang type migration** — `agent` + `agent_ui` + `sidebar` + `agent_settings` + `agent_servers` all switch from `acp_thread`/`agent-client-protocol` to `agent_thread` simultaneously | ✅ Done |
| 9 | Switch `xenomorphic` and `eval_cli` | ✅ Done |
| 10 | Remove feature flag (`AcpBetaFeatureFlag`) | ✅ Done |
| 11 | Remove ACP from `project` crate (registry store, settings) | ✅ Done |
| 12 | Remove `onboarding` ACP references | ✅ Done |
| 13 | Remove `extensions_ui` ACP upsell | ✅ Done |
| 14 | Delete `acp_thread` crate and `agent-client-protocol` dep | ✅ Done |
| 15 | Remaining scattered references (variable names, comments) | ✅ Done — all `acp_thread`/`Acp` renames complete |
| 16 | Compile and fix | ✅ Done — workspace compiles cleanly |
| 17 | Fix crash bug: `Agent::Custom.server()` | ✅ Done — `UnsupportedAgentServer` returns graceful error |
| 18 | Remove dead restart button | ✅ Done |
| 19 | Vestigial code audit | ✅ Done — see below |
| 20 | Remove ModeSelector + ConfigOptionsView modules | ✅ Done — deleted ~1050 lines |
| 21 | Remove CycleModeSelector action | ✅ Done |
| 22 | Make auth trait methods default no-ops | ✅ Done — `auth_methods()→&[]`, `authenticate()→Ok(())`, `terminal_auth_task()→None`, `supports_resume_session()→false`, `resume_session()→Err(..)` are now defaults on `AgentConnection` trait |
| 23 | Remove broken ACP tests from conversation_view | ✅ Done — 2 tests that required `FakeAcpAgentServer`
| 24 | Remove auth flow from ConversationView | ✅ Done — removed `AuthState` enum, `auth_state` field, `handle_auth_required`, `authenticate`, `spawn_external_agent_login`, `render_auth_required_state`, `reauthenticate`, `has_auth_methods`, `auth_task` field, `authenticate_button`, `ReauthenticateAgent` action, resume_session branch, AuthRequired error handling. ~716 lines removed from conversation_view/thread_view

## Vestigial Code (Dead but Harmless)

The following items exist in the codebase but are **provably unreachable** for the native agent. They can be removed in a follow-up cleanup:

- **Schema types `AuthMethod`, `AuthMethodId`, `AuthMethodAgent`** in `agent_thread/src/schema.rs` — only used by test impls of `AgentConnection` that override the default `auth_methods()` / `authenticate()` methods
- **`AgentConnection` trait default methods**: `auth_methods()`, `authenticate()`, `terminal_auth_task()`, `supports_resume_session()`, `resume_session()` — kept as no-op defaults so test impls that override them continue to compile
- **`ResumeOnlyAgentConnection`** test stub — overrides `supports_resume_session()` and `resume_session()`, used by test `test_title_editor_is_read_only_when_set_title_unsupported`
- **`AuthGatedAgentConnection`** test stub — overrides `auth_methods()` and `authenticate()`, tests ACP auth behavior
- **`schema::ErrorCode::AuthRequired`** match arm in conversation_view.rs error classification — kept for protocol-level error matching compatibility

## Architecture Context

`acp_thread::AcpThread` is the **core entity type for ALL agent threads** — both external ACP agents AND the native built-in agent. `acp_thread::AgentConnection` is the trait implemented by both `AcpConnection` (external ACP) and `NativeAgentConnection` (built-in). `acp_thread::Diff`, `acp_thread::Terminal`, `acp_thread::TokenUsage`, etc. are used pervasively by the native agent.

The two-layer architecture works like this: every session has an internal `Thread` (the LLM conversation engine in the `agent` crate) and an `AcpThread` (the view model the UI subscribes to, renders from, and emits events on). When the native agent processes a prompt, `NativeAgentConnection::prompt()` calls `run_turn()`, which consumes `ThreadEvent`s from the internal `Thread` and forwards each one into `AcpThread`. The UI then subscribes to `AcpThreadEvent`s emitted by `AcpThread`.

`AcpThread` uses `agent-client-protocol::schema` types (`SessionId`, `ContentBlock`, `ToolCallId`, `StopReason`, etc.) as its field types. `NativeAgentConnection` implements `acp_thread::AgentConnection` — the trait that defines `new_session()`, `load_session()`, `prompt()`, `cancel()`, etc.

## Decision: Rehome everything, delete both external packages

We will rehome all types currently in `acp_thread` and `agent-client-protocol` into a new `agent_thread` crate, replacing `agent-client-protocol::schema` types with locally-defined equivalents, then delete both the `acp_thread` crate and the `agent-client-protocol` dependency. No legacy "ACP" names or crates will remain.

The `agent-client-protocol` crate is an external crates.io dependency (`=0.11.1`). Its types must be replaced with local definitions — we copy/rewrite what the native agent needs and drop the rest. The `acp_thread` crate's code will be moved in-tree under the new `agent_thread` name, with all `Acp`/`ACP` naming replaced.

## Key lesson: type migration must be atomic across crate boundaries

The `Entity<AcpThread>` is created in the `agent` crate, and its methods return `acp::SessionId`, `acp::ContentBlock`, etc. These types are the *same* `agent-client-protocol::schema` types used throughout `agent_ui` as `acp::SessionId`.

When we tried to migrate `agent_ui` first (Phase 6) — replacing `acp::SessionId` with `agent_thread::schema::SessionId` and `Entity<AcpThread>` with `Entity<AgentThread>` — we discovered that even though both types are `Arc<str>` newtypes with identical derivations, they are **different Rust types**. The `agent` crate returns `acp::SessionId`, `agent_ui` expects `agent_thread::schema::SessionId`, and Rust correctly rejects the mismatch. There is no bridge conversion possible without `.0.clone()` noise at every boundary.

Similarly, `agent_thread::AgentThread` and `acp_thread::AcpThread` are different structs — `Entity<AgentThread>` and `Entity<AcpThread>` are different GPUI entities. The `agent` crate creates `Entity<AcpThread>`; `agent_ui` cannot read it as `Entity<AgentThread>`.

**Conclusion:** The type migration (replacing `acp::` with `agent_thread::schema::`, `AcpThread` with `AgentThread`, `acp_thread::` with `agent_thread::`) must happen **simultaneously** in the `agent` crate and ALL its consumers. We cannot incrementally migrate one crate at a time. The migration is now consolidated into a single Phase 8.

---

## Phase 0: Create the rehoming target — `agent_thread` crate ✅ DONE

Before touching any consumer, create a new `crates/agent_thread/` crate that will absorb all the types we need.

### 0a. Scaffold the crate

```
crates/agent_thread/
├── Cargo.toml
├── LICENSE-GPL
└── src/
    ├── lib.rs        # re-exports
    ├── thread.rs      # AcpThread → AgentThread, AcpThreadEvent → AgentThreadEvent
    ├── connection.rs  # AgentConnection trait, UserMessageId, etc.
    ├── diff.rs        # Diff (moved from acp_thread)
    ├── mention.rs     # MentionUri, selection_name (moved from acp_thread)
    ├── terminal.rs    # Terminal (moved from acp_thread)
    └── schema.rs      # LOCAL replacements for agent-client-protocol::schema types
```

### 0b. Define local schema types in `schema.rs`

These replace `agent-client-protocol::schema` types. Copy the struct/enum definitions, not the crate dependency. Key types:

```bash
# Get the full list of acp:: types used across the codebase
grep -rn "acp::" --include="*.rs" . | grep -v target/ | grep -v ".fingerprint" | \
  grep -v "use agent_client_protocol::schema as acp" | \
  sed 's/.*acp:://' | sed 's/[^A-Za-z_].*//' | sort -u
```

Types to define locally (not exhaustive — run the command above):
- `SessionId` (newtype over `Arc<str>`, derive `Display`, `From`, `Hash`, `Serialize`, `Deserialize`)
- `ModelId` (newtype over `Arc<str>`)
- `ToolKind` (enum)
- `PermissionOptionKind` (enum)
- `PermissionOptionId` (newtype)
- `ContentBlock` (enum: Text, Image, Audio, ResourceLink, Resource)
- `ContentChunk` (struct)
- `StopReason` (enum)
- `SessionUpdate` (enum)
- `ToolCallId` (newtype)
- `ToolCallUpdateFields` (struct)
- `ToolCallStatus` (enum)
- `ToolCallContent` (enum)
- `PromptCapabilities` (struct)
- `PromptRequest` / `PromptResponse` (structs)
- `AuthMethod` / `AuthMethodId` (enum/newtype — only the variants the native agent uses)
- `AvailableCommand` (struct)
- `Plan*` types (`Plan`, `PlanEntry`, `PlanEntryStatus`, `PlanEntryPriority`)
- `Meta` (type alias for `Option<serde_json::Map<String, Value>>` or `HashMap<String, Value>`)
- `TerminalId` (newtype)
- `SessionModeId`, `SessionConfigId`, `SessionConfigValueId`, `SessionConfigOption` (newtypes/structs)
- `ResourceLink`, `TextContent`, `ImageContent`, `EmbeddedResource*`, `TextResourceContents`
- `Cost`, `UsageUpdate`, `Usage`

**Important:** Only define the variants and fields that the native agent actually uses. Many `agent-client-protocol` types have variants (e.g. `AuthMethod::EnvVar`, `AuthMethod::Terminal`, `McpServer::*`) that exist solely for external agents. Drop those.

### 0c. Move `acp_thread` code into `agent_thread`, renaming as you go

| Old name | New name |
|----------|----------|
| `acp_thread::AcpThread` | `agent_thread::AgentThread` |
| `acp_thread::AcpThreadEvent` | `agent_thread::AgentThreadEvent` |
| `acp_thread::AcpThreadImportOnboarding` | **delete** (ACP-specific) |
| `acp_thread::Diff` | `agent_thread::Diff` |
| `acp_thread::Terminal` | `agent_thread::Terminal` |
| `acp_thread::TokenUsage` | `agent_thread::TokenUsage` |
| `acp_thread::SessionCost` | `agent_thread::SessionCost` |
| `acp_thread::UserMessageId` | `agent_thread::UserMessageId` |
| `acp_thread::AgentConnection` | `agent_thread::AgentConnection` |
| `acp_thread::AgentSessionModes` | `agent_thread::AgentSessionModes` |
| `acp_thread::AgentModelSelector` | `agent_thread::AgentModelSelector` |
| `acp_thread::AgentSessionConfigOptions` | `agent_thread::AgentSessionConfigOptions` |
| `acp_thread::AgentSessionList` | `agent_thread::AgentSessionList` |
| `acp_thread::AgentSessionInfo` | `agent_thread::AgentSessionInfo` |
| `acp_thread::AgentModelList/Info/Icon/GroupName` | `agent_thread::AgentModelList/Info/Icon/GroupName` |
| `acp_thread::SelectedPermissionOutcome` | `agent_thread::SelectedPermissionOutcome` |
| `acp_thread::SelectedPermissionParams` | `agent_thread::SelectedPermissionParams` |
| `acp_thread::PermissionOptions` | `agent_thread::PermissionOptions` |
| `acp_thread::PermissionOptionChoice` | `agent_thread::PermissionOptionChoice` |
| `acp_thread::AuthorizationKind` | `agent_thread::AuthorizationKind` |
| `acp_thread::ToolCallUpdate` | `agent_thread::ToolCallUpdate` |
| `acp_thread::ToolCallUpdateDiff` | `agent_thread::ToolCallUpdateDiff` |
| `acp_thread::ToolCallUpdateTerminal` | `agent_thread::ToolCallUpdateTerminal` |
| `acp_thread::RetryStatus` | `agent_thread::RetryStatus` |
| `acp_thread::MentionUri` | `agent_thread::MentionUri` |
| `acp_thread::selection_name` | `agent_thread::selection_name` |
| `acp_thread::SubagentSessionInfo` | `agent_thread::SubagentSessionInfo` |
| `acp_thread::SUBAGENT_SESSION_INFO_META_KEY` | `agent_thread::SUBAGENT_SESSION_INFO_META_KEY` |
| `acp_thread::RequestPermissionOutcome` | `agent_thread::RequestPermissionOutcome` |
| `acp_thread::ThreadStatus` | `agent_thread::ThreadStatus` |
| `acp_thread::LoadError` | `agent_thread::LoadError` |
| `acp_thread::AgentSessionTruncate` | `agent_thread::AgentSessionTruncate` |
| `acp_thread::AgentSessionRetry` | `agent_thread::AgentSessionRetry` |
| `acp_thread::AgentSessionSetTitle` | `agent_thread::AgentSessionSetTitle` |
| `acp_thread::AgentTelemetry` | `agent_thread::AgentTelemetry` |
| `acp_thread::build_terminal_auth_task` | **delete** (only used by external ACP auth) |
| `acp_thread::create_terminal_entity` | `agent_thread::create_terminal_entity` |
| `acp_thread::StubAgentConnection` | `agent_thread::StubAgentConnection` (test support) |
| `acp_thread::AgentThreadEntry` | `agent_thread::AgentThreadEntry` |
| `acp_thread::ToolCall` | `agent_thread::ToolCall` |
| `acp_thread::UserMessage` | `agent_thread::UserMessage` |
| `acp_thread::AssistantMessage` | `agent_thread::AssistantMessage` |
| `acp_thread::AssistantMessageChunk` | `agent_thread::AssistantMessageChunk` |
| `acp_thread::Plan` | `agent_thread::Plan` |
| `acp_thread::ContentBlock` | `agent_thread::ContentBlock` (the local one from schema.rs) |
| `acp_thread::SessionListUpdate` | `agent_thread::SessionListUpdate` |

In `agent_thread` code, replace all `use agent_client_protocol::schema as acp;` / `acp::` references with the local `schema::*` types. Replace all `use crate::AcpThread` with `use crate::AgentThread`, etc.

### 0d. Strip ACP-specific code from the moved files

When moving `acp_thread` code into `agent_thread`, **do not** move:

- Any code that handles `acp::SessionUpdate` variants only produced by external agents (e.g. certain `SessionUpdate` variants that the native agent never emits)
- `build_terminal_auth_task()` — only used by external ACP auth flows
- The `AcpBetaFeatureFlag` check in `AcpThread::new()` — remove it
- `AgentConnection::auth_methods()`, `AgentConnection::authenticate()`, `AgentConnection::terminal_auth_task()` — these exist solely for external agent auth. The native agent returns `&[]` / no-op. Remove from the trait; delete the implementations on `NativeAgentConnection`.
- `AgentConnection::supports_resume_session()`, `resume_session()` — only used by external ACP agents. Remove.
- `AgentConnection::session_modes()`, `session_config_options()` — keep only if the native agent uses them (it does, for mode switching). Keep.

### 0e. Add `agent_thread` to workspace

In `Cargo.toml`:
```toml
[workspace]
members = [
    # ... existing members ...
    "crates/agent_thread",
]

[workspace.dependencies]
agent_thread = { path = "crates/agent_thread" }
```

---

## Phase 1: Non-code cleanup (no compilation impact) ✅ DONE

### 1a. Documentation

```bash
grep -rn "acp\|Acp\|ACP\|Agent Client Protocol\|agent-client-protocol\|agentclientprotocol" --include="*.md" docs/
```

Files to remove or rewrite:
- `docs/src/ai/external-agents.md` — **delete entirely**
- `docs/src/extensions/agent-servers.md` — **delete entirely** or gut to remove ACP references
- `docs/src/ai/overview.md` — remove external agents / ACP references
- `docs/src/ai/ai-improvement.md` — remove ACP references

### 1b. Other non-code files

```bash
# Eval test referencing codex-acp
rm crates/edit_prediction_cli/evals/codex-acp--add-derive.md

# Bug report template
# .github/ISSUE_TEMPLATE/10_bug_report.yml — remove "ACPs" from the list

# Legal
# legal/subprocessors.md — remove the External Agents / ACP paragraph

# README
# README.md — update the ACP bullet
```

---

## Phase 2: Remove `acp_tools` crate (leaf, only used by xenomorphic) ✅ DONE

```bash
# Confirm sole consumer
grep -rn "acp_tools" --include="*.toml" crates/*/Cargo.toml
# → only xenomorphic/Cargo.toml depends on it
```

Steps:
1. Remove `acp_tools` usage from `crates/xenomorphic/`:
   - `crates/xenomorphic/src/main.rs:699` — remove `acp_tools::init(cx);`
   - `crates/xenomorphic/src/xenomorphic_app.rs:1323-1324` — remove `AcpToolsToolbarItemView` creation and toolbar add
2. Remove from `crates/xenomorphic/Cargo.toml` — delete `acp_tools.workspace = true`
3. Remove from workspace `Cargo.toml`:
   - Remove `"crates/acp_tools"` from `[workspace].members`
   - Remove `acp_tools = { path = "crates/acp_tools" }` from `[workspace.dependencies]`
4. `rm -rf crates/acp_tools/`

Also remove the action it defines:
- `crates/acp_tools/src/acp_tools.rs` defines `OpenAcpLogs` — find and remove all references:
  ```bash
  grep -rn "OpenAcpLogs" --include="*.rs" . | grep -v target/
  ```

---

## Phase 3: Remove ACP-specific UI features ✅ DONE

### 3a. ACP Registry UI

```bash
grep -rn "AcpRegistry" --include="*.rs" . | grep -v target/
```

Remove from:
- `crates/xenomorphic_actions/src/lib.rs:112` — delete `pub struct AcpRegistry;` action
- `crates/agent_ui/src/agent_ui.rs:448` — remove `AcpRegistry` action handler
- `crates/agent_ui/src/agent_configuration.rs:1023` — remove ACP registry link
- `crates/agent_ui/src/agent_panel.rs:3654` — remove ACP registry action dispatch
- `crates/agent_ui/src/agent_registry_ui.rs` — **delete entire file**
- `crates/extensions_ui/src/extensions_ui.rs` — remove `acp_registry_upsell_keywords()`, `render_acp_registry_upsell()`, `show_acp_registry_upsell` field
- `crates/icons/src/icons.rs:11` — remove `AcpRegistry` icon name
- `crates/ui/src/components/ai/ai_setting_item.rs:55` — remove `Registry` variant or change `Self::Registry => IconName::AcpRegistry` to a generic icon
- `crates/client/src/xenomorphic_urls.rs:55-58` — remove `acp_registry_blog()`

### 3b. ACP Thread Import Onboarding

```bash
grep -rn "AcpThreadImportOnboarding\|acp-thread-import\|dismissed-acp-thread-import" --include="*.rs" . | grep -v target/
```

Remove from:
- `crates/agent_ui/src/thread_import.rs` — **delete entire file** (it's entirely about importing ACP sessions)
- `crates/agent_ui/src/agent_ui.rs` — remove `AcpThreadImportOnboarding` re-exports/registrations
- `crates/agent_ui/src/agent_panel.rs` — remove import and usage of `AcpThreadImportOnboarding`
- `crates/sidebar/src/sidebar.rs` — remove `should_render_acp_import_onboarding()`, `render_acp_import_onboarding()`, and their call sites

### 3c. ACP Debug Logs

The `AcpConnection` debug log infrastructure (in `agent_servers::acp`) was only consumed by `acp_tools::AcpToolsToolbarItemView` (already removed in Phase 2). No remaining consumers — will be cleaned when `agent_servers::acp` is removed.

---

## Phase 4: Rename keymap context entries ✅ DONE

```bash
grep -n "AcpThread\|acp_thread" assets/keymaps/default-macos.json
grep -n "AcpThread\|acp_thread" assets/keymaps/default-linux.json
grep -n "AcpThread\|acp_thread" assets/keymaps/default-windows.json
```

Rename `"AcpThread"` → `"AgentThread"` and `"acp_thread"` → `"agent_thread"` in all three keymap files (~14 references each).

The key contexts that set these:
- `crates/agent_ui/src/conversation_view/thread_view.rs:8969` — `.key_context("AcpThread")` → `.key_context("AgentThread")`
- `crates/agent_ui/src/agent_panel.rs:4079` — `key_context.add("acp_thread")` → `key_context.add("agent_thread")`

---

## Phase 5: Remove `agent_servers::acp` module and custom agent server ACP code ✅ DONE

This is the **external ACP protocol connection implementation** — the core of what "ACP support" means.

### 5a. Remove `agent_servers::acp` module

```bash
# 3778 lines in:
wc -l crates/agent_servers/src/acp.rs
```

This module contains:
- `AcpConnection` — the stdio-based ACP protocol client
- `AcpSession`, `AcpSessionList`, `PendingAcpSession`
- `AcpDebugMessage*` — debug log types (already orphaned by Phase 2/3c)
- `AcpSessionModes`, `AcpSessionConfigOptions`
- `test_support` submodule (`FakeAcpAgentServer`, `FakeAcpConnectionHarness`, `connect_fake_acp_connection`)
- `GEMINI_TERMINAL_AUTH_METHOD_ID`
- `UnsupportedVersion`
- The `connect()` async fn that establishes an ACP stdio connection

Steps:
1. Delete `crates/agent_servers/src/acp.rs`
2. Remove `mod acp;` from `crates/agent_servers/src/agent_servers.rs`
3. Remove all `pub use acp::` re-exports from `crates/agent_servers/src/agent_servers.rs`:
   - `pub use acp::test_support::{FakeAcpAgentServer, FakeAcpConnectionHarness, connect_fake_acp_connection};`
   - `pub use acp::{AcpConnection, AcpDebugMessage, AcpDebugMessageContent, AcpDebugMessageDirection, GEMINI_TERMINAL_AUTH_METHOD_ID};`
4. Remove `use acp_thread::AgentConnection;` from `agent_servers.rs`
5. Remove `use agent_client_protocol::schema as acp_schema;` from `agent_servers.rs`

### 5b. Remove `agent_servers::custom` entirely

`crates/agent_servers/src/custom.rs` contains `CustomAgentServer` — the user-configurable agent server that spawns an external process and connects via ACP. Delete entirely:
- Delete `crates/agent_servers/src/custom.rs`
- Remove `mod custom;` and `pub use custom::*;` from `agent_servers.rs`

### 5c. Remove ACP-specific `AgentServer` trait methods

In `crates/agent_servers/src/agent_servers.rs`, the `AgentServer` trait has methods that return `acp_schema::*` types:
```bash
grep -n "acp_schema::" crates/agent_servers/src/agent_servers.rs
```

Remove:
- `fn default_mode(&self, _cx: &App) -> Option<acp_schema::SessionModeId>`
- `fn set_default_mode(...)`
- `fn default_model(&self, _cx: &App) -> Option<acp_schema::ModelId>`
- `fn set_default_model(...)`
- `fn favorite_model_ids(&self, _cx: &mut App) -> HashSet<acp_schema::ModelId>`
- `fn toggle_favorite_model(...)`
- `fn default_config_option(...)`
- `fn set_default_config_option(...)`
- `fn favorite_config_option_value_ids(...)`
- `fn toggle_favorite_config_option_value(...)`

### 5d. Remove e2e tests

```bash
wc -l crates/agent_servers/src/e2e_tests.rs
```

Delete entirely — tests use `FakeAcpAgentServer` and `AcpConnection`.

### 5e. Update `agent_servers/Cargo.toml`

Remove dependencies:
- `acp_thread.workspace = true`
- `agent-client-protocol.workspace = true`
- `async-channel.workspace = true` (only used by acp module)
- `google_ai.workspace = true` (used for gemini API key)
- `xenomorphic_credentials_provider.workspace = true` (used for gemini API key)
- `feature_flags.workspace = true` (used for `AcpBetaFeatureFlag`)
- Feature `"test-support"` — remove `acp_thread/test-support`

After Phase 5, `agent_servers` may be nearly empty. Consider deleting the crate entirely or merging the `NativeAgentServer` registration into `agent`.

---

## Phase 6: Remove ACP features from `agent_ui` ✅ PARTIAL — type migration merged into Phase 8

The ACP-feature removals are done. The type migration (`acp::` → `agent_thread::schema::`, `AcpThread` → `AgentThread`, `acp_thread::` → `agent_thread::`) is deferred to Phase 8 because it requires the `agent` crate to be migrated simultaneously.

**Completed:**
- Removed `AcpRegistry` action handler from `agent_ui.rs`
- Deleted `agent_registry_ui.rs` entirely
- Removed `AcpConnection` downcast from `agent_connection_store.rs` (changed `ActiveAcpConnection` to use `Rc<dyn AgentConnection>` instead of `Rc<AcpConnection>`)
- Stubbed `CustomAgentServer::new()` calls (will be fully removed when Custom agent variant is deleted)
- Replaced `GEMINI_TERMINAL_AUTH_METHOD_ID` with string literal `"gemini-terminal-auth"`
- Removed all `AgentServer` trait method calls that were removed in Phase 5:
  - `favorite_model_ids`, `default_model`, `set_default_model`, `toggle_favorite_model` in `model_selector.rs`
  - `favorite_config_option_value_ids`, `default_config_option`, `set_default_config_option`, `toggle_favorite_config_option_value` in `config_options.rs`
  - `default_mode`, `set_default_mode` in `mode_selector.rs`
- Replaced `AiSettingItemSource::Registry` with `AiSettingItemSource::Custom`

**Remaining (done in Phase 8):**
- Replace `use agent_client_protocol::schema as acp;` → `use agent_thread::schema;` in all files
- Replace `acp::` → `schema::` throughout (559 references)
- Replace `use acp_thread::` → `use agent_thread::` (24 references)
- Replace `acp_thread::` → `agent_thread::` in paths (78 references)
- Replace `AcpThread` → `AgentThread` (161 references)
- Replace `AcpThreadEvent` → `AgentThreadEvent` (84 references)

---

## Phase 7: Remove ACP features from `sidebar` ❌ NOT STARTED

Like Phase 6, remove ACP-feature code (import onboarding) but defer type migration to Phase 8.

**To do:**
- Remove `should_render_acp_import_onboarding`, `render_acp_import_onboarding` and call sites in `sidebar.rs`

**Remaining (done in Phase 8):**
- Replace `acp::` → `schema::` and `acp_thread::` → `agent_thread::` type refs
- Replace `AcpThread` → `AgentThread` etc.

---

## Phase 8: Big-bang type migration ❌ NOT STARTED

This is the **critical phase**. The `agent` crate, `agent_ui`, `sidebar`, `agent_settings`, `agent_servers`, and `agent_thread` must all be migrated **simultaneously**. You cannot incrementally migrate one crate at a time because:

1. `Entity<AcpThread>` (created in the `agent` crate) and `Entity<AgentThread>` (from `agent_thread`) are different GPUI entity types — `agent_ui` can't read one as the other.
2. `AcpThread`'s methods return `acp::SessionId`, `acp::ContentBlock`, etc. If `agent_ui` switches to expecting `agent_thread::schema::SessionId`, the types won't match across the crate boundary.
3. `NativeAgentConnection` implements `acp_thread::AgentConnection`, not `agent_thread::AgentConnection`. The `AgentServer::connect()` return type changes accordingly.

### 8a. Migrate the `agent` crate

The `agent` crate is the root producer of all the types. It creates `Entity<AcpThread>`, implements `acp_thread::AgentConnection` on `NativeAgentConnection`, and its `ThreadStore` returns `acp::SessionId` etc.

Mechanical sed replacements across `crates/agent/src/`:

| Pattern | Replacement | ~Count |
|---------|-------------|--------|
| `use agent_client_protocol::schema as acp;` | `use agent_thread::schema;` | ~27 files |
| `acp::` | `schema::` | ~200 refs |
| `use acp_thread::` | `use agent_thread::` | ~15 refs |
| `acp_thread::` | `agent_thread::` | ~40 refs |
| `AcpThread` | `AgentThread` | ~30 refs |
| `AcpThreadEvent` | `AgentThreadEvent` | ~15 refs |

Semantic changes in `agent/src/agent.rs`:
- Remove from `NativeAgentConnection` impl: `auth_methods()`, `authenticate()`, `terminal_auth_task()`, `supports_resume_session()`, `resume_session()`
- Change `impl acp_thread::AgentConnection for NativeAgentConnection` → `impl agent_thread::AgentConnection for NativeAgentConnection`

Update `agent/Cargo.toml`:
- Replace `acp_thread.workspace = true` → `agent_thread.workspace = true`
- Remove `agent-client-protocol.workspace = true`

### 8b. Migrate `agent_ui`

After 8a is done, the `agent` crate now produces `Entity<AgentThread>` and `Rc<dyn agent_thread::AgentConnection>`. Now `agent_ui` can be migrated:

Mechanical sed replacements across `crates/agent_ui/src/`:

| Pattern | Replacement | ~Count |
|---------|-------------|--------|
| `use agent_client_protocol::schema as acp;` | `use agent_thread::schema;` | ~19 files |
| `acp::` | `schema::` | ~559 refs |
| `use acp_thread::` | `use agent_thread::` | ~24 refs |
| `acp_thread::` | `agent_thread::` | ~78 refs |
| `AcpThread` | `AgentThread` | ~161 refs |
| `AcpThreadEvent` | `AgentThreadEvent` | ~84 refs |
| `AcpServerViewEvent` | `AgentServerViewEvent` | ~5 refs |
| `handle_acp_thread_event` | `handle_agent_thread_event` | ~5 refs |

Cleanup after sed:
- Fix `agent_thread::AgentThread` vs `agent_panel::AgentThread` name clash (the local panel `AcpThread` struct must be renamed to `ActiveAgentThread` or similar)
- Fix `SessionMode.id` → `SessionMode.mode_id` (our schema uses `mode_id` not `id`)
- Fix `SessionConfigSelectOptions::Ungrouped` → our enum uses `Ungrouped` (verified matching)
- Remove `unimplemented!("CustomAgentServer removed")` stubs — delete the `Agent::Custom` variant entirely
- Remove `"gemini-terminal-auth"` string literals — delete the auth method matching code
- Remove `acp_thread` and `agent-client-protocol` from `agent_ui/Cargo.toml`
- Add `agent_thread.workspace = true` to `agent_ui/Cargo.toml`
- Update test-support feature: `"acp_thread/test-support"` → `"agent_thread/test-support"`

### 8c. Migrate `sidebar`

Much smaller — ~20 references total.

Replace `acp::` → `schema::`, `acp_thread::` → `agent_thread::`, `AcpThread` → `AgentThread`. Update `Cargo.toml`.

### 8d. Migrate `agent_settings`

Only usage: `acp::ModelId` in `AgentSettings::favorite_model_ids()`. Replace with `agent_thread::schema::ModelId`. Update `Cargo.toml`.

### 8e. Migrate `agent_servers`

Currently uses `acp_thread::AgentConnection` in the `AgentServer` trait. Switch to `agent_thread::AgentConnection`. Update `Cargo.toml`. Remove `agent-client-protocol` dep.

### 8f. Verify compilation

```bash
cargo check -p agent -p agent_ui -p sidebar -p agent_settings -p agent_servers
```

Expect cascading errors from type mismatches at crate boundaries. Fix each one. Common patterns:
- `acp::SessionId` vs `schema::SessionId` — use `.0.clone()` bridge temporarily, then clean up
- `acp_thread::AgentSessionList` vs `agent_thread::AgentSessionList` — change `thread_import.rs` to use `agent_thread` version
- `FakeAcpAgentServer` test references — replace with `StubAgentConnection`

---

## Phase 9: Switch `xenomorphic` and `eval_cli` ❌ NOT STARTED

After Phase 8, the remaining consumer crates can be migrated independently:

### `xenomorphic`

```bash
grep -rn "acp_thread\|agent_client_protocol\|acp::" --include="*.rs" crates/xenomorphic/ | grep -v target/
```

- `src/main.rs` — replace `agent_client_protocol::schema as acp` → `agent_thread::schema`
- `src/xenomorphic_app.rs` — replace `acp::` → `agent_thread::`
- `src/visual_test_runner.rs` — replace `acp_thread::StubAgentConnection` → `agent_thread::StubAgentConnection`
- Update `Cargo.toml`: remove `agent-client-protocol`, replace `acp_thread` → `agent_thread`

### `eval_cli`

```bash
grep -rn "acp_thread\|agent_client_protocol\|acp::" --include="*.rs" crates/eval_cli/
```

- Replace all `acp_thread::` → `agent_thread::` and `acp::` → `agent_thread::schema::*`
- Update `Cargo.toml`: remove `agent-client-protocol`, replace `acp_thread` → `agent_thread`

---

## Phase 10: Remove feature flag ❌ NOT STARTED

```bash
grep -rn "AcpBetaFeatureFlag\|acp-beta" --include="*.rs" . | grep -v target/
```

- `crates/feature_flags/src/flags.rs` — delete `AcpBetaFeatureFlag` struct and registration
- Remove all `has_flag::<AcpBetaFeatureFlag>()` checks

---

## Phase 11: Remove ACP from `project` crate ❌ NOT STARTED

```bash
grep -rn "acp\|Acp\|ACP" --include="*.rs" crates/project/ | grep -v target/
```

- `crates/project/src/agent_server_store.rs` — remove `Registry` variant from `CustomAgentServerSettings` handling, remove `ExternalAgentSource::Registry` handling
- `crates/project/src/agent_registry_store.rs` — **delete entire file** (the ACP registry client)
- Remove `AgentRegistryStore` global and all its consumers:

```bash
grep -rn "AgentRegistryStore" --include="*.rs" . | grep -v target/
```

- Remove `CustomAgentServerSettings::Registry` variant from settings types
- Remove registry agent entries (`claude-acp`, `codex-acp`) from migration defaults
- Remove ACP references from `onboarding`: `"claude-acp"`, `"codex-acp"` in `basics_page.rs`

---

## Phase 12: Delete `acp_thread` crate and `agent-client-protocol` dep ❌ NOT STARTED (blocked by Phase 8)

Once Phase 8 is complete and `agent_thread` is the sole provider:

1. `rm -rf crates/acp_thread/`
2. Remove from workspace `Cargo.toml`:
   - `"crates/acp_thread"` from `[workspace].members`
   - `acp_thread = { path = "crates/acp_thread" }` from `[workspace.dependencies]`
   - `agent-client-protocol = { version = "=0.11.1", features = ["unstable"] }` from `[workspace.dependencies]`
3. Remove any remaining `acp_thread` / `agent-client-protocol` deps from `Cargo.toml` files

---

## Phase 13: Remaining scattered references ❌ NOT STARTED

```bash
# Terminal crate (just a comment)
grep -n "codex-acp" crates/terminal/src/terminal.rs

# File finder test (references acp.rs path)
grep -n "acp" crates/file_finder/src/file_finder_tests.rs

# Any remaining ACP naming in code
grep -rn "AcpThread\|acp_thread\|AcpConnection\|GEMINI_TERMINAL_AUTH" --include="*.rs" . | grep -v target/ | grep -v agent_thread
```

---

## Phase 14: Compile and fix ❌ NOT STARTED

```bash
cargo check --workspace
```

Search for stragglers:
```bash
grep -rn "acp_thread\|agent-client-protocol\|acp::" --include="*.rs" . | grep -v target/
```

## Quick-reference: Find-all commands

```bash
# All Rust source files referencing acp_thread crate
grep -rn "use acp_thread\|acp_thread::" --include="*.rs" . | grep -v target/

# All Rust source files referencing agent_client_protocol
grep -rn "agent_client_protocol" --include="*.rs" . | grep -v target/

# All Cargo.toml files with ACP dependencies
grep -rn "acp_thread\|acp_tools\|agent-client-protocol" --include="*.toml" . | grep -v target/

# All keymap ACP references
grep -rn "AcpThread\|acp_thread" assets/keymaps/

# All doc references
grep -rn "ACP\|Agent Client Protocol\|acp" --include="*.md" docs/ README.md legal/

# All icon references
grep -rn "AcpRegistry" --include="*.rs" . | grep -v target/

# All action references
grep -rn "AcpRegistry\|OpenAcpLogs" --include="*.rs" . | grep -v target/

# All test-support references
grep -rn "StubAgentConnection\|FakeAcpAgentServer\|FakeAcpConnectionHarness\|connect_fake_acp" --include="*.rs" . | grep -v target/
```

## Phase 25: Complete Vestigial Code Removal ✅ DONE

Removed ALL remaining ACP vestigial code from production paths:

### From `agent_thread/src/connection.rs`:
- Removed `fn auth_methods()` default trait method
- Removed `fn authenticate()` default trait method  
- Removed `fn terminal_auth_task()` default trait method
- Removed `fn supports_resume_session()` default trait method
- Removed `fn resume_session()` default trait method
- Removed `use task::SpawnInTerminal` import

### From `agent_thread/src/schema.rs`:
- Removed `AuthMethodId` struct + `impl`
- Removed `AuthMethod` enum + `impl`
- Removed `AuthMethodAgent` struct + `impl`
- Removed "Auth types" section header

### From `agent_ui/src/conversation_view.rs`:
- Removed `AuthState` enum (both variants)
- Removed `auth_state` field from `ConnectedServerState`
- Removed `auth_task` field from `ConversationView`
- Removed `has_auth_methods()` method
- Removed `handle_auth_required()` method
- Removed `authenticate()` method
- Removed `spawn_external_agent_login()` method
- Removed `render_auth_required_state()` method
- Removed `reauthenticate()` method
- Removed ConfigOptionsView/ModeSelector creation logic
- Removed AuthState::Unauthenticated match arm from Render
- Removed supports_resume_session branch from initial_state
- Removed AuthRequired error handling from initial_state

### From `agent_ui/src/conversation_view/thread_view.rs`:
- Previously removed `authenticate_button()` method
- Previously removed `/login` auth flow trigger

### From `agent_ui/src/agent_panel.rs`:
- Removed `has_auth_methods` check
- Removed "Reauthenticate" menu item
- Removed `ReauthenticateAgent` action handler

### From `xenomorphic_actions`:
- Removed `ReauthenticateAgent` action definition

### Removed test code:
- `AuthGatedAgentConnection` struct + impls
- `test_auth_required_on_initial_connect` test
- `test_notification_for_error` test (needed FakeAcpAgentServer)
- `test_acp_server_exit` test (needed FakeAcpAgentServer)
- `test_resume_without_history_adds_notice` test
- All `fn auth_methods()` / `fn authenticate()` overrides from test `impl AgentConnection` blocks
- `SimpleTestAgentServer` from agent_servers

### What remains (kept intentionally):
- `AuthRequired` error struct in `connection.rs` — still used as a protocol error type
- `ErrorCode::AuthRequired` in `schema.rs` — legitimate protocol error code
- `resume_session_id` variable name in conversation_view — just a parameter name for session persistence (uses `load_session`, not the removed `resume_session` trait method)
