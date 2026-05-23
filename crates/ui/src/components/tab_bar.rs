use gpui::{AnyElement, ScrollHandle};
use smallvec::SmallVec;

use crate::Tab;
use crate::prelude::*;

#[derive(IntoElement, RegisterComponent)]
pub struct TabBar {
    id: ElementId,
    start_children: SmallVec<[AnyElement; 2]>,
    children: SmallVec<[AnyElement; 2]>,
    end_children: SmallVec<[AnyElement; 2]>,
    scroll_handle: Option<ScrollHandle>,
}

impl TabBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            start_children: SmallVec::new(),
            children: SmallVec::new(),
            end_children: SmallVec::new(),
            scroll_handle: None,
        }
    }

    pub fn track_scroll(mut self, scroll_handle: &ScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle.clone());
        self
    }

    pub fn start_children_mut(&mut self) -> &mut SmallVec<[AnyElement; 2]> {
        &mut self.start_children
    }

    pub fn start_child(mut self, start_child: impl IntoElement) -> Self
    where
        Self: Sized,
    {
        self.start_children_mut()
            .push(start_child.into_element().into_any());
        self
    }

    pub fn start_children(
        mut self,
        start_children: impl IntoIterator<Item = impl IntoElement>,
    ) -> Self
    where
        Self: Sized,
    {
        self.start_children_mut().extend(
            start_children
                .into_iter()
                .map(|child| child.into_any_element()),
        );
        self
    }

    pub fn end_children_mut(&mut self) -> &mut SmallVec<[AnyElement; 2]> {
        &mut self.end_children
    }

    pub fn end_child(mut self, end_child: impl IntoElement) -> Self
    where
        Self: Sized,
    {
        self.end_children_mut()
            .push(end_child.into_element().into_any());
        self
    }

    pub fn end_children(mut self, end_children: impl IntoIterator<Item = impl IntoElement>) -> Self
    where
        Self: Sized,
    {
        self.end_children_mut().extend(
            end_children
                .into_iter()
                .map(|child| child.into_any_element()),
        );
        self
    }
}

impl ParentElement for TabBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for TabBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // Check if the tab content overflows and requires scrolling.
        // We render end_children in one of two ways:
        //   - Inline: inside #tabs as a sibling after the tab content, right next to the last tab
        //   - Overlay: absolutely positioned on the right with a background (when scrollable)
        // Only one is rendered at a time based on the scroll handle state.
        let is_scrollable = self
            .scroll_handle
            .as_ref()
            .map_or(false, |handle| handle.max_offset().x > px(2.0));

        let has_end_children = !self.end_children.is_empty();

        let tabs = h_flex()
            .id("tabs")
            .flex_grow()
            .overflow_x_scroll();

        // Build the tabs container, placing end_children either inline (inside #tabs)
        // or as an overlay (absolute right). Can't use two .when() closures because
        // both would move self.end_children, so we branch once.
        let tabs_container = if !has_end_children {
            // No end_children at all — just the tabs.
            div()
                .relative()
                .flex_1()
                .h_full()
                .overflow_x_hidden()
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .border_b_1()
                        .border_color(cx.theme().colors().border),
                )
                .child(tabs.children(self.children))
        } else if is_scrollable {
            // Scrollable — overlay end_children on the right with opaque background.
            div()
                .relative()
                .flex_1()
                .h_full()
                .overflow_x_hidden()
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .border_b_1()
                        .border_color(cx.theme().colors().border),
                )
                .child(
                    tabs.children(self.children)
                        // Spacer: invisible element at the end of the scrollable
                        // tabs area that matches the overlay's width, so that when
                        // scrolled all the way right, the last tab isn't hidden
                        // under the overlay.
                        .child(
                            h_flex()
                                .flex_none()
                                .gap(DynamicSpacing::Base04.rems(cx))
                                .px(DynamicSpacing::Base06.rems(cx)),
                        ),
                )
                .child(
                    h_flex()
                        .absolute()
                        .right_0()
                        .top_0()
                        .h_full()
                        .flex_none()
                        .gap(DynamicSpacing::Base04.rems(cx))
                        .px(DynamicSpacing::Base06.rems(cx))
                        .bg(cx.theme().colors().tab_bar_background)
                        .border_color(cx.theme().colors().border)
                        .border_b_1()
                        .border_l_1()
                        .children(self.end_children),
                )
        } else {
            // Not scrollable — inline end_children inside #tabs,
            // right next to the last tab.
            div()
                .relative()
                .flex_1()
                .h_full()
                .overflow_x_hidden()
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .border_b_1()
                        .border_color(cx.theme().colors().border),
                )
                .child(
                    tabs.children(self.children)
                        .child(
                            h_flex()
                                .flex_none()
                                .gap(DynamicSpacing::Base04.rems(cx))
                                .children(self.end_children),
                        ),
                )
        };

        div()
            .id(self.id)
            .group("tab_bar")
            .flex()
            .flex_none()
            .w_full()
            .h(Tab::container_height(cx))
            .bg(cx.theme().colors().tab_bar_background)
            .when(!self.start_children.is_empty(), |this| {
                this.child(
                    h_flex()
                        .flex_none()
                        .gap(DynamicSpacing::Base04.rems(cx))
                        .px(DynamicSpacing::Base06.rems(cx))
                        .border_b_1()
                        .border_r_1()
                        .border_color(cx.theme().colors().border)
                        .children(self.start_children),
                )
            })
            .child(tabs_container)
    }
}

impl Component for TabBar {
    fn scope() -> ComponentScope {
        ComponentScope::Navigation
    }

    fn name() -> &'static str {
        "TabBar"
    }

    fn description() -> Option<&'static str> {
        Some("A horizontal bar containing tabs for navigation between different views or sections.")
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        Some(
            v_flex()
                .gap_6()
                .children(vec![
                    example_group_with_title(
                        "Basic Usage",
                        vec![
                            single_example(
                                "Empty TabBar",
                                TabBar::new("empty_tab_bar").into_any_element(),
                            ),
                            single_example(
                                "With Tabs",
                                TabBar::new("tab_bar_with_tabs")
                                    .child(Tab::new("tab1"))
                                    .child(Tab::new("tab2"))
                                    .child(Tab::new("tab3"))
                                    .into_any_element(),
                            ),
                        ],
                    ),
                    example_group_with_title(
                        "With Start and End Children",
                        vec![single_example(
                            "Full TabBar",
                            TabBar::new("full_tab_bar")
                                .start_child(Button::new("start_button", "Start"))
                                .child(Tab::new("tab1"))
                                .child(Tab::new("tab2"))
                                .child(Tab::new("tab3"))
                                .end_child(Button::new("end_button", "End"))
                                .into_any_element(),
                        )],
                    ),
                ])
                .into_any_element(),
        )
    }
}
