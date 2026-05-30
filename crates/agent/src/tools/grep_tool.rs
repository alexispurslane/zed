use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_thread::schema;
use anyhow::Result;
use futures::{FutureExt as _, StreamExt};
use gpui::{App, Entity, SharedString, Task};
use language::{Anchor, OffsetRangeExt, OutlineItem, ParseStatus, Point};
use project::{
    Project, SearchResults, WorktreeSettings,
    search::{SearchQuery, SearchResult},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings::Settings;
use std::{cmp, fmt::Write, sync::Arc};
use util::RangeExt;
use util::markdown::MarkdownInlineCode;
use util::paths::PathMatcher;

/// Searches the contents of files in the project with a regular expression.
///
/// When called without a `path` parameter:
/// - If there are 50 or fewer matches, returns all matches with full details (file paths, line
///   numbers, code snippets, and parent symbol chains).
/// - If there are more than 50 matches, returns a summary listing each file with a match, the
///   number of matches in that file, and a total match count. Call grep again with the `path`
///   parameter set to a specific file to see detailed results for that file.
/// When called with a `path` parameter, returns all matches within that file with full details.
///
/// - Prefer this tool to path search when searching for symbols in the project, because you won't need to guess what path it's in.
/// - Supports full regex syntax (eg. "log.*Error", "function\\s+\\w+", etc.)
/// - Pass an `include_pattern` if you know how to narrow your search on the files system
/// - Never use this tool to search for paths. Only search file contents with this tool.
/// - DO NOT use HTML entities solely to escape characters in the tool parameters.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct GrepToolInput {
    /// A regex pattern to search for in the entire project. Note that the regex will be parsed by the Rust `regex` crate.
    ///
    /// Do NOT specify a path here! This will only be matched against the code **content**.
    pub regex: String,
    /// A glob pattern for the paths of files to include in the search.
    /// Supports standard glob patterns like "**/*.rs" or "frontend/src/**/*.ts".
    /// If omitted, all files in the project will be searched.
    ///
    /// The glob pattern is matched against the full path including the project root directory.
    ///
    /// <example>
    /// If the project has the following root directories:
    ///
    /// - /a/b/backend
    /// - /c/d/frontend
    ///
    /// Use "backend/**/*.rs" to search only Rust files in the backend root directory.
    /// Use "frontend/src/**/*.ts" to search TypeScript files only in the frontend root directory (sub-directory "src").
    /// Use "**/*.rs" to search Rust files across all root directories.
    /// </example>
    pub include_pattern: Option<String>,
    /// An optional relative path to filter matches by. When provided, only matches
    /// in the specified file are shown with full details. When omitted, all matches
    /// are returned (as a summary if there are many, or in full if there are few).
    ///
    /// This path should never be absolute, and the first component of the path should
    /// always be a root directory in a project. Use this to drill into the file-level
    /// details after seeing the summary from a previous grep call.
    ///
    /// <example>
    /// If the project has the following root directories:
    ///
    /// - lorem
    /// - ipsum
    ///
    /// If you want matches for `dolor.rs` in `ipsum`, you should use the path `ipsum/dolor.rs`.
    /// </example>
    pub path: Option<String>,
    /// Optional starting position for paginated results (0-based).
    /// When not provided, starts from the beginning.
    #[serde(default)]
    pub offset: u32,
    /// Whether the regex is case-sensitive. Defaults to false (case-insensitive).
    #[serde(default)]
    pub case_sensitive: bool,
}

impl GrepToolInput {
    /// Which page of search results this is.
    pub fn page(&self) -> u32 {
        1 + (self.offset / RESULTS_PER_PAGE)
    }

    fn is_same_query(&self, other: &Self) -> bool {
        self.regex == other.regex
            && self.case_sensitive == other.case_sensitive
            && self.include_pattern == other.include_pattern
    }
}

const RESULTS_PER_PAGE: u32 = 20;
const SUMMARY_THRESHOLD: usize = 50;

const CONTEXT_LINES: u32 = 2;
const MAX_ANCESTOR_LINES: u32 = 10;
const MAX_LINE_DISPLAY_LEN: usize = 200;

fn truncate_long_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if line.len() > MAX_LINE_DISPLAY_LEN {
            let truncated: String = line.chars().take(MAX_LINE_DISPLAY_LEN).collect();
            out.push_str(&truncated);
            out.push('…');
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    // Remove the trailing newline added by the loop if the original
    // text did not end with one.
    if !text.ends_with('\n') {
        out.pop();
    }
    out
}

pub type GrepResultStore = Entity<Option<PendingGrepResults>>;

#[derive(Clone)]
pub struct PendingGrepResults {
    pub query_input: GrepToolInput,
    pub file_results: Vec<FileGrepResults>,
    pub total_matches: usize,
}

impl PendingGrepResults {
    fn is_empty(&self) -> bool {
        self.total_matches == 0
    }
}

#[derive(Clone)]
pub struct FileGrepResults {
    pub path: String,
    pub buffer: Entity<language::Buffer>,
    pub match_ranges: Vec<MatchRange>,
}

#[derive(Clone)]
pub struct MatchRange {
    pub display_range: std::ops::Range<Point>,
    pub ancestor_range: Option<std::ops::Range<Point>>,
    pub parent_symbols: Vec<OutlineItem<Anchor>>,
}

