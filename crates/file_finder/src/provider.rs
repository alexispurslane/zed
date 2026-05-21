//! Extensible provider trait for the unified file finder.
//!
//! The `FinderProvider` trait allows external crates (like `agent_ui`) to
//! contribute search results to the `cmd-p` file finder without creating a
//! hard dependency from `file_finder` to those crates.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐   registers   ┌────────────────────────────┐
//! │  agent_ui   │──────────────▶│  FinderProviderRegistry   │ (GPUI Global)
//! └─────────────┘               └──────────────┬─────────────┘
//!                                              │ reads from
//!                                              ▼
//!                                      ┌──────────────────┐
//!                                      │ FileFinderDelegate│
//!                                      └──────────────────┘
//! ```
//!
//! The existing file search logic becomes the implicit default "file
//! provider" and is not registered through this trait — it is built into
//! `FileFinderDelegate` directly. External providers are additive.

use gpui::{App, Context, Global, Window};
use project::WorktreePaths;
use std::sync::Arc;
use ui::{Color, IconName, SharedString};
use workspace::Workspace;

// ---------------------------------------------------------------------------
// SearchMode
// ---------------------------------------------------------------------------

/// Determines which kinds of results the finder should show.
///
/// The mode is derived from a prefix the user types in the query field:
///
/// | Prefix | Mode | Effect |
/// |--------|------|--------|
/// | `#` | `ThreadsOnly` | Only agent-thread results |
/// | `$` | `FilesOnly` | Only file results (same as today) |
/// | *(none)* | `Unified` | Interleave files + threads by score |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchMode {
    /// Show both file and thread results, interleaved by match score.
    Unified,
    /// Show only agent-thread results (triggered by `#` prefix).
    ThreadsOnly,
    /// Show only file results (triggered by `$` prefix).
    FilesOnly,
}

impl SearchMode {
    /// Parse the raw query string into a `(SearchMode, stripped_query)` tuple.
    ///
    /// The prefix is stripped from the returned query.
    pub fn from_query(raw_query: &str) -> (Self, &str) {
        if let Some(rest) = raw_query.strip_prefix('#') {
            (SearchMode::ThreadsOnly, rest.trim_start())
        } else if let Some(rest) = raw_query.strip_prefix('$') {
            (SearchMode::FilesOnly, rest.trim_start())
        } else {
            (SearchMode::Unified, raw_query)
        }
    }

    /// Whether file results should be shown in this mode.
    pub fn show_files(self) -> bool {
        matches!(self, SearchMode::Unified | SearchMode::FilesOnly)
    }

    /// Whether thread results should be shown in this mode.
    pub fn show_threads(self) -> bool {
        matches!(self, SearchMode::Unified | SearchMode::ThreadsOnly)
    }
}

// ---------------------------------------------------------------------------
// ProviderMatchData
// ---------------------------------------------------------------------------

/// Type-erased payload carried inside a [`ProviderMatch`].
///
/// Each variant corresponds to a distinct kind of entity that a provider
/// can surface. The enum is intentionally defined here (rather than in
/// `agent_ui`) so that the `file_finder` crate can render and dispatch
/// on match types without depending on agent-specific crates.
///
/// New variants should be added here when new provider types are introduced
/// (e.g. symbols, commands).
#[derive(Debug, Clone)]
pub enum ProviderMatchData {
    /// An existing agent thread.
    Thread {
        /// Opaque identifier for the thread (UUID stored as a string so we
        /// don't depend on the `agent_ui::ThreadId` type).
        thread_id: Arc<str>,
        /// Optional session identifier associated with the thread.
        session_id: Option<Arc<str>>,
    },

    /// The "New Agent Session" entry shown in the empty-query section and at
    /// the bottom of thread-only results.
    NewSession,

    /// "Start agent session: <query>" — creates a new session with the query
    /// text pre-filled as the first message.
    CreateSession {
        /// The query text to pre-fill.
        query: Arc<str>,
    },

    /// A non-selectable section header row (e.g. "Recent Files",
    /// "Recent Agent Sessions").
    SectionHeader {
        /// The header label.
        label: &'static str,
    },
}

// ---------------------------------------------------------------------------
// ProviderMatch
// ---------------------------------------------------------------------------

/// A single searchable result returned by a [`FinderProvider`].
///
/// This is the provider-side representation. The `FileFinderDelegate`
/// internally wraps these into its own `Match` enum for rendering and
/// scoring, but the data lives here.
#[derive(Debug, Clone)]
pub struct ProviderMatch {
    /// Opaque identifier used for deduplication.
    pub id: u64,

    /// Primary label (e.g. thread title or file name).
    pub label: SharedString,

    /// Secondary label shown next to the primary (e.g. worktree name for
    /// files, or worktree chips for threads).
    pub secondary_label: Option<SharedString>,

    /// Icon shown in the start slot of the list row.
    pub icon: Option<IconName>,

    /// Tint color for the icon. `None` uses the default color.
    pub icon_color: Option<Color>,

    /// Fuzzy-match score (higher = better match). Used for interleaving
    /// results from different providers.
    pub score: f64,

    /// Character positions in `label` that should be highlighted as fuzzy
    /// matches.
    pub highlight_positions: Vec<usize>,

