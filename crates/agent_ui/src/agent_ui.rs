mod agent_configuration;
pub mod agent_connection_store;
mod agent_diff;
pub mod agent_session_item;
mod agent_model_selector;
mod agent_panel;
pub mod thread_finder_provider;
mod buffer_codegen;
mod completion_provider;
mod context;
mod context_server_configuration;
pub(crate) mod conversation_view;
mod diagnostics;
mod entry_view_state;
mod external_source_prompt;
mod favorite_models;
mod inline_assistant;
mod inline_prompt_editor;
mod language_model_selector;
mod mention_set;
mod message_editor;
mod model_selector;
mod model_selector_popover;
mod profile_selector;
mod terminal_codegen;
mod terminal_inline_assistant;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
mod thread_import;
pub mod thread_metadata_store;
pub mod thread_worktree_archive;
pub mod threads_archive_view;
mod ui;

use std::rc::Rc;
use std::sync::Arc;

use ::ui::IconName;
use agent_thread::schema;
use agent_settings::{AgentProfileId, AgentSettings};
use command_palette_hooks::CommandPaletteFilter;
use fs::Fs;
use gpui::{Action, App, Context, Entity, SharedString, Window, actions};
use language::{
    LanguageRegistry,
    language_settings::{AllLanguageSettings, EditPredictionProvider},
};
use language_model::{
    ConfiguredModel, LanguageModelId, LanguageModelProviderId, LanguageModelRegistry,
};
use project::{AgentId, DisableAiSettings};
use prompt_store::PromptBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings::{LanguageModelSelection, Settings as _, SettingsStore};
use std::any::TypeId;
use workspace::Workspace;

use crate::agent_configuration::{ConfigureContextServerModal, ManageProfilesModal};
pub use crate::agent_connection_store::{ActiveAgentConnection, AgentConnectionStore};
pub use crate::inline_assistant::InlineAssistant;
pub use crate::thread_metadata_store::ThreadId;
pub use agent_diff::{AgentDiffPane, AgentDiffToolbar};
pub use agent_session_item::AgentSessionItem;
pub use conversation_view::ConversationView;
pub use external_source_prompt::ExternalSourcePrompt;
pub(crate) use model_selector::ModelSelector;
pub(crate) use model_selector_popover::ModelSelectorPopover;
pub use thread_import::{
    CrossChannelImportOnboarding,
    channels_with_threads, import_threads_from_other_channels,
};
use xenomorphic_actions;
pub use xenomorphic_actions::{CreateWorktree, NewWorktreeBranchTarget, SwitchWorktree};

