//! [`FinderProvider`] implementation for agent threads.
//!
//! This module implements the provider trait so that the unified file finder
//! (`cmd-p`) can surface agent thread results alongside file results. Thread
//! results are fetched from [`ThreadMetadataStore`], fuzzy-matched on their
//! display title, and rendered with an icon, worktree chips, and relative
//! timestamps.

use std::sync::Arc;

use agent::ThreadStore;
use agent_thread::schema;
use chrono::{DateTime, Utc};
use fs::Fs;
use gpui::{App, AppContext as _, Context, Entity, SharedString, Window};
use ui::IconName;
use workspace::Workspace;


use crate::thread_metadata_store::{ThreadId, ThreadMetadata, ThreadMetadataStore};
use crate::{Agent, AgentInitialContent, AgentSessionItem, ConversationView};

/// The maximum number of recent threads to show in the empty-query state.
const MAX_RECENT_THREADS: usize = 10;

/// The maximum number of search results to return from a thread search.
const MAX_SEARCH_RESULTS: usize = 20;

// ---------------------------------------------------------------------------
// Re-export FinderProvider types from file_finder for convenience
// ---------------------------------------------------------------------------

pub use file_finder::provider::{FinderProvider, ProviderMatch, ProviderMatchData, SearchMode};

// ---------------------------------------------------------------------------
// ThreadFinderProvider
// ---------------------------------------------------------------------------

/// [`FinderProvider`] that searches agent threads from [`ThreadMetadataStore`].
pub struct ThreadFinderProvider;

