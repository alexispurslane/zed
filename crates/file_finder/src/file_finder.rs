#[cfg(test)]
mod file_finder_tests;

pub mod provider;

pub use provider::{FinderProvider, FinderProviderRegistry, ProviderMatch, ProviderMatchData, SearchMode, finder_providers, register_finder_provider};

use futures::future::join_all;
pub use open_path_prompt::OpenPathDelegate;



use collections::HashMap;
use editor::Editor;
use file_icons::FileIcons;
// fuzzy module kept for potential future use
use fuzzy_nucleo::{PathMatch, PathMatchCandidate};
use gpui::{
    Action, AnyElement, App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    KeyContext, Modifiers, ModifiersChangedEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, Task, TaskExt, WeakEntity, Window, actions, rems,
};
use open_path_prompt::{
    OpenPathPrompt,
    file_finder_settings::{FileFinderSettings, FileFinderWidth},
};
use picker::{Picker, PickerDelegate};
use project::{
    PathMatchCandidateSet, Project, ProjectPath, WorktreeId, WorktreePaths,
    worktree_store::WorktreeStore,
};
use project_panel::project_panel_settings::ProjectPanelSettings;
use settings::Settings;
use std::{
    borrow::Cow,
    cmp,
    ops::Range,
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{self, AtomicBool},
    },
};
use ui::{
    ButtonLike, CommonAnimationExt, ContextMenu, HighlightedLabel, Indicator, KeyBinding, ListItem,
    ListItemSpacing, ListSubHeader, PopoverMenu, PopoverMenuHandle, TintColor, Tooltip, prelude::*,
};
use ui_input::ErasedEditor;
use util::{
    ResultExt, maybe,
    paths::{PathStyle, PathWithPosition},
    post_inc,
    rel_path::RelPath,
};
use workspace::{
    ModalView, NewFile, OpenOptions, OpenVisible, SplitDirection, Workspace,
    item::PreviewTabsSettings, notifications::NotifyResultExt, pane,
};
use xenomorphic_actions::search::ToggleIncludeIgnored;

actions!(
    file_finder,
    [
        /// Selects the previous item in the file finder.
        SelectPrevious,
        /// Toggles the file filter menu.
        ToggleFilterMenu,
        /// Toggles the split direction menu.
        ToggleSplitMenu
    ]
);

impl ModalView for FileFinder {
    fn on_before_dismiss(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> workspace::DismissDecision {
        let submenu_focused = self.picker.update(cx, |picker, cx| {
            picker
                .delegate
                .filter_popover_menu_handle
                .is_focused(window, cx)
                || picker
                    .delegate
                    .split_popover_menu_handle
                    .is_focused(window, cx)
        });
        workspace::DismissDecision::Dismiss(!submenu_focused)
    }
}

pub struct FileFinder {
    picker: Entity<Picker<FileFinderDelegate>>,
    picker_focus_handle: FocusHandle,
    init_modifiers: Option<Modifiers>,
}

pub fn init(cx: &mut App) {
    cx.observe_new(FileFinder::register).detach();
    cx.observe_new(OpenPathPrompt::register).detach();
    cx.observe_new(OpenPathPrompt::register_new_path).detach();
}

impl FileFinder {
    fn register(
        workspace: &mut Workspace,
        _window: Option<&mut Window>,
        _: &mut Context<Workspace>,
    ) {
        workspace.register_action(
            |workspace, action: &workspace::ToggleFileFinder, window, cx| {
                let Some(file_finder) = workspace.active_modal::<Self>(cx) else {
                    Self::open(workspace, action.separate_history, window, cx).detach();
                    return;
                };

                file_finder.update(cx, |file_finder, cx| {
                    file_finder.init_modifiers = Some(window.modifiers());
                    file_finder.picker.update(cx, |picker, cx| {
                        picker.cycle_selection(window, cx);
                    });
                });
            },
        );
    }

    fn open(
        workspace: &mut Workspace,
        separate_history: bool,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Task<()> {
        let project = workspace.project().read(cx);
        let fs = project.fs();

        let currently_opened_path = workspace.active_item(cx).and_then(|item| {
            let project_path = item.project_path(cx)?;
            let abs_path = project
                .worktree_for_id(project_path.worktree_id, cx)?
                .read(cx)
                .absolutize(&project_path.path);
            Some(FoundPath::new(project_path, abs_path))
        });

        let history_items = workspace
            .recent_navigation_history(Some(MAX_RECENT_SELECTIONS), cx)
            .into_iter()
            .filter_map(|(project_path, abs_path)| {
                if project.entry_for_path(&project_path, cx).is_some() {
                    return Some(Task::ready(Some(FoundPath::new(project_path, abs_path?))));
                }
                let abs_path = abs_path?;
                if project.is_local() {
                    let fs = fs.clone();
                    Some(cx.background_spawn(async move {
                        if fs.is_file(&abs_path).await {
                            Some(FoundPath::new(project_path, abs_path))
                        } else {
                            None
                        }
                    }))
                } else {
                    Some(Task::ready(Some(FoundPath::new(project_path, abs_path))))
                }
            })
            .collect::<Vec<_>>();
        cx.spawn_in(window, async move |workspace, cx| {
            let history_items = join_all(history_items).await.into_iter().flatten();

            workspace
                .update_in(cx, |workspace, window, cx| {
                    let project = workspace.project().clone();
                    let weak_workspace = cx.entity().downgrade();
                    workspace.toggle_modal(window, cx, |window, cx| {
                        let delegate = FileFinderDelegate::new(
                            cx.entity().downgrade(),
                            weak_workspace,
                            project,
                            currently_opened_path,
                            history_items.collect(),
                            separate_history,
                            window,
                            cx,
                        );

                        FileFinder::new(delegate, window, cx)
                    });
                })
                .ok();
        })
    }

    fn new(delegate: FileFinderDelegate, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let picker = cx.new(|cx| Picker::uniform_list(delegate, window, cx));
        let picker_focus_handle = picker.focus_handle(cx);
        picker.update(cx, |picker, _| {
            picker.delegate.focus_handle = picker_focus_handle.clone();
        });
        Self {
            picker,
            picker_focus_handle,
            init_modifiers: window.modifiers().modified().then_some(window.modifiers()),
        }
    }

    fn handle_modifiers_changed(
        &mut self,
        event: &ModifiersChangedEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(init_modifiers) = self.init_modifiers.take() else {
            return;
        };
        if self.picker.read(cx).delegate.has_changed_selected_index
            && (!event.modified() || !init_modifiers.is_subset_of(event))
        {
            self.init_modifiers = None;
            window.dispatch_action(menu::Confirm.boxed_clone(), cx);
        }
    }

    fn handle_select_prev(
        &mut self,
        _: &SelectPrevious,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.init_modifiers = Some(window.modifiers());
        window.dispatch_action(Box::new(menu::SelectPrevious), cx);
    }

    fn handle_filter_toggle_menu(
        &mut self,
        _: &ToggleFilterMenu,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.picker.update(cx, |picker, cx| {
            let menu_handle = &picker.delegate.filter_popover_menu_handle;
            if menu_handle.is_deployed() {
                menu_handle.hide(cx);
            } else {
                menu_handle.show(window, cx);
            }
        });
    }

    fn handle_split_toggle_menu(
        &mut self,
        _: &ToggleSplitMenu,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.picker.update(cx, |picker, cx| {
            let menu_handle = &picker.delegate.split_popover_menu_handle;
            if menu_handle.is_deployed() {
                menu_handle.hide(cx);
            } else {
                menu_handle.show(window, cx);
            }
        });
    }

    fn handle_toggle_ignored(
        &mut self,
        _: &ToggleIncludeIgnored,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.picker.update(cx, |picker, cx| {
            picker.delegate.include_ignored = match picker.delegate.include_ignored {
                Some(true) => FileFinderSettings::get_global(cx)
                    .include_ignored
                    .map(|_| false),
                Some(false) => Some(true),
                None => Some(true),
            };
            picker.delegate.include_ignored_refresh =
                picker.delegate.update_matches(picker.query(cx), window, cx);
        });
    }

    fn go_to_file_split_left(
        &mut self,
        _: &pane::SplitLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.go_to_file_split_inner(SplitDirection::Left, window, cx)
    }

    fn go_to_file_split_right(
        &mut self,
        _: &pane::SplitRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.go_to_file_split_inner(SplitDirection::Right, window, cx)
    }

    fn go_to_file_split_up(
        &mut self,
        _: &pane::SplitUp,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.go_to_file_split_inner(SplitDirection::Up, window, cx)
    }

    fn go_to_file_split_down(
        &mut self,
        _: &pane::SplitDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.go_to_file_split_inner(SplitDirection::Down, window, cx)
    }

    fn go_to_file_split_inner(
        &mut self,
        split_direction: SplitDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.picker.update(cx, |picker, cx| {
            let delegate = &mut picker.delegate;
            if let Some(workspace) = delegate.workspace.upgrade()
                && let Some(m) = delegate.matches.get(delegate.selected_index())
            {
                match m {
                    Match::History { path, .. } => {
                        let worktree_id = path.project.worktree_id;
                        let project_path = ProjectPath {
                            worktree_id,
                            path: Arc::clone(&path.project.path),
                        };
                        let open_task = workspace.update(cx, move |workspace, cx| {
                            workspace.split_path_preview(
                                project_path,
                                false,
                                Some(split_direction),
                                window,
                                cx,
                            )
                        });
                        open_task.detach_and_log_err(cx);
                    }
                    Match::Search(m) => {
                        let project_path = ProjectPath {
                            worktree_id: WorktreeId::from_usize(m.0.worktree_id),
                            path: m.0.path.clone(),
                        };
                        let open_task = workspace.update(cx, move |workspace, cx| {
                            workspace.split_path_preview(
                                project_path,
                                false,
                                Some(split_direction),
                                window,
                                cx,
                            )
                        });
                        open_task.detach_and_log_err(cx);
                    }
                    Match::CreateNew(p) => {
                        let project_path = p.clone();
                        let open_task = workspace.update(cx, move |workspace, cx| {
                            workspace.split_path_preview(
                                project_path,
                                false,
                                Some(split_direction),
                                window,
                                cx,
                            )
                        });
                        open_task.detach_and_log_err(cx);
                    }
                    Match::Thread(thread_match) => {
                        // Delegate to the provider's confirm() with
                        // secondary=true (which conventionally means "open in
                        // split"), then dismiss the finder.
                        let pmatch = delegate.provider_match_for_thread(thread_match);
                        if let Some((provider, pmatch)) = pmatch {
                            workspace.update(cx, |workspace, cx| {
                                provider.confirm(&pmatch, true, workspace, window, cx);
                            });
                        }
                        delegate
                            .file_finder
                            .update(cx, |_, cx| cx.emit(DismissEvent))
                            .log_err();
                    }
                    // SectionHeader, NewSession, CreateSession, NewFile are not
                    // applicable to split — they are either non-selectable
                    // visual groupings or item-creation actions that don't
                    // target a file. Silently return rather than crashing.
                    Match::SectionHeader(_)
                    | Match::NewSession
                    | Match::CreateSession(_)
                    | Match::NewFile => return,
                }
            }
        })
    }

    pub fn modal_max_width(width_setting: FileFinderWidth, window: &mut Window) -> Pixels {
        let window_width = window.viewport_size().width;
        let small_width = rems(34.).to_pixels(window.rem_size());

        match width_setting {
            FileFinderWidth::Small => small_width,
            FileFinderWidth::Full => window_width,
            FileFinderWidth::XLarge => (window_width - px(512.)).max(small_width),
            FileFinderWidth::Large => (window_width - px(768.)).max(small_width),
            FileFinderWidth::Medium => (window_width - px(1024.)).max(small_width),
        }
    }
}

impl EventEmitter<DismissEvent> for FileFinder {}

impl Focusable for FileFinder {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.picker_focus_handle.clone()
    }
}

impl Render for FileFinder {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let key_context = self.picker.read(cx).delegate.key_context(window, cx);