pub const DEFAULT_THREAD_TITLE: &str = "New Agent Thread";
const PARALLEL_AGENT_LAYOUT_BACKFILL_KEY: &str = "parallel_agent_layout_backfilled";
actions!(
    agent,
    [
        /// Toggles the menu to create new agent threads.
        ToggleNewThreadMenu,
        /// Toggles the options menu for agent settings and preferences.
        ToggleOptionsMenu,
        /// Toggles the profile or mode selector for switching between agent profiles.
        ToggleProfileSelector,
        /// Cycles through favorited models in the model selector.
        CycleFavoriteModels,
        /// Expands the message editor to full size.
        ExpandMessageEditor,
        /// Adds a context server to the configuration.
        AddContextServer,
        /// Archives the currently selected thread.
        ArchiveSelectedThread,
        /// Removes the currently selected thread.
        RemoveSelectedThread,
        /// Starts a chat conversation with follow-up enabled.
        ChatWithFollow,
        /// Cycles to the next inline assist suggestion.
        CycleNextInlineAssist,
        /// Cycles to the previous inline assist suggestion.
        CyclePreviousInlineAssist,
        /// Moves focus up in the interface.
        FocusUp,
        /// Moves focus down in the interface.
        FocusDown,
        /// Moves focus left in the interface.
        FocusLeft,
        /// Moves focus right in the interface.
        FocusRight,
        /// Opens the active thread as a markdown file.
        OpenActiveThreadAsMarkdown,
        /// Opens the agent diff view to review changes.
        OpenAgentDiff,
        /// Copies the current thread to the clipboard as JSON for debugging.
        CopyThreadToClipboard,
        /// Loads a thread from the clipboard JSON for debugging.
        LoadThreadFromClipboard,
        /// Keeps the current suggestion or change.
        Keep,
        /// Rejects the current suggestion or change.
        Reject,
        /// Rejects all suggestions or changes.
        RejectAll,
        /// Undoes the most recent reject operation, restoring the rejected changes.
        UndoLastReject,
        /// Keeps all suggestions or changes.
        KeepAll,
        /// Allow this operation only this time.
        AllowOnce,
        /// Allow this operation and remember the choice.
        AllowAlways,
        /// Reject this operation only this time.
        RejectOnce,
        /// Follows the agent's suggestions.
        Follow,
        /// Opens the "Add Context" menu in the message editor.
        OpenAddContextMenu,
        /// Continues the current thread.
        ContinueThread,
        /// Interrupts the current generation and sends the message immediately.
        SendImmediately,
        /// Sends the next queued message immediately.
        SendNextQueuedMessage,
        /// Removes the first message from the queue (the next one to be sent).
        RemoveFirstQueuedMessage,
        /// Edits the first message in the queue (the next one to be sent).
        EditFirstQueuedMessage,
        /// Clears all messages from the queue.
        ClearMessageQueue,
        /// Opens the permission granularity dropdown for the current tool call.
        OpenPermissionDropdown,
        /// Toggles thinking mode for models that support extended thinking.
        ToggleThinkingMode,
        /// Cycles through available thinking effort levels for the current model.
        CycleThinkingEffort,
        /// Toggles the thinking effort selector menu open or closed.
        ToggleThinkingEffortMenu,
        /// Toggles fast mode for models that support it.
        ToggleFastMode,
        /// Scroll the output by one page up.
        ScrollOutputPageUp,
        /// Scroll the output by one page down.
        ScrollOutputPageDown,
        /// Scroll the output up by three lines.
        ScrollOutputLineUp,
        /// Scroll the output down by three lines.
        ScrollOutputLineDown,
        /// Scroll the output to the top.
        ScrollOutputToTop,
        /// Scroll the output to the bottom.
        ScrollOutputToBottom,
        /// Scroll the output to the previous user message.
        ScrollOutputToPreviousMessage,
        /// Scroll the output to the next user message.
        ScrollOutputToNextMessage,
        /// Import agent threads from other Xenomorphic release channels (e.g. Preview, Nightly).
        ImportThreadsFromOtherChannels,
    ]
);

actions!(
    dev,
    [
        /// Shows metadata for the currently active thread.
        ShowThreadMetadata,
    ]
);

/// Action to authorize a tool call with a specific permission option.
/// This is used by the permission granularity dropdown to authorize tool calls.
#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = agent)]
#[serde(deny_unknown_fields)]
pub struct AuthorizeToolCall {
    /// The tool call ID to authorize.
    pub tool_call_id: String,
    /// The permission option ID to use.
    pub option_id: String,
    /// The kind of permission option (serialized as string).
    pub option_kind: String,
}

/// Action to select a permission granularity option from the dropdown.
/// This updates the selected granularity without triggering authorization.
#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = agent)]
#[serde(deny_unknown_fields)]
pub struct SelectPermissionGranularity {
    /// The tool call ID for which to select the granularity.
    pub tool_call_id: String,
    /// The index of the selected granularity option.
    pub index: usize,
}

/// Action to toggle a command pattern checkbox in the permission dropdown.
#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = agent)]
#[serde(deny_unknown_fields)]
pub struct ToggleCommandPattern {
    /// The tool call ID for which to toggle the pattern.
    pub tool_call_id: String,
    /// The index of the command pattern to toggle.
    pub pattern_index: usize,
}

/// Creates a new conversation thread, optionally based on an existing thread.
#[derive(Default, Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = agent)]
#[serde(deny_unknown_fields)]
pub struct NewThread;

/// Creates a new agent conversation thread.
#[derive(Default, Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = agent)]
#[serde(deny_unknown_fields)]
pub struct NewAgentThread;