impl FinderProvider for ThreadFinderProvider {
    fn section_label(&self) -> &'static str {
        "Recent Agent Sessions"
    }

    fn supports_mode(&self, mode: SearchMode) -> bool {
        match mode {
            SearchMode::Unified | SearchMode::ThreadsOnly => true,
            SearchMode::FilesOnly => false,
        }
    }

    fn search(&self, query: &str, cx: &App) -> Vec<ProviderMatch> {
        if query.is_empty() {
            return self.recent_items(cx);
        }

        let store = match ThreadMetadataStore::try_global(cx) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let entries: Vec<&ThreadMetadata> = store
            .read(cx)
            .entries()
            .filter(|t| !t.archived)
            .collect();

        // Synchronous fuzzy matching on thread titles.
        //
        // We deliberately avoid `fuzzy::match_strings` (which is async and
        // requires `block_on` — a deadlock risk when called from the main
        // thread). Since thread counts are typically in the tens (not
        // thousands like files), a simple synchronous scoring pass is both
        // safe and fast.
        let query_lower: Vec<char> = query.chars().map(|c| c.to_ascii_lowercase()).collect();
        let smart_case = query.chars().any(|c| c.is_uppercase());

        let mut scored: Vec<(usize, f64, Vec<usize>)> = entries
            .iter()
            .enumerate()
            .filter_map(|(i, meta)| {
                let title = meta.display_title();
                let positions = fuzzy_match_positions(
                    &query_lower,
                    &title,
                    smart_case,
                );
                let score = positions.as_ref().map(|p| score_from_positions(p, title.len()))?;
                Some((i, score, positions?))
            })
            .collect();

        // Sort by score descending.
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(MAX_SEARCH_RESULTS);

        let mut results = Vec::with_capacity(scored.len());

        for (entry_idx, score, positions) in scored {
            let Some(meta) = entries.get(entry_idx) else {
                continue;
            };

            let thread_id = meta.thread_id;
            let session_id = meta.session_id.clone();

            results.push(ProviderMatch {
                id: thread_id_to_u64(thread_id),
                label: meta.display_title(),
                secondary_label: worktree_display(meta),
                icon: Some(IconName::XenomorphicAssistant),
                icon_color: Some(ui::Color::Accent),
                score,
                highlight_positions: positions,
                relative_time: Some(relative_time(meta.updated_at)),
                worktree_paths: Some(meta.worktree_paths.clone()),
                data: ProviderMatchData::Thread {
                    thread_id: Arc::from(serde_json::to_string(&thread_id).unwrap_or_default()),
                    session_id: session_id.map(|sid| Arc::from(sid.0.to_string())),
                },
            });
        }

        results
    }

    fn recent_items(&self, cx: &App) -> Vec<ProviderMatch> {
        let store = match ThreadMetadataStore::try_global(cx) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut entries: Vec<&ThreadMetadata> = store
            .read(cx)
            .entries()
            .filter(|t| !t.archived)
            .collect();

        // Sort by updated_at descending (most recent first).
        entries.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        entries.truncate(MAX_RECENT_THREADS);

        let mut results = Vec::with_capacity(entries.len() + 1);

        for meta in &entries {
            let thread_id = meta.thread_id;
            let session_id = meta.session_id.clone();

            results.push(ProviderMatch {
                id: thread_id_to_u64(thread_id),
                label: meta.display_title(),
                secondary_label: worktree_display(meta),
                icon: Some(IconName::XenomorphicAssistant),
                icon_color: Some(ui::Color::Accent),
                score: 0.0,
                highlight_positions: Vec::new(),
                relative_time: Some(relative_time(meta.updated_at)),
                worktree_paths: Some(meta.worktree_paths.clone()),
                data: ProviderMatchData::Thread {
                    thread_id: Arc::from(serde_json::to_string(&thread_id).unwrap_or_default()),
                    session_id: session_id.map(|sid| Arc::from(sid.0.to_string())),
                },
            });
        }

        // Always append "New Agent Session" at the bottom.
        results.push(ProviderMatch {
            id: u64::MAX, // always last
            label: "New Agent Session".into(),
            secondary_label: None,
            icon: Some(IconName::Plus),
            icon_color: Some(ui::Color::Accent),
            score: f64::NEG_INFINITY, // always at the very bottom
            highlight_positions: Vec::new(),
            relative_time: None,
            worktree_paths: None,
            data: ProviderMatchData::NewSession,
        });

        results
    }

    fn confirm(
        &self,
        match_data: &ProviderMatch,
        _secondary: bool,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        match &match_data.data {
            ProviderMatchData::Thread {
                thread_id,
                session_id,
            } => {
                // Unarchive the thread if it's archived.
                if let Some(store) = ThreadMetadataStore::try_global(cx) {
                    if let Ok(tid) = serde_json::from_str::<ThreadId>(thread_id.as_ref()) {
                        store.update(cx, |store, cx| {
                            store.unarchive(tid, cx);
                        });
                    }
                }

                // Open the thread as an AgentSessionItem tab.
                // A thread may not have a session_id yet (e.g. if it was
                // archived before the first message was sent). In that case
                // we pass None and ConversationView will create a fresh session.
                let session_id_to_load = session_id
                    .as_ref()
                    .map(|sid| schema::SessionId::new(sid.to_string()));

                let conversation_view = create_conversation_view(
                    session_id_to_load,
                    None,
                    None,
                    None,
                    workspace,
                    window,
                    cx,
                );
                let item = cx.new(|_| {
                    AgentSessionItem::new(
                        conversation_view,
                        workspace.weak_handle(),
                    )
                });
                workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
            }
            ProviderMatchData::NewSession => {
                // Create a new agent session tab.
                let conversation_view = create_conversation_view(
                    None,
                    None,
                    None,
                    None,
                    workspace,
                    window,
                    cx,
                );
                let item = cx.new(|_| {
                    AgentSessionItem::new(
                        conversation_view,
                        workspace.weak_handle(),
                    )
                });
                workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
            }
            ProviderMatchData::CreateSession { query } => {
                // Create a new session with the query pre-filled as the first message.
                let initial_content = AgentInitialContent::ContentBlock {
                    blocks: vec![query.to_string().into()],
                    auto_submit: true,
                };
                let conversation_view = create_conversation_view(
                    None,
                    None,
                    None,
                    Some(initial_content),
                    workspace,
                    window,
                    cx,
                );
                let item = cx.new(|_| {
                    AgentSessionItem::new(
                        conversation_view,
                        workspace.weak_handle(),
                    )
                });
                workspace.add_item_to_active_pane(Box::new(item), None, true, window, cx);
            }
            ProviderMatchData::SectionHeader { .. } => {
                // Section headers are non-selectable and should never reach confirm.
            }
        }
    }

    fn create_from_query(&self, query: &str) -> Option<ProviderMatch> {
        Some(ProviderMatch {
            id: u64::MAX - 1, // Just above NewSession
            label: format!("Create new thread: {}…", query).into(),
            secondary_label: None,
            icon: Some(IconName::XenomorphicAssistant),
            icon_color: Some(ui::Color::Muted),
            score: f64::NEG_INFINITY,
            highlight_positions: Vec::new(),
            relative_time: None,
            worktree_paths: None,
            data: ProviderMatchData::CreateSession {
                query: Arc::from(query.to_string()),
            },
        })
    }
}

// ---------------------------------------------------------------------------
// Synchronous fuzzy matching
// ---------------------------------------------------------------------------