        let file_finder_settings = FileFinderSettings::get_global(cx);
        let modal_max_width = Self::modal_max_width(file_finder_settings.modal_max_width, window);

        v_flex()
            .key_context(key_context)
            .w(modal_max_width)
            .on_modifiers_changed(cx.listener(Self::handle_modifiers_changed))
            .on_action(cx.listener(Self::handle_select_prev))
            .on_action(cx.listener(Self::handle_filter_toggle_menu))
            .on_action(cx.listener(Self::handle_split_toggle_menu))
            .on_action(cx.listener(Self::handle_toggle_ignored))
            .on_action(cx.listener(Self::go_to_file_split_left))
            .on_action(cx.listener(Self::go_to_file_split_right))
            .on_action(cx.listener(Self::go_to_file_split_up))
            .on_action(cx.listener(Self::go_to_file_split_down))
            .child(self.picker.clone())
    }
}

pub struct FileFinderDelegate {
    file_finder: WeakEntity<FileFinder>,
    workspace: WeakEntity<Workspace>,
    project: Entity<Project>,
    search_count: usize,
    latest_search_id: usize,
    latest_search_did_cancel: bool,
    latest_search_query: Option<FileSearchQuery>,
    currently_opened_path: Option<FoundPath>,
    matches: Matches,
    selected_index: usize,
    has_changed_selected_index: bool,
    cancel_flag: Arc<AtomicBool>,
    history_items: Vec<FoundPath>,
    separate_history: bool,
    first_update: bool,
    filter_popover_menu_handle: PopoverMenuHandle<ContextMenu>,
    split_popover_menu_handle: PopoverMenuHandle<ContextMenu>,
    focus_handle: FocusHandle,
    include_ignored: Option<bool>,
    include_ignored_refresh: Task<()>,
    // NEW: Current search mode derived from query prefix (#, $, or none)
    search_mode: SearchMode,
    // NEW: Registered providers that contribute non-file results
    providers: Vec<Arc<dyn FinderProvider>>,
}

/// Use a custom ordering for file finder: the regular one
/// defines max element with the highest score and the latest alphanumerical path (in case of a tie on other params), e.g:
/// `[{score: 0.5, path = "c/d" }, { score: 0.5, path = "/a/b" }]`
///
/// In the file finder, we would prefer to have the max element with the highest score and the earliest alphanumerical path, e.g:
/// `[{ score: 0.5, path = "/a/b" }, {score: 0.5, path = "c/d" }]`
/// as the files are shown in the project panel lists.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectPanelOrdMatch(PathMatch);

impl Ord for ProjectPanelOrdMatch {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.0
            .score
            .partial_cmp(&other.0.score)
            .unwrap_or(cmp::Ordering::Equal)
            .then_with(|| self.0.worktree_id.cmp(&other.0.worktree_id))
            .then_with(|| {
                other
                    .0
                    .distance_to_relative_ancestor
                    .cmp(&self.0.distance_to_relative_ancestor)
            })
            .then_with(|| self.0.path.cmp(&other.0.path).reverse())
    }
}

impl PartialOrd for ProjectPanelOrdMatch {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Default)]
struct Matches {
    separate_history: bool,
    matches: Vec<Match>,
}

/// A match representing an existing agent thread, returned by a
/// [`FinderProvider`](crate::provider::FinderProvider).
#[derive(Debug, Clone)]
struct ThreadMatch {
    thread_id: Arc<str>,
    session_id: Option<Arc<str>>,
    title: SharedString,
    worktree_paths: Option<WorktreePaths>,
    relative_time: Option<SharedString>,
    score: f64,
    highlight_positions: Vec<usize>,
    /// Index into `FileFinderDelegate::providers` identifying which provider
    /// produced this match. Used to dispatch `confirm()` to the correct
    /// provider instead of guessing.
    provider_index: usize,
}

#[derive(Debug, Clone)]
enum Match {
    History {
        path: FoundPath,
        panel_match: Option<ProjectPanelOrdMatch>,
    },
    Search(ProjectPanelOrdMatch),
    CreateNew(ProjectPath),
    // NEW: An existing thread result from a FinderProvider.
    Thread(ThreadMatch),
    // NEW: "Start agent session: <query>" — creates a new session with query
    // pre-filled as the first message.
    CreateSession(String),
    // NEW: Non-selectable visual section header (e.g. "Recent Files",
    // "Recent Agent Sessions").
    SectionHeader(&'static str),
    // NEW: "New Agent Session" entry — always at the bottom of the threads
    // section in empty-query or `#` mode.
    NewSession,
    // NEW: "New File" entry shown at the top of the empty-query recent files
    // list, so users can create a new file from the picker without typing.
    NewFile,
}

impl Match {
    fn relative_path(&self) -> Option<&Arc<RelPath>> {
        match self {
            Match::History { path, .. } => Some(&path.project.path),
            Match::Search(panel_match) => Some(&panel_match.0.path),
            Match::CreateNew(_) => None,
            Match::Thread(_) => None,
            Match::CreateSession(_) => None,
            Match::SectionHeader(_) => None,
            Match::NewSession => None,
            Match::NewFile => None,
        }
    }

    fn abs_path(&self, project: &Entity<Project>, cx: &App) -> Option<PathBuf> {
        match self {
            Match::History { path, .. } => Some(path.absolute.clone()),
            Match::Search(ProjectPanelOrdMatch(path_match)) => Some(
                project
                    .read(cx)
                    .worktree_for_id(WorktreeId::from_usize(path_match.worktree_id), cx)?
                    .read(cx)
                    .absolutize(&path_match.path),
            ),
            Match::CreateNew(_) => None,
            Match::Thread(_) => None,
            Match::CreateSession(_) => None,
            Match::SectionHeader(_) => None,
            Match::NewSession => None,
            Match::NewFile => None,
        }
    }

    fn panel_match(&self) -> Option<&ProjectPanelOrdMatch> {
        match self {
            Match::History { panel_match, .. } => panel_match.as_ref(),
            Match::Search(panel_match) => Some(panel_match),
            Match::CreateNew(_) => None,
            Match::Thread(_) => None,
            Match::CreateSession(_) => None,
            Match::SectionHeader(_) => None,
            Match::NewSession => None,
            Match::NewFile => None,
        }
    }

    /// Whether this match is selectable in the picker.
    ///
    /// Section headers are non-selectable visual groupings that
    /// arrow-key navigation should skip over.
    fn is_selectable(&self) -> bool {
        !matches!(self, Match::SectionHeader(_))
    }
}

impl Matches {
    fn len(&self) -> usize {
        self.matches.len()
    }

    /// The number of *selectable* matches (excluding SectionHeaders).
    fn selectable_len(&self) -> usize {
        self.matches.iter().filter(|m| m.is_selectable()).count()
    }

    fn get(&self, index: usize) -> Option<&Match> {
        self.matches.get(index)
    }

    fn position(
        &self,
        entry: &Match,
        currently_opened: Option<&FoundPath>,
    ) -> Result<usize, usize> {
        if let Match::History {
            path,
            panel_match: None,
        } = entry
        {
            // Slow case: linear search by path. Should not happen actually,
            // since we call `position` only if matches set changed, but the query has not changed.
            // And History entries do not have panel_match if query is empty, so there's no
            // reason for the matches set to change.
            self.matches
                .iter()
                .position(|m| match m.relative_path() {
                    Some(p) => path.project.path == *p,
                    None => false,
                })
                .ok_or(0)
        } else if let Match::Thread(thread_match) = entry {
            // Thread matches: linear search by thread_id
            self.matches
                .iter()
                .position(|m| match m {
                    Match::Thread(existing) => existing.thread_id == thread_match.thread_id,
                    _ => false,
                })
                .ok_or(0)
        } else {
            self.matches.binary_search_by(|m| {
                // `reverse()` since if cmp_matches(a, b) == Ordering::Greater, then a is better than b.
                // And we want the better entries go first.
                Self::cmp_matches(self.separate_history, currently_opened, m, entry).reverse()
            })
        }
    }

