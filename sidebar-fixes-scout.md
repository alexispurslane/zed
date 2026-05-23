# Sidebar Fixes — Scout Findings

## 1. macOS Traffic Light Padding

**Constant:** `crates/ui/src/utils/constants.rs:9-12`

```rust
#[cfg(macos_sdk_26)]
pub const TRAFFIC_LIGHT_PADDING: f32 = 78.;
#[cfg(not(macos_sdk_26))]
pub const TRAFFIC_LIGHT_PADDING: f32 = 71.;
```

**Usage pattern** (from `crates/platform_title_bar/src/platform_title_bar.rs:238` and `crates/agent_ui/src/threads_archive_view.rs:851-852`):

```rust
// Condition: macOS + not fullscreen + sidebar is on the left
let traffic_lights = cfg!(target_os = "macos") && not_fullscreen && sidebar_on_left;

// Apply padding:
if traffic_lights {
    this.pl(px(ui::utils::TRAFFIC_LIGHT_PADDING))
} else {
    this.pl_1p5()  // normal padding when no traffic lights
}
```

**Key point:** The padding is applied as `pl(px(TRAFFIC_LIGHT_PADDING))` on the header row of a left-side panel, only when on macOS, not fullscreen, and no other sidebar is stealing the left dock. In the workspace sidebar, you'd apply this to the header `h_flex()` in the `Render` impl.

---

## 2. Workspace-Appropriate Icon Names (from `crates/icons/src/icons.rs`)

The `IconName` enum is the full list. Icons that could represent a workspace/space/screen (NOT directional arrows):

| IconName | Notes |
|---|---|
| `Screen` | Best match for a workspace/screen metaphor |
| `Space` | Directly evokes "workspace/space" |
| `Circle` | Generic indicator dot |
| `Indicator` | Small status indicator |
| `SquareDot` | Filled square with dot — could be workspace |
| `SquarePlus` | Could represent "add workspace" |
| `SquareMinus` | Remove workspace |
| `Dot` | *(not present — no `Dot` icon)* |
| `Thread` | Could represent an active context |
| `Blocks` | Multiple blocks — workspace-like |
| `Box` | Generic container/workspace |
| `BoxOpen` | Open workspace |
| `LoadCircle` | Circular indicator |
| `Sparkle` | Active/special item |
| `Folder` / `FolderOpen` | Already used for project groups; could reuse at different size |
| `Library` | Collection/container metaphor |

**Recommendation:** Use `Screen` or `Space` as the workspace icon. Avoid `Tab` (it looks like `→|`).

---

## 3. Section Headers / Visual Dividers in Panels

**Best option: `ListSubHeader` component** — `crates/ui/src/components/list/list_sub_header.rs`

```rust
// Usage:
ListSubHeader::new("Section Title")
    .left_icon(Some(IconName::Folder))
    .inset(true)  // adds inner padding
```

Its render implementation uses:
- `h_flex()` container with `pb(DynamicSpacing::Base04.rems(cx))` and `px(DynamicSpacing::Base02.rems(cx))`
- Inner `div()` with `h_5()`, optional `.px_2()` for inset, optional `.bg(ghost_element_selected)` for selected state
- `Label` with `Color::Muted` and `LabelSize::Small`
- `start_slot` icon rendered as `Icon::new(i).color(Color::Muted).size(IconSize::Small)`

**Usage in the wild** (file finder `crates/file_finder/src/file_finder.rs:2097-2098`):
```rust
// Non-selectable section header rendered as ListSubHeader.
Match::SectionHeader(label) => { ... ListSubHeader::new(label) ... }
```

**For the workspace sidebar specifically**, the current group header in `workspace_sidebar.rs` uses a manual `div()` with `.bg(cx.theme().colors().element_background)`. To make it look like a subtle divider, either:
1. Replace with `ListSubHeader::new(name).left_icon(Some(IconName::Folder))` — gives consistent styling with other panels, **or**
2. Keep the custom div but switch the background to `cx.theme().colors().title_bar_background` or use `cx.theme().colors().surface_background` for a slightly different shade from the panel background (`panel_background`), and add `border_b_1().border_color(cx.theme().colors().border)`.

---

## 4. New Workspace Button — Current Code & Fix

**File:** `crates/workspace/src/workspace_sidebar.rs` (lines near the end of `render()`)

Current code:
```rust
let new_workspace_button = ListItem::new(...)
    .spacing(ListItemSpacing::Dense)   // ← wrong: should be Sparse to match workspace entries
    .child(
        h_flex()
            .gap_1()
            .items_center()
            .child(Icon::new(IconName::Plus).size(IconSize::XSmall).color(Color::Muted))
            .child(Label::new("New Workspace").size(LabelSize::XSmall).color(Color::Muted)),
    )
```

**Changes needed:**
1. Change `.spacing(ListItemSpacing::Dense)` → `.spacing(ListItemSpacing::Sparse)` to match workspace entry height
2. Remove the `Label::new("New Workspace")` child — show only the plus icon
3. Center the icon: replace `h_flex().gap_1().items_center()` with `h_flex().items_center().justify_center()` containing just the icon
4. Upsize the icon: `.size(IconSize::XSmall)` → `.size(IconSize::Small)` (matches workspace entry icon size)

Result:
```rust
let new_workspace_button = ListItem::new(...)
    .spacing(ListItemSpacing::Sparse)
    .child(
        h_flex()
            .items_center()
            .justify_center()
            .child(
                Icon::new(IconName::Plus)
                    .size(IconSize::Small)
                    .color(Color::Muted),
            ),
    )
```