/// Perform a simple subsequence-based fuzzy match, returning the matching
/// character positions if the query matches, or `None` otherwise.
///
/// This implements a greedy left-to-right subsequence match. Each query
/// character must appear in order within the haystack. Matching is
/// case-insensitive unless `smart_case` is true and the query contains
/// uppercase letters, in which case an exact case match is required.
///
/// This is deliberately simple — thread counts are small (tens, not
/// thousands), so an O(n·m) scan is fast and avoids the deadlock risk
/// of calling `block_on(fuzzy::match_strings(...))` from the main thread.
fn fuzzy_match_positions(
    query_lower: &[char],
    haystack: &SharedString,
    smart_case: bool,
) -> Option<Vec<usize>> {
    if query_lower.is_empty() {
        return Some(Vec::new());
    }

    let haystack_lower: Vec<char> = haystack.chars().map(|c| c.to_ascii_lowercase()).collect();
    let haystack_chars: Vec<char> = haystack.chars().collect();

    let mut positions = Vec::with_capacity(query_lower.len());
    let mut hi = 0;

    for (_qi, &query_char) in query_lower.iter().enumerate() {
        let found = {
            let search_lower = &haystack_lower[hi..];
            search_lower.iter().position(|&h| h == query_char)
        };

        let Some(offset) = found else {
            return None; // Query character not found — no match.
        };

        let pos = hi + offset;

        // If smart_case is on and the original query char is uppercase,
        // require an exact case match at this position.
        if smart_case {
            let original_query_char = query_char;
            if original_query_char.is_uppercase() && haystack_chars[pos] != original_query_char {
                return None;
            }
        }

        positions.push(pos);
        hi = pos + 1;
    }

    Some(positions)
}

/// Compute a match score from the positions and haystack length.
///
/// Higher scores are better. The scoring rewards:
/// - Shorter haystacks (more specific matches)
/// - Consecutive positions (better coverage)
/// - Matches earlier in the string (prefix preference)
fn score_from_positions(positions: &[usize], haystack_len: usize) -> f64 {
    if positions.is_empty() {
        return 0.0;
    }

    // Base score: inversely proportional to haystack length (shorter = better).
    let base = 1.0 / (haystack_len as f64).max(1.0);

    // Bonus for consecutive matches.
    let consecutive_bonus: f64 = positions
        .windows(2)
        .map(|w| if w[1] == w[0] + 1 { 0.1 } else { 0.0 })
        .sum();

    // Bonus for matches early in the string.
    let first_pos_bonus = if positions[0] == 0 { 0.2 } else { 0.0 };

    // Bonus for high coverage (ratio of query length to haystack length).
    let coverage = positions.len() as f64 / haystack_len.max(1) as f64;

    1.0 + base + consecutive_bonus + first_pos_bonus + coverage * 0.5
}

// ---------------------------------------------------------------------------
// Other helpers
// ---------------------------------------------------------------------------

/// Convert a [`ThreadId`] to a `u64` for use as a stable identifier in the
/// picker. We hash the UUID's bytes to produce a u64.
fn thread_id_to_u64(thread_id: ThreadId) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = collections::FxHasher::default();
    thread_id.hash(&mut hasher);
    hasher.finish()
}

/// Format a [`DateTime<Utc>`] as a human-readable relative time string.
///
/// Examples: "just now", "2m ago", "3h ago", "yesterday", "5 days ago".
pub fn relative_time(dt: DateTime<Utc>) -> SharedString {
    let now = Utc::now();
    let diff = now.signed_duration_since(dt);
    let secs = diff.num_seconds();

    if secs < 60 {
        "just now".into()
    } else if secs < 3600 {
        let mins = secs / 60;
        if mins == 1 {
            "1m ago".into()
        } else {
            format!("{}m ago", mins).into()
        }
    } else if secs < 86400 {
        let hours = secs / 3600;
        if hours == 1 {
            "1h ago".into()
        } else {
            format!("{}h ago", hours).into()
        }
    } else if secs < 172800 {
        "yesterday".into()
    } else {
        let days = secs / 86400;
        format!("{} days ago", days).into()
    }
}

/// Produce a secondary label for a thread from its worktree paths.
///
/// Shows the first worktree's directory name, similar to how the sidebar
/// shows worktree chips.
fn worktree_display(meta: &ThreadMetadata) -> Option<SharedString> {
    let paths = meta.folder_paths();
    if paths.is_empty() {
        return None;
    }

    // Show the first path's directory name
    let first = paths.paths().first()?;
    first
        .file_name()
        .map(|name| SharedString::from(name.to_string_lossy().to_string()))
}

// ---------------------------------------------------------------------------
// Creating ConversationView without AgentPanel
// ---------------------------------------------------------------------------