    fn push_new_matches<'a>(
        &'a mut self,
        worktree_store: Entity<WorktreeStore>,
        cx: &'a App,
        history_items: impl IntoIterator<Item = &'a FoundPath> + Clone,
        currently_opened: Option<&'a FoundPath>,
        query: Option<&FileSearchQuery>,
        new_search_matches: impl Iterator<Item = ProjectPanelOrdMatch>,
        extend_old_matches: bool,
        path_style: PathStyle,
        // NEW: Provider-related parameters
        search_mode: SearchMode,
        providers: &[Arc<dyn FinderProvider>],
    ) {
        let Some(query) = query else {
            // assuming that if there's no query, then there's no search matches.
            self.matches.clear();
            let path_to_entry = |found_path: &FoundPath| Match::History {
                path: found_path.clone(),
                panel_match: None,
            };

            // --- Sectioned empty-query display ---
            if search_mode.show_files() {
                self.matches.push(Match::SectionHeader("Recent Files"));
                self.matches
                    .extend(history_items.into_iter().map(path_to_entry));
                self.matches.push(Match::NewFile);
            }

            if search_mode.show_threads() {
                // Add provider recent items with section headers
                for (provider_idx, provider) in providers.iter().enumerate() {
                    if provider.supports_mode(search_mode) {
                        self.matches.push(Match::SectionHeader(provider.section_label()));
                        for pmatch in provider.recent_items(cx) {
                            match pmatch.data {
                                ProviderMatchData::SectionHeader { label } => {
                                    self.matches.push(Match::SectionHeader(label));
                                }
                                ProviderMatchData::NewSession => {
                                    self.matches.push(Match::NewSession);
                                }
                                ProviderMatchData::CreateSession { query } => {
                                    self.matches.push(Match::CreateSession(query.to_string()));
                                }
                                ProviderMatchData::Thread { thread_id, session_id } => {
                                    self.matches.push(Match::Thread(ThreadMatch {
                                        thread_id,
                                        session_id,
                                        title: pmatch.label.clone(),
                                        worktree_paths: pmatch.worktree_paths.clone(),
                                        relative_time: pmatch.relative_time.clone(),
                                        score: pmatch.score,
                                        highlight_positions: pmatch.highlight_positions.clone(),
                                        provider_index: provider_idx,
                                    }));
                                }
                            }
                        }
                        // Note: The provider's recent_items() already includes a
                        // NewSession entry at the end, so we don't need to add
                        // another one here. Previously this caused a duplicate.
                    }
                }
            }

            return;
        };

        let worktree_name_by_id = if should_hide_root_in_entry_path(&worktree_store, cx) {
            None
        } else {
            Some(
                worktree_store
                    .read(cx)
                    .worktrees()
                    .map(|worktree| {
                        let snapshot = worktree.read(cx).snapshot();
                        (snapshot.id(), snapshot.root_name().into())
                    })
                    .collect(),
            )
        };
        let new_history_matches = matching_history_items(
            history_items,
            currently_opened,
            worktree_name_by_id,
            query,
            path_style,
        );
        let new_search_matches: Vec<Match> = new_search_matches
            .filter(|path_match| {
                !new_history_matches.contains_key(&ProjectPath {
                    path: path_match.0.path.clone(),
                    worktree_id: WorktreeId::from_usize(path_match.0.worktree_id),
                })
            })
            .map(Match::Search)
            .collect();

        if extend_old_matches {
            // since we take history matches instead of new search matches
            // and history matches has not changed(since the query has not changed and we do not extend old matches otherwise),
            // old matches can't contain paths present in history_matches as well.
            self.matches.retain(|m| matches!(m, Match::Search(_)));
        } else {
            self.matches.clear();
        }

        // At this point we have an unsorted set of new history matches, an unsorted set of new search matches
        // and a sorted set of old search matches.
        // It is possible that the new search matches' paths contain some of the old search matches' paths.
        // History matches' paths are unique, since store in a HashMap by path.
        // We build a sorted Vec<Match>, eliminating duplicate search matches.
        // Search matches with the same paths should have equal `ProjectPanelOrdMatch`, so we should
        // not have any duplicates after building the final list.
        for new_match in new_history_matches.into_values().chain(new_search_matches) {
            match self.position(&new_match, currently_opened) {
                Ok(_duplicate) => continue,
                Err(i) => {
                    self.matches.insert(i, new_match);
                    if self.matches.len() == 100 {
                        break;
                    }
                }
            }
        }
    }

    /// If a < b, then a is a worse match, aligning with the `ProjectPanelOrdMatch` ordering.
    fn cmp_matches(
        separate_history: bool,
        currently_opened: Option<&FoundPath>,
        a: &Match,
        b: &Match,
    ) -> cmp::Ordering {
        // Handle non-scoring variants that always go at extremes
        match (a, b) {
            // SectionHeaders are positioned by insertion, not by score — treat as equal
            (Match::SectionHeader(_), Match::SectionHeader(_)) => return cmp::Ordering::Equal,
            (Match::SectionHeader(_), _) => return cmp::Ordering::Greater, // headers are "better" (placed first in group)
            (_, Match::SectionHeader(_)) => return cmp::Ordering::Less,

            // CreateNew and CreateSession and NewSession always go at the very bottom
            (Match::CreateNew(_), Match::CreateNew(_)) => return cmp::Ordering::Equal,
            (Match::CreateSession(_), Match::CreateSession(_)) => return cmp::Ordering::Equal,
            (Match::NewSession, Match::NewSession) => return cmp::Ordering::Equal,
            (Match::NewFile, Match::NewFile) => return cmp::Ordering::Equal,
            (Match::CreateNew(_), _) => return cmp::Ordering::Less,
            (_, Match::CreateNew(_)) => return cmp::Ordering::Greater,
            (Match::CreateSession(_), _) => return cmp::Ordering::Less,
            (_, Match::CreateSession(_)) => return cmp::Ordering::Greater,
            (Match::NewSession, _) => return cmp::Ordering::Less,
            (_, Match::NewSession) => return cmp::Ordering::Greater,
            (Match::NewFile, _) => return cmp::Ordering::Less,
            (_, Match::NewFile) => return cmp::Ordering::Greater,

            _ => {}
        }

        // Bubble currently opened files to the top
        match (&a, &b) {
            (Match::History { path, .. }, _) if Some(path) == currently_opened => {
                return cmp::Ordering::Greater;
            }
            (_, Match::History { path, .. }) if Some(path) == currently_opened => {
                return cmp::Ordering::Less;
            }
            _ => {}
        }

        if separate_history {
            match (a, b) {
                (Match::History { .. }, Match::Search(_)) => return cmp::Ordering::Greater,
                (Match::Search(_), Match::History { .. }) => return cmp::Ordering::Less,
                _ => {}
            }
        }

        // For file-vs-file matches, use the existing detailed comparison.
        if let (Some(a_panel), Some(b_panel)) = (a.panel_match(), b.panel_match()) {
            return a_panel.cmp(b_panel);
        }

        // Thread-vs-thread: compare by thread score
        if let (Match::Thread(a_thread), Match::Thread(b_thread)) = (a, b) {
            return a_thread
                .score
                .partial_cmp(&b_thread.score)
                .unwrap_or(cmp::Ordering::Equal);
        }

        // Thread-vs-file or file-vs-thread: interleave by score.
        // File results with higher scores come first, then threads with
        // high scores, then lower-scored files, then lower-scored threads.
        let a_score = Self::match_score(a);
        let b_score = Self::match_score(b);
        a_score
            .partial_cmp(&b_score)
            .unwrap_or(cmp::Ordering::Equal)
    }

    fn match_score(m: &Match) -> f64 {
        match m {
            Match::History { panel_match, .. } => panel_match.as_ref().map_or(0.0, |pm| pm.0.score),
            Match::Search(pm) => pm.0.score,
            Match::CreateNew(_) => 0.0,
            Match::Thread(thread) => thread.score,
            Match::CreateSession(_) => 0.0,
            Match::SectionHeader(_) => 0.0,
            Match::NewSession => 0.0,
            Match::NewFile => 0.0,
        }
    }
}

fn matching_history_items<'a>(
    history_items: impl IntoIterator<Item = &'a FoundPath>,
    currently_opened: Option<&'a FoundPath>,
    worktree_name_by_id: Option<HashMap<WorktreeId, Arc<RelPath>>>,
    query: &FileSearchQuery,
    path_style: PathStyle,
) -> HashMap<ProjectPath, Match> {
    let mut candidates_paths = HashMap::default();

    let history_items_by_worktrees = history_items
        .into_iter()
        .chain(currently_opened)
        .map(|found_path| {
            // Only match history items names, otherwise their paths may match too many queries,
            // producing false positives. E.g. `foo` would match both `something/foo/bar.rs` and
            // `something/foo/foo.rs` and if the former is a history item, it would be shown first
            // always, despite the latter being a better match.
            let candidate = PathMatchCandidate::new(
                &found_path.project.path,
                false,
                worktree_name_by_id
                    .as_ref()
                    .and_then(|m| m.get(&found_path.project.worktree_id))
                    .map(|prefix| prefix.as_ref()),
            );
            candidates_paths.insert(&found_path.project, found_path);
            (found_path.project.worktree_id, candidate)
        })
        .fold(
            HashMap::default(),
            |mut candidates, (worktree_id, new_candidate)| {
                candidates
                    .entry(worktree_id)
                    .or_insert_with(Vec::new)
                    .push(new_candidate);
                candidates
            },
        );
    let mut matching_history_paths = HashMap::default();
    for (worktree, candidates) in history_items_by_worktrees {
        let max_results = candidates.len() + 1;
        let worktree_root_name = worktree_name_by_id
            .as_ref()
            .and_then(|w| w.get(&worktree).cloned());

        matching_history_paths.extend(
            fuzzy_nucleo::match_fixed_path_set(
                candidates,
                worktree.to_usize(),
                worktree_root_name,
                query.path_query(),
                fuzzy_nucleo::Case::Ignore,
                max_results,
                path_style,
            )
            .into_iter()
            // filter matches where at least one matched position is in filename portion, to prevent directory matches, nucleo scores them higher as history items are matched against their full path
            .filter(|path_match| {
                if let Some(filename) = path_match.path.file_name() {
                    let filename_start = path_match.path.as_unix_str().len() - filename.len();
                    path_match
                        .positions
                        .iter()
                        .any(|&pos| pos >= filename_start)
                } else {
                    true
                }
            })
            .filter_map(|path_match| {
                candidates_paths
                    .remove_entry(&ProjectPath {
                        worktree_id: WorktreeId::from_usize(path_match.worktree_id),
                        path: Arc::clone(&path_match.path),
                    })
                    .map(|(project_path, found_path)| {
                        (
                            project_path.clone(),
                            Match::History {
                                path: found_path.clone(),
                                panel_match: Some(ProjectPanelOrdMatch(path_match)),
                            },
                        )
                    })
            }),
        );
    }
    matching_history_paths
}

