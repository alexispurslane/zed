use gpui::App;
use web_search::WebSearchRegistry;

pub fn init(_cx: &mut App) {
    // Cloud web search provider removed — direct-to-provider web search
    // can be added here in the future.
    let _ = WebSearchRegistry::global;
}
