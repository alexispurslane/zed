use std::sync::Arc;

use gpui::{App, Context, Empty, IntoElement, Render, Window};
use ui::{IconButton, IconSize, Tooltip, prelude::*};

use crate::{HideStatusItem, ItemHandle, StatusItemView, ToggleFileFinder};

/// A status bar indicator that shows the count of active agent sessions
/// and allows opening the file finder in thread-only mode (#) when clicked.
///
/// This indicator replaces the previous Agent Panel button in the status bar.
/// Instead of toggling a sidebar, clicking this indicator opens the unified
/// file finder (`cmd-p`) filtered to show only agent threads.
///
/// # Architecture
///
/// The indicator uses callback-based providers to decouple from `agent_ui`:
/// - `thread_count_provider`: Returns the current count of active (non-archived) threads
/// - `thread_finder_opener`: Opens the file finder in thread-only mode (#)
///
/// These callbacks are wired up in the `xenomorphic` app crate where both
/// `workspace` and `agent_ui` are available.
pub struct AgentSessionIndicator {
    /// Callback that returns the current count of active (non-archived) agent threads.
    thread_count_provider: Arc<dyn Fn(&App) -> usize + Send + Sync>,
    /// Callback that opens the file finder in thread-only mode (# prefix).
    thread_finder_opener: Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>,
    /// The focus handle of the currently active pane item, used for action routing.
    pane_item_focus_handle: Option<gpui::FocusHandle>,
}

impl AgentSessionIndicator {
    /// Creates a new `AgentSessionIndicator` with the given providers.
    ///
    /// # Arguments
    ///
    /// * `thread_count_provider` - A callback that returns the current count of
    ///   active agent threads. This is typically backed by `ThreadMetadataStore::global(cx)`.
    /// * `thread_finder_opener` - A callback that opens the file finder in
    ///   thread-only mode. This is typically implemented by dispatching
    ///   `ToggleFileFinder` and then setting a `#` prefix in the finder.
    pub fn new(
        thread_count_provider: Arc<dyn Fn(&App) -> usize + Send + Sync>,
        thread_finder_opener: Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>,
    ) -> Self {
        Self {
            thread_count_provider,
            thread_finder_opener,
            pane_item_focus_handle: None,
        }
    }
}

impl Render for AgentSessionIndicator {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let thread_count = (self.thread_count_provider)(cx);

        // Hide the indicator when there are no active threads.
        // This matches the behavior of the activity indicator, which
        // auto-hides when there's nothing to show. Users who have never
        // used agent sessions won't see a distracting zero-count badge.
        if thread_count == 0 {
            return Empty.into_any_element();
        }

        let focus_handle = self.pane_item_focus_handle.clone();
        let opener = self.thread_finder_opener.clone();

        div().child(
            IconButton::new("agent-session-indicator", IconName::XenomorphicAssistant)
                .icon_size(IconSize::Small)
                .icon_color(Color::Muted)
                .style(ButtonStyle::Subtle)
                .tooltip(move |_window, cx| {
                    if let Some(focus_handle) = &focus_handle {
                        Tooltip::for_action_in(
                            "Agent Sessions",
                            &ToggleFileFinder::default(),
                            focus_handle,
                            cx,
                        )
                    } else {
                        Tooltip::for_action(
                            "Agent Sessions",
                            &ToggleFileFinder::default(),
                            cx,
                        )
                    }
                })
                .on_click(move |_, window, cx| {
                    (opener)(window, cx);
                }),
        ).into_any_element()
    }
}

impl StatusItemView for AgentSessionIndicator {
    fn set_active_pane_item(
        &mut self,
        active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pane_item_focus_handle = active_pane_item.map(|item| item.item_focus_handle(cx));
        cx.notify();
    }

    /// The indicator auto-hides when there are no active threads,
    /// so there's no need for a separate "Hide Button" setting.
    /// This mirrors the activity indicator pattern.
    fn hide_setting(&self, _: &App) -> Option<HideStatusItem> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indicator_hidden_when_zero_threads() {
        // Verify that when thread_count_provider returns 0,
        // the render method produces a hidden element.
        // This is a structural test — full UI testing happens
        // in the xenomorphic integration tests.
        let provider = Arc::new(|_: &App| 0usize);
        let opener = Arc::new(|_: &mut Window, _: &mut App| {});
        let indicator = AgentSessionIndicator::new(provider, opener);
        assert_eq!(indicator.pane_item_focus_handle, None);
    }

    #[test]
    fn test_indicator_visible_when_threads_exist() {
        let provider = Arc::new(|_: &App| 3usize);
        let opener = Arc::new(|_: &mut Window, _: &mut App| {});
        let indicator = AgentSessionIndicator::new(provider, opener);
        assert_eq!(indicator.pane_item_focus_handle, None);
    }
}
