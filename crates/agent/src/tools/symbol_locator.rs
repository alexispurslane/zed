use std::collections::HashSet;
use std::fmt;
use std::ops::Range;

use language::{Buffer, Location, OffsetRangeExt, ToOffset, ToPoint};
use gpui::{App, AsyncApp, Entity};
use project::{CodeAction, Project};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use text::{Anchor, Point};

const MAX_DEFINITION_LINES: u32 = 10;
const MAX_DISPLAY_LEN: usize = 200;

pub const EXPANSION_THRESHOLD: usize = 50;

/// Identifies a symbol in source code by name within a file.
///
/// Used by `go_to_definition` and `find_references`. The tool finds
/// all occurrences of the symbol name in the file, deduplicates by
/// enclosing outline chain, queries the LSP at one position per unique
/// chain, and deduplicates the results.
///
/// If the symbol name appears in multiple scopes and you only want
/// results for one scope, provide `enclosing_scope` to narrow the
/// search. Otherwise, all scopes are queried automatically.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct SymbolSearch {
    /// The relative file path, e.g. "crates/agent/src/lib.rs".
    pub file_path: String,
    /// The name of the symbol to find.
    pub symbol_name: String,
    /// The *enclosing* scope as a list of ancestor names from outermost to
    /// innermost, NOT including the target symbol itself.
    /// When provided, only occurrences within the matching scope(s) are
    /// used as query positions for the LSP. When omitted, all occurrences
    /// are used.
    /// For example, to find references to `new` inside `impl Editor`, use
    /// `["Editor"]`.
    /// To find references to `process` inside `fn run` inside `impl Thread`,
    /// use `["Thread", "run"]`.
    /// Each segment is matched if the outline item's full text ends with
    /// that segment, so `"Editor"` matches both `"struct Editor"` and
    /// `"impl Editor"`, while `"run"` does not match `"fn run_and_wait"`.
    pub enclosing_scope: Option<Vec<String>>,
}

/// Identifies a symbol to rename by name within a file.
///
/// Used by `rename_symbol`. If the symbol name appears only once in the
/// file, no `enclosing_scope` is needed. If it appears in multiple scopes,
/// the tool will list each scope with the line text at that occurrence —
/// provide `enclosing_scope` matching the scope you want. The rename will
/// operate on the first matching symbol within the chosen scope.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct SymbolRename {
    /// The relative file path, e.g. "crates/agent/src/lib.rs".
    pub file_path: String,
    /// The name of the symbol to rename.
    pub symbol_name: String,
    /// The *enclosing* scope as a list of ancestor names from outermost to
    /// innermost, NOT including the target symbol itself.
    /// Only needed when the symbol name appears in multiple scopes and the
    /// tool asks you to disambiguate.
    /// For example, to rename `new` inside `impl Editor`, use `["Editor"]`.
    /// To rename `process` inside `fn run` inside `impl Thread`, use
    /// `["Thread", "run"]`.
    /// Each segment is matched if the outline item's full text ends with
    /// that segment, so `"Editor"` matches both `"struct Editor"` and
    /// `"impl Editor"`, while `"run"` does not match `"fn run_and_wait"`.
    pub enclosing_scope: Option<Vec<String>>,
}

/// Identifies a location in source code by file and line number.
///
/// Used by `get_code_actions` and `apply_code_action` to target a specific
/// line with no ambiguity.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct LineLocator {
    /// The relative file path, e.g. "crates/agent/src/lib.rs".
    pub file_path: String,
    /// The 1-based line number.
    pub line: u32,
}

pub struct PendingCodeActions {
    pub actions: Vec<CodeAction>,
    pub buffer: Entity<Buffer>,
}

pub type CodeActionStore = Entity<Option<PendingCodeActions>>;

pub struct ResolvedSymbol {
    pub buffer: Entity<Buffer>,
    pub position: Anchor,
    pub line_text: String,
    pub truncated: bool,
}

/// A symbol occurrence deduplicated by its enclosing outline chain.
/// Used by search tools that query the LSP at one position per unique
/// scope and then deduplicate the results.
pub struct ResolvedSymbolWithChain {
    pub resolved: ResolvedSymbol,
    /// The outline chain text for this occurrence, from outermost to
    /// innermost. Empty if the occurrence is at top level.
    pub chain: Vec<String>,
}

pub struct LocationDisplay {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub snippet: String,
    pub truncated: bool,
}

impl LocationDisplay {
    pub fn from_location(location: &Location, cx: &App) -> Self {
        Self::from_location_with_expansion(location, true, cx)
    }

