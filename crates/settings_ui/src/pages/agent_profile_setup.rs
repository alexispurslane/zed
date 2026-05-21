//! Agent profile setup sub-page for the AI settings page.
//!
//! This replaces the `ManageProfilesModal` with an inline settings sub-page
//! that lets users view, create, edit, and delete agent profiles.
//! It follows the same pattern as `tool_permissions_setup.rs`.

use agent_settings::{
    AgentProfileId, AgentProfileSettings, AgentSettings,
    builtin_profiles,
};
use gpui::{ScrollHandle, point, prelude::*};
use settings::Settings;
use ui::{
    Button, ButtonSize, ButtonStyle, Headline, Icon, IconName, IconSize,
    Label, LabelSize, Switch, ToggleState, prelude::*,
};

use crate::{SettingsWindow, components::SettingsInputField};

// ── Public entry point ──────────────────────────────────────────────

/// Renders the main agent profiles sub-page showing a list of profiles
/// and a "New Profile" button.
pub(crate) fn render_agent_profiles_setup_page(
    _settings_window: &SettingsWindow,
    scroll_handle: &ScrollHandle,
    _window: &mut Window,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let settings = AgentSettings::get_global(cx);

    let mut builtin_profiles_list: Vec<(AgentProfileId, AgentProfileSettings)> = Vec::new();
    let mut custom_profiles_list: Vec<(AgentProfileId, AgentProfileSettings)> = Vec::new();

    for (profile_id, profile) in settings.profiles.iter() {
        if builtin_profiles::is_builtin(profile_id) {
            builtin_profiles_list.push((profile_id.clone(), profile.clone()));
        } else {
            custom_profiles_list.push((profile_id.clone(), profile.clone()));
        }
    }

    builtin_profiles_list.sort_unstable_by(|a, b| a.1.name.cmp(&b.1.name));
    custom_profiles_list.sort_unstable_by(|a, b| a.1.name.cmp(&b.1.name));

    let active_profile_id = settings.default_profile.clone();

    let scroll_step = px(40.);

    v_flex()
        .id("agent-profiles-page")
        .on_action({
            let scroll_handle = scroll_handle.clone();
            move |_: &menu::SelectNext, window, cx| {
                window.focus_next(cx);
                let current_offset = scroll_handle.offset();
                scroll_handle.set_offset(point(current_offset.x, current_offset.y - scroll_step));
            }
        })
        .on_action({
            let scroll_handle = scroll_handle.clone();
            move |_: &menu::SelectPrevious, window, cx| {
                window.focus_prev(cx);
                let current_offset = scroll_handle.offset();
                scroll_handle.set_offset(point(current_offset.x, current_offset.y + scroll_step));
            }
        })
        .min_w_0()
        .size_full()
        .pt_2p5()
        .px_8()
        .pb_16()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .child(
            v_flex()
                .gap_2()
                // Built-in profiles section
                .child(Headline::new("Built-in Profiles").size(ui::HeadlineSize::Small))
                .children(
                    builtin_profiles_list
                        .iter()
                        .map(|(id, profile)| render_profile_list_item(id, profile, &active_profile_id, cx)),
                )
                // Custom profiles section
                .when(!custom_profiles_list.is_empty(), |this| {
                    this.child(div().mt_4().child(Headline::new("Custom Profiles").size(ui::HeadlineSize::Small)))
                        .children(
                            custom_profiles_list
                                .iter()
                                .map(|(id, profile)| render_profile_list_item(id, profile, &active_profile_id, cx)),
                        )
                })
                // New profile button
                .child(
                    div().mt_4().child(
                        Button::new("new-agent-profile", "New Profile")
                            .style(ButtonStyle::Outlined)
                            .start_icon(
                                Icon::new(IconName::Plus)
                                    .size(IconSize::Small)
                                    .color(Color::Muted),
                            )
                            .label_size(LabelSize::Small),
                    ),
                ),
        )
        .into_any_element()
}

// ── Profile list item rendering ─────────────────────────────────────

fn render_profile_list_item(
    profile_id: &AgentProfileId,
    profile: &AgentProfileSettings,
    active_profile_id: &AgentProfileId,
    _cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let is_active = profile_id == active_profile_id;

    let model_label = profile
        .default_model
        .as_ref()
        .map(|m| m.model.clone())
        .unwrap_or_else(|| "Default".into());

    h_flex()
        .w_full()
        .min_w_0()
        .py_2()
        .justify_between()
        .child(
            v_flex()
                .w_full()
                .min_w_0()
                .child(
                    h_flex()
                        .gap_1p5()
                        .child(Label::new(profile.name.clone()))
                        .when(is_active, |this| {
                            this.child(
                                Label::new("Active")
                                    .size(LabelSize::XSmall)
                                    .color(Color::Accent),
                            )
                        }),
                )
                .child(
                    Label::new(format!("Model: {}", model_label))
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                ),
        )
        .child(
            h_flex().gap_1().child(
                Button::new(format!("edit-profile-{}", profile_id.as_str()), "Edit")
                    .tab_index(0_isize)
                    .style(ButtonStyle::OutlinedGhost)
                    .size(ButtonSize::Medium)
                    .end_icon(
                        Icon::new(IconName::ChevronRight)
                            .size(IconSize::Small)
                            .color(Color::Muted),
                    ),
            ),
        )
        .into_any_element()
}