#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = agent)]
#[serde(deny_unknown_fields)]
pub struct NewNativeAgentThreadFromSummary {
    from_session_id: schema::SessionId,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Agent {
    #[default]
    #[serde(alias = "NativeAgent", alias = "TextThread")]
    NativeAgent,
    #[cfg(any(test, feature = "test-support"))]
    Stub,
}

impl Serialize for Agent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::NativeAgent => serializer.serialize_str("native_agent"),
            #[cfg(any(test, feature = "test-support"))]
            Self::Stub => serializer.serialize_str("stub"),
        }
    }
}

impl<'de> Deserialize<'de> for Agent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor, MapAccess};

        struct AgentVisitor;

        impl<'de> Visitor<'de> for AgentVisitor {
            type Value = Agent;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an agent variant")
            }

            fn visit_str<E>(self, v: &str) -> Result<Agent, E>
            where
                E: de::Error,
            {
                match v {
                    "native_agent" | "NativeAgent" | "TextThread" => Ok(Agent::NativeAgent),
                    #[cfg(any(test, feature = "test-support"))]
                    "stub" | "Stub" => Ok(Agent::Stub),
                    // Unknown variants (e.g. removed "Custom") fall back to NativeAgent
                    _ => Ok(Agent::NativeAgent),
                }
            }

            fn visit_map<M>(self, mut map: M) -> Result<Agent, M::Error>
    where
                M: MapAccess<'de>,
            {
                // Consume the key (e.g. "custom") and discard its value
                let _ = map.next_key::<String>()?;
                let _ = map.next_value::<serde::de::IgnoredAny>()?;
                // Map any struct variant (like the removed "Custom") to NativeAgent
                Ok(Agent::NativeAgent)
            }
        }

        deserializer.deserialize_any(AgentVisitor)
    }
}

impl Agent {
    pub fn id(&self) -> AgentId {
        match self {
            Self::NativeAgent => agent::XENOMORPHIC_AGENT_ID.clone(),
            #[cfg(any(test, feature = "test-support"))]
            Self::Stub => "stub".into(),
        }
    }

    pub fn is_native(&self) -> bool {
        matches!(self, Self::NativeAgent)
    }

    pub fn label(&self) -> SharedString {
        match self {
            Self::NativeAgent => "Xenomorphic Agent".into(),
            #[cfg(any(test, feature = "test-support"))]
            Self::Stub => "Stub Agent".into(),
        }
    }

    pub fn icon(&self) -> Option<IconName> {
        match self {
            Self::NativeAgent => None,
            #[cfg(any(test, feature = "test-support"))]
            Self::Stub => None,
        }
    }

    pub fn server(
        &self,
        fs: Arc<dyn fs::Fs>,
        thread_store: Entity<agent::ThreadStore>,
    ) -> Rc<dyn agent_servers::AgentServer> {
        match self {
            Self::NativeAgent => Rc::new(agent::NativeAgentServer::new(fs, thread_store)),
            #[cfg(any(test, feature = "test-support"))]
            Self::Stub => Rc::new(crate::test_support::StubAgentServer::default_response()),
        }
    }
}

/// Content to initialize new external agent with.
pub enum AgentInitialContent {
    ThreadSummary {
        session_id: schema::SessionId,
        title: Option<SharedString>,
    },
    ContentBlock {
        blocks: Vec<schema::ContentBlock>,
        auto_submit: bool,
    },
    FromExternalSource(ExternalSourcePrompt),
}

impl From<ExternalSourcePrompt> for AgentInitialContent {
    fn from(prompt: ExternalSourcePrompt) -> Self {
        Self::FromExternalSource(prompt)
    }
}

/// Opens the profile management interface for configuring agent tools and settings.
#[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = agent)]
#[serde(deny_unknown_fields)]
pub struct ManageProfiles {
    #[serde(default)]
    pub customize_tools: Option<AgentProfileId>,
}

impl ManageProfiles {
    pub fn customize_tools(profile_id: AgentProfileId) -> Self {
        Self {
            customize_tools: Some(profile_id),
        }
    }
}

#[derive(Clone)]
pub(crate) enum ModelUsageContext {
    InlineAssistant,
}

impl ModelUsageContext {
    pub fn configured_model(&self, cx: &App) -> Option<ConfiguredModel> {
        match self {
            Self::InlineAssistant => {
                LanguageModelRegistry::read_global(cx).inline_assistant_model()
            }
        }
    }
}