    pub fn from_location_with_expansion(location: &Location, expand: bool, cx: &App) -> Self {
        let snapshot = location.buffer.read(cx).snapshot();
        let lsp_range = location.range.start.to_point(&snapshot)
            ..location.range.end.to_point(&snapshot);
        let path = location
            .buffer
            .read(cx)
            .file()
            .map(|f| f.full_path(cx).display().to_string())
            .unwrap_or_else(|| "<untitled>".to_string());

        let display_range = if expand {
            Self::expand_to_definition(&snapshot, &lsp_range)
        } else {
            Point::new(lsp_range.start.row, 0)
                ..Point::new(lsp_range.end.row, snapshot.line_len(lsp_range.end.row))
        };

        let start_line = display_range.start.row + 1;
        let end_line = display_range.end.row + 1;
        let snippet_chars: String = snapshot.text_for_range(display_range.clone()).collect();
        let truncated = snippet_chars.len() > MAX_DISPLAY_LEN;
        let snippet: String = snippet_chars.chars().take(MAX_DISPLAY_LEN).collect();
        let snippet = snippet.trim_end().to_string();

        Self {
            path,
            start_line,
            end_line,
            snippet,
            truncated,
        }
    }

    fn expand_to_definition(
        snapshot: &language::BufferSnapshot,
        lsp_range: &Range<Point>,
    ) -> Range<Point> {
        let full_lines = Point::new(lsp_range.start.row, 0)
            ..Point::new(lsp_range.end.row, snapshot.line_len(lsp_range.end.row));

        if let Some(node) = snapshot.syntax_ancestor(full_lines.clone()) {
            let ancestor_range = node.byte_range().to_point(snapshot);
            let end_row = ancestor_range
                .end
                .row
                .min(ancestor_range.start.row + MAX_DEFINITION_LINES);
            let capped = Point::new(ancestor_range.start.row, 0)
                ..Point::new(end_row, snapshot.line_len(end_row));
            if full_lines.start >= capped.start && full_lines.end <= capped.end {
                return capped;
            }
        }
        full_lines
    }
}

impl fmt::Display for LocationDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let trunc = if self.truncated { " (truncated)" } else { "" };
        write!(f, "{}#L", self.path)?;
        if self.start_line == self.end_line {
            write!(f, "{}", self.start_line)?;
        } else {
            write!(f, "{}-{}", self.start_line, self.end_line)?;
        }
        write!(f, "{trunc}\n```\n{}\n```", self.snippet)
    }
}

impl SymbolSearch {

    /// Resolves to one position per unique outline chain in the file.
    /// The LSP is semantically scoped, so querying at any occurrence within
    /// the same chain yields the same result. By deduplicating to one
    /// position per chain, the caller can query the LSP at each position
    /// in parallel and deduplicate the results.
    ///
    /// Used by `go_to_definition` and `find_references` (search tools).
    pub async fn resolve_for_search(
        &self,
        project: &Entity<Project>,
        cx: &mut AsyncApp,
    ) -> Result<Vec<ResolvedSymbolWithChain>, String> {
        let (buffer, snapshot) = self.open_buffer(project, cx).await?;

        // Determine the byte ranges to search within.
        let search_ranges = scope_to_search_ranges(
            &self.enclosing_scope,
            &snapshot,
            &self.file_path,
        )?;

        // Find name-like syntax nodes matching the symbol name in the search ranges.
        let mut all_offsets: Vec<usize> = Vec::new();
        for range in &search_ranges {
            all_offsets.extend(find_symbol_offsets_in_range(
                &snapshot,
                &self.symbol_name,
                range.clone(),
            ));
        }

        if all_offsets.is_empty() {
            return Err(format!(
                "No AST occurrence of '{}' found in {} — the language server needs a position in the file to query from. Make sure the symbol name appears in this file as an identifier or type name.",
                self.symbol_name, self.file_path
            ));
        }

        // Deduplicate by outline chain: one position per unique chain.
        let mut seen_chains: HashSet<Vec<String>> = HashSet::new();
        let mut results: Vec<ResolvedSymbolWithChain> = Vec::new();

        for offset in all_offsets {
            let point = snapshot.anchor_before(offset).to_point(&snapshot);
            let chain: Vec<String> = snapshot
                .symbols_containing(point, None)
                .iter()
                .map(|item| item.text.clone())
                .collect();
            let chain_key = if chain.is_empty() {
                vec!["(top level)".to_string()]
            } else {
                chain
            };

            if seen_chains.insert(chain_key.clone()) {
                results.push(ResolvedSymbolWithChain {
                    resolved: make_resolved(buffer.clone(), snapshot.clone(), offset),
                    chain: chain_key,
                });
            }
        }

        Ok(results)
    }

