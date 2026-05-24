use std::collections::HashMap;
use std::sync::Arc;

use fs::Fs;
use gpui::{
    App, ClickEvent, Context, Entity, EntityId, EventEmitter, FocusHandle, Focusable, Pixels,
    Render, SharedString, Window, actions, px,
};
use schemars::JsonSchema;
use serde::Deserialize;
use settings::SidebarSide;
use ui::{
    ContextMenu, Icon, IconName, Label, LabelSize, ListItem, ListItemSpacing,
    prelude::*, right_click_menu, utils::TRAFFIC_LIGHT_PADDING, v_flex,
};
use util::path_list::PathList;

use crate::{MultiWorkspace, Sidebar, SidebarEvent};

actions!(
    workspace_sidebar,
    [
        /// Collapse the currently focused project group.
        CollapseProjectGroup,
        /// Expand the currently focused project group.
        ExpandProjectGroup,
    ]
);

/// Rename the selected workspace.
#[derive(Clone, PartialEq, Deserialize, Default, JsonSchema, gpui::Action)]
#[action(namespace = workspace)]
pub struct RenameWorkspace {
    /// The entity ID of the workspace to rename.
    pub workspace_entity_id: u64,
}

const DEFAULT_WORKSPACE_SIDEBAR_WIDTH: Pixels = px(240.);
const MIN_WORKSPACE_SIDEBAR_WIDTH: Pixels = px(192.);
const MAX_WORKSPACE_SIDEBAR_WIDTH: Pixels = px(400.);

pub struct WorkspaceSidebar {
    multi_workspace: Option<gpui::WeakEntity<MultiWorkspace>>,
    focus_handle: FocusHandle,
    width: Pixels,
    #[allow(dead_code)]
    fs: Arc<dyn Fs>,
}

impl WorkspaceSidebar {
    pub fn new(
        multi_workspace: gpui::WeakEntity<MultiWorkspace>,
        fs: Arc<dyn Fs>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            multi_workspace: Some(multi_workspace),
            focus_handle,
            width: DEFAULT_WORKSPACE_SIDEBAR_WIDTH,
            fs,
        }
    }

    fn multi_workspace(&self, _cx: &App) -> Option<Entity<MultiWorkspace>> {
        self.multi_workspace.as_ref().and_then(|mw| mw.upgrade())
    }

    fn project_name(paths: &PathList) -> SharedString {
        let joined = paths
            .ordered_paths()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect::<Vec<_>>()
            .join(", ");
        if joined.is_empty() {
            "Untitled".into()
        } else {
            joined.into()
        }
    }
}

impl Sidebar for WorkspaceSidebar {
    fn width(&self, _cx: &App) -> Pixels {
        self.width
    }

    fn set_width(&mut self, width: Option<Pixels>, cx: &mut Context<Self>) {
        self.width = width.unwrap_or(DEFAULT_WORKSPACE_SIDEBAR_WIDTH);
        self.width = self
            .width
            .max(MIN_WORKSPACE_SIDEBAR_WIDTH)
            .min(MAX_WORKSPACE_SIDEBAR_WIDTH);
        cx.emit(SidebarEvent::SerializeNeeded);
        cx.notify();
    }

    fn has_notifications(&self, _cx: &App) -> bool {
        false
    }

    fn side(&self, _cx: &App) -> SidebarSide {
        SidebarSide::Left
    }

    fn cycle_project(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(multi_workspace) = self.multi_workspace(cx) else {
            return;
        };
        multi_workspace.update(cx, |multi_workspace, inner_cx| {
            let keys = multi_workspace.project_group_keys();
            if keys.len() < 2 {
                return;
            }

            let active_key = multi_workspace.workspace().read(inner_cx).project_group_key(inner_cx);
            let current_index = keys.iter().position(|k| k == &active_key);
            let next_index = match current_index {
                Some(i) => {
                    if forward {
                        (i + 1) % keys.len()
                    } else {
                        (i + keys.len() - 1) % keys.len()
                    }
                }
                None => 0,
            };

            if let Some(target_key) = keys.get(next_index) {
                if let Some(workspace) =
                    multi_workspace.last_active_workspace_for_group(target_key, inner_cx)
                {
                    multi_workspace.activate(workspace, None, window, inner_cx);
                }
            }
        });
    }