    /// Relative timestamp like "2h ago" or "yesterday", shown in the end
    /// slot for thread results.
    pub relative_time: Option<SharedString>,

    /// Worktree paths associated with this match, used for project-scoped
    /// filtering and for rendering worktree chips.
    pub worktree_paths: Option<WorktreePaths>,

    /// Type-specific data (thread id, session id, section header, etc.).
    pub data: ProviderMatchData,
}

impl ProviderMatch {
    /// Whether this match is selectable in the picker.
    ///
    /// Section headers are *not* selectable — they are purely visual
    /// groupings that arrow-key navigation skips over.
    pub fn is_selectable(&self) -> bool {
        !matches!(self.data, ProviderMatchData::SectionHeader { .. })
    }

    /// Whether this match represents a provider-provided thread result
    /// (as opposed to a creation entry or section header).
    pub fn is_thread_result(&self) -> bool {
        matches!(self.data, ProviderMatchData::Thread { .. })
    }

    /// Whether this match is a creation action (NewSession or CreateSession).
    pub fn is_create_action(&self) -> bool {
        matches!(
            self.data,
            ProviderMatchData::NewSession | ProviderMatchData::CreateSession { .. }
        )
    }
}

// ---------------------------------------------------------------------------
// FinderProvider trait
// ---------------------------------------------------------------------------

/// A plugin that contributes search results to the unified file finder.
///
/// # Lifecycle
///
/// Providers are registered once at application init (e.g. in
/// `agent_ui::init`) and held by the `FileFinderDelegate` for the lifetime
/// of each finder session.
///
/// # Thread Safety
///
/// All methods receive `&self`, so implementations must be internally
/// synchronized if they access mutable state. In practice, providers
/// typically read from a global store (`ThreadMetadataStore`, project
/// index, etc.) that is already synchronized via GPUI's entity system.
pub trait FinderProvider: Send + Sync + 'static {
    /// The section header label shown in the empty-query state (e.g.
    /// "Recent Agent Sessions").
    fn section_label(&self) -> &'static str;

    /// Whether this provider's results should be shown for the given
    /// [`SearchMode`].
    ///
    /// A file-only provider returns `true` only for `FilesOnly` and
    /// `Unified`; a thread-only provider returns `true` only for
    /// `ThreadsOnly` and `Unified`.
    fn supports_mode(&self, mode: SearchMode) -> bool;

    /// Search this provider's data for the given query, returning
    /// scored matches.
    ///
    /// Called on a background thread or async context. The `cx` parameter
    /// allows reading global state.
    fn search(&self, query: &str, cx: &App) -> Vec<ProviderMatch>;

    /// Return recent items for the empty-query state.
    ///
    /// These are shown under the section header returned by
    /// [`Self::section_label`].
    fn recent_items(&self, cx: &App) -> Vec<ProviderMatch>;

    /// Handle the user confirming (pressing Enter on) a match.
    ///
    /// This is responsible for opening the item in the workspace (e.g.
    /// opening a thread tab, creating a new session).
    fn confirm(
        &self,
        match_data: &ProviderMatch,
        secondary: bool,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    );

    /// Optionally return a "create from query" entry when no existing
    /// results match the user's typed query.
    ///
    /// For example, the thread provider returns
    /// `ProviderMatchData::CreateSession { query }` so the user can
    /// create a new agent session from the finder.
    ///
    /// Return `None` if this provider does not support create-from-query.
    fn create_from_query(&self, query: &str) -> Option<ProviderMatch>;
}

// ---------------------------------------------------------------------------
// Global provider registry
// ---------------------------------------------------------------------------

/// A GPUI [`Global`] that stores [`FinderProvider`]s registered during
/// application init.
///
/// External crates register providers once at startup (e.g. in
/// `agent_ui::init`) by calling [`register_finder_provider`]. When a
/// `FileFinderDelegate` is created, it pulls providers from this global
/// registry via [`finder_providers`].
///
/// This indirection is necessary because the `FileFinder` is a modal view
/// created on-the-fly when the user presses `cmd-p`; providers must be
/// persisted somewhere that outlives any individual finder session.
#[derive(Default)]
pub struct FinderProviderRegistry {
    providers: Vec<Arc<dyn FinderProvider>>,
}

impl Global for FinderProviderRegistry {}

/// Register a [`FinderProvider`] with the global registry.
///
/// Call this during application init (e.g. `agent_ui::init`) so that
/// provider results appear alongside file results in the unified finder.
///
/// # Example
///
/// ```ignore
/// // In agent_ui::init():
/// file_finder::register_finder_provider(ThreadFinderProvider, cx);
/// ```
pub fn register_finder_provider(provider: impl FinderProvider, cx: &mut App) {
    cx.default_global::<FinderProviderRegistry>()
        .providers
        .push(Arc::new(provider));
}

/// Returns references to all registered [`FinderProvider`]s.
///
/// This is called by `FileFinderDelegate::new()` to populate its
/// `providers` field with the globally registered providers.
///
/// Each finder session shares the same provider `Arc`s — providers are
/// stateless (all methods take `&self`), so sharing is safe.
pub fn finder_providers(cx: &App) -> Vec<Arc<dyn FinderProvider>> {
    cx.try_global::<FinderProviderRegistry>()
        .map(|registry| registry.providers.clone())
        .unwrap_or_default()
}