pub struct GrepTool {
    project: Entity<Project>,
    result_store: GrepResultStore,
}

impl GrepTool {
    pub fn new(project: Entity<Project>, result_store: GrepResultStore) -> Self {
        Self {
            project,
            result_store,
        }
    }
}

impl AgentTool for GrepTool {
    type Input = GrepToolInput;
    type Output = String;

    const NAME: &'static str = "grep";

    fn kind() -> schema::ToolKind {
        schema::ToolKind::Search
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input {
            Ok(input) => {
                let page = input.page();
                let regex_str = MarkdownInlineCode(&input.regex);
                let case_info = if input.case_sensitive {
                    " (case-sensitive)"
                } else {
                    ""
                };

                if let Some(path) = &input.path {
                    format!(
                        "Search {} for regex {regex_str}{case_info}",
                        MarkdownInlineCode(path)
                    )
                } else if page > 1 {
                    format!("Get page {page} of search results for regex {regex_str}{case_info}")
                } else {
                    format!("Search files for regex {regex_str}{case_info}")
                }
            }
            Err(_) => "Search with regex".into(),
        }
        .into()
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        let project = self.project.clone();
        let result_store = self.result_store.clone();
        cx.spawn(async move |cx| {
            let input = input
                .recv()
                .await
                .map_err(|e| e.to_string())?;

            let filter_path = input.path.as_deref().filter(|p| !p.is_empty());

            // If a filter path is provided, try to use cached results from a previous summary call.
            if let Some(filter_path) = filter_path {
                let cached = result_store.read_with(cx, |store, _| store.clone());
                if let Some(pending) = cached {
                    if pending.query_input.is_same_query(&input) {
                        return render_file_detail(&pending, filter_path, cx);
                    }
                }
                // No matching cache; fall through to re-run the search with an include_pattern filter.
                return run_search_and_render_file(
                    project,
                    &input,
                    filter_path,
                    event_stream,
                    result_store,
                    cx,
                )
                .await;
            }

            // No filter path: run the full search and decide summary vs. detail.
            run_search_and_render(
                project,
                &input,
                event_stream,
                result_store,
                cx,
            )
            .await
        })
    }
}

async fn run_search_and_render(
    project: Entity<Project>,
    input: &GrepToolInput,
    event_stream: ToolCallEventStream,
    result_store: GrepResultStore,
    cx: &mut gpui::AsyncApp,
) -> Result<String, String> {
    let collected = collect_search_results(project, input, event_stream, cx).await?;

    if collected.is_empty() {
        return Ok("No matches found".into());
    }

    if collected.total_matches > SUMMARY_THRESHOLD {
        // Store results for drill-in, then show summary.
        result_store.update(cx, |store, _| {
            *store = Some(collected.clone());
        });

        let mut file_counts: Vec<(String, usize)> = collected
            .file_results
            .iter()
            .map(|fr| (fr.path.clone(), fr.match_ranges.len()))
            .collect();
        file_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let mut output = format!(
            "Found {} matches across {} file(s):\n\n",
            collected.total_matches,
            file_counts.len(),
        );

        for (path, count) in &file_counts {
            writeln!(output, "- {path}: {count} match(es)").ok();
        }

        write!(
            output,
            "\nCall grep again with a `path` parameter set to one of the files above to see detailed match results within that file."
        )
        .ok();

        Ok(output)
    } else {
        // Few enough results to show in full.
        render_full_detail(&collected, input, cx)
    }
}

async fn run_search_and_render_file(
    project: Entity<Project>,
    input: &GrepToolInput,
    filter_path: &str,
    event_stream: ToolCallEventStream,
    result_store: GrepResultStore,
    cx: &mut gpui::AsyncApp,
) -> Result<String, String> {
    let collected = collect_search_results(project, input, event_stream, cx).await?;

    if collected.is_empty() {
        return Ok(format!("No matches found in '{filter_path}'."));
    }

    // Store results in case the agent wants to drill into another file.
    result_store.update(cx, |store, _| {
        *store = Some(collected.clone());
    });

    render_file_detail(&collected, filter_path, cx)
}