fn should_hide_root_in_entry_path(worktree_store: &Entity<WorktreeStore>, cx: &App) -> bool {
    let multiple_worktrees = worktree_store
        .read(cx)
        .visible_worktrees(cx)
        .filter(|worktree| !worktree.read(cx).is_single_file())
        .nth(1)
        .is_some();
    ProjectPanelSettings::get_global(cx).hide_root && !multiple_worktrees
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct FoundPath {
    project: ProjectPath,
    absolute: PathBuf,
}

impl FoundPath {
    fn new(project: ProjectPath, absolute: PathBuf) -> Self {
        Self { project, absolute }
    }
}

const MAX_RECENT_SELECTIONS: usize = 20;

pub enum Event {
    Selected(ProjectPath),
    Dismissed,
}

#[derive(Debug, Clone)]
struct FileSearchQuery {
    raw_query: String,
    file_query_end: Option<usize>,
    path_position: PathWithPosition,
}

impl FileSearchQuery {
    fn path_query(&self) -> &str {
        match self.file_query_end {
            Some(file_path_end) => &self.raw_query[..file_path_end],
            None => &self.raw_query,
        }
    }
}

impl FileFinderDelegate {
    fn new(
        file_finder: WeakEntity<FileFinder>,
        workspace: WeakEntity<Workspace>,
        project: Entity<Project>,
        currently_opened_path: Option<FoundPath>,
        history_items: Vec<FoundPath>,
        separate_history: bool,
        window: &mut Window,
        cx: &mut Context<FileFinder>,
    ) -> Self {
        Self::subscribe_to_updates(&project, window, cx);
        Self {
            file_finder,
            workspace,
            project,
            search_count: 0,
            latest_search_id: 0,
            latest_search_did_cancel: false,
            latest_search_query: None,
            currently_opened_path,
            matches: Matches::default(),
            has_changed_selected_index: false,
            selected_index: 0,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            history_items,
            separate_history,
            first_update: true,
            filter_popover_menu_handle: PopoverMenuHandle::default(),
            split_popover_menu_handle: PopoverMenuHandle::default(),
            focus_handle: cx.focus_handle(),
            include_ignored: FileFinderSettings::get_global(cx).include_ignored,
            include_ignored_refresh: Task::ready(()),
            search_mode: SearchMode::Unified,
            providers: finder_providers(cx),
        }
    }

    fn subscribe_to_updates(
        project: &Entity<Project>,
        window: &mut Window,
        cx: &mut Context<FileFinder>,
    ) {
        cx.subscribe_in(project, window, |file_finder, _, event, window, cx| {
            match event {
                project::Event::WorktreeUpdatedEntries(_, _)
                | project::Event::WorktreeAdded(_)
                | project::Event::WorktreeRemoved(_) => file_finder
                    .picker
                    .update(cx, |picker, cx| picker.refresh(window, cx)),
                _ => {}
            };
        })
        .detach();
    }

    /// Register a [`FinderProvider`] that contributes search results to the
    /// unified file finder.
    ///
    /// Prefer using [`register_finder_provider`] during application init instead;
    /// this method exists for programmatic registration after the finder is open.
    pub fn register_provider(&mut self, provider: Arc<dyn FinderProvider>) {
        self.providers.push(provider);
    }

    fn spawn_search(
        &mut self,
        query: FileSearchQuery,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Task<()> {
        let relative_to = self
            .currently_opened_path
            .as_ref()
            .map(|found_path| Arc::clone(&found_path.project.path));
        let worktree_store = self.project.read(cx).worktree_store();
        let worktrees = worktree_store
            .read(cx)
            .visible_worktrees_and_single_files(cx)
            .collect::<Vec<_>>();
        let include_root_name = !should_hide_root_in_entry_path(&worktree_store, cx);
        let candidate_sets = worktrees
            .into_iter()
            .map(|worktree| {
                let worktree = worktree.read(cx);
                PathMatchCandidateSet {
                    snapshot: worktree.snapshot(),
                    include_ignored: self.include_ignored.unwrap_or_else(|| {
                        worktree.root_entry().is_some_and(|entry| entry.is_ignored)
                    }),
                    include_root_name,
                    candidates: project::Candidates::Files,
                }
            })
            .collect::<Vec<_>>();

        let search_id = util::post_inc(&mut self.search_count);
        self.cancel_flag.store(true, atomic::Ordering::Release);
        self.cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_flag = self.cancel_flag.clone();
        cx.spawn_in(window, async move |picker, cx| {
            let matches = fuzzy_nucleo::match_path_sets(
                candidate_sets.as_slice(),
                query.path_query(),
                &relative_to,
                fuzzy_nucleo::Case::Ignore,
                100,
                &cancel_flag,
                cx.background_executor().clone(),
            )
            .await
            .into_iter()
            .map(ProjectPanelOrdMatch);
            let did_cancel = cancel_flag.load(atomic::Ordering::Acquire);
            picker
                .update(cx, |picker, cx| {
                    picker
                        .delegate
                        .set_search_matches(search_id, did_cancel, query, matches, cx)
                })
                .log_err();
        })
    }

    fn set_search_matches(
        &mut self,
        search_id: usize,
        did_cancel: bool,
        query: FileSearchQuery,
        matches: impl IntoIterator<Item = ProjectPanelOrdMatch>,
        cx: &mut Context<Picker<Self>>,
    ) {
        if search_id >= self.latest_search_id {
            self.latest_search_id = search_id;
            let query_changed = Some(query.path_query())
                != self
                    .latest_search_query
                    .as_ref()
                    .map(|query| query.path_query());
            let extend_old_matches = self.latest_search_did_cancel && !query_changed;

            let selected_match = if query_changed {
                None
            } else {
                self.matches.get(self.selected_index).cloned()
            };

            let path_style = self.project.read(cx).path_style(cx);
            self.matches.push_new_matches(
                self.project.read(cx).worktree_store(),
                cx,
                &self.history_items,
                self.currently_opened_path.as_ref(),
                Some(&query),
                matches.into_iter(),
                extend_old_matches,
                path_style,
                self.search_mode,
                &self.providers,
            );

            let query_path = query.raw_query.as_str();
            // Only add CreateNew file entry when showing files in search results
            if self.search_mode.show_files() {
                if let Ok(mut query_path) = RelPath::new(Path::new(query_path), path_style) {
                let available_worktree = self
                    .project
                    .read(cx)
                    .visible_worktrees(cx)
                    .filter(|worktree| !worktree.read(cx).is_single_file())
                    .collect::<Vec<_>>();
                let worktree_count = available_worktree.len();
                let mut expect_worktree = available_worktree.first().cloned();
                for worktree in &available_worktree {
                    let worktree_root = worktree.read(cx).root_name();
                    if worktree_count > 1 {
                        if let Ok(suffix) = query_path.strip_prefix(worktree_root) {
                            query_path = Cow::Owned(suffix.to_owned());
                            expect_worktree = Some(worktree.clone());
                            break;
                        }
                    }
                }

                if let Some(FoundPath { ref project, .. }) = self.currently_opened_path {
                    let worktree_id = project.worktree_id;
                    let focused_file_in_available_worktree = available_worktree
                        .iter()
                        .any(|wt| wt.read(cx).id() == worktree_id);

                    if focused_file_in_available_worktree {
                        expect_worktree = self.project.read(cx).worktree_for_id(worktree_id, cx);
                    }
                }

                if let Some(worktree) = expect_worktree {
                    let worktree = worktree.read(cx);
                    if worktree.entry_for_path(&query_path).is_none()
                        && !query.raw_query.ends_with("/")
                        && !(path_style.is_windows() && query.raw_query.ends_with("\\"))
                    {
                        self.matches.matches.push(Match::CreateNew(ProjectPath {
                            worktree_id: worktree.id(),
                            path: query_path.into_arc(),
                        }));
                    }
                }
            } // close if let Ok(mut query_path)
            } // close if self.search_mode.show_files()

            // --- NEW: Thread provider results ---
            // Query all registered providers for thread matches and insert
            // them into the results list, interleaved by score.
            if self.search_mode.show_threads() {
                let query_str = query.path_query();
                for (provider_idx, provider) in self.providers.iter().enumerate() {
                    if !provider.supports_mode(self.search_mode) {
                        continue;
                    }
                    let provider_matches = provider.search(query_str, cx);
                    for pmatch in provider_matches {
                        match pmatch.data {
                            ProviderMatchData::Thread { thread_id, session_id } => {
                                let thread_match = ThreadMatch {
                                    thread_id,
                                    session_id,
                                    title: pmatch.label.clone(),
                                    worktree_paths: pmatch.worktree_paths.clone(),
                                    relative_time: pmatch.relative_time.clone(),
                                    score: pmatch.score,
                                    highlight_positions: pmatch.highlight_positions.clone(),
                                    provider_index: provider_idx,
                                };
                                match self.matches.position(&Match::Thread(thread_match.clone()), self.currently_opened_path.as_ref()) {
                                    Ok(_) => continue, // duplicate
                                    Err(i) => {
                                        self.matches.matches.insert(i, Match::Thread(thread_match));
                                        if self.matches.len() == 100 {
                                            break;
                                        }
                                    }
                                }
                            }
                            // Non-thread provider match types are not inserted here;
                            // they appear in the empty-query sectioned display.
                            _ => {}
                        }
                    }
                }

                // Check if we need "Create from query" entries
                let _has_file_matches = self.matches.matches.iter().any(|m| {
                    matches!(m, Match::History { .. } | Match::Search(_))
                });
                let has_thread_matches = self.matches.matches.iter().any(|m| {
                    matches!(m, Match::Thread(_))
                });

                // When no file matches exist and mode shows files, CreateNew is
                // already added above.

                // When no thread matches exist and mode shows threads, offer
                // CreateSession from providers.
                if !has_thread_matches {
                    for provider in self.providers.iter() {
                        if let Some(pmatch) = provider.create_from_query(query_str) {
                            match pmatch.data {
                                ProviderMatchData::CreateSession { query } => {
                                    self.matches.matches.push(
                                        Match::CreateSession(query.to_string())
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            self.selected_index = selected_match.map_or_else(
                || self.calculate_selected_index(cx),
                |m| {
                    self.matches
                        .position(&m, self.currently_opened_path.as_ref())
                        .unwrap_or(0)
                },
            );

            self.latest_search_query = Some(query);
            self.latest_search_did_cancel = did_cancel;

            cx.notify();
        }
    }

    fn labels_for_match(
        &self,
        path_match: &Match,
        window: &mut Window,
        cx: &App,
    ) -> (HighlightedLabel, HighlightedLabel) {
        let path_style = self.project.read(cx).path_style(cx);
        let (file_name, file_name_positions, mut full_path, mut full_path_positions) =
            match &path_match {
                Match::History {
                    path: entry_path,
                    panel_match,
                } => {
                    let worktree_id = entry_path.project.worktree_id;
                    let worktree = self
                        .project
                        .read(cx)
                        .worktree_for_id(worktree_id, cx)
                        .filter(|worktree| worktree.read(cx).is_visible());

                    if let Some(panel_match) = panel_match {
                        self.labels_for_path_match(&panel_match.0, path_style)
                    } else if let Some(worktree) = worktree {
                        let worktree_store = self.project.read(cx).worktree_store();
                        let full_path = if should_hide_root_in_entry_path(&worktree_store, cx) {
                            entry_path.project.path.clone()
                        } else {
                            worktree.read(cx).root_name().join(&entry_path.project.path)
                        };
                        let mut components = full_path.components();
                        let filename = components.next_back().unwrap_or("");
                        let prefix = components.rest();
                        (
                            filename.to_string(),
                            Vec::new(),
                            prefix.display(path_style).to_string() + path_style.primary_separator(),
                            Vec::new(),
                        )
                    } else {
                        (
                            entry_path
                                .absolute
                                .file_name()
                                .map_or(String::new(), |f| f.to_string_lossy().into_owned()),
                            Vec::new(),
                            entry_path.absolute.parent().map_or(String::new(), |path| {
                                path.to_string_lossy().into_owned() + path_style.primary_separator()
                            }),
                            Vec::new(),
                        )
                    }
                }
                Match::Search(path_match) => self.labels_for_path_match(&path_match.0, path_style),
                Match::CreateNew(project_path) => (
                    format!("Create file: {}", project_path.path.display(path_style)),
                    vec![],
                    String::from(""),
                    vec![],
                ),
                // Thread-related variants are rendered directly in render_match
                // and never reach this function. Provide placeholder labels.
                Match::Thread(_) => (String::new(), vec![], String::new(), vec![]),
                Match::CreateSession(_) => (String::new(), vec![], String::new(), vec![]),
                Match::SectionHeader(_) => (String::new(), vec![], String::new(), vec![]),
                Match::NewSession => (String::new(), vec![], String::new(), vec![]),
                Match::NewFile => ("New File".to_string(), vec![], String::new(), vec![]),
            };

        if file_name_positions.is_empty() {
            let user_home_path = util::paths::home_dir().to_string_lossy();
            if !user_home_path.is_empty() && full_path.starts_with(&*user_home_path) {
                full_path.replace_range(0..user_home_path.len(), "~");
                full_path_positions.retain_mut(|pos| {
                    if *pos >= user_home_path.len() {
                        *pos -= user_home_path.len();
                        *pos += 1;
                        true
                    } else {
                        false
                    }
                })
            }
        }

        if full_path.is_ascii() {
            let file_finder_settings = FileFinderSettings::get_global(cx);
            let max_width =
                FileFinder::modal_max_width(file_finder_settings.modal_max_width, window);
            let (normal_em, small_em) = {
                let style = window.text_style();
                let font_id = window.text_system().resolve_font(&style.font());
                let font_size = TextSize::Default.rems(cx).to_pixels(window.rem_size());
                let normal = cx
                    .text_system()
                    .em_width(font_id, font_size)
                    .unwrap_or(px(16.));
                let font_size = TextSize::Small.rems(cx).to_pixels(window.rem_size());
                let small = cx
                    .text_system()
                    .em_width(font_id, font_size)
                    .unwrap_or(px(10.));
                (normal, small)
            };
            let budget = full_path_budget(&file_name, normal_em, small_em, max_width);
            // If the computed budget is zero, we certainly won't be able to achieve it,
            // so no point trying to elide the path.
            if budget > 0 && full_path.len() > budget {
                let components = PathComponentSlice::new(&full_path);
                if let Some(elided_range) =
                    components.elision_range(budget - 1, &full_path_positions)
                {
                    let elided_len = elided_range.end - elided_range.start;
                    let placeholder = "…";
                    full_path_positions.retain_mut(|mat| {
                        if *mat >= elided_range.end {
                            *mat -= elided_len;
                            *mat += placeholder.len();
                        } else if *mat >= elided_range.start {
                            return false;
                        }
                        true
                    });
                    full_path.replace_range(elided_range, placeholder);
                }
            }
        }

        (
            HighlightedLabel::new(file_name, file_name_positions),
            HighlightedLabel::new(full_path, full_path_positions)
                .size(LabelSize::Small)
                .color(Color::Muted),
        )
    }

    fn labels_for_path_match(
        &self,
        path_match: &PathMatch,
        path_style: PathStyle,
    ) -> (String, Vec<usize>, String, Vec<usize>) {
        let full_path = path_match.path_prefix.join(&path_match.path);
        let mut path_positions = path_match.positions.clone();

        let file_name = full_path.file_name().unwrap_or("");
        let file_name_start = full_path.as_unix_str().len() - file_name.len();
        let file_name_positions = path_positions
            .iter()
            .filter_map(|pos| {
                if pos >= &file_name_start {
                    Some(pos - file_name_start)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let full_path = full_path
            .display(path_style)
            .trim_end_matches(&file_name)
            .to_string();
        path_positions.retain(|idx| *idx < full_path.len());

        debug_assert!(
            file_name_positions
                .iter()
                .all(|ix| file_name[*ix..].chars().next().is_some()),
            "invalid file name positions {file_name:?} {file_name_positions:?}"
        );
        debug_assert!(
            path_positions
                .iter()
                .all(|ix| full_path[*ix..].chars().next().is_some()),
            "invalid path positions {full_path:?} {path_positions:?}"
        );

        (
            file_name.to_string(),
            file_name_positions,
            full_path,
            path_positions,
        )
    }

    /// Attempts to resolve an absolute file path and update the search matches if found.
    ///
    /// If the query path resolves to an absolute file that exists in the project,
    /// this method will find the corresponding worktree and relative path, create a
    /// match for it, and update the picker's search results.
    ///
    /// Returns `true` if the absolute path exists, otherwise returns `false`.
    fn lookup_absolute_path(
        &self,
        query: FileSearchQuery,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Task<bool> {
        cx.spawn_in(window, async move |picker, cx| {
            let Some(project) = picker
                .read_with(cx, |picker, _| picker.delegate.project.clone())
                .log_err()
            else {
                return false;
            };

            let query_path = Path::new(query.path_query());
            let mut path_matches = Vec::new();

            let abs_file_exists = project
                .update(cx, |this, cx| {
                    this.resolve_abs_file_path(query.path_query(), cx)
                })
                .await
                .is_some();

            if abs_file_exists {
                project.update(cx, |project, cx| {
                    if let Some((worktree, relative_path)) = project.find_worktree(query_path, cx) {
                        path_matches.push(ProjectPanelOrdMatch(PathMatch {
                            score: 1.0,
                            positions: Vec::new(),
                            worktree_id: worktree.read(cx).id().to_usize(),
                            path: relative_path,
                            path_prefix: RelPath::empty().into(),
                            is_dir: false, // File finder doesn't support directories
                            distance_to_relative_ancestor: usize::MAX,
                        }));
                    }
                });
            }

            picker
                .update_in(cx, |picker, _, cx| {
                    let picker_delegate = &mut picker.delegate;
                    let search_id = util::post_inc(&mut picker_delegate.search_count);
                    picker_delegate.set_search_matches(search_id, false, query, path_matches, cx);

                    anyhow::Ok(())
                })
                .log_err();
            abs_file_exists
        })
    }

    /// Skips first history match (that is displayed topmost) if it's currently opened.
    fn calculate_selected_index(&self, cx: &mut Context<Picker<Self>>) -> usize {
        if FileFinderSettings::get_global(cx).skip_focus_for_active_in_search
            && let Some(Match::History { path, .. }) = self.matches.get(0)
            && Some(path) == self.currently_opened_path.as_ref()
        {
            let elements_after_first = self.matches.len() - 1;
            if elements_after_first > 0 {
                return 1;
            }
        }

        0
    }

    fn key_context(&self, window: &Window, cx: &App) -> KeyContext {
        let mut key_context = KeyContext::new_with_defaults();
        key_context.add("FileFinder");

        if self.filter_popover_menu_handle.is_focused(window, cx) {
            key_context.add("filter_menu_open");
        }

        if self.split_popover_menu_handle.is_focused(window, cx) {
            key_context.add("split_menu_open");
        }
        key_context
    }

    /// Given a `ThreadMatch`, find the owning provider and reconstruct the
    /// corresponding `ProviderMatch` for delegation to `confirm()`.
    fn provider_match_for_thread(
        &self,
        thread_match: &ThreadMatch,
    ) -> Option<(&dyn FinderProvider, ProviderMatch)> {
        // Use the recorded provider_index to dispatch to the correct
        // provider, rather than iterating and guessing.
        let provider = self.providers.get(thread_match.provider_index)?;
        let pmatch = ProviderMatch {
            id: 0, // Not used for confirm
            label: thread_match.title.clone(),
            secondary_label: None,
            icon: Some(IconName::XenomorphicAssistant),
            icon_color: None,
            score: thread_match.score,
            highlight_positions: thread_match.highlight_positions.clone(),
            relative_time: thread_match.relative_time.clone(),
            worktree_paths: thread_match.worktree_paths.clone(),
            data: ProviderMatchData::Thread {
                thread_id: thread_match.thread_id.clone(),
                session_id: thread_match.session_id.clone(),
            },
        };
        Some((provider.as_ref(), pmatch))
    }
}

fn full_path_budget(
    file_name: &str,
    normal_em: Pixels,
    small_em: Pixels,
    max_width: Pixels,
) -> usize {
    (((max_width / 0.8) - file_name.len() * normal_em) / small_em) as usize
}

impl PickerDelegate for FileFinderDelegate {
    type ListItem = ListItem;

    fn placeholder_text(&self, _window: &mut Window, _cx: &mut App) -> Arc<str> {
        match self.search_mode {
            SearchMode::ThreadsOnly => "Search agent sessions...".into(),
            SearchMode::FilesOnly => "Search project files...".into(),
            SearchMode::Unified => "Search files and threads...".into(),
        }
    }

    fn match_count(&self) -> usize {
        self.matches.len()
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn can_select(
        &self,
        ix: usize,
        _window: &mut Window,
        _cx: &mut Context<Picker<Self>>,
    ) -> bool {
        self.matches.get(ix).map_or(false, |m| m.is_selectable())
    }

    fn set_selected_index(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Picker<Self>>) {
        self.has_changed_selected_index = true;
        self.selected_index = ix;
        cx.notify();
    }

    fn separators_after_indices(&self) -> Vec<usize> {
        let mut separators = Vec::new();

        if self.separate_history {
            let first_non_history_index = self
                .matches
                .matches
                .iter()
                .enumerate()
                .find(|(_, m)| !matches!(m, Match::History { .. }))
                .map(|(i, _)| i);
            if let Some(first_non_history_index) = first_non_history_index
                && first_non_history_index > 0
            {
                separators.push(first_non_history_index - 1);
            }
        }

        // Add separators after section headers (i.e. before each new section)
        // We put a separator *before* each SectionHeader (after the item at i-1)
        // except for the very first item.
        for (i, m) in self.matches.matches.iter().enumerate() {
            if matches!(m, Match::SectionHeader(_)) && i > 0 {
                separators.push(i - 1);
            }
        }

        separators
    }

    fn update_matches(
        &mut self,
        raw_query: String,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Task<()> {
        // --- NEW: Parse # and $ prefixes for search mode ---
        let (parsed_mode, trimmed_query) = SearchMode::from_query(raw_query.trim());
        self.search_mode = parsed_mode;
        let mut raw_query = trimmed_query.to_owned();

        // If not in FilesOnly mode, skip the a\\/b\\ prefix stripping since
        // the query might be a thread search, not a file path.
        if parsed_mode.show_files() {
            raw_query = match &raw_query.get(0..2) {
                Some(".\\" | "./") => raw_query[2..].to_owned(),
                Some(prefix @ ("a\\" | "a/" | "b\\" | "b/")) => {
                    if self
                        .workspace
                        .upgrade()
                        .into_iter()
                        .flat_map(|workspace| workspace.read(cx).worktrees(cx))
                        .all(|worktree| {
                            worktree
                                .read(cx)
                                .entry_for_path(RelPath::unix(prefix.split_at(1).0).unwrap())
                                .is_none_or(|entry| !entry.is_dir())
                        })
                    {
                        raw_query[2..].to_owned()
                    } else {
                        raw_query
                    }
                }
                _ => raw_query,
            };
        }

        if raw_query.is_empty() {
            // if there was no query before, and we already have some (history) matches
            // there's no need to update anything, since nothing has changed.
            // We also want to populate matches set from history entries on the first update.
            if self.latest_search_query.is_some() || self.first_update {
                let project = self.project.read(cx);

                self.latest_search_id = post_inc(&mut self.search_count);
                self.latest_search_query = None;
                self.matches = Matches {
                    separate_history: self.separate_history,
                    ..Matches::default()
                };
                let path_style = self.project.read(cx).path_style(cx);

                self.matches.push_new_matches(
                    project.worktree_store(),
                    cx,
                    self.history_items.iter().filter(|history_item| {
                        project
                            .worktree_for_id(history_item.project.worktree_id, cx)
                            .is_some()
                            || project.is_local()
                            || project.is_via_remote_server()
                    }),
                    self.currently_opened_path.as_ref(),
                    None,
                    None.into_iter(),
                    false,
                    path_style,
                    self.search_mode,
                    &self.providers,
                );

                self.first_update = false;
                self.selected_index = 0;
            }
            cx.notify();
            Task::ready(())
        } else {
            let path_position = PathWithPosition::parse_str(&raw_query);
            let raw_query = raw_query.trim().trim_end_matches(':').to_owned();
            let path = path_position.path.clone();
            let path_str = path_position.path.to_str();
            let path_trimmed = path_str.unwrap_or(&raw_query).trim_end_matches(':');
            let file_query_end = if path_trimmed == raw_query {
                None
            } else {
                // Safe to unwrap as we won't get here when the unwrap in if fails
                Some(path_str.unwrap().len())
            };

            let query = FileSearchQuery {
                raw_query,
                file_query_end,
                path_position,
            };

            let show_files = self.search_mode.show_files();
            let show_threads = self.search_mode.show_threads();

            cx.spawn_in(window, async move |this, cx| {
                let _ = maybe!(async move {
                    // Only search files if the mode includes files
                    if show_files {
                        let is_absolute_path = path.is_absolute();
                        let did_resolve_abs_path = is_absolute_path
                            && this
                                .update_in(cx, |this, window, cx| {
                                    this.delegate
                                        .lookup_absolute_path(query.clone(), window, cx)
                                })?
                                .await;

                        // Only check for relative paths if no absolute paths were
                        // found.
                        if !did_resolve_abs_path {
                            this.update_in(cx, |this, window, cx| {
                                this.delegate.spawn_search(query.clone(), window, cx)
                            })?
                            .await;
                        }
                    }

                    // Thread provider results are integrated inside set_search_matches
                    // which is called by the file search path. When in ThreadsOnly mode,
                    // we need to call set_search_matches with empty file matches to
                    // trigger thread search and scoring.
                    if show_threads && !show_files {
                        this.update_in(cx, |this, window, cx| {
                            let search_id = util::post_inc(&mut this.delegate.search_count);
                            this.delegate.set_search_matches(
                                search_id,
                                false,
                                query,
                                std::iter::empty(),
                                cx,
                            );
                        })?;
                    }

                    anyhow::Ok(())
                })
                .await;
            })
        }
    }

    fn confirm(
        &mut self,
        secondary: bool,
        window: &mut Window,
        cx: &mut Context<Picker<FileFinderDelegate>>,
    ) {
        if let Some(m) = self.matches.get(self.selected_index())
            && let Some(workspace) = self.workspace.upgrade()
        {
            // Handle provider-based matches (Thread, CreateSession, NewSession)
            // by delegating to the appropriate FinderProvider.
            match m {
                Match::Thread(thread_match) => {
                    // Find the provider that owns this thread and delegate confirm
                    let pmatch = self.provider_match_for_thread(thread_match);
                    if let Some((provider, pmatch)) = pmatch {
                        workspace.update(cx, |workspace, cx| {
                            provider.confirm(&pmatch, secondary, workspace, window, cx);
                        });
                    }
                    self.file_finder
                        .update(cx, |_, cx| cx.emit(DismissEvent))
                        .log_err();
                    return;
                }
                Match::CreateSession(query) => {
                    // Find a provider that supports CreateSession
                    for provider in &self.providers {
                        if let Some(pmatch) = provider.create_from_query(query) {
                            workspace.update(cx, |workspace, cx| {
                                provider.confirm(&pmatch, secondary, workspace, window, cx);
                            });
                            break;
                        }
                    }
                    self.file_finder
                        .update(cx, |_, cx| cx.emit(DismissEvent))
                        .log_err();
                    return;
                }
                Match::NewSession => {
                    // Find a provider that can handle NewSession by checking
                    // if it supports the current search mode AND can produce
                    // a create-new match for an empty query (which is the
                    // semantic equivalent of "New Session").
                    //
                    // We try `create_from_query("")` first; if the provider
                    // returns None, fall back to dispatching a synthetic
                    // `ProviderMatchData::NewSession` match directly.
                    for provider in &self.providers {
                        if !provider.supports_mode(self.search_mode) {
                            continue;
                        }
                        if let Some(pmatch) = provider.create_from_query("") {
                            workspace.update(cx, |workspace, cx| {
                                provider.confirm(&pmatch, secondary, workspace, window, cx);
                            });
                        } else {
                            // Provider doesn't implement create_from_query,
                            // but does support this mode. Use the synthetic match.
                            let pmatch = ProviderMatch {
                                id: 0,
                                label: "New Agent Session".into(),
                                secondary_label: None,
                                icon: Some(IconName::Plus),
                                icon_color: None,
                                score: 0.0,
                                highlight_positions: Vec::new(),
                                relative_time: None,
                                worktree_paths: None,
                                data: ProviderMatchData::NewSession,
                            };
                            workspace.update(cx, |workspace, cx| {
                                provider.confirm(&pmatch, secondary, workspace, window, cx);
                            });
                        }
                        break; // Only dispatch to the first matching provider
                    }
                    self.file_finder
                        .update(cx, |_, cx| cx.emit(DismissEvent))
                        .log_err();
                    return;
                }
                Match::SectionHeader(_) => {
                    // Non-selectable — should never be confirmed, but no-op if it happens
                    return;
                }
                Match::NewFile => {
                    // Dismiss the picker first, then dispatch the NewFile
                    // action so focus returns to the workspace before the
                    // action handler runs. Without this, App::dispatch_action
                    // fails with "window not found" because the picker modal
                    // holds focus.
                    self.file_finder
                        .update(cx, |_, cx| cx.emit(DismissEvent))
                        .log_err();
                    window.dispatch_action(NewFile.boxed_clone(), cx);
                    return;
                }
                _ => {} // File matches handled below
            }

            let open_task = workspace.update(cx, |workspace, cx| {
                let split_or_open =
                    |workspace: &mut Workspace,
                     project_path,
                     window: &mut Window,
                     cx: &mut Context<Workspace>| {
                        let allow_preview =
                            PreviewTabsSettings::get_global(cx).enable_preview_from_file_finder;
                        if secondary {
                            workspace.split_path_preview(
                                project_path,
                                allow_preview,
                                None,
                                window,
                                cx,
                            )
                        } else {
                            workspace.open_path_preview(
                                project_path,
                                None,
                                true,
                                allow_preview,
                                true,
                                window,
                                cx,
                            )
                        }
                    };
                match &m {
                    Match::CreateNew(project_path) => {
                        // Create a new file with the given filename
                        if secondary {
                            workspace.split_path_preview(
                                project_path.clone(),
                                false,
                                None,
                                window,
                                cx,
                            )
                        } else {
                            workspace.open_path_preview(
                                project_path.clone(),
                                None,
                                true,
                                false,
                                true,
                                window,
                                cx,
                            )
                        }
                    }

                    Match::History { path, .. } => {
                        let worktree_id = path.project.worktree_id;
                        if workspace
                            .project()
                            .read(cx)
                            .worktree_for_id(worktree_id, cx)
                            .is_some()
                        {
                            split_or_open(
                                workspace,
                                ProjectPath {
                                    worktree_id,
                                    path: Arc::clone(&path.project.path),
                                },
                                window,
                                cx,
                            )
                        } else if secondary {
                            workspace.split_abs_path(path.absolute.clone(), false, window, cx)
                        } else {
                            workspace.open_abs_path(
                                path.absolute.clone(),
                                OpenOptions {
                                    visible: Some(OpenVisible::None),
                                    ..Default::default()
                                },
                                window,
                                cx,
                            )
                        }
                    }
                    Match::Search(m) => split_or_open(
                        workspace,
                        ProjectPath {
                            worktree_id: WorktreeId::from_usize(m.0.worktree_id),
                            path: m.0.path.clone(),
                        },
                        window,
                        cx,
                    ),
                    // Thread, CreateSession, NewSession, SectionHeader, NewFile
                    // are all handled by early returns above, so this arm is
                    // unreachable. We enumerate the types explicitly so the
                    // compiler can verify exhaustiveness of the early returns.
                    Match::Thread(_)
                    | Match::CreateSession(_)
                    | Match::NewSession
                    | Match::SectionHeader(_)
                    | Match::NewFile => unreachable!(),
                }
            });

            let row = self
                .latest_search_query
                .as_ref()
                .and_then(|query| query.path_position.row)
                .map(|row| row.saturating_sub(1));
            let col = self
                .latest_search_query
                .as_ref()
                .and_then(|query| query.path_position.column)
                .unwrap_or(0)
                .saturating_sub(1);
            let finder = self.file_finder.clone();
            let workspace = self.workspace.clone();

            cx.spawn_in(window, async move |_, mut cx| {
                let item = open_task
                    .await
                    .notify_workspace_async_err(workspace, &mut cx)?;
                if let Some(row) = row
                    && let Some(active_editor) = item.downcast::<Editor>()
                {
                    active_editor
                        .downgrade()
                        .update_in(cx, |editor, window, cx| {
                            let Some(buffer) = editor.buffer().read(cx).as_singleton() else {
                                return;
                            };
                            let buffer_snapshot = buffer.read(cx).snapshot();
                            let point = buffer_snapshot.point_from_external_input(row, col);
                            editor.go_to_singleton_buffer_point(point, window, cx);
                        })
                        .log_err();
                }
                finder.update(cx, |_, cx| cx.emit(DismissEvent)).ok()?;

                Some(())
            })
            .detach();
        }
    }

    fn dismissed(&mut self, _: &mut Window, cx: &mut Context<Picker<FileFinderDelegate>>) {
        self.file_finder
            .update(cx, |_, cx| cx.emit(DismissEvent))
            .log_err();
    }

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let path_match = self.matches.get(ix)?;

        // --- NEW: Handle thread-related and section header matches ---
        match path_match {
            Match::SectionHeader(label) => {
                // Non-selectable section header rendered as ListSubHeader.
                // We return a ListItem that wraps a ListSubHeader visual,
                // but since ListSubHeader is a separate element type, we
                // render it as a special case.
                // Note: This returns a ListItem with `can_select` returning false,
                // so the picker will never select or confirm it.
                return Some(
                    ListItem::new(ix)
                        .spacing(ListItemSpacing::Sparse)
                        .inset(true)
                        .child(
                            ListSubHeader::new(*label)
                                .inset(true)
                                .toggle_state(selected),
                        ),
                );
            }
            Match::Thread(thread_match) => {
                // Thread result row with assistant icon, highlighted title,
                // worktree chips, and relative time.
                let title_label = HighlightedLabel::new(
                    thread_match.title.clone(),
                    thread_match.highlight_positions.clone(),
                );

                let end_slot = thread_match.relative_time.as_ref().map(|time| {
                    Label::new(time.clone())
                        .color(Color::Muted)
                        .size(LabelSize::Small)
                        .into_any_element()
                });

                return Some(
                    ListItem::new(ix)
                        .spacing(ListItemSpacing::Sparse)
                        .start_slot(
                            Icon::new(IconName::XenomorphicAssistant)
                                .color(Color::Accent)
                                .size(IconSize::Small),
                        )
                        .end_slot::<AnyElement>(end_slot)
                        .inset(true)
                        .toggle_state(selected)
                        .child(
                            h_flex()
                                .gap_2()
                                .py_px()
                                .child(title_label),
                        ),
                );
            }
            Match::CreateSession(query) => {
                return Some(
                    ListItem::new(ix)
                        .spacing(ListItemSpacing::Sparse)
                        .start_slot(
                            Icon::new(IconName::XenomorphicAssistant)
                                .color(Color::Muted)
                                .size(IconSize::Small),
                        )
                        .inset(true)
                        .toggle_state(selected)
                        .child(
                            h_flex()
                                .gap_2()
                                .py_px()
                                .child(Label::new(format!(
                                    "Start agent session: {}",
                                    query
                                ))),
                        ),
                );
            }
            Match::NewSession => {
                return Some(
                    ListItem::new(ix)
                        .spacing(ListItemSpacing::Sparse)
                        .start_slot(
                            Icon::new(IconName::Plus)
                                .color(Color::Accent)
                                .size(IconSize::Small),
                        )
                        .inset(true)
                        .toggle_state(selected)
                        .child(
                            h_flex()
                                .gap_2()
                                .py_px()
                                .child(
                                    Label::new("New Agent Session")
                                        .color(Color::Accent),
                                ),
                        ),
                );
            }
            Match::NewFile => {
                return Some(
                    ListItem::new(ix)
                        .spacing(ListItemSpacing::Sparse)
                        .start_slot(
                            Icon::new(IconName::Plus)
                                .color(Color::Accent)
                                .size(IconSize::Small),
                        )
                        .inset(true)
                        .toggle_state(selected)
                        .child(
                            h_flex()
                                .gap_2()
                                .py_px()
                                .child(
                                    Label::new("New File")
                                        .color(Color::Accent),
                                ),
                        ),
                );
            }
            _ => {} // File matches handled below
        }

        // --- Existing file match rendering ---
        let settings = FileFinderSettings::get_global(cx);

        let end_icon = match path_match {
            Match::History { .. } => Icon::new(IconName::HistoryRerun)
                .color(Color::Muted)
                .size(IconSize::Small)
                .into_any_element(),
            Match::Search(_) => v_flex()
                .flex_none()
                .size(IconSize::Small.rems())
                .into_any_element(),
            Match::CreateNew(_) => Icon::new(IconName::Plus)
                .color(Color::Muted)
                .size(IconSize::Small)
                .into_any_element(),
            Match::NewFile => Icon::new(IconName::Plus)
                .color(Color::Accent)
                .size(IconSize::Small)
                .into_any_element(),
            // Thread variants are handled above, but include fallback
            _ => v_flex()
                .flex_none()
                .size(IconSize::Small.rems())
                .into_any_element(),
        };
        let (file_name_label, full_path_label) = self.labels_for_match(path_match, window, cx);

        let file_icon = maybe!({
            if !settings.file_icons {
                return None;
            }
            let abs_path = path_match.abs_path(&self.project, cx)?;
            let file_name = abs_path.file_name()?;
            let icon = FileIcons::get_icon(file_name.as_ref(), cx)?;
            Some(Icon::from_path(icon).color(Color::Muted))
        });

        Some(
            ListItem::new(ix)
                .spacing(ListItemSpacing::Sparse)
                .start_slot::<Icon>(file_icon)
                .end_slot::<AnyElement>(end_icon)
                .inset(true)
                .toggle_state(selected)
                .child(
                    h_flex()
                        .gap_2()
                        .py_px()
                        .child(file_name_label)
                        .child(full_path_label),
                ),
        )
    }

    fn render_editor(
        &self,
        editor: &Arc<dyn ErasedEditor>,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Div {
        let has_search_query = self.latest_search_query.is_some();
        let is_project_scan_running = {
            let worktree_store = self.project.read(cx).worktree_store();
            !worktree_store.read(cx).initial_scan_completed()
        };

        h_flex()
            .flex_none()
            .h_9()
            .px_2p5()
            .justify_between()
            .border_b_1()
            .border_color(cx.theme().colors().border_variant)
            .child(editor.render(window, cx))
            .when(is_project_scan_running && has_search_query, |this| {
                this.child(
                    h_flex()
                        .id("project-scan-indicator")
                        .tooltip(Tooltip::text("Project Scan in Progress…"))
                        .child(
                            Icon::new(IconName::LoadCircle)
                                .color(Color::Accent)
                                .size(IconSize::Small)
                                .with_rotate_animation(2),
                        ),
                )
            })
    }

    fn render_footer(&self, _: &mut Window, cx: &mut Context<Picker<Self>>) -> Option<AnyElement> {
        let focus_handle = self.focus_handle.clone();

        Some(
            h_flex()
                .w_full()
                .p_1p5()
                .justify_between()
                .border_t_1()
                .border_color(cx.theme().colors().border_variant)
                .child(
                    PopoverMenu::new("filter-menu-popover")
                        .with_handle(self.filter_popover_menu_handle.clone())
                        .attach(gpui::Anchor::BottomRight)
                        .anchor(gpui::Anchor::BottomLeft)
                        .offset(gpui::Point {
                            x: px(1.0),
                            y: px(1.0),
                        })
                        .trigger_with_tooltip(
                            IconButton::new("filter-trigger", IconName::Sliders)
                                .icon_size(IconSize::Small)
                                .icon_size(IconSize::Small)
                                .toggle_state(self.include_ignored.unwrap_or(false))
                                .when(self.include_ignored.is_some(), |this| {
                                    this.indicator(Indicator::dot().color(Color::Info))
                                }),
                            {
                                let focus_handle = focus_handle.clone();
                                move |_window, cx| {
                                    Tooltip::for_action_in(
                                        "Filter Options",
                                        &ToggleFilterMenu,
                                        &focus_handle,
                                        cx,
                                    )
                                }
                            },
                        )
                        .menu({
                            let focus_handle = focus_handle.clone();
                            let include_ignored = self.include_ignored;

                            move |window, cx| {
                                Some(ContextMenu::build(window, cx, {
                                    let focus_handle = focus_handle.clone();
                                    move |menu, _, _| {
                                        menu.context(focus_handle.clone())
                                            .header("Filter Options")
                                            .toggleable_entry(
                                                "Include Ignored Files",
                                                include_ignored.unwrap_or(false),
                                                ui::IconPosition::End,
                                                Some(ToggleIncludeIgnored.boxed_clone()),
                                                move |window, cx| {
                                                    window.focus(&focus_handle, cx);
                                                    window.dispatch_action(
                                                        ToggleIncludeIgnored.boxed_clone(),
                                                        cx,
                                                    );
                                                },
                                            )
                                    }
                                }))
                            }
                        }),
                )
                .child(
                    h_flex()
                        .gap_0p5()
                        .child(
                            PopoverMenu::new("split-menu-popover")
                                .with_handle(self.split_popover_menu_handle.clone())
                                .attach(gpui::Anchor::BottomRight)
                                .anchor(gpui::Anchor::BottomLeft)
                                .offset(gpui::Point {
                                    x: px(1.0),
                                    y: px(1.0),
                                })
                                .trigger(
                                    ButtonLike::new("split-trigger")
                                        .child(Label::new("Split…"))
                                        .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                                        .child(
                                            KeyBinding::for_action_in(
                                                &ToggleSplitMenu,
                                                &focus_handle,
                                                cx,
                                            )
                                            .size(rems_from_px(12.)),
                                        ),
                                )
                                .menu({
                                    let focus_handle = focus_handle.clone();

                                    move |window, cx| {
                                        Some(ContextMenu::build(window, cx, {
                                            let focus_handle = focus_handle.clone();
                                            move |menu, _, _| {
                                                menu.context(focus_handle)
                                                    .action(
                                                        "Split Left",
                                                        pane::SplitLeft::default().boxed_clone(),
                                                    )
                                                    .action(
                                                        "Split Right",
                                                        pane::SplitRight::default().boxed_clone(),
                                                    )
                                                    .action(
                                                        "Split Up",
                                                        pane::SplitUp::default().boxed_clone(),
                                                    )
                                                    .action(
                                                        "Split Down",
                                                        pane::SplitDown::default().boxed_clone(),
                                                    )
                                            }
                                        }))
                                    }
                                }),
                        )
                        .child(
                            Button::new("open-selection", "Open")
                                .key_binding(
                                    KeyBinding::for_action_in(&menu::Confirm, &focus_handle, cx)
                                        .map(|kb| kb.size(rems_from_px(12.))),
                                )
                                .on_click(|_, window, cx| {
                                    window.dispatch_action(menu::Confirm.boxed_clone(), cx)
                                }),
                        ),
                )
                .into_any(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PathComponentSlice<'a> {
    path: Cow<'a, Path>,
    path_str: Cow<'a, str>,
    component_ranges: Vec<(Component<'a>, Range<usize>)>,
}

impl<'a> PathComponentSlice<'a> {
    fn new(path: &'a str) -> Self {
        let trimmed_path = Path::new(path).components().as_path().as_os_str();
        let mut component_ranges = Vec::new();
        let mut components = Path::new(trimmed_path).components();
        let len = trimmed_path.as_encoded_bytes().len();
        let mut pos = 0;
        while let Some(component) = components.next() {
            component_ranges.push((component, pos..0));
            pos = len - components.as_path().as_os_str().as_encoded_bytes().len();
        }
        for ((_, range), ancestor) in component_ranges
            .iter_mut()
            .rev()
            .zip(Path::new(trimmed_path).ancestors())
        {
            range.end = ancestor.as_os_str().as_encoded_bytes().len();
        }
        Self {
            path: Cow::Borrowed(Path::new(path)),
            path_str: Cow::Borrowed(path),
            component_ranges,
        }
    }

    fn elision_range(&self, budget: usize, matches: &[usize]) -> Option<Range<usize>> {
        let eligible_range = {
            assert!(matches.windows(2).all(|w| w[0] <= w[1]));
            let mut matches = matches.iter().copied().peekable();
            let mut longest: Option<Range<usize>> = None;
            let mut cur = 0..0;
            let mut seen_normal = false;
            for (i, (component, range)) in self.component_ranges.iter().enumerate() {
                let is_normal = matches!(component, Component::Normal(_));
                let is_first_normal = is_normal && !seen_normal;
                seen_normal |= is_normal;
                let is_last = i == self.component_ranges.len() - 1;
                let contains_match = matches.peek().is_some_and(|mat| range.contains(mat));
                if contains_match {
                    matches.next();
                }
                if is_first_normal || is_last || !is_normal || contains_match {
                    if longest
                        .as_ref()
                        .is_none_or(|old| old.end - old.start <= cur.end - cur.start)
                    {
                        longest = Some(cur);
                    }
                    cur = i + 1..i + 1;
                } else {
                    cur.end = i + 1;
                }
            }
            if longest
                .as_ref()
                .is_none_or(|old| old.end - old.start <= cur.end - cur.start)
            {
                longest = Some(cur);
            }
            longest
        };

        let eligible_range = eligible_range?;
        assert!(eligible_range.start <= eligible_range.end);
        if eligible_range.is_empty() {
            return None;
        }

        let elided_range: Range<usize> = {
            let byte_range = self.component_ranges[eligible_range.start].1.start
                ..self.component_ranges[eligible_range.end - 1].1.end;
            let midpoint = self.path_str.len() / 2;
            let distance_from_start = byte_range.start.abs_diff(midpoint);
            let distance_from_end = byte_range.end.abs_diff(midpoint);
            let pick_from_end = distance_from_start > distance_from_end;
            let mut len_with_elision = self.path_str.len();
            let mut i = eligible_range.start;
            while i < eligible_range.end {
                let x = if pick_from_end {
                    eligible_range.end - i + eligible_range.start - 1
                } else {
                    i
                };
                len_with_elision -= self.component_ranges[x]
                    .0
                    .as_os_str()
                    .as_encoded_bytes()
                    .len()
                    + 1;
                if len_with_elision <= budget {
                    break;
                }
                i += 1;
            }
            if len_with_elision > budget {
                return None;
            } else if pick_from_end {
                let x = eligible_range.end - i + eligible_range.start - 1;
                x..eligible_range.end
            } else {
                let x = i;
                eligible_range.start..x + 1
            }
        };

        let byte_range = self.component_ranges[elided_range.start].1.start
            ..self.component_ranges[elided_range.end - 1].1.end;
        Some(byte_range)
    }
}
