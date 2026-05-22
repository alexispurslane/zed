use std::collections::HashMap;
use std::sync::Arc;

use fs::Fs;
use gpui::{
    App, ClickEvent, Context, Entity, EntityId, EventEmitter, FocusHandle, Focusable, Pixels, Render,
    SharedString, Window, actions, px,
};
use project::ProjectGroupKey;
use settings::SidebarSide;
use ui::{Icon, IconName, Label, LabelSize, ListItem, Tooltip, prelude::*, v_flex};
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

    fn multi_workspace(&self, cx: &App) -> Option<Entity<MultiWorkspace>> {
        self.multi_workspace.as_ref().and_then(|mw| mw.upgrade())
    }

    fn toggle_project_group(&mut self, group_key: &ProjectGroupKey, cx: &mut Context<Self>) {
        let Some(multi_workspace) = self.multi_workspace(cx) else {
            return;
        };
        multi_workspace.update(cx, |multi_workspace, cx| {
            if let Some(group) = multi_workspace.group_state_by_key_mut(group_key) {
                group.expanded = !group.expanded;
            }
            cx.notify();
        });
        cx.emit(SidebarEvent::SerializeNeeded);
        cx.notify();
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

        let header = h_flex()
            .w_full()
            .px_2()
            .py_1()
            .justify_between()
            .items_center()
            .child(
                Label::new("Workspaces")
                    .size(LabelSize::Small)
                    .weight(gpui::FontWeight::SEMIBOLD),
            )
            .child(
                IconButton::new("close-sidebar", IconName::ThreadsSidebarLeftOpen)
                    .icon_size(IconSize::Small)
                    .tooltip(move |_, cx| {
                        Tooltip::for_action(
                            "Close Workspace Sidebar",
                            &crate::CloseWorkspaceSidebar,
                            cx,
                        )
                    })
                    .on_click(|_, window, cx| {
                        if let Some(multi_workspace) = window.root::<MultiWorkspace>().flatten() {
                            multi_workspace.update(cx, |multi_workspace, cx| {
                                multi_workspace.close_sidebar_action(window, cx);
                            });
                        }
                    }),
            );

        let mut group_elements: Vec<AnyElement> = Vec::new();
        for group in &groups {
            let group_key = group.key.clone();
            let is_expanded = group.expanded;
            let workspaces: Vec<_> = group.workspaces.clone();

            let name = Self::project_name(group_key.path_list());
            let is_active = self
                .multi_workspace(cx)
                .is_some_and(|mw| mw.read(cx).workspace().read(cx).project_group_key(cx) == group_key);

            let chevron_icon = if is_expanded {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            };

            let toggle_key = group_key.clone();
            let click_key = group_key.clone();

            let group_header = ListItem::new(SharedString::from(format!(
                "project-group-{}",
                group_key
                    .path_list()
                    .paths()
                    .first()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            )))
            .spacing(ui::ListItemSpacing::Dense)
            .toggle_state(is_active)
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        div()
                            .id("expand-toggle")
                            .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                                this.toggle_project_group(&toggle_key, cx);
                            }))
                            .cursor_pointer()
                            .child(Icon::new(chevron_icon).size(IconSize::Small)),
                    )
                    .child(Icon::new(IconName::Folder).size(IconSize::Small))
                    .child(
                        Label::new(name)
                            .size(LabelSize::Small)
                            .single_line()
                            .truncate(),
                    ),
            )
            .on_click(cx.listener(
                move |this, _event: &ClickEvent, _window, cx| {
                    this.toggle_project_group(&click_key, cx);
                },
            ))
            .into_any_element();

            let mut entry_elements = vec![group_header];
            if is_expanded {
                let mut same_project_count: HashMap<EntityId, usize> = HashMap::default();
                for (index, workspace) in workspaces.iter().enumerate() {
                    let project = workspace.read(cx).project().clone();
                    let entry = same_project_count.entry(project.entity_id()).or_insert(0);
                    *entry += 1;
                    // Only assign a disambiguation index if this project
                    // appears more than once in this group.
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

                    let workspace_for_click = workspace.clone();

                    let root_paths = workspace.read(cx).root_paths(cx);
                    let base_name: SharedString = if root_paths.is_empty() {
                        "Empty".into()
                    } else {
                        root_paths
                            .iter()
                            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                            .collect::<Vec<_>>()
                            .join(", ")
                            .into()
                    };
                    let display_name: SharedString = match disambiguation_index {
                        Some(idx) if idx > 1 => format!("{} ({})", base_name, idx).into(),
                        _ => base_name,
                    };

                    let text_color = if is_active_workspace {
                        Color::Default
                    } else {
                        Color::Muted
                    };

                    let entry = ListItem::new(SharedString::from(format!(
                        "workspace-{}",
                        workspace.entity_id().as_u64()
                    )))
                    .spacing(ui::ListItemSpacing::Dense)
                    .toggle_state(is_active_workspace)
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .pl_3()
                            .child(Icon::new(IconName::File).size(IconSize::Small).color(text_color))
                            .child(
                                Label::new(display_name)
                                    .size(LabelSize::Small)
                                    .single_line()
                                    .truncate()
                                    .color(text_color),
                            ),
                    )
                    .on_click(cx.listener(
                        move |this, _event: &ClickEvent, window, cx| {
                            let Some(multi_workspace) = this.multi_workspace(cx) else {
                                return;
                            };
                            let workspace = workspace_for_click.clone();
                            multi_workspace.update(cx, |multi_workspace, inner_cx| {
                                multi_workspace.activate(workspace, None, window, inner_cx);
                            });
                        },
                    ))
                    .into_any_element();

                    entry_elements.push(entry);
                }

                let new_workspace_group_key = group_key.clone();
                let new_workspace_button = ListItem::new(SharedString::from(format!(
                    "new-workspace-{}",
                    group_key
                        .path_list()
                        .paths()
                        .first()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default()
                )))
                .spacing(ui::ListItemSpacing::Dense)
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .pl_3()
                        .child(Icon::new(IconName::Plus).size(IconSize::Small).color(Color::Muted))
                        .child(
                            Label::new("New Workspace")
                                .size(LabelSize::Small)
                                .single_line()
                                .color(Color::Muted),
                        ),
                )
                .on_click(cx.listener(
                    move |this, _event: &ClickEvent, window, cx| {
                        let Some(multi_workspace) = this.multi_workspace(cx) else {
                            return;
                        };
                        let group_key = new_workspace_group_key.clone();
                        multi_workspace.update(cx, |multi_workspace, inner_cx| {
                            if let Some(workspace) =
                                multi_workspace.last_active_workspace_for_group(&group_key, inner_cx)
                            {
                                // The project is already open — activate it and
                                // add a new layout tab sharing the same Project.
                                multi_workspace.activate(workspace, None, window, inner_cx);
                                multi_workspace
                                    .add_layout_workspace(window, inner_cx)
                                    .detach_and_log_err(inner_cx);
                            } else {
                                // No workspace for this project group is loaded
                                // — open the project (creating a new Project
                                // entity) like the recent-projects flow does.
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
                    },
                ))
                .into_any_element();

                entry_elements.push(new_workspace_button);
            }

            group_elements.push(
                v_flex()
                    .w_full()
                    .children(entry_elements)
                    .into_any_element(),
            );
        }

        v_flex()
            .size_full()
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
                        v_flex().w_full().children(group_elements),
                    ),
                )
            })
    }
}
