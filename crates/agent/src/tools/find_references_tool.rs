use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;

use super::symbol_locator::{LocationDisplay, ResolvedSymbolWithChain, SymbolSearch, EXPANSION_THRESHOLD, proximity_ord, file_path_from_location};
use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_thread::schema;
use gpui::{App, Entity, SharedString, Task};
use language::Location;
use project::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const LSP_RETRY_ATTEMPTS: usize = 2;
const LSP_RETRY_DELAY: Duration = Duration::from_secs(2);

/// Finds all references to a symbol across the project using the language server.
///
/// When called without a filter_path, returns a summary listing each file that contains
/// references, the number of references in that file, and a total reference count.
/// When called with a filter_path, returns the full list of references within that file,
/// including line numbers and code snippets.
///
/// The tool finds all occurrences of the symbol name in the file, queries the
/// LSP at one position per unique enclosing scope, and deduplicates the results.
/// If the symbol is ambiguous (same name in multiple scopes), you can provide
/// `enclosing_scope` to narrow the query to a specific scope.
///
/// <example>
/// To get a summary of all references to a symbol:
/// {
/// "symbol": { "file_path": "crates/editor/src/editor.rs", "symbol_name": "Editor" }
/// }
///
/// To see detailed references within a specific file:
/// {
/// "symbol": { "file_path": "crates/editor/src/editor.rs", "symbol_name": "Editor" },
/// "filter_path": "crates/editor/src/editor.rs"
/// }
/// </example>
///
/// <guidelines>
/// When there are many references (e.g. more than 3 files worth), start by calling
/// this tool without a filter_path to see which files contain references and how many each
/// has. Then call again with the filter_path parameter set to individual files you want to
/// inspect in detail.
/// </guidelines>
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct FindReferencesToolInput {
    /// The symbol to find references of.
    pub symbol: SymbolSearch,

    /// An optional relative path to filter references by. When provided, only
    /// references in this file are shown with full details (line numbers and
    /// code snippets). When omitted, a summary grouped by file is returned.
    ///
    /// This path should never be absolute, and the first component
    /// of the path should always be a root directory in a project.
    ///
    /// <example>
    /// If the project has the following root directories:
    ///
    /// - lorem
    /// - ipsum
    ///
    /// If you want references for `dolor.rs` in `ipsum`, you should use the filter_path `ipsum/dolor.rs`.
    /// </example>
    pub filter_path: Option<String>,
}

pub struct FindReferencesTool {
    project: Entity<Project>,
}

impl FindReferencesTool {
    pub fn new(project: Entity<Project>) -> Self {
        Self { project }
    }
}

impl AgentTool for FindReferencesTool {
    type Input = FindReferencesToolInput;
    type Output = String;

    const NAME: &'static str = "find_references";