pub(crate) fn humanize_token_count(count: u64) -> String {
    match count {
        0..=999 => count.to_string(),
        1000..=9999 => {
            let thousands = count / 1000;
            let hundreds = (count % 1000 + 50) / 100;
            if hundreds == 0 {
                format!("{}k", thousands)
            } else if hundreds == 10 {
                format!("{}k", thousands + 1)
            } else {
                format!("{}.{}k", thousands, hundreds)
            }
        }
        10_000..=999_999 => format!("{}k", (count + 500) / 1000),
        1_000_000..=9_999_999 => {
            let millions = count / 1_000_000;
            let hundred_thousands = (count % 1_000_000 + 50_000) / 100_000;
            if hundred_thousands == 0 {
                format!("{}M", millions)
            } else if hundred_thousands == 10 {
                format!("{}M", millions + 1)
            } else {
                format!("{}.{}M", millions, hundred_thousands)
            }
        }
        10_000_000.. => format!("{}M", (count + 500_000) / 1_000_000),
    }
}

/// Initializes the `agent` crate.
pub fn init(
    fs: Arc<dyn Fs>,
    prompt_builder: Arc<PromptBuilder>,
    language_registry: Arc<LanguageRegistry>,
    is_new_install: bool,
    is_eval: bool,
    cx: &mut App,
) {
    agent::ThreadStore::init_global(cx);
    if !is_eval {
        // Initializing the language model from the user settings messes with the eval, so we only initialize them when
        // we're not running inside of the eval.
        init_language_model_settings(cx);
    }
    agent_panel::init(cx);
    context_server_configuration::init(language_registry.clone(), fs.clone(), cx);
    thread_metadata_store::init(cx);

    // Register the ThreadFinderProvider with the unified file finder
    // so that cmd-p shows agent thread results alongside file results.
    file_finder::register_finder_provider(crate::thread_finder_provider::ThreadFinderProvider, cx);

    // Register AgentSessionItem as a serializable workspace item so that
    // open agent tabs are persisted and restored across sessions.
    workspace::register_serializable_item::<crate::agent_session_item::AgentSessionItem>(cx);

    // Register "New Agent Thread" in the + button menu on tab bars.
    workspace::new_item_menu::register_new_item_menu_entry(
        workspace::new_item_menu::NewItemMenuEntry {
            label: "New Agent Thread",
            action: crate::NewThread.boxed_clone(),
        },
        cx,
    );

    inline_assistant::init(fs.clone(), prompt_builder.clone(), cx);
    terminal_inline_assistant::init(fs.clone(), prompt_builder, cx);
    cx.observe_new(move |workspace, window, cx| {
        ConfigureContextServerModal::register(workspace, language_registry.clone(), window, cx)
    })
    .detach();
    cx.observe_new(|_workspace: &mut Workspace, _window, _cx| {
    })
    .detach();
    // Initialize the global AgentConnectionStore when the first
    // workspace is created. All AgentSessionItem tabs share this
    // store so they share the same NativeAgent (and thus the same
    // session registry). Without this, each tab would create its
    // own NativeAgent and prompts would fail because the session
    // wouldn't exist in that agent's session map.
    cx.observe_new(|workspace: &mut Workspace, _window, cx| {
        let project = workspace.project().clone();
        crate::agent_connection_store::AgentConnectionStore::init_global(project, cx);
    })
    .detach();

    cx.observe_new(ManageProfilesModal::register).detach();
    cx.observe_new(|workspace: &mut Workspace, _window, _cx| {
        workspace.register_action(
            |workspace: &mut Workspace,
             _: &ImportThreadsFromOtherChannels,
             _window: &mut Window,
             cx: &mut Context<Workspace>| {
                import_threads_from_other_channels(workspace, cx);
            },
        );
    })
    .detach();

    // Update command palette filter based on AI settings
    update_command_palette_filter(cx);

    // Watch for settings changes
    cx.observe_global::<SettingsStore>(|app_cx| {
        // When settings change, update the command palette filter
        update_command_palette_filter(app_cx);
    })
    .detach();

    maybe_backfill_editor_layout(fs, is_new_install, cx);
}