async fn collect_search_results(
    project: Entity<Project>,
    input: &GrepToolInput,
    event_stream: ToolCallEventStream,
    cx: &mut gpui::AsyncApp,
) -> Result<PendingGrepResults, String> {
    let results = cx.update(|cx| {
        let path_style = project.read(cx).path_style(cx);

        let include_matcher = PathMatcher::new(
            input
                .include_pattern
                .as_ref()
                .into_iter()
                .collect::<Vec<_>>(),
            path_style,
        )
        .map_err(|error| format!("invalid include glob pattern: {error}"))?;

        let exclude_matcher = {
            let global_settings = WorktreeSettings::get_global(cx);
            let exclude_patterns = global_settings
                .file_scan_exclusions
                .sources()
                .chain(global_settings.private_files.sources());

            PathMatcher::new(exclude_patterns, path_style)
                .map_err(|error| format!("invalid exclude pattern: {error}"))?
        };

        let query = SearchQuery::regex(
            &input.regex,
            false,
            input.case_sensitive,
            false,
            false,
            include_matcher,
            exclude_matcher,
            true,
            None,
        )
        .map_err(|error| error.to_string())?;

        Ok::<_, String>(
            project.update(cx, |project, cx| project.search(query, cx)),
        )
    })?;

    let project_weak = project.downgrade();
    let SearchResults {
        rx,
        _task_handle,
    } = results;
    futures::pin_mut!(rx);

    let mut file_results: Vec<FileGrepResults> = Vec::new();
    let mut total_matches: usize = 0;

    loop {
        let search_result = futures::select! {
            result = rx.next().fuse() => result,
            _ = event_stream.cancelled_by_user().fuse() => {
                return Err("Search cancelled by user".to_string());
            }
        };

        match search_result {
            Some(SearchResult::Buffer { buffer, ranges }) => {
                if ranges.is_empty() {
                    continue;
                }

                let Some(path) = buffer.read_with(cx, |buffer, cx| {
                    buffer.file().map(|file| file.full_path(cx).display().to_string())
                }) else {
                    continue;
                };

                // Check worktree-level exclusions
                if let Ok(Some(project_path)) = project_weak.read_with(cx, |project, cx| {
                    project.find_project_path(&path, cx)
                }) {
                    if cx.update(|cx| {
                        let worktree_settings = WorktreeSettings::get(Some((&project_path).into()), cx);
                        worktree_settings.is_path_excluded(&project_path.path)
                            || worktree_settings.is_path_private(&project_path.path)
                    }) {
                        continue;
                    }
                }

                // Wait for parsing to finish so syntax info is available.
                let mut parse_status = buffer.read_with(cx, |buffer, _cx| buffer.parse_status());
                while *parse_status.borrow() != ParseStatus::Idle {
                    parse_status.changed().await.map_err(|e| e.to_string())?;
                }

                let snapshot = buffer.read_with(cx, |buffer, _cx| buffer.snapshot());

                let match_ranges: Vec<MatchRange> = ranges
                    .into_iter()
                    .map(|range| {
                        let matched = range.to_point(&snapshot);
                        let matched_end_line_len = snapshot.line_len(matched.end.row);
                        let full_lines = Point::new(matched.start.row, 0)
                            ..Point::new(matched.end.row, matched_end_line_len);
                        let symbols = snapshot.symbols_containing(matched.start, None);

                        if let Some(ancestor_node) = snapshot.syntax_ancestor(full_lines.clone()) {
                            let full_ancestor_range =
                                ancestor_node.byte_range().to_point(&snapshot);
                            let end_row = full_ancestor_range
                                .end
                                .row
                                .min(full_ancestor_range.start.row + MAX_ANCESTOR_LINES);
                            let end_col = snapshot.line_len(end_row);
                            let capped_ancestor_range = Point::new(
                                full_ancestor_range.start.row,
                                0,
                            )..Point::new(end_row, end_col);

                            if capped_ancestor_range.contains_inclusive(&full_lines) {
                                return MatchRange {
                                    display_range: capped_ancestor_range,
                                    ancestor_range: Some(full_ancestor_range),
                                    parent_symbols: symbols,
                                };
                            }
                        }

                        let mut matched = matched;
                        matched.start.column = 0;
                        matched.start.row =
                            matched.start.row.saturating_sub(CONTEXT_LINES);
                        matched.end.row = cmp::min(
                            snapshot.max_point().row,
                            matched.end.row + CONTEXT_LINES,
                        );
                        matched.end.column = snapshot.line_len(matched.end.row);

                        MatchRange {
                            display_range: matched,
                            ancestor_range: None,
                            parent_symbols: symbols,
                        }
                    })
                    .collect();

                total_matches += match_ranges.len();
                file_results.push(FileGrepResults {
                    path,
                    buffer,
                    match_ranges,
                });
            }
            Some(SearchResult::LimitReached) => break,
            Some(SearchResult::WaitingForScan | SearchResult::Searching) => continue,
            None => break,
        }
    }

    Ok(PendingGrepResults {
        query_input: input.clone(),
        file_results,
        total_matches,
    })
}

fn render_full_detail(
    results: &PendingGrepResults,
    _input: &GrepToolInput,
    cx: &gpui::AsyncApp,
) -> Result<String, String> {
    let mut output = String::new();
    let mut matches_found = 0;

    for file_result in &results.file_results {
        let snapshot = file_result.buffer.read_with(cx, |buffer, _cx| buffer.snapshot());

        let mut file_header_written = false;
        let mut ranges = file_result.match_ranges.iter().peekable();

        while let Some(match_range) = ranges.next() {
            // Merge overlapping/adjacent ranges for display.
            let mut display_range = match_range.display_range.clone();
            let ancestor_range = match_range.ancestor_range.clone();
            let parent_symbols = match_range.parent_symbols.clone();

            while let Some(next) = ranges.peek() {
                if display_range.end.row >= next.display_range.start.row {
                    display_range.end = next.display_range.end.clone();
                    ranges.next();
                } else {
                    break;
                }
            }

            if !file_header_written {
                writeln!(output, "\n## Matches in {}", file_result.path).ok();
                file_header_written = true;
            }

            let end_row = display_range.end.row;
            output.push_str("\n### ");

            for symbol in &parent_symbols {
                write!(output, "{} › ", symbol.text).ok();
            }

            if display_range.start.row == end_row {
                writeln!(output, "L{}", display_range.start.row + 1).ok();
            } else {
                writeln!(
                    output,
                    "L{}-{}",
                    display_range.start.row + 1,
                    end_row + 1
                )
                .ok();
            }

            output.push_str("```\n");
            let raw_text: String = snapshot.text_for_range(display_range.clone()).collect();
            output.push_str(&truncate_long_lines(&raw_text));
            output.push_str("\n```\n");

            if let Some(ancestor_range) = ancestor_range && end_row < ancestor_range.end.row {
                let remaining_lines = ancestor_range.end.row - end_row;
                writeln!(
                    output,
                    "\n{} lines remaining in ancestor node. Read the file to see all.",
                    remaining_lines
                )
                .ok();
            }

            matches_found += 1;
        }
    }

    let header = if matches_found == 0 {
        return Ok("No matches found".into());
    } else {
        format!("Found {matches_found} matches:")
    };

    Ok(format!("{header}\n{output}"))
}