    fn kind() -> schema::ToolKind {
        schema::ToolKind::Search
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        if let Ok(input) = input {
            let suffix = match &input.filter_path {
                Some(path) => format!(" in {}", path),
                None => String::new(),
            };
            format!("Find references to `{}`{suffix}", input.symbol.symbol_name).into()
        } else {
            "Find references".into()
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

            eprintln!(
                "[find_references] resolved {} symbol(s) for '{}' in {}",
                symbols.len(),
                input.symbol.symbol_name,
                input.symbol.file_path,
            );

            // Query references at one position per unique scope chain,
            // with retries to allow the LSP time to finish analyzing
            // the file after it was opened.
            let mut all_references: Vec<Location> = Vec::new();
            for attempt in 0..LSP_RETRY_ATTEMPTS {
                for ResolvedSymbolWithChain { resolved, chain } in &symbols {
                    eprintln!(
                        "[find_references]   querying at line '{}' (chain: [{}]}})",
                        resolved.line_text,
                        chain.join(", "),
                    );

                    let references_task = project.update(cx, |project, cx| {
                        project.references(&resolved.buffer, resolved.position, cx)
                    });

                    let references = references_task
                        .await
                        .map_err(|e| format!("Find references failed: {e}"))?
                        .unwrap_or_default();

                    eprintln!(
                        "[find_references]   got {} reference(s)",
                        references.len(),
                    );

                    all_references.extend(references);
                }

                if !all_references.is_empty()
                    || attempt + 1 == LSP_RETRY_ATTEMPTS
                {
                    break;
                }

                cx.background_executor()
                    .timer(LSP_RETRY_DELAY)
                    .await;
            }

            let mut references = deduplicate_references(all_references);
            let source_path = input.symbol.file_path.clone();
            sort_by_proximity(&mut references, &source_path, cx);

            if references.is_empty() {
                return Ok(format!(
                    "No references found for '{}'. The language server may not have finished indexing this file yet — try again shortly.",
                    input.symbol.symbol_name
                ));
            }

            let filter_path = input.filter_path.as_deref().filter(|p| !p.is_empty());

            match filter_path {
                Some(filter_path) => {
                    let filter_path_owned = filter_path.to_string();
                    let (filtered_displays, count) = references[0]
                        .buffer
                        .read_with(cx, |_, cx| {
                            let filter_path = &filter_path_owned;
                            let filtered: Vec<_> = references
                                .iter()
                                .filter(|location| {
                                    location
                                        .buffer
                                        .read(cx)
                                        .file()
                                        .map(|f| {
                                            let full = f.full_path(cx).display().to_string();
                                            full == *filter_path
                                                || full.ends_with(&format!("/{filter_path}"))
                                        })
                                        .unwrap_or(false)
                                })
                                .collect();

                            let count = filtered.len();
                            let expand = count <= EXPANSION_THRESHOLD;
                            let displays: Vec<String> = filtered
                                .iter()
                                .map(|location| {
                                    let display = LocationDisplay::from_location_with_expansion(
                                        location, expand, cx,
                                    );
                                    format!("## {display}")
                                })
                                .collect();
                            (displays, count)
                        });

                    if count == 0 {
                        return Ok(format!(
                            "No references found in '{}' for the given symbol. The language server may not have finished indexing this file yet — try again shortly.",
                            filter_path
                        ));
                    }

                    let mut output = format!(
                        "Found {} reference(s) in `{}`:\n",
                        count, filter_path
                    );

                    for display in &filtered_displays {
                        write!(output, "\n{display}\n").ok();
                    }

                    Ok(output)
                }
                None => {
                    let total = references.len();

                    if total <= EXPANSION_THRESHOLD {
                        let mut output = format!(
                            "Found {} reference(s) to `{}`:\n",
                            total, input.symbol.symbol_name,
                        );

                        for location in &references {
                            let display = location.buffer.read_with(cx, |_, cx| {
                                LocationDisplay::from_location_with_expansion(location, true, cx)
                            });
                            write!(output, "\n## {display}\n").ok();
                        }

                        return Ok(output);
                    }

                    let path_counts: Vec<(String, usize)> = references
                        .iter()
                        .fold(HashMap::new(), |mut acc, location| {
                            let path = location.buffer.read_with(cx, |buffer, cx| {
                                buffer
                                    .file()
                                    .map(|f| f.full_path(cx).display().to_string())
                                    .unwrap_or_else(|| "<untitled>".to_string())
                            });
                            *acc.entry(path).or_default() += 1;
                            acc
                        })
                        .into_iter()
                        .collect();

                    let file_count = path_counts.len();

                    let mut entries = path_counts;
                    entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

                    let mut output = format!(
                        "Found {} references to `{}` across {} file(s):\n\n",
                        total, input.symbol.symbol_name, file_count,
                    );

                    for (path, count) in &entries {
                        writeln!(output, "- {path}: {count} reference(s)").ok();
                    }

                    write!(
                        output,
                        "\nCall find_references again with a `filter_path` parameter to see detailed references within a specific file."
                    )
                    .ok();

                    Ok(output)
                }
            }
        })
    }
}

/// Deduplicate reference locations by their buffer and range. When the
/// symbol name appears in multiple scopes, querying the LSP at each
/// may return overlapping results.
fn deduplicate_references(references: Vec<Location>) -> Vec<Location> {
    let mut seen = std::collections::HashSet::new();
    references
        .into_iter()
        .filter(|location| {
            let buffer_id = location.buffer.entity_id();
            let start = location.range.start;
            let end = location.range.end;
            seen.insert((buffer_id, start, end))
        })
        .collect()
}

/// Sort locations by proximity to `source_path`: references in the same
/// file come first, then the same directory, then directories that share
/// more path components with the source, then external files.
fn sort_by_proximity(locations: &mut [Location], source_path: &str, cx: &mut gpui::AsyncApp) {
    locations.sort_by(|a, b| {
        let path_a = file_path_from_location(a, cx);
        let path_b = file_path_from_location(b, cx);
        proximity_ord(source_path, &path_a, &path_b)
    });
}