// ── Individual profile editor sub-page ───────────────────────────────

/// Renders the edit sub-page for a specific agent profile.
/// Called via `push_dynamic_sub_page` when the user clicks "Edit" on a profile.
#[allow(dead_code)]
pub(crate) fn render_agent_profile_edit_page(
    _settings_window: &SettingsWindow,
    scroll_handle: &ScrollHandle,
    _window: &mut Window,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let scroll_step = px(40.);

    v_flex()
        .id("agent-profile-edit-page")
        .on_action({
            let scroll_handle = scroll_handle.clone();
            move |_: &menu::SelectNext, window, cx| {
                window.focus_next(cx);
                let current_offset = scroll_handle.offset();
                scroll_handle.set_offset(point(current_offset.x, current_offset.y - scroll_step));
            }
        })
        .on_action({
            let scroll_handle = scroll_handle.clone();
            move |_: &menu::SelectPrevious, window, cx| {
                window.focus_prev(cx);
                let current_offset = scroll_handle.offset();
                scroll_handle.set_offset(point(current_offset.x, current_offset.y + scroll_step));
            }
        })
        .min_w_0()
        .size_full()
        .pt_2p5()
        .px_8()
        .pb_16()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .child(
            v_flex()
                .gap_6()
                // Profile name section
                .child(
                    v_flex()
                        .gap_1()
                        .child(Headline::new("Profile Name").size(ui::HeadlineSize::Small))
                        .child(
                            Label::new("The display name for this profile.")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                        .child(render_profile_name_field(cx)),
                )
                // Default model section
                .child(
                    v_flex()
                        .gap_1()
                        .child(Headline::new("Default Model").size(ui::HeadlineSize::Small))
                        .child(
                            Label::new(
                                "The default language model used when this profile is active.",
                            )
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                        )
                        .child(render_default_model_selector(cx)),
                )
                // Tools section
                .child(
                    v_flex()
                        .gap_1()
                        .child(Headline::new("Tools").size(ui::HeadlineSize::Small))
                        .child(
                            Label::new(
                                "Configure which built-in tools are available for this profile.",
                            )
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                        )
                        .child(render_tool_list(cx)),
                )
                // MCP servers section
                .child(
                    v_flex()
                        .gap_1()
                        .child(Headline::new("MCP Servers").size(ui::HeadlineSize::Small))
                        .child(
                            Label::new(
                                "Configure which MCP servers are available for this profile.",
                            )
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                        )
                        .child(render_mcp_server_list(cx)),
                ),
        )
        .into_any_element()
}

// ── Profile name field ──────────────────────────────────────────────

fn render_profile_name_field(_cx: &mut Context<SettingsWindow>) -> AnyElement {
    SettingsInputField::new()
        .with_id("profile-name-input")
        .with_placeholder("Enter profile name…")
        .tab_index(0)
        .into_any_element()
}

// ── Default model selector ──────────────────────────────────────────

fn render_default_model_selector(_cx: &mut Context<SettingsWindow>) -> AnyElement {
    h_flex()
        .w_full()
        .child(
            Button::new("default-model-selector", "Use Global Default")
                .style(ButtonStyle::Outlined)
                .end_icon(
                    Icon::new(IconName::ChevronDown)
                        .size(IconSize::Small)
                        .color(Color::Muted),
                ),
        )
        .into_any_element()
}

// ── Tool list ───────────────────────────────────────────────────────

fn render_tool_list(_cx: &mut Context<SettingsWindow>) -> AnyElement {
    let tool_names: Vec<&'static str> = agent::ALL_TOOL_NAMES
        .iter()
        .copied()
        .collect();

    v_flex()
        .gap_0p5()
        .children(tool_names.iter().map(|tool_name| {
            h_flex()
                .w_full()
                .py_1()
                .justify_between()
                .child(Label::new(tool_name.to_string()).size(LabelSize::Small))
                .child(Switch::new(format!("tool-switch-{}", tool_name), ToggleState::Selected))
        }))
        .into_any_element()
}

// ── MCP server list ────────────────────────────────────────────────

fn render_mcp_server_list(cx: &mut Context<SettingsWindow>) -> AnyElement {
    v_flex()
        .p_3()
        .border_1()
        .border_dashed()
        .border_color(cx.theme().colors().border_variant)
        .rounded_md()
        .child(
            Label::new("MCP server configuration is available when the profile is used with an active project.")
                .size(LabelSize::Small)
                .color(Color::Muted),
        )
        .into_any_element()
}
