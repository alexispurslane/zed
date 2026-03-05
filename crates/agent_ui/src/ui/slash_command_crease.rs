use gpui::{App, IntoElement, RenderOnce, SharedString, Window};
use ui::{
    ButtonLike, ButtonSize, ButtonStyle, Color, Icon, IconName, IconSize, Label, TintColor,
    prelude::*,
};

/// A crease/bubble UI component for slash commands in the message editor.
/// Renders as a tinted button with a slash icon, command name, and optional argument hint.
#[derive(IntoElement)]
pub struct SlashCommandCrease {
    id: ElementId,
    command_name: SharedString,
    argument_hint: Option<SharedString>,
    is_loading: bool,
}

impl SlashCommandCrease {
    pub fn new(id: impl Into<ElementId>, command_name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            command_name: command_name.into(),
            argument_hint: None,
            is_loading: false,
        }
    }

    pub fn argument_hint(mut self, hint: Option<SharedString>) -> Self {
        self.argument_hint = hint;
        self
    }

    pub fn is_loading(mut self, loading: bool) -> Self {
        self.is_loading = loading;
        self
    }
}

impl RenderOnce for SlashCommandCrease {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        ButtonLike::new(self.id)
            .style(ButtonStyle::Tinted(TintColor::Accent))
            .size(ButtonSize::Compact)
            .child(
                h_flex()
                    .gap_1()
                    .child(Icon::new(IconName::Slash).size(IconSize::XSmall))
                    .child(Label::new(self.command_name.clone()))
                    .when_some(self.argument_hint, |this, hint| {
                        this.child(Label::new(format!(" {}", hint)).color(Color::Muted))
                    }),
            )
            .when(self.is_loading, |this| {
                this.child(Icon::new(IconName::ArrowCircle).size(IconSize::XSmall))
            })
    }
}