    fn serialized_state(&self, _cx: &App) -> Option<String> {
        serde_json::to_string(&self.width.as_f32()).ok()
    }

    fn restore_serialized_state(
        &mut self,
        state: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Ok(width) = serde_json::from_str::<f32>(state) {
            self.width = px(width)
                .max(MIN_WORKSPACE_SIDEBAR_WIDTH)
                .min(MAX_WORKSPACE_SIDEBAR_WIDTH);
            cx.notify();
        }
    }
}

impl EventEmitter<SidebarEvent> for WorkspaceSidebar {}

impl Focusable for WorkspaceSidebar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for WorkspaceSidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let groups = self
            .multi_workspace(cx)
            .map(|mw| mw.read(cx).project_groups(cx))
            .unwrap_or_default();

        let has_groups = !groups.is_empty();

        // Header — with macOS traffic light padding when sidebar is on the left
        let header = h_flex()
            .w_full()
            .py_1()
            // On macOS, the traffic light buttons occupy the top-left corner.
            // Add extra left padding so the "Workspaces" label doesn't overlap them.
            .when(cfg!(target_os = "macos"), |el| {
                el.pl(px(TRAFFIC_LIGHT_PADDING))
            })
            .when(!cfg!(target_os = "macos"), |el| el.pl_2())
            .pr_2()
            .child(
                Label::new("Workspaces")
                    .size(LabelSize::Small)
                    .weight(gpui::FontWeight::SEMIBOLD),
            );

        let mut list_elements: Vec<AnyElement> = Vec::new();
        for (group_index, group) in groups.iter().enumerate() {
            let group_key = group.key.clone();
            let is_expanded = group.expanded;
            let workspaces: Vec<_> = group.workspaces.clone();
            let workspace_count = workspaces.len();

            let name = Self::project_name(group_key.path_list());

            let chevron_icon = if is_expanded {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            };

            let toggle_key = group_key.clone();
            let click_key = group_key.clone();
            let group_id = SharedString::from(format!(
                "project-group-{}",
                group_key
                    .path_list()
                    .paths()
                    .first()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            ));

            // Plus button (right side of header) — needs its own key clone
            let plus_group_key = group_key.clone();
            // Context menu clones
            let menu_group_key = group_key.clone();
            let menu_mw = self.multi_workspace.clone();

            // Clones for click handlers inside trigger closure (can't use cx.listener there)
            let toggle_mw = self.multi_workspace.clone();
            let toggle_click_key = click_key.clone();
            let toggle_toggle_key = toggle_key.clone();
            let plus_mw = self.multi_workspace.clone();

            let group_header = right_click_menu::<ContextMenu>(group_id.clone())
                .trigger(move |_is_active, _window, _cx| {
                    let mw_for_click = toggle_mw.clone();
                    let click_key_inner = toggle_click_key.clone();
                    let mw_for_chevron = toggle_mw.clone();
                    let chevron_key_inner = toggle_toggle_key.clone();
                    let mw_for_plus = plus_mw.clone();
                    let plus_key_inner = plus_group_key.clone();

                    div()
                        .id(group_id.clone())
                        .w_full()
                        .px_1p5()
                        .py_0p5()
                        .bg(_cx.theme().colors().ghost_element_hover)
                        .when(group_index > 0, |el| {
                            el.border_t_1()
                                .border_color(_cx.theme().colors().border)
                        })
                        .cursor_pointer()
                        .on_click(move |_event: &ClickEvent, _window, cx| {
                            if let Some(multi_workspace) = mw_for_click.as_ref().and_then(|w| w.upgrade()) {
                                multi_workspace.update(cx, |mw, cx| {
                                    if let Some(group) = mw.group_state_by_key_mut(&click_key_inner) {
                                        group.expanded = !group.expanded;
                                    }
                                    mw.serialize(cx);
                                    cx.notify();
                                });
                            }
                        })
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .child(
                                    h_flex()
                                        .gap_0p5()
                                        .items_center()
                                        .flex_1()
                                        .min_w_0()
                                        .child(
                                            div()
                                                .id(SharedString::from(format!("expand-toggle-{}", group_index)))
                                                .on_click(move |_event: &ClickEvent, _window, cx| {
                                                    cx.stop_propagation();
                                                    if let Some(multi_workspace) = mw_for_chevron.as_ref().and_then(|w| w.upgrade()) {
                                                        multi_workspace.update(cx, |mw, cx| {
                                                            if let Some(group) = mw.group_state_by_key_mut(&chevron_key_inner) {
                                                                group.expanded = !group.expanded;
                                                            }
                                                            mw.serialize(cx);
                                                            cx.notify();
                                                        });
                                                    }
                                                })
                                                .cursor_pointer()
                                                .child(
                                                    Icon::new(chevron_icon)
                                                        .size(IconSize::XSmall)
                                                        .color(Color::Muted),
                                                ),
                                        )
                                        .child(
                                            Icon::new(IconName::Folder)
                                                .size(IconSize::XSmall)
                                                .color(Color::Muted),
                                        )
                                        .child(
                                            Label::new(name.clone())
                                                .size(LabelSize::XSmall)
                                                .color(Color::Muted)
                                                .single_line()
                                                .truncate(),
                                        )
                                        // Workspace count badge when collapsed
                                        .when(!is_expanded, |el| {
                                            el.child(
                                                Label::new(format!("{}", workspace_count))
                                                    .size(LabelSize::XSmall)
                                                    .color(Color::Muted),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .id(SharedString::from(format!("new-ws-{}", group_index)))
                                        .h_full()
                                        .w(px(20.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .on_click(move |_event: &ClickEvent, window, cx| {
                                            cx.stop_propagation();
                                            if let Some(multi_workspace) = mw_for_plus.as_ref().and_then(|w| w.upgrade()) {
                                                let group_key = plus_key_inner.clone();
                                                multi_workspace.update(cx, |multi_workspace, inner_cx| {
                                                    if let Some(workspace) =
                                                        multi_workspace.last_active_workspace_for_group(&group_key, inner_cx)
                                                    {
                                                        multi_workspace.activate(workspace, None, window, inner_cx);
                                                        multi_workspace
                                                            .add_layout_workspace(window, inner_cx)
                                                            .detach_and_log_err(inner_cx);
                                                    } else {
                                                        multi_workspace
                                                            .open_project(
                                                                group_key.path_list().paths().to_vec(),
                                                                crate::OpenMode::Activate,
                                                                window,
                                                                inner_cx,
                                                            )
                                                            .detach_and_log_err(inner_cx);
                                                    }
                                                });
                                            }
                                        })
                                        .cursor_pointer()
                                        .child(
                                            Icon::new(IconName::Plus)
                                                .size(IconSize::XSmall)
                                                .color(Color::Muted),
                                        ),
                                ),
                        )
                        .into_any_element()
                })
                .menu(move |window, cx| {
                    let gk = menu_group_key.clone();
                    let mw = menu_mw.clone();
                    let focus_handle = cx.focus_handle();

                    ContextMenu::build(window, cx, move |mut menu, _window, _cx| {
                        menu = menu.entry("Close Project", None, move |window, cx| {
                            if let Some(multi_workspace) = mw.as_ref().and_then(|w| w.upgrade()) {
                                let gk = gk.clone();
                                multi_workspace.update(cx, |mw, cx| {
                                    mw.remove_project_group(&gk, window, cx)
                                        .detach_and_log_err(cx);
                                });
                            }
                        });
                        menu.context(focus_handle)
                    })
                })
                .into_any_element();

            list_elements.push(group_header);

            if is_expanded {
                let mut same_project_count: HashMap<EntityId, usize> = HashMap::default();
                for (index, workspace) in workspaces.iter().enumerate() {
                    let project = workspace.read(cx).project().clone();
                    let entry = same_project_count.entry(project.entity_id()).or_insert(0);
                    *entry += 1;
                    let disambiguation_index = if workspaces.iter().any(|ws| {
                        ws.read(cx).project().entity_id() == project.entity_id()
                            && ws != workspace
                    }) {
                        Some(index + 1)
                    } else {
                        None
                    };

                    let is_active_workspace = self
                        .multi_workspace(cx)
                        .is_some_and(|mw| mw.read(cx).workspace() == workspace);

                    // Check for custom workspace name
                    let custom_name = self
                        .multi_workspace(cx)
                        .and_then(|mw| mw.read(cx).workspace_name(workspace.entity_id()));

                    // Auto-name: use the active tab's title if available
                    let active_item_title = workspace.read(cx).active_item(cx).map(|item| {
                        item.tab_content_text(0, cx)
                    });

                    let root_paths = workspace.read(cx).root_paths(cx);
                    let project_name: SharedString = if root_paths.is_empty() {
                        "Empty".into()
                    } else {
                        root_paths
                            .iter()
                            .filter_map(|p| {
                                p.file_name().map(|n| n.to_string_lossy().to_string())
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                            .into()
                    };

                    // Priority: custom name > active tab title > project folder name
                    let auto_name: SharedString = active_item_title.unwrap_or(project_name);

                    let display_name: SharedString = match (&custom_name, disambiguation_index) {
                        (Some(name), Some(idx)) if idx > 1 => {
                            format!("{} ({})", name, idx).into()
                        }
                        (Some(name), _) => name.clone(),
                        (None, Some(idx)) if idx > 1 => {
                            format!("{} ({})", auto_name, idx).into()
                        }
                        (None, _) => auto_name,
                    };

                    let text_color = if is_active_workspace {
                        Color::Default
                    } else {
                        Color::Muted
                    };

                    let ws_id = SharedString::from(format!(
                        "workspace-{}",
                        workspace.entity_id().as_u64()
                    ));

                    // Data captured for both the click handler and context menu
                    let workspace_for_click = workspace.clone();
                    let workspace_for_context = workspace.clone();
                    let mw_for_click = self.multi_workspace(cx);
                    let mw_for_context = self.multi_workspace(cx);
                    let group_key_for_move = group_key.clone();
                    let rename_id = workspace.entity_id().as_u64();

                    // Compute pane & tab counts for the status text (needs cx, not available in trigger closure)
                    let pane_count = workspace.read(cx).panes().len();
                    let tab_count: usize = workspace.read(cx).panes().iter().map(|p| p.read(cx).items_len()).sum();
                    let status_text: SharedString = format!("{} pane{}, {} tab{}",
                        pane_count, if pane_count != 1 { "s" } else { "" },
                        tab_count, if tab_count != 1 { "s" } else { "" }
                    ).into();

                    // Check if any item in any pane is dirty (unsaved changes)
                    let is_dirty = workspace.read(cx).panes().iter().any(|pane| {
                        pane.read(cx).items().any(|item| item.is_dirty(cx))
                    });

                    // Clone for double-click rename handler
                    let rename_ws_id_for_dblclick = rename_id;

                    // Close button needs its own clone of the multi_workspace entity
                    let mw_for_close = self.multi_workspace(cx);

                    let entry = right_click_menu::<ContextMenu>(ws_id.clone())
                        .trigger(move |_is_active, _window, _cx| {
                            let ws = workspace_for_click.clone();
                            let mw = mw_for_click.clone();
                            let ws_close = workspace_for_click.clone();
                            let mw_close_btn = mw_for_close.clone();
                            let rename_id_for_click = rename_ws_id_for_dblclick;

                            ListItem::new(ws_id.clone())
                                .spacing(ListItemSpacing::Sparse)
                                .toggle_state(is_active_workspace)
                                .on_click(move |event, window, cx| {
                                    // Double-click to rename
                                    if event.click_count() == 2 {
                                        window.dispatch_action(
                                            Box::new(RenameWorkspace {
                                                workspace_entity_id: rename_id_for_click,
                                            }),
                                            cx,
                                        );
                                        return;
                                    }
                                    // Single click to activate
                                    if let Some(multi_workspace) = mw.clone() {
                                        let ws = ws.clone();
                                        multi_workspace.update(cx, |mw, cx| {
                                            mw.activate(ws, None, window, cx);
                                        });
                                    }
                                })
                                .start_slot(
                                    h_flex()
                                        .gap_1()
                                        .items_center()
                                        .child(
                                            Icon::new(IconName::Screen)
                                                .size(IconSize::Small)
                                                .color(text_color),
                                        )
                                        .when(is_dirty, |el| {
                                            el.child(
                                                div()
                                                    .size_1()
                                                    .rounded_full()
                                                    .bg(_cx.theme().colors().editor_foreground),
                                            )
                                        }),
                                )
                                .child(
                                    v_flex()
                                        .flex_grow()
                                        .overflow_x_hidden()
                                        .min_w_0()
                                        .child(
                                            Label::new(display_name.clone())
                                                .size(LabelSize::Default)
                                                .single_line()
                                                .truncate()
                                                .color(text_color),
                                        )
                                        .child(
                                            Label::new(status_text.clone())
                                                .size(LabelSize::XSmall)
                                                .single_line()
                                                .color(Color::Muted),
                                        ),
                                )
                                .show_end_slot_on_hover()
                                .end_slot(
                                    div()
                                        .id(SharedString::from(format!("close-ws-{}",
                                            workspace_for_click.entity_id().as_u64())))
                                        .flex_none()
                                        .cursor_pointer()
                                        .child(
                                            Icon::new(IconName::Close)
                                                .size(IconSize::XSmall)
                                                .color(Color::Muted),
                                        )
                                        .on_click(move |_event: &ClickEvent, window, cx| {
                                            cx.stop_propagation();
                                            if let Some(multi_workspace) = mw_close_btn.clone() {
                                                let ws = ws_close.clone();
                                                multi_workspace.update(cx, |mw, cx| {
                                                    mw.close_workspace(&ws, window, cx)
                                                        .detach_and_log_err(cx);
                                                });
                                            }
                                        }),
                                )
                                .into_any_element()
                        })
                        .menu(move |window, cx| {
                            let ws = workspace_for_context.clone();
                            let mw = mw_for_context.clone();
                            let gk = group_key_for_move.clone();
                            let rename_ws_id = rename_id;

                            let focus_handle = cx.focus_handle();

                            ContextMenu::build(window, cx, move |mut menu, _window, _cx| {
                                // "Close Workspace"
                                let ws_close = ws.clone();
                                let mw_close = mw.clone();
                                menu = menu.entry("Close Workspace", None, move |window, cx| {
                                    if let Some(multi_workspace) = mw_close.clone() {
                                        let ws = ws_close.clone();
                                        multi_workspace.update(cx, |mw, cx| {
                                            mw.close_workspace(&ws, window, cx)
                                                .detach_and_log_err(cx);
                                        });
                                    }
                                });

                                // "Rename Workspace"
                                let id_for_rename = rename_ws_id;
                                menu = menu.entry("Rename Workspace", None, move |window, cx| {
                                    window.dispatch_action(
                                        Box::new(RenameWorkspace {
                                            workspace_entity_id: id_for_rename,
                                        }),
                                        cx,
                                    );
                                });

                                menu = menu.separator();

                                // "Move to New Window"
                                let mw_move = mw.clone();
                                let gk_move = gk.clone();
                                menu = menu.entry("Move to New Window", None, move |window, cx| {
                                    if let Some(multi_workspace) = mw_move.clone() {
                                        multi_workspace.update(cx, |mw, cx| {
                                            mw.open_project_group_in_new_window(&gk_move, window, cx)
                                                .detach_and_log_err(cx);
                                        });
                                    }
                                });

                                menu.context(focus_handle)
                            })
                        });

                    list_elements.push(entry.into_any_element());
                }
            }
        }

        v_flex()
            .size_full()
            .bg(cx.theme().colors().panel_background)
            .track_focus(&self.focus_handle(cx))
            .child(header)
            .child(ui::Divider::horizontal())
            .when(!has_groups, |el| {
                el.child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            Label::new("No projects open")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                )
            })
            .when(has_groups, |el| {
                el.child(
                    div().id("workspace-sidebar-list").flex_1().overflow_y_scroll().child(
                        v_flex().w_full().children(list_elements),
                    ),
                )
            })
    }
}