    /// Open the buffer, check for indexing, and return it with its snapshot.
    async fn open_buffer(
        &self,
        project: &Entity<Project>,
        cx: &mut AsyncApp,
    ) -> Result<(Entity<Buffer>, language::BufferSnapshot), String> {
        open_buffer(&self.file_path, project, cx).await
    }
}

/// Resolves an optional `enclosing_scope` to a list of byte ranges
/// within the buffer that the search should be confined to.
///
/// - If `enclosing_scope` is `None`, returns a single range covering the
///   entire buffer.
/// - If `enclosing_scope` maps to exactly one outline chain, returns that
///   chain's body range.
/// - If it maps to multiple chains, returns all their body ranges.
///
/// Returns an error if the scope is provided but matches no outline items.
fn scope_to_search_ranges(
    enclosing_scope: &Option<Vec<String>>,
    snapshot: &language::BufferSnapshot,
    file_path: &str,
) -> Result<Vec<Range<usize>>, String> {
    let Some(scope) = enclosing_scope else {
        return Ok(vec![0..snapshot.len()]);
    };

    let outline = snapshot.outline(None);
    let chains = resolve_scope_chains(&outline, scope);

    if chains.is_empty() {
        return Err(format!(
            "No outline item found matching scope [{}] in {}",
            scope.join(", "),
            file_path
        ));
    }

    Ok(chains
        .iter()
        .map(|chain| {
            let leaf_idx = *chain.last().unwrap();
            let item = &outline.items[leaf_idx];
            if let Some(body) = &item.body_range {
                body.start.to_offset(snapshot)..body.end.to_offset(snapshot)
            } else {
                0..snapshot.len()
            }
        })
        .collect())
}

impl SymbolRename {
    /// Resolves to a single symbol position for rename. If the symbol
    /// appears in only one scope (or only once), auto-resolves. If it
    /// appears in multiple scopes, returns an error listing the full
    /// deduped outline chain text for each occurrence so the agent can
    /// qualify in one shot.
    ///
    /// The error includes the line text at each occurrence so the agent
    /// can make an informed choice. The rename will operate on the first
    /// matching symbol within the chosen scope.
    pub async fn resolve_for_rename(
        &self,
        project: &Entity<Project>,
        cx: &mut AsyncApp,
    ) -> Result<ResolvedSymbol, String> {
        let (buffer, snapshot) = open_buffer(&self.file_path, project, cx).await?;

        let search_ranges = scope_to_search_ranges(
            &self.enclosing_scope,
            &snapshot,
            &self.file_path,
        )?;

        // Find AST name nodes within the search ranges.
        let mut all_offsets: Vec<usize> = Vec::new();
        for range in &search_ranges {
            for offset in find_symbol_offsets_in_range(&snapshot, &self.symbol_name, range.clone()) {
                all_offsets.push(offset);
            }
        }

        if all_offsets.is_empty() {
            return Err(format!(
                "No AST occurrence of '{}' found in {} — the language server needs a position in the file to query from. Make sure the symbol name appears in this file as an identifier or type name.",
                self.symbol_name, self.file_path
            ));
        }

        // Unambiguous: single occurrence or a single enclosing scope was provided.
        if all_offsets.len() == 1 || search_ranges.len() == 1 && self.enclosing_scope.is_some() {
            return Ok(make_resolved(buffer, snapshot, all_offsets[0]));
        }

        // Ambiguous: show full chain with line text for each match.
        let match_info = build_rename_disambiguation(&snapshot, &all_offsets);
        let chain_display: Vec<String> = match_info
            .iter()
            .map(|(chain, line_text)| {
                let chain_text = format!("[{}]", chain.join(", "));
                format!("{chain_text} — line: `{line_text}`")
            })
            .collect();

        Err(format!(
            "Symbol '{}' occurs {} times in {}. Matches:\n{}\n\n\
             Provide `enclosing_scope` to choose which one to rename. \
             The rename will operate on the first matching symbol within the chosen scope.",
            self.symbol_name,
            all_offsets.len(),
            self.file_path,
            chain_display.join("\n"),
        ))
    }
}

