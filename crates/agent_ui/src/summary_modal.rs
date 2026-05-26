use agent::SummaryProgress;
use gpui::{
    DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, Render, ScrollHandle,
    prelude::*,
};
use markdown::{Markdown, MarkdownElement, MarkdownFont, MarkdownStyle};
use ui::{KeyBinding, Modal, ModalFooter, ModalHeader, Section, SectionHeader, WithScrollbar, prelude::*};
use workspace::ModalView;

/// A modal that displays the live-updating output of a thread summary generation.
/// Shows the partial text as it streams in, with a token count and status indicator.
pub struct SummaryModal {
    progress: Entity<SummaryProgress>,
    markdown: Entity<Markdown>,
    thread_title: SharedString,
    scroll_handle: ScrollHandle,
    focus_handle: FocusHandle,
}

impl SummaryModal {
    pub fn new(
        progress: Entity<SummaryProgress>,
        thread_title: SharedString,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        let initial_text = progress.read(cx).partial_text.clone();
        let markdown = cx.new(|cx| Markdown::new(initial_text.into(), None, None, cx));

        cx.observe(&progress, |this, progress, cx| {
            let text = progress.read(cx).partial_text.clone();
            this.markdown.update(cx, |md, cx| {
                md.replace(SharedString::from(text), cx);
            });
            cx.notify();
        })
        .detach();

        Self {
            progress,
            markdown,
            thread_title,
            scroll_handle: ScrollHandle::new(),
            focus_handle: cx.focus_handle(),
        }
    }

    fn cancel(&mut self, _: &menu::Cancel, cx: &mut gpui::Context<Self>) {
        cx.emit(DismissEvent);
    }
}

impl ModalView for SummaryModal {
    fn fade_out_background(&self) -> bool {
        true
    }
}

impl EventEmitter<DismissEvent> for SummaryModal {}

impl Focusable for SummaryModal {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SummaryModal {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let progress = self.progress.read(cx);
        let is_complete = progress.is_complete;
        let has_error = progress.error.is_some();
        let token_count = progress.output_tokens;

        let status_text = if has_error {
            "Error".to_string()
        } else if is_complete {
            "Complete".to_string()
        } else {
            format!("Generating\u{2026} {} tokens", token_count)
        };

        let status_color = if has_error {
            Color::Error
        } else if is_complete {
            Color::Success
        } else {
            Color::Muted
        };

        let markdown_style = MarkdownStyle::themed(MarkdownFont::Agent, window, cx);

        let title = self.thread_title.clone();
        let focus_handle = self.focus_handle.clone();

        div()
            .elevation_3(cx)
            .w(rems(40.))
            .max_h(vh(0.85, window))
            .key_context("SummaryModal")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &menu::Cancel, _window, cx| {
                this.cancel(&menu::Cancel, cx)
            }))
            .capture_any_mouse_down(cx.listener(|this, _, window, cx| {
                this.focus_handle(cx).focus(window, cx);
            }))
            .child(
                Modal::new("summary-modal", None)
                    .header(
                        ModalHeader::new()
                            .headline(format!("Summary: {}", title))
                            .show_dismiss_button(true),
                    )
                    .section(
                        Section::new()
                            .header(SectionHeader::new("Status").end_slot(
                                Label::new(status_text)
                                    .size(LabelSize::Small)
                                    .color(status_color),
                            ))
                            .child(
                                div()
                                    .size_full()
                                    .child(
                                        div()
                                            .id("summary-content")
                                            .max_h(vh(0.7, window))
                                            .overflow_y_scroll()
                                            .track_scroll(&self.scroll_handle)
                                            .p_2()
                                            .child(
                                                div()
                                                    .max_w_full()
                                                    .overflow_x_hidden()
                                                    .child(
                                                        MarkdownElement::new(
                                                            self.markdown.clone(),
                                                            markdown_style,
                                                        ),
                                                    ),
                                            )
                                            .vertical_scrollbar_for(
                                                &self.scroll_handle,
                                                window,
                                                cx,
                                            ),
                                    ),
                            ),
                    )
                    .footer(
                        ModalFooter::new().end_slot(
                            Button::new("dismiss", if is_complete || has_error { "Dismiss" } else { "Cancel" })
                                .key_binding(
                                    KeyBinding::for_action_in(
                                        &menu::Cancel,
                                        &focus_handle,
                                        cx,
                                    )
                                    .map(|kb| kb.size(rems_from_px(12.))),
                                )
                                .on_click(cx.listener(|_this, _, _window, cx| {
                                    cx.emit(DismissEvent);
                                })),
                        ),
                    ),
            )
    }
}
