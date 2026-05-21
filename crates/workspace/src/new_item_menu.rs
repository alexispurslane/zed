//! Extensible registry for the "+" button menu in tab bars.
//!
//! Crates that want to add items to the "New..." popover menu (e.g. "New
//! Agent Thread") register a [`NewItemMenuEntry`] with this registry, without
//! the `workspace` crate depending on those crates.

use std::sync::Arc;

use gpui::App;

/// A single entry in the "+" button's context menu.
pub struct NewItemMenuEntry {
    /// The label shown in the context menu.
    pub label: &'static str,
    /// The action to dispatch when the entry is selected.
    /// This is a boxed action that will be dispatched on the focused pane.
    pub action: Box<dyn gpui::Action>,
}

struct NewItemMenuRegistry {
    entries: Vec<NewItemMenuEntry>,
}

impl Default for NewItemMenuRegistry {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

impl gpui::Global for NewItemMenuRegistry {}

/// Register an extra entry to appear in the "+" button context menu.
///
/// Call this during your crate's `init()` function. For example,
/// `agent_ui::init()` registers a "New Agent Thread" entry.
pub fn register_new_item_menu_entry(entry: NewItemMenuEntry, cx: &mut App) {
    if cx.try_global::<NewItemMenuRegistry>().is_none() {
        cx.set_global(NewItemMenuRegistry::default());
    }
    let registry = cx.global_mut::<NewItemMenuRegistry>();
    registry.entries.push(entry);
}

/// Returns all registered extra entries for the "+" button menu.
///
/// Called from the pane's tab bar rendering code.
pub fn new_item_menu_entries(cx: &App) -> Vec<&NewItemMenuEntry> {
    cx.try_global::<NewItemMenuRegistry>()
        .map(|r| r.entries.iter().collect())
        .unwrap_or_default()
}