fn maybe_backfill_editor_layout(fs: Arc<dyn Fs>, is_new_install: bool, cx: &mut App) {
    let kvp = db::kvp::KeyValueStore::global(cx);
    let already_backfilled =
        util::ResultExt::log_err(kvp.read_kvp(PARALLEL_AGENT_LAYOUT_BACKFILL_KEY))
            .flatten()
            .is_some();

    if !already_backfilled {
        if !is_new_install {
            AgentSettings::backfill_editor_layout(fs, cx);
        }

        db::write_and_log(cx, move || async move {
            kvp.write_kvp(
                PARALLEL_AGENT_LAYOUT_BACKFILL_KEY.to_string(),
                "1".to_string(),
            )
            .await
        });
    }
}

fn update_command_palette_filter(cx: &mut App) {
    let disable_ai = DisableAiSettings::get_global(cx).disable_ai;
    let agent_enabled = AgentSettings::get_global(cx).enabled;

    let edit_prediction_provider = AllLanguageSettings::get_global(cx)
        .edit_predictions
        .provider;

    CommandPaletteFilter::update_global(cx, |filter, _| {
        use editor::actions::{
            AcceptEditPrediction, AcceptNextLineEditPrediction, AcceptNextWordEditPrediction,
            NextEditPrediction, PreviousEditPrediction, ShowEditPrediction, ToggleEditPrediction,
        };
        let edit_prediction_actions = [
            TypeId::of::<AcceptEditPrediction>(),
            TypeId::of::<AcceptNextWordEditPrediction>(),
            TypeId::of::<AcceptNextLineEditPrediction>(),
            TypeId::of::<AcceptEditPrediction>(),
            TypeId::of::<ShowEditPrediction>(),
            TypeId::of::<NextEditPrediction>(),
            TypeId::of::<PreviousEditPrediction>(),
            TypeId::of::<ToggleEditPrediction>(),
        ];

        if disable_ai {
            filter.hide_namespace("agent");
            filter.hide_namespace("agents");
            filter.hide_namespace("assistant");
            filter.hide_namespace("copilot");
            filter.hide_namespace("edit_prediction");

            filter.hide_action_types(&edit_prediction_actions);
        } else {
            if agent_enabled {
                filter.show_namespace("agent");
                filter.show_namespace("agents");
                filter.show_namespace("assistant");
            } else {
                filter.hide_namespace("agent");
                filter.hide_namespace("agents");
                filter.hide_namespace("assistant");
            }

            match edit_prediction_provider {
                EditPredictionProvider::None => {
                    filter.hide_namespace("edit_prediction");
                    filter.hide_namespace("copilot");
                    filter.hide_action_types(&edit_prediction_actions);
                }
                EditPredictionProvider::Copilot => {
                    filter.show_namespace("edit_prediction");
                    filter.show_namespace("copilot");
                    filter.show_action_types(edit_prediction_actions.iter());
                }
                EditPredictionProvider::Codestral
                | EditPredictionProvider::Ollama
                | EditPredictionProvider::OpenAiCompatibleApi
                | EditPredictionProvider::Mercury => {
                    filter.show_namespace("edit_prediction");
                    filter.hide_namespace("copilot");
                    filter.show_action_types(edit_prediction_actions.iter());
                }
            }

            filter.show_namespace("multi_workspace");
        }
    });
}

fn init_language_model_settings(cx: &mut App) {
    update_active_language_model_from_settings(cx);

    cx.observe_global::<SettingsStore>(update_active_language_model_from_settings)
        .detach();
    cx.subscribe(
        &LanguageModelRegistry::global(cx),
        |_, event: &language_model::Event, cx| match event {
            language_model::Event::ProviderStateChanged(_)
            | language_model::Event::AddedProvider(_)
            | language_model::Event::RemovedProvider(_)
            | language_model::Event::ProvidersChanged => {
                update_active_language_model_from_settings(cx);
            }
            _ => {}
        },
    )
    .detach();
}

