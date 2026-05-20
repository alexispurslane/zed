use gpui::{Empty, Render};
use ui::prelude::*;

/// Stub: auto-update was removed. This component now renders nothing.
pub struct UpdateVersion;

impl UpdateVersion {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self
    }

    pub fn show_update_in_menu_bar(&self) -> bool {
        false
    }
}

impl Render for UpdateVersion {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Empty.into_any_element()
    }
}
