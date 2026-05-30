use std::fmt::Write;
use std::sync::Arc;

use action_log::ActionLog;
use agent_thread::schema;
use gpui::{App, Entity, SharedString, Task};
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use util::ResultExt;

use super::symbol_locator::SymbolRename;
use crate::{AgentTool, ToolCallEventStream, ToolInput};

/// Renames a symbol across the project using the language server.
///
/// This performs a semantic rename, updating all references to the symbol
/// across all files in the project. The language server determines which
/// occurrences to rename based on the symbol's type and scope.
///
/// If the symbol name appears only once in the file (or in only one scope),
/// no `enclosing_scope` is needed. If it appears in multiple scopes, the
/// tool will list each scope with the line text at that occurrence — provide
/// `enclosing_scope` matching the scope you want to rename. The rename will
/// operate on the first matching symbol within the chosen scope.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct RenameToolInput {
    /// The symbol to rename.
    pub symbol: SymbolRename,

    /// The new name for the symbol.
    pub new_name: String,
}

pub struct RenameTool {
    project: Entity<Project>,
    action_log: Entity<ActionLog>,
}

impl RenameTool {
    pub fn new(project: Entity<Project>, action_log: Entity<ActionLog>) -> Self {
        Self { project, action_log }
    }
}

impl AgentTool for RenameTool {
    type Input = RenameToolInput;
    type Output = String;

    const NAME: &'static str = "rename_symbol";

    fn kind() -> schema::ToolKind {
        schema::ToolKind::Other
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            format!(
                "Rename `{}` to `{}`",
                input.symbol.symbol_name, input.new_name
            )
            .into()
        } else {
            "Rename symbol".into()
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        _event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<String, String>> {
        let project = self.project.clone();
        let action_log = self.action_log.clone();
        cx.spawn(async move |cx| {
            let input = input
                .recv()
                .await
                .map_err(|e| format!("Failed to receive tool input: {e}"))?;

            let resolved = input.symbol.resolve_for_rename(&project, cx).await?;

            let rename_task = project.update(cx, |project, cx| {
                project.perform_rename(
                    resolved.buffer.clone(),
                    resolved.position,
                    input.new_name.clone(),
                    cx,
                )
            });

            let transaction = rename_task
                .await
                .map_err(|e| format!("Rename failed: {e}"))?;

            if transaction.0.is_empty() {
                return Ok(format!(
                    "No changes were made. The language server could not rename '{}'.",
                    input.symbol.symbol_name
                ));
            }

            let mut output = format!(
                "Renamed `{}` to `{}` in {} file(s):\n",
                input.symbol.symbol_name,
                input.new_name,
                transaction.0.len()
            );

            for (buffer, _) in &transaction.0 {
                buffer.read_with(cx, |buffer, cx| {
                    let path = buffer
                        .file()
                        .map(|f| f.full_path(cx).display().to_string())
                        .unwrap_or_else(|| "<untitled>".to_string());
                    writeln!(output, "- {path}").ok();
                });

                // Save the buffer to disk so the rename is actually persisted.
                project
                    .update(cx, |project, cx| project.save_buffer(buffer.clone(), cx))
                    .await
                    .log_err();

                action_log.update(cx, |log, cx| {
                    log.buffer_edited(buffer.clone(), cx);
                });
            }

            Ok(output)
        })
    }
}