impl LineLocator {
    /// Opens the buffer and resolves the line number to an anchor position.
    pub async fn resolve(
        &self,
        project: &Entity<Project>,
        cx: &mut AsyncApp,
    ) -> Result<(Entity<Buffer>, Anchor), String> {
        let (buffer, snapshot) = open_buffer(&self.file_path, project, cx).await?;

        if self.line == 0 {
            return Err("Line numbers are 1-based, got 0.".to_string());
        }
        let row = self.line - 1;
        if row >= snapshot.row_count() as u32 {
            return Err(format!(
                "Line {} is beyond end of file ({} lines).",
                self.line,
                snapshot.row_count()
            ));
        }

        let anchor = snapshot.anchor_before(Point::new(row, 0));
        Ok((buffer, anchor))
    }
}

/// Open a buffer by relative file path, checking that the language server is ready.
async fn open_buffer(
    file_path: &str,
    project: &Entity<Project>,
    cx: &mut AsyncApp,
) -> Result<(Entity<Buffer>, language::BufferSnapshot), String> {
    let open_task = project.update(cx, |project, cx| {
        let Some(project_path) = project.find_project_path(file_path, cx) else {
            return Err(format!("Path '{}' not found in project", file_path));
        };
        Ok(project.open_buffer(project_path, cx))
    })
    .map_err(|e| e.to_string())?;
    let buffer = open_task.await.map_err(|e| format!("{e}"))?;

    let still_starting = buffer.update(cx, |buf, cx| {
        project.update(cx, |project, cx| {
            project.has_starting_language_servers_for(buf, cx)
        })
    });
    if still_starting {
        return Err(
            "Language servers are still indexing this file. \
             Wait for them to finish and try again."
                .to_string(),
        );
    }

    let snapshot = buffer.read_with(cx, |buf, _| buf.snapshot());
    Ok((buffer, snapshot))
}

fn make_resolved(
    buffer: Entity<Buffer>,
    snapshot: language::BufferSnapshot,
    absolute_offset: usize,
) -> ResolvedSymbol {
    let anchor = snapshot.anchor_before(absolute_offset);
    let point = anchor.to_point(&snapshot);
    let row = point.row;
    let line_len = snapshot.line_len(row);
    let line_text: String = snapshot
        .text_for_range(Point::new(row, 0)..Point::new(row, line_len))
        .collect();
    let display = line_text.trim_end().to_string();
    ResolvedSymbol {
        buffer,
        position: anchor,
        line_text: display.clone(),
        truncated: display.len() > MAX_DISPLAY_LEN,
    }
}

fn build_rename_disambiguation(
    snapshot: &language::BufferSnapshot,
    all_offsets: &[usize],
) -> Vec<(Vec<String>, String)> {
    let mut seen: HashSet<Vec<String>> = HashSet::new();
    let mut result: Vec<(Vec<String>, String)> = Vec::new();

    for &offset in all_offsets {
        let point = snapshot.anchor_before(offset).to_point(snapshot);
        let chain: Vec<String> = snapshot
            .symbols_containing(point, None)
            .iter()
            .map(|item| item.text.clone())
            .collect();
        let chain_key = if chain.is_empty() {
            vec!["(top level)".to_string()]
        } else {
            chain
        };

        if seen.insert(chain_key.clone()) {
            let line_text = line_text_at(snapshot, offset);
            result.push((chain_key, line_text));
        }
    }

    result
}

fn line_text_at(snapshot: &language::BufferSnapshot, offset: usize) -> String {
    let anchor = snapshot.anchor_before(offset);
    let point = anchor.to_point(snapshot);
    let row = point.row;
    let line_len = snapshot.line_len(row);
    let line_text: String = snapshot
        .text_for_range(Point::new(row, 0)..Point::new(row, line_len))
        .collect();
    let display = line_text.trim_end().to_string();
    if display.len() > MAX_DISPLAY_LEN {
        display.chars().take(MAX_DISPLAY_LEN).collect()
    } else {
        display
    }
}

