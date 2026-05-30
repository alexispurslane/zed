use std::fmt::Write;
use std::sync::Arc;

use agent_thread::schema;
use gpui::{App, Entity, SharedString, Task};
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::symbol_locator::{CodeActionStore, LineLocator, PendingCodeActions};
use crate::{AgentTool, ToolCallEventStream, ToolInput};

/// Gets the list of available code actions at a specific line in a file from
/// the language server.
///
/// Code actions include quick fixes, refactorings, and other automated transformations
/// suggested by the language server (e.g. "Add missing import", "Extract to function").
///
/// Returns a numbered list of available actions. Use apply_code_action with the
/// corresponding number to apply one.
///
/// You must specify the file path and the exact line number where you want
/// to see code actions.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct GetCodeActionsToolInput {
    /// The location to get code actions for.
    pub location: LineLocator,
}

pub struct GetCodeActionsTool {
    project: Entity<Project>,
    code_action_store: CodeActionStore,
}

impl GetCodeActionsTool {
    pub fn new(project: Entity<Project>, code_action_store: CodeActionStore) -> Self {
        Self {
            project,
            code_action_store,
        }
    }
}

impl AgentTool for GetCodeActionsTool {
    type Input = GetCodeActionsToolInput;
    type Output = String;

    const NAME: &'static str = "get_code_actions";

    fn kind() -> schema::ToolKind {
        schema::ToolKind::Search
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            format!(
                "Get code actions at {}:{}",
                input.location.file_path, input.location.line
            )
            .into()
        } else {
            "Get code actions".into()
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        _event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<String, String>> {
        let project = self.project.clone();
        let store = self.code_action_store.clone();
        cx.spawn(async move |cx| {
            let input = input
                .recv()
                .await
                .map_err(|e| format!("Failed to receive tool input: {e}"))?;

            let (buffer, position) = input.location.resolve(&project, cx).await?;

            let actions_task = project.update(cx, |project, cx| {
                let range = position..position;
                project.code_actions(&buffer, range, None, cx)
            });

            let actions = actions_task
                .await
                .map_err(|e| format!("Failed to get code actions: {e}"))?
                .unwrap_or_default();

            if actions.is_empty() {
                store.update(cx, |store, _cx| *store = None);
                return Ok(format!(
                    "No code actions available at {}:{}.",
                    input.location.file_path, input.location.line
                ));
            }

            let mut output = format!("Found {} code action(s):\n", actions.len());
            for (i, action) in actions.iter().enumerate() {
                writeln!(output, "{}. {}", i + 1, action.lsp_action.title()).ok();
            }
            write!(
                output,
                "\nUse apply_code_action with the number of the action you want to apply."
            )
            .ok();

            store.update(cx, |store, _cx| {
                *store = Some(PendingCodeActions {
                    actions,
                    buffer,
                });
            });

            Ok(output)
        })
    }
}
