use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;

use super::symbol_locator::{LocationDisplay, ResolvedSymbolWithChain, SymbolSearch, proximity_ord, file_path_from_location};
use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_thread::schema;
use gpui::{App, Entity, SharedString, Task};
use project::{LocationLink, Project};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const LSP_RETRY_ATTEMPTS: usize = 2;
const LSP_RETRY_DELAY: Duration = Duration::from_secs(2);

/// Jumps to the definition of a symbol using the language server.
///
/// Returns the file path and line number of the symbol's definition,
/// along with a snippet of the source code at that location.
///
/// Provide the file path and symbol name. The tool finds all occurrences
/// of the name in the file, queries the LSP at one position per unique
/// enclosing scope, and returns all deduplicated definitions.
/// If the symbol is ambiguous (same name in multiple scopes), you can
/// provide `enclosing_scope` to narrow the query to a specific scope.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct GoToDefinitionToolInput {
    /// The symbol to find the definition of.
    pub symbol: SymbolSearch,
}

pub struct GoToDefinitionTool {
    project: Entity<Project>,
}

impl GoToDefinitionTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for GoToDefinitionTool {
    type Input = GoToDefinitionToolInput;
    type Output = String;

    const NAME: &'static str = "go_to_definition";

    fn kind() -> schema::ToolKind {
        schema::ToolKind::Search
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            format!("Go to definition of `{}`", input.symbol.symbol_name).into()
        } else {
            "Go to definition".into()
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        _event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<String, String>> {
        let project = self.project.clone();
        cx.spawn(async move |cx| {
            let input = input
                .recv()
                .await
                .map_err(|e| format!("Failed to receive tool input: {e}"))?;

            let symbols = input.symbol.resolve_for_search(&project, cx).await?;

            // Query definitions at one position per unique scope chain,
            // with retries to allow the LSP time to finish analyzing
            // the file after it was opened.
            let mut all_definitions: Vec<LocationLink> = Vec::new();
            for attempt in 0..LSP_RETRY_ATTEMPTS {
                for ResolvedSymbolWithChain { resolved, .. } in &symbols {
                    let definitions_task = project.update(cx, |project, cx| {
                        project.definitions(&resolved.buffer, resolved.position, cx)
                    });

                    let definitions = definitions_task
                        .await
                        .map_err(|e| format!("Go to definition failed: {e}"))?
                        .unwrap_or_default();

                    all_definitions.extend(definitions);
                }

                if !all_definitions.is_empty()
                    || attempt + 1 == LSP_RETRY_ATTEMPTS
                {
                    break;
                }

                cx.background_executor()
                    .timer(LSP_RETRY_DELAY)
                    .await;
            }

            let mut definitions = deduplicate_definitions(all_definitions);
            let source_path = input.symbol.file_path.clone();
            definitions.sort_by(|a, b| {
                let path_a = file_path_from_location(&a.target, cx);
                let path_b = file_path_from_location(&b.target, cx);
                proximity_ord(&source_path, &path_a, &path_b)
            });

            if definitions.is_empty() {
                return Ok(format!(
                    "No definition found for '{}'. The language server may not have finished indexing this file yet — try again shortly.",
                    input.symbol.symbol_name
                ));
            }

            let mut output = String::new();

            if definitions.len() == 1 {
                write!(output, "Definition of `{}`:\n", input.symbol.symbol_name).ok();
            } else {
                write!(
                    output,
                    "Found {} definitions of `{}`:\n",
                    definitions.len(),
                    input.symbol.symbol_name
                )
                .ok();
            }

            for link in &definitions {
                let display = link
                    .target
                    .buffer
                    .read_with(cx, |_, cx| LocationDisplay::from_location(&link.target, cx));
                write!(output, "\n## {display}\n").ok();
            }

            Ok(output)
        })
    }
}

/// Deduplicate definition links by their target location (same buffer + same
/// range). Multiple occurrences of a symbol name in the source may all resolve
/// to the same definition, so we keep only one copy of each unique target.
fn deduplicate_definitions(definitions: Vec<LocationLink>) -> Vec<LocationLink> {
    let mut seen = std::collections::HashSet::new();
    definitions
        .into_iter()
        .filter(|link| {
            let buffer_id = link.target.buffer.entity_id();
            let start = link.target.range.start;
            let end = link.target.range.end;
            seen.insert((buffer_id, start, end))
        })
        .collect()
}