fn render_file_detail(
    results: &PendingGrepResults,
    filter_path: &str,
    cx: &gpui::AsyncApp,
) -> Result<String, String> {
    let mut output = String::new();
    let mut matches_found = 0;

    for file_result in &results.file_results {
        if file_result.path != filter_path
            && !file_result.path.ends_with(&format!("/{filter_path}"))
        {
            continue;
        }

        let snapshot = file_result.buffer.read_with(cx, |buffer, _cx| buffer.snapshot());

        let mut file_header_written = false;
        let mut ranges = file_result.match_ranges.iter().peekable();

        while let Some(match_range) = ranges.next() {
            let mut display_range = match_range.display_range.clone();
            let ancestor_range = match_range.ancestor_range.clone();
            let parent_symbols = match_range.parent_symbols.clone();

            while let Some(next) = ranges.peek() {
                if display_range.end.row >= next.display_range.start.row {
                    display_range.end = next.display_range.end.clone();
                    ranges.next();
                } else {
                    break;
                }
            }

            if !file_header_written {
                writeln!(output, "\n## Matches in {}", file_result.path).ok();
                file_header_written = true;
            }

            let end_row = display_range.end.row;
            output.push_str("\n### ");

            for symbol in &parent_symbols {
                write!(output, "{} › ", symbol.text).ok();
            }

            if display_range.start.row == end_row {
                writeln!(output, "L{}", display_range.start.row + 1).ok();
            } else {
                writeln!(
                    output,
                    "L{}-{}",
                    display_range.start.row + 1,
                    end_row + 1
                )
                .ok();
            }

            output.push_str("```\n");
            let raw_text: String = snapshot.text_for_range(display_range.clone()).collect();
            output.push_str(&truncate_long_lines(&raw_text));
            output.push_str("\n```\n");

            if let Some(ancestor_range) = ancestor_range && end_row < ancestor_range.end.row {
                let remaining_lines = ancestor_range.end.row - end_row;
                writeln!(
                    output,
                    "\n{} lines remaining in ancestor node. Read the file to see all.",
                    remaining_lines
                )
                .ok();
            }

            matches_found += 1;
        }
    }

    if matches_found == 0 {
        Ok(format!("No matches found in '{filter_path}'."))
    } else {
        Ok(format!(
            "Found {matches_found} match(es) in `{filter_path}`:\n{output}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::ToolCallEventStream;

    use super::*;
    use gpui::{TestAppContext, UpdateGlobal};
    use project::{FakeFs, Project};
    use serde_json::json;
    use settings::SettingsStore;
    use unindent::Unindent;
    use util::path;

    #[gpui::test]
    async fn test_grep_tool_with_include_pattern(cx: &mut TestAppContext) {
        init_test(cx);
        cx.executor().allow_parking();

        let fs = FakeFs::new(cx.executor());
        fs.insert_tree(
            path!("/root"),
            serde_json::json!({
                "src": {
                    "main.rs": "fn main() {\n    println!(\"Hello, world!\");\n}",
                    "utils": {
                        "helper.rs": "fn helper() {\n    println!(\"I'm a helper!\");\n}",
                    },
                },
                "tests": {
                    "test_main.rs": "fn test_main() {\n    assert!(true);\n}",
                }
            }),
        )
        .await;

        let project = Project::test(fs.clone(), [path!("/root").as_ref()], cx).await;

        // Test with include pattern for Rust files inside the root of the project
        let input = GrepToolInput {
            regex: "println".to_string(),
            include_pattern: Some("root/**/*.rs".to_string()),
            path: None,
            offset: 0,
            case_sensitive: false,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        assert!(result.contains("main.rs"), "Should find matches in main.rs");
        assert!(
            result.contains("helper.rs"),
            "Should find matches in helper.rs"
        );
        assert!(
            !result.contains("test_main.rs"),
            "Should not include test_main.rs even though it's a .rs file (because it doesn't have the pattern)"
        );

        // Test with include pattern for src directory only
        let input = GrepToolInput {
            regex: "fn".to_string(),
            include_pattern: Some("root/**/src/**".to_string()),
            path: None,
            offset: 0,
            case_sensitive: false,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        assert!(
            result.contains("main.rs"),
            "Should find matches in src/main.rs"
        );
        assert!(
            result.contains("helper.rs"),
            "Should find matches in src/utils/helper.rs"
        );
        assert!(
            !result.contains("test_main.rs"),
            "Should not include test_main.rs as it's not in src directory"
        );

        // Test with empty include pattern (should default to all files)
        let input = GrepToolInput {
            regex: "fn".to_string(),
            include_pattern: None,
            path: None,
            offset: 0,
            case_sensitive: false,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        assert!(result.contains("main.rs"), "Should find matches in main.rs");
        assert!(
            result.contains("helper.rs"),
            "Should find matches in helper.rs"
        );
        assert!(
            result.contains("test_main.rs"),
            "Should include test_main.rs"
        );
    }

    #[gpui::test]
    async fn test_grep_tool_with_case_sensitivity(cx: &mut TestAppContext) {
        init_test(cx);
        cx.executor().allow_parking();

        let fs = FakeFs::new(cx.executor());
        fs.insert_tree(
            path!("/root"),
            serde_json::json!({
                "case_test.txt": "This file has UPPERCASE and lowercase text.\nUPPERCASE patterns should match only with case_sensitive: true",
            }),
        )
        .await;

        let project = Project::test(fs.clone(), [path!("/root").as_ref()], cx).await;

        // Test case-insensitive search (default)
        let input = GrepToolInput {
            regex: "uppercase".to_string(),
            include_pattern: Some("**/*.txt".to_string()),
            path: None,
            offset: 0,
            case_sensitive: false,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        assert!(
            result.contains("UPPERCASE"),
            "Case-insensitive search should match uppercase"
        );

        // Test case-sensitive search
        let input = GrepToolInput {
            regex: "uppercase".to_string(),
            include_pattern: Some("**/*.txt".to_string()),
            path: None,
            offset: 0,
            case_sensitive: true,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        assert!(
            !result.contains("UPPERCASE"),
            "Case-sensitive search should not match uppercase"
        );

        // Test case-sensitive search
        let input = GrepToolInput {
            regex: "LOWERCASE".to_string(),
            include_pattern: Some("**/*.txt".to_string()),
            path: None,
            offset: 0,
            case_sensitive: true,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;

        assert!(
            !result.contains("lowercase"),
            "Case-sensitive search should match lowercase"
        );

        // Test case-sensitive search for lowercase pattern
        let input = GrepToolInput {
            regex: "lowercase".to_string(),
            include_pattern: Some("**/*.txt".to_string()),
            path: None,
            offset: 0,
            case_sensitive: true,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        assert!(
            result.contains("lowercase"),
            "Case-sensitive search should match lowercase text"
        );
    }

    /// Helper function to set up a syntax test environment
    async fn setup_syntax_test(cx: &mut TestAppContext) -> Entity<Project> {
        use unindent::Unindent;
        init_test(cx);
        cx.executor().allow_parking();

        let fs = FakeFs::new(cx.executor());

        // Create test file with syntax structures
        fs.insert_tree(
            path!("/root"),
            serde_json::json!({
                "test_syntax.rs": r#"
                    fn top_level_function() {
                        println!("This is at the top level");
                    }

                    mod feature_module {
                        pub mod nested_module {
                            pub fn nested_function(
                                first_arg: String,
                                second_arg: i32,
                            ) {
                                println!("Function in nested module");
                                println!("{first_arg}");
                                println!("{second_arg}");
                            }
                        }
                    }

                    struct MyStruct {
                        field1: String,
                        field2: i32,
                    }

                    impl MyStruct {
                        fn method_with_block() {
                            let condition = true;
                            if condition {
                                println!("Inside if block");
                            }
                        }

                        fn long_function() {
                            println!("Line 1");
                            println!("Line 2");
                            println!("Line 3");
                            println!("Line 4");
                            println!("Line 5");
                            println!("Line 6");
                            println!("Line 7");
                            println!("Line 8");
                            println!("Line 9");
                            println!("Line 10");
                            println!("Line 11");
                            println!("Line 12");
                        }
                    }

                    trait Processor {
                        fn process(&self, input: &str) -> String;
                    }

                    impl Processor for MyStruct {
                        fn process(&self, input: &str) -> String {
                            format!("Processed: {}", input)
                        }
                    }
                "#.unindent().trim(),
            }),
        )
        .await;

        let project = Project::test(fs.clone(), [path!("/root").as_ref()], cx).await;

        project.update(cx, |project, _cx| {
            project.languages().add(language::rust_lang())
        });

        project
    }

    #[gpui::test]
    async fn test_grep_top_level_function(cx: &mut TestAppContext) {
        let project = setup_syntax_test(cx).await;

        let input = GrepToolInput {
            regex: "This is at the top level".to_string(),
            include_pattern: Some("**/*.rs".to_string()),
            path: None,
            offset: 0,
            case_sensitive: false,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        let expected = r#"
            Found 1 matches:

            ## Matches in root/test_syntax.rs

            ### fn top_level_function › L1-3
            ```
            fn top_level_function() {
                println!("This is at the top level");
            }
            ```
            "#
        .unindent();
        assert_eq!(result, expected);
    }

    #[gpui::test]
    async fn test_grep_function_body(cx: &mut TestAppContext) {
        let project = setup_syntax_test(cx).await;

        let input = GrepToolInput {
            regex: "Function in nested module".to_string(),
            include_pattern: Some("**/*.rs".to_string()),
            path: None,
            offset: 0,
            case_sensitive: false,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        let expected = r#"
            Found 1 matches:

            ## Matches in root/test_syntax.rs

            ### mod feature_module › pub mod nested_module › pub fn nested_function › L10-14
            ```
                    ) {
                        println!("Function in nested module");
                        println!("{first_arg}");
                        println!("{second_arg}");
                    }
            ```
            "#
        .unindent();
        assert_eq!(result, expected);
    }

    #[gpui::test]
    async fn test_grep_function_args_and_body(cx: &mut TestAppContext) {
        let project = setup_syntax_test(cx).await;

        let input = GrepToolInput {
            regex: "second_arg".to_string(),
            include_pattern: Some("**/*.rs".to_string()),
            path: None,
            offset: 0,
            case_sensitive: false,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        let expected = r#"
            Found 1 matches:

            ## Matches in root/test_syntax.rs

            ### mod feature_module › pub mod nested_module › pub fn nested_function › L7-14
            ```
                    pub fn nested_function(
                        first_arg: String,
                        second_arg: i32,
                    ) {
                        println!("Function in nested module");
                        println!("{first_arg}");
                        println!("{second_arg}");
                    }
            ```
            "#
        .unindent();
        assert_eq!(result, expected);
    }

    #[gpui::test]
    async fn test_grep_if_block(cx: &mut TestAppContext) {
        use unindent::Unindent;
        let project = setup_syntax_test(cx).await;

        let input = GrepToolInput {
            regex: "Inside if block".to_string(),
            include_pattern: Some("**/*.rs".to_string()),
            path: None,
            offset: 0,
            case_sensitive: false,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        let expected = r#"
            Found 1 matches:

            ## Matches in root/test_syntax.rs

            ### impl MyStruct › fn method_with_block › L26-28
            ```
                    if condition {
                        println!("Inside if block");
                    }
            ```
            "#
        .unindent();
        assert_eq!(result, expected);
    }

    #[gpui::test]
    async fn test_grep_long_function_top(cx: &mut TestAppContext) {
        use unindent::Unindent;
        let project = setup_syntax_test(cx).await;

        let input = GrepToolInput {
            regex: "Line 5".to_string(),
            include_pattern: Some("**/*.rs".to_string()),
            path: None,
            offset: 0,
            case_sensitive: false,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        let expected = r#"
            Found 1 matches:

            ## Matches in root/test_syntax.rs

            ### impl MyStruct › fn long_function › L31-41
            ```
                fn long_function() {
                    println!("Line 1");
                    println!("Line 2");
                    println!("Line 3");
                    println!("Line 4");
                    println!("Line 5");
                    println!("Line 6");
                    println!("Line 7");
                    println!("Line 8");
                    println!("Line 9");
                    println!("Line 10");
            ```

            3 lines remaining in ancestor node. Read the file to see all.
            "#
        .unindent();
        assert_eq!(result, expected);
    }

    #[gpui::test]
    async fn test_grep_long_function_bottom(cx: &mut TestAppContext) {
        use unindent::Unindent;
        let project = setup_syntax_test(cx).await;

        let input = GrepToolInput {
            regex: "Line 12".to_string(),
            include_pattern: Some("**/*.rs".to_string()),
            path: None,
            offset: 0,
            case_sensitive: false,
        };

        let result = run_grep_tool(input, project.clone(), cx).await;
        let expected = r#"
            Found 1 matches:

            ## Matches in root/test_syntax.rs

            ### impl MyStruct › fn long_function › L41-45
            ```
                    println!("Line 10");
                    println!("Line 11");
                    println!("Line 12");
                }
            }
            ```
            "#
        .unindent();
        assert_eq!(result, expected);
    }

    async fn run_grep_tool(
        input: GrepToolInput,
        project: Entity<Project>,
        cx: &mut TestAppContext,
    ) -> String {
        let result_store: GrepResultStore = cx.new(|_cx| None);
        let tool = Arc::new(GrepTool {
            project,
            result_store,
        });
        let task = cx.update(|cx| {
            tool.run(
                ToolInput::resolved(input),
                ToolCallEventStream::test().0,
                cx,
            )
        });

        match task.await {
            Ok(result) => {
                if false {
                    result.replace("root\\", "root/")
                } else {
                    result
                }
            }
            Err(e) => panic!("Failed to run grep tool: {}", e),
        }
    }

    fn init_test(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let settings_store = SettingsStore::test(cx);
            cx.set_global(settings_store);
        });
    }

    #[gpui::test]
    async fn test_grep_security_boundaries(cx: &mut TestAppContext) {
        init_test(cx);

        let fs = FakeFs::new(cx.executor());

        fs.insert_tree(
            path!("/"),
            json!({
                "project_root": {
                    "allowed_file.rs": "fn main() { println!(\"This file is in the project\"); }",
                    ".mysecrets": "SECRET_KEY=abc123\nfn secret() { /* private */ }",
                    ".secretdir": {
                        "config": "fn special_configuration() { /* excluded */ }"
                    },
                    ".mymetadata": "fn custom_metadata() { /* excluded */ }",
                    "subdir": {
                        "normal_file.rs": "fn normal_file_content() { /* Normal */ }",
                        "special.privatekey": "fn private_key_content() { /* private */ }",
                        "data.mysensitive": "fn sensitive_data() { /* private */ }"
                    }
                },
                "outside_project": {
                    "sensitive_file.rs": "fn outside_function() { /* This file is outside the project */ }"
                }
            }),
        )
        .await;

        cx.update(|cx| {
            use gpui::UpdateGlobal;
            use settings::SettingsStore;
            SettingsStore::update_global(cx, |store, cx| {
                store.update_user_settings(cx, |settings| {
                    settings.project.worktree.file_scan_exclusions = Some(vec![
                        "**/.secretdir".to_string(),
                        "**/.mymetadata".to_string(),
                    ]);
                    settings.project.worktree.private_files = Some(
                        vec![
                            "**/.mysecrets".to_string(),
                            "**/*.privatekey".to_string(),
                            "**/*.mysensitive".to_string(),
                        ]
                        .into(),
                    );
                });
            });
        });

        let project = Project::test(fs.clone(), [path!("/project_root").as_ref()], cx).await;

        // Searching for files outside the project worktree should return no results
        let result = run_grep_tool(
            GrepToolInput {
                regex: "outside_function".to_string(),
                include_pattern: None,
                path: None,
                offset: 0,
                case_sensitive: false,
            },
            project.clone(),
            cx,
        )
        .await;
        let paths = extract_paths_from_results(&result);
        assert!(
            paths.is_empty(),
            "grep_tool should not find files outside the project worktree"
        );

        // Searching within the project should succeed
        let result = run_grep_tool(
            GrepToolInput {
                regex: "main".to_string(),
                include_pattern: None,
                path: None,
                offset: 0,
                case_sensitive: false,
            },
            project.clone(),
            cx,
        )
        .await;
        let paths = extract_paths_from_results(&result);
        assert!(
            paths.iter().any(|p| p.contains("allowed_file.rs")),
            "grep_tool should be able to search files inside worktrees"
        );

        // Searching files that match file_scan_exclusions should return no results
        let result = run_grep_tool(
            GrepToolInput {
                regex: "special_configuration".to_string(),
                include_pattern: None,
                path: None,
                offset: 0,
                case_sensitive: false,
            },
            project.clone(),
            cx,
        )
        .await;
        let paths = extract_paths_from_results(&result);
        assert!(
            paths.is_empty(),
            "grep_tool should not search files in .secretdir (file_scan_exclusions)"
        );

        let result = run_grep_tool(
            GrepToolInput {
                regex: "custom_metadata".to_string(),
                include_pattern: None,
                path: None,
                offset: 0,
                case_sensitive: false,
            },
            project.clone(),
            cx,
        )
        .await;
        let paths = extract_paths_from_results(&result);
        assert!(
            paths.is_empty(),
            "grep_tool should not search .mymetadata files (file_scan_exclusions)"
        );

        // Searching private files should return no results
        let result = run_grep_tool(
            GrepToolInput {
                regex: "SECRET_KEY".to_string(),
                include_pattern: None,
                path: None,
                offset: 0,
                case_sensitive: false,
            },
            project.clone(),
            cx,
        )
        .await;
        let paths = extract_paths_from_results(&result);
        assert!(
            paths.is_empty(),
            "grep_tool should not search .mysecrets (private_files)"
        );

        let result = run_grep_tool(
            GrepToolInput {
                regex: "private_key_content".to_string(),
                include_pattern: None,
                path: None,
                offset: 0,
                case_sensitive: false,
            },
            project.clone(),
            cx,
        )
        .await;
        let paths = extract_paths_from_results(&result);

        assert!(
            paths.is_empty(),
            "grep_tool should not search .privatekey files (private_files)"
        );

        let result = run_grep_tool(
            GrepToolInput {
                regex: "sensitive_data".to_string(),
                include_pattern: None,
                path: None,
                offset: 0,
                case_sensitive: false,
            },
            project.clone(),
            cx,
        )
        .await;
        let paths = extract_paths_from_results(&result);
        assert!(
            paths.is_empty(),
            "grep_tool should not search .mysensitive files (private_files)"
        );

        // Searching a normal file should still work, even with private_files configured
        let result = run_grep_tool(
            GrepToolInput {
                regex: "normal_file_content".to_string(),
                include_pattern: None,
                path: None,
                offset: 0,
                case_sensitive: false,
            },
            project.clone(),
            cx,
        )
        .await;
        let paths = extract_paths_from_results(&result);
        assert!(
            paths.iter().any(|p| p.contains("normal_file.rs")),
            "Should be able to search normal files"
        );

        // Path traversal attempts with .. in include_pattern should not escape project
        let result = run_grep_tool(
            GrepToolInput {
                regex: "outside_function".to_string(),
                include_pattern: Some("../outside_project/**/*.rs".to_string()),
                path: None,
                offset: 0,
                case_sensitive: false,
            },
            project.clone(),
            cx,
        )
        .await;
        let paths = extract_paths_from_results(&result);
        assert!(
            paths.is_empty(),
            "grep_tool should not allow escaping project boundaries with relative paths"
        );
    }

    #[gpui::test]
    async fn test_grep_with_multiple_worktree_settings(cx: &mut TestAppContext) {
        init_test(cx);

        let fs = FakeFs::new(cx.executor());

        // Create first worktree with its own private files
        fs.insert_tree(
            path!("/worktree1"),
            json!({
                ".xenomorphic": {
                    "settings.json": r#"{
                        "file_scan_exclusions": ["**/fixture.*"],
                        "private_files": ["**/secret.rs"]
                    }"#
                },
                "src": {
                    "main.rs": "fn main() { let secret_key = \"hidden\"; }",
                    "secret.rs": "const API_KEY: &str = \"secret_value\";",
                    "utils.rs": "pub fn get_config() -> String { \"config\".to_string() }"
                },
                "tests": {
                    "test.rs": "fn test_secret() { assert!(true); }",
                    "fixture.sql": "SELECT * FROM secret_table;"
                }
            }),
        )
        .await;

        // Create second worktree with different private files
        fs.insert_tree(
            path!("/worktree2"),
            json!({
                ".xenomorphic": {
                    "settings.json": r#"{
                        "file_scan_exclusions": ["**/internal.*"],
                        "private_files": ["**/private.js", "**/data.json"]
                    }"#
                },
                "lib": {
                    "public.js": "export function getSecret() { return 'public'; }",
                    "private.js": "const SECRET_KEY = \"private_value\";",
                    "data.json": "{\"secret_data\": \"hidden\"}"
                },
                "docs": {
                    "README.md": "# Documentation with secret info",
                    "internal.md": "Internal secret documentation"
                }
            }),
        )
        .await;

        // Set global settings
        cx.update(|cx| {
            SettingsStore::update_global(cx, |store, cx| {
                store.update_user_settings(cx, |settings| {
                    settings.project.worktree.file_scan_exclusions =
                        Some(vec!["**/.git".to_string(), "**/node_modules".to_string()]);
                    settings.project.worktree.private_files =
                        Some(vec!["**/.env".to_string()].into());
                });
            });
        });

        let project = Project::test(
            fs.clone(),
            [path!("/worktree1").as_ref(), path!("/worktree2").as_ref()],
            cx,
        )
        .await;

        // Wait for worktrees to be fully scanned
        cx.executor().run_until_parked();

        // Search for "secret" - should exclude files based on worktree-specific settings
        let result = run_grep_tool(
            GrepToolInput {
                regex: "secret".to_string(),
                include_pattern: None,
                path: None,
                offset: 0,
                case_sensitive: false,
            },
            project.clone(),
            cx,
        )
        .await;
        let paths = extract_paths_from_results(&result);

        // Should find matches in non-private files
        assert!(
            paths.iter().any(|p| p.contains("main.rs")),
            "Should find 'secret' in worktree1/src/main.rs"
        );
        assert!(
            paths.iter().any(|p| p.contains("test.rs")),
            "Should find 'secret' in worktree1/tests/test.rs"
        );
        assert!(
            paths.iter().any(|p| p.contains("public.js")),
            "Should find 'secret' in worktree2/lib/public.js"
        );
        assert!(
            paths.iter().any(|p| p.contains("README.md")),
            "Should find 'secret' in worktree2/docs/README.md"
        );

        // Should NOT find matches in private/excluded files based on worktree settings
        assert!(
            !paths.iter().any(|p| p.contains("secret.rs")),
            "Should not search in worktree1/src/secret.rs (local private_files)"
        );
        assert!(
            !paths.iter().any(|p| p.contains("fixture.sql")),
            "Should not search in worktree1/tests/fixture.sql (local file_scan_exclusions)"
        );
        assert!(
            !paths.iter().any(|p| p.contains("private.js")),
            "Should not search in worktree2/lib/private.js (local private_files)"
        );
        assert!(
            !paths.iter().any(|p| p.contains("data.json")),
            "Should not search in worktree2/lib/data.json (local private_files)"
        );
        assert!(
            !paths.iter().any(|p| p.contains("internal.md")),
            "Should not search in worktree2/docs/internal.md (local file_scan_exclusions)"
        );

        // Test with `include_pattern` specific to one worktree
        let result = run_grep_tool(
            GrepToolInput {
                regex: "secret".to_string(),
                include_pattern: Some("worktree1/**/*.rs".to_string()),
                path: None,
                offset: 0,
                case_sensitive: false,
            },
            project.clone(),
            cx,
        )
        .await;

        let paths = extract_paths_from_results(&result);

        // Should only find matches in worktree1 *.rs files (excluding private ones)
        assert!(
            paths.iter().any(|p| p.contains("main.rs")),
            "Should find match in worktree1/src/main.rs"
        );
        assert!(
            paths.iter().any(|p| p.contains("test.rs")),
            "Should find match in worktree1/tests/test.rs"
        );
        assert!(
            !paths.iter().any(|p| p.contains("secret.rs")),
            "Should not find match in excluded worktree1/src/secret.rs"
        );
        assert!(
            paths.iter().all(|p| !p.contains("worktree2")),
            "Should not find any matches in worktree2"
        );
    }

    // Helper function to extract file paths from grep results
    fn extract_paths_from_results(results: &str) -> Vec<String> {
        results
            .lines()
            .filter(|line| line.starts_with("## Matches in "))
            .map(|line| {
                line.strip_prefix("## Matches in ")
                    .unwrap()
                    .trim()
                    .to_string()
            })
            .collect()
    }
}