/// Creates a new [`ConversationView`] ready to be wrapped in an [`AgentSessionItem`]
/// and added to a workspace pane.
///
/// This mirrors the construction logic in `AgentPanel::create_agent_thread`
/// but does not require an `AgentPanel`. It builds all the necessary
/// components (agent server, connection store, conversation view) from the
/// workspace's project.
pub fn create_conversation_view(
    session_id_to_load: Option<schema::SessionId>,
    work_dirs: Option<workspace::PathList>,
    title: Option<SharedString>,
    initial_content: Option<AgentInitialContent>,
    workspace: &Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> Entity<ConversationView> {
    let project = workspace.project().clone();
    let fs = <dyn Fs>::global(cx);
    let thread_store = ThreadStore::global(cx);

    let agent = Agent::NativeAgent;
    let server = agent.server(fs.clone(), thread_store.clone());

    let thread_store_handle = Some(thread_store);

    // Use the global AgentConnectionStore so all tabs share the same
    // NativeAgent. Each tab creating its own store/agent meant that
    // session lookups in prompt() would fail ("Session not found")
    // because the session was registered in a *different* NativeAgent.
    let connection_store =
        crate::agent_connection_store::AgentConnectionStore::global(cx);

    let existing_metadata = session_id_to_load.as_ref().and_then(|sid| {
        ThreadMetadataStore::try_global(cx)
            .and_then(|store| store.read(cx).entry_by_session(sid).cloned())
    });
    let thread_id = existing_metadata
        .as_ref()
        .map(|m| m.thread_id)
        .unwrap_or_else(ThreadId::new);

    let weak_workspace = workspace.weak_handle();

    cx.new(|cx| {
        ConversationView::new(
            server,
            connection_store,
            agent,
            session_id_to_load,
            Some(thread_id),
            work_dirs,
            title,
            initial_content,
            weak_workspace,
            project,
            thread_store_handle,
            "thread_finder",
            window,
            cx,
        )
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_time_just_now() {
        let now = Utc::now();
        let result = relative_time(now);
        assert_eq!(result.as_ref(), "just now");
    }

    #[test]
    fn test_relative_time_minutes() {
        let now = Utc::now();
        let five_min_ago = now - chrono::Duration::minutes(5);
        let result = relative_time(five_min_ago);
        assert_eq!(result.as_ref(), "5m ago");
    }

    #[test]
    fn test_relative_time_hours() {
        let now = Utc::now();
        let three_hours_ago = now - chrono::Duration::hours(3);
        let result = relative_time(three_hours_ago);
        assert_eq!(result.as_ref(), "3h ago");
    }

    #[test]
    fn test_relative_time_yesterday() {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);
        let result = relative_time(yesterday);
        assert_eq!(result.as_ref(), "yesterday");
    }

    #[test]
    fn test_relative_time_days() {
        let now = Utc::now();
        let five_days_ago = now - chrono::Duration::days(5);
        let result = relative_time(five_days_ago);
        assert_eq!(result.as_ref(), "5 days ago");
    }

    #[test]
    fn test_fuzzy_match_basic() {
        let query: Vec<char> = "auth".chars().map(|c| c.to_ascii_lowercase()).collect();
        let result = fuzzy_match_positions(&query, &"Fix login auth bug".into(), false);
        assert!(result.is_some());
        let positions = result.unwrap();
        assert_eq!(positions.len(), 4); // "auth" matched
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        let query: Vec<char> = "xyz".chars().map(|c| c.to_ascii_lowercase()).collect();
        let result = fuzzy_match_positions(&query, &"Fix login auth bug".into(), false);
        assert!(result.is_none());
    }

    #[test]
    fn test_fuzzy_match_prefix_preference() {
        let query: Vec<char> = "auth".chars().map(|c| c.to_ascii_lowercase()).collect();
        let prefix_score = score_from_positions(&[0, 1, 2, 3], 10);
        let later_score = score_from_positions(&[5, 6, 7, 8], 10);
        assert!(prefix_score > later_score, "prefix matches should score higher");
    }

    #[test]
    fn test_fuzzy_match_consecutive_bonus() {
        let consecutive = score_from_positions(&[2, 3, 4, 5], 10);
        let scattered = score_from_positions(&[1, 3, 5, 7], 10);
        assert!(consecutive > scattered, "consecutive matches should score higher");
    }

    #[test]
    fn test_fuzzy_match_smart_case() {
        let query: Vec<char> = "Auth".chars().map(|c| c.to_ascii_lowercase()).collect();
        // smart_case = true, query has uppercase 'A' — requires exact case match
        let result = fuzzy_match_positions(&query, &"Fix login auth bug".into(), true);
        // 'A' is uppercase in query but 'a' is lowercase in haystack at the match
        // position — this should NOT match under smart_case
        assert!(result.is_none());
    }

    #[test]
    fn test_fuzzy_match_smart_case_matches() {
        let query: Vec<char> = "Auth".chars().map(|c| c.to_ascii_lowercase()).collect();
        // With smart_case=true, the uppercase 'A' requires an uppercase 'A'
        let result = fuzzy_match_positions(&query, &"Auth service refactor".into(), true);
        assert!(result.is_some());
    }

    #[test]
    fn test_score_from_positions_empty() {
        assert_eq!(score_from_positions(&[], 10), 0.0);
    }
}