/// Walk the outline tree, matching each scope segment to an outline item
/// if the item's text ends with the segment. Returns all complete chains
/// of indices from root to leaf that match the full scope path.
fn resolve_scope_chains(outline: &language::Outline<Anchor>, scope: &[String]) -> Vec<Vec<usize>> {
    if scope.is_empty() {
        return Vec::new();
    }

    let mut chains: Vec<Vec<usize>> = outline
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.depth == 0 && segment_matches(item, &scope[0]))
        .map(|(i, _)| vec![i])
        .collect();

    for seg in &scope[1..] {
        let mut next_chains = Vec::new();
        for chain in &chains {
            let parent_idx = *chain.last().unwrap();
            let parent_depth = outline.items[parent_idx].depth;
            let mut child_idx = parent_idx + 1;
            while child_idx < outline.items.len() {
                let child = &outline.items[child_idx];
                if child.depth <= parent_depth {
                    break;
                }
                if child.depth == parent_depth + 1 && segment_matches(child, seg) {
                    let mut extended = chain.clone();
                    extended.push(child_idx);
                    next_chains.push(extended);
                }
                child_idx += 1;
            }
        }
        chains = next_chains;
    }

    chains
}

/// Check whether a scope segment matches an outline item.
/// Matches if the item's full text ends with the segment.
/// e.g. "Editor" matches both "struct Editor" and "impl Editor",
/// but "run" does not match "fn run_and_wait".
fn segment_matches(item: &language::OutlineItem<Anchor>, segment: &str) -> bool {
    item.text.ends_with(segment)
}

/// Find all offsets where a name-like syntax node in the tree matches
/// `symbol_name`, restricted to a byte range within the buffer.
/// Only nodes whose start offset falls within the range are returned.
fn find_symbol_offsets_in_range(
    snapshot: &language::BufferSnapshot,
    symbol_name: &str,
    byte_range: Range<usize>,
) -> Vec<usize> {
    let mut results = Vec::new();
    let needle_len = symbol_name.len();

    for layer in snapshot.syntax_layers() {
        let mut cursor = layer.node().walk();
        visit_name_nodes(
            &mut cursor,
            symbol_name,
            needle_len,
            snapshot,
            &byte_range,
            &mut results,
        );
    }

    results.sort();
    results.dedup();
    results
}

fn visit_name_nodes(
    cursor: &mut language::TreeCursor,
    symbol_name: &str,
    needle_len: usize,
    snapshot: &language::BufferSnapshot,
    byte_range: &Range<usize>,
    results: &mut Vec<usize>,
) {
    loop {
        let node = cursor.node();
        let node_start = node.start_byte();

        // Prune: if this node is entirely past the end of our range, skip.
        if node_start >= byte_range.end {
            break;
        }

        if is_name_node(&node) && node.byte_range().len() == needle_len && byte_range.contains(&node_start) {
            let end = node_start + needle_len;
            let text: String = snapshot.text_for_range(node_start..end).collect();
            if text == symbol_name {
                results.push(node_start);
            }
        }
        if cursor.goto_first_child() {
            visit_name_nodes(cursor, symbol_name, needle_len, snapshot, byte_range, results);
            cursor.goto_parent();
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

/// Whether a tree-sitter node could represent a symbol name.
///
/// We consider a node a candidate if it's a named leaf (no named children)
/// — this covers `identifier`, `type_identifier`, `field_identifier`,
/// `shorthand_field_identifier`, and any language-specific name kinds
/// without hardcoding them. Named leaf nodes correspond to the atomic
/// tokens (identifiers, literals) in the grammar; keywords and punctuation
/// are anonymous nodes and are filtered out by `is_named()`.
fn is_name_node(node: &language::Node) -> bool {
    node.is_named() && node.child_count() == 0
}

/// Compare two file paths by proximity to a source path.
///
/// Proximity is the number of directory traversals needed to navigate
/// from `source_path` to the reference path: count the shared leading
/// path components, then `distance = (source_len - shared) + (ref_len - shared)`.
/// Same file = 0, same directory = 2, etc. Ties are broken alphabetically.
pub fn proximity_ord(source_path: &str, path_a: &str, path_b: &str) -> std::cmp::Ordering {
    let source = path_components(source_path);
    let a = path_components(path_a);
    let b = path_components(path_b);

    let dist_a = path_distance(&source, &a);
    let dist_b = path_distance(&source, &b);

    dist_a.cmp(&dist_b).then_with(|| path_a.cmp(path_b))
}

pub fn path_components(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

pub fn path_distance(source: &[&str], target: &[&str]) -> usize {
    let shared = source
        .iter()
        .zip(target.iter())
        .take_while(|(a, b)| a == b)
        .count();
    (source.len() - shared) + (target.len() - shared)
}

pub fn file_path_from_location(location: &language::Location, cx: &mut gpui::AsyncApp) -> String {
    cx.update(|cx| {
        location
            .buffer
            .read(cx)
            .file()
            .map(|f| f.full_path(cx).display().to_string())
            .unwrap_or_else(|| "<untitled>".to_string())
    })
}