fn update_active_language_model_from_settings(cx: &mut App) {
    let settings = AgentSettings::get_global(cx);

    fn to_selected_model(selection: &LanguageModelSelection) -> language_model::SelectedModel {
        language_model::SelectedModel {
            provider: LanguageModelProviderId::from(selection.provider.0.clone()),
            model: LanguageModelId::from(selection.model.clone()),
        }
    }

    let default = settings.default_model.as_ref().map(to_selected_model);
    let inline_assistant = settings
        .inline_assistant_model
        .as_ref()
        .map(to_selected_model);
    let commit_message = settings
        .commit_message_model
        .as_ref()
        .map(to_selected_model);
    let thread_summary = settings
        .thread_summary_model
        .as_ref()
        .map(to_selected_model);
    let inline_alternatives = settings
        .inline_alternatives
        .iter()
        .map(to_selected_model)
        .collect::<Vec<_>>();

    LanguageModelRegistry::global(cx).update(cx, |registry, cx| {
        registry.select_default_model(default.as_ref(), cx);
        registry.select_inline_assistant_model(inline_assistant.as_ref(), cx);
        registry.select_commit_message_model(commit_message.as_ref(), cx);
        registry.select_thread_summary_model(thread_summary.as_ref(), cx);
        registry.select_inline_alternative_models(inline_alternatives, cx);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_settings::{AgentProfileId, AgentSettings};
    use command_palette_hooks::CommandPaletteFilter;
    use db::kvp::KeyValueStore;
    use editor::actions::AcceptEditPrediction;
    use gpui::{BorrowAppContext, TestAppContext, px};
    use project::DisableAiSettings;
    use settings::{
        DockPosition, NotifyWhenAgentWaiting, PlaySoundWhenAgentDone, Settings, SettingsStore,
    };

    #[gpui::test]
    fn test_agent_command_palette_visibility(cx: &mut TestAppContext) {
        // Init settings
        cx.update(|cx| {
            let store = SettingsStore::test(cx);
            cx.set_global(store);
            command_palette_hooks::init(cx);
            AgentSettings::register(cx);
            DisableAiSettings::register(cx);
            AllLanguageSettings::register(cx);
        });

        let agent_settings = AgentSettings {
            enabled: true,
            button: true,
            dock: DockPosition::Right,
            flexible: true,
            default_width: px(300.),
            default_height: px(600.),
            max_content_width: Some(px(850.)),
            default_model: None,
            subagent_model: None,
            inline_assistant_model: None,
            inline_assistant_use_streaming_tools: false,
            commit_message_model: None,
            thread_summary_model: None,
            inline_alternatives: vec![],
            favorite_models: vec![],
            default_profile: AgentProfileId::default(),
            profiles: Default::default(),
            notify_when_agent_waiting: NotifyWhenAgentWaiting::default(),
            play_sound_when_agent_done: PlaySoundWhenAgentDone::Never,
            single_file_review: false,
            model_parameters: vec![],
            enable_feedback: false,
            expand_edit_card: true,
            expand_terminal_card: true,
            cancel_generation_on_terminal_stop: true,
            use_modifier_to_send: true,
            message_editor_min_lines: 1,
            tool_permissions: Default::default(),
            show_turn_stats: false,
            show_merge_conflict_indicator: true,
            sidebar_side: Default::default(),
            thinking_display: Default::default(),
        };

        cx.update(|cx| {
            AgentSettings::override_global(agent_settings.clone(), cx);
            DisableAiSettings::override_global(DisableAiSettings { disable_ai: false }, cx);

            // Initial update
            update_command_palette_filter(cx);
        });

        // Assert visible
        cx.update(|cx| {
            let filter = CommandPaletteFilter::try_global(cx).unwrap();
            assert!(
                !filter.is_hidden(&NewThread),
                "NewThread should be visible by default"
            );
        });

        // Disable agent
        cx.update(|cx| {
            let mut new_settings = agent_settings.clone();
            new_settings.enabled = false;
            AgentSettings::override_global(new_settings, cx);

            // Trigger update
            update_command_palette_filter(cx);
        });

        // Assert hidden
        cx.update(|cx| {
            let filter = CommandPaletteFilter::try_global(cx).unwrap();
            assert!(
                filter.is_hidden(&NewThread),
                "NewThread should be hidden when agent is disabled"
            );
        });

        // Test EditPredictionProvider
        // Enable EditPredictionProvider::Copilot
        cx.update(|cx| {
            cx.update_global::<SettingsStore, _>(|store, cx| {
                store.update_user_settings(cx, |s| {
                    s.project
                        .all_languages
                        .edit_predictions
                        .get_or_insert(Default::default())
                        .provider = Some(EditPredictionProvider::Copilot);
                });
            });
            update_command_palette_filter(cx);
        });

        cx.update(|cx| {
            let filter = CommandPaletteFilter::try_global(cx).unwrap();
            assert!(
                !filter.is_hidden(&AcceptEditPrediction),
                "EditPrediction should be visible when provider is Copilot"
            );
        });

        // Disable EditPredictionProvider (None)
        cx.update(|cx| {
            cx.update_global::<SettingsStore, _>(|store, cx| {
                store.update_user_settings(cx, |s| {
                    s.project
                        .all_languages
                        .edit_predictions
                        .get_or_insert(Default::default())
                        .provider = Some(EditPredictionProvider::None);
                });
            });
            update_command_palette_filter(cx);
        });

        cx.update(|cx| {
            let filter = CommandPaletteFilter::try_global(cx).unwrap();
            assert!(
                filter.is_hidden(&AcceptEditPrediction),
                "EditPrediction should be hidden when provider is None"
            );
        });
    }

    async fn setup_backfill_test(cx: &mut TestAppContext) -> Arc<dyn Fs> {
        let fs = fs::FakeFs::new(cx.background_executor.clone());
        fs.save(
            paths::settings_file().as_path(),
            &"{}".into(),
            Default::default(),
        )
        .await
        .unwrap();

        cx.update(|cx| {
            cx.set_global(db::AppDatabase::test_new());
            let store = SettingsStore::test(cx);
            cx.set_global(store);
            AgentSettings::register(cx);
            DisableAiSettings::register(cx);
            cx.set_staff(true);
        });

        fs
    }

    #[gpui::test]
    async fn test_backfill_sets_kvp_flag(cx: &mut TestAppContext) {
        let fs = setup_backfill_test(cx).await;

        cx.update(|cx| {
            let kvp = KeyValueStore::global(cx);
            assert!(
                kvp.read_kvp(PARALLEL_AGENT_LAYOUT_BACKFILL_KEY)
                    .unwrap()
                    .is_none()
            );

            maybe_backfill_editor_layout(fs.clone(), false, cx);
        });

        cx.run_until_parked();

        let kvp = cx.update(|cx| KeyValueStore::global(cx));
        assert!(
            kvp.read_kvp(PARALLEL_AGENT_LAYOUT_BACKFILL_KEY)
                .unwrap()
                .is_some(),
            "flag should be set after backfill"
        );
    }

    #[gpui::test]
    async fn test_backfill_new_install_sets_flag_without_writing_settings(cx: &mut TestAppContext) {
        let fs = setup_backfill_test(cx).await;

        cx.update(|cx| {
            maybe_backfill_editor_layout(fs.clone(), true, cx);
        });

        cx.run_until_parked();

        let kvp = cx.update(|cx| KeyValueStore::global(cx));
        assert!(
            kvp.read_kvp(PARALLEL_AGENT_LAYOUT_BACKFILL_KEY)
                .unwrap()
                .is_some(),
            "flag should be set even for new installs"
        );

        let written = fs.load(paths::settings_file().as_path()).await.unwrap();
        assert_eq!(written.trim(), "{}", "settings file should be unchanged");
    }

    #[gpui::test]
    async fn test_backfill_is_idempotent(cx: &mut TestAppContext) {
        let fs = setup_backfill_test(cx).await;

        cx.update(|cx| {
            maybe_backfill_editor_layout(fs.clone(), false, cx);
        });

        cx.run_until_parked();

        let after_first = fs.load(paths::settings_file().as_path()).await.unwrap();

        cx.update(|cx| {
            maybe_backfill_editor_layout(fs.clone(), false, cx);
        });

        cx.run_until_parked();

        let after_second = fs.load(paths::settings_file().as_path()).await.unwrap();
        assert_eq!(
            after_first, after_second,
            "second call should not change settings"
        );
    }

    #[test]
    fn test_deserialize_agent_variants() {
        assert_eq!(
            serde_json::from_str::<Agent>(r#""NativeAgent""#).unwrap(),
            Agent::NativeAgent,
        );
        assert_eq!(
            serde_json::from_str::<Agent>(r#""native_agent""#).unwrap(),
            Agent::NativeAgent,
        );
    }
}
