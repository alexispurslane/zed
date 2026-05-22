use std::sync::Arc;

use gpui::{FontStyle, FontWeight, HighlightStyle, Hsla, WindowBackgroundAppearance, hsla};

use crate::{
    AccentColors, Appearance, DEFAULT_DARK_THEME, PlayerColors, StatusColors,
    StatusColorsRefinement, SyntaxTheme, SystemColors, Theme, ThemeColors, ThemeColorsRefinement,
    ThemeFamily, ThemeStyles, default_color_scales,
};

/// The default theme family for Xenomorphic.
///
/// This is used to construct the default theme fallback values, as well as to
/// have a theme available at compile time for tests.
pub fn zed_default_themes() -> ThemeFamily {
    ThemeFamily {
        id: "zed-default".to_string(),
        name: "Xenomorphic".into(),
        author: "".into(),
        themes: vec![xenomorph_dark(), zed_default_dark()],
        scales: default_color_scales(),
    }
}

pub(crate) fn xenomorph_dark() -> Theme {
    let bg = hsla(0. / 360., 0. / 100., 0.4 / 100., 1.); // Singularity #010101
    let editor = hsla(0. / 360., 0. / 100., 0.4 / 100., 1.); // Singularity #010101
    let elevated_surface = hsla(0. / 360., 0. / 100., 3.9 / 100., 1.); // Hibernation #0A0A0A
    let hover = hsla(0. / 360., 0. / 100., 3.1 / 100., 1.); // Dark Matter #080808

    let accent = hsla(92.7 / 360., 84.9 / 100., 49.2 / 100., 1.0); // Molecular Acid #74E813
    let red = hsla(0. / 360., 57.9 / 100., 48.4 / 100., 1.0); // Self-Destruct #C33434
    let yellow = hsla(32.4 / 360., 64.0 / 100., 58.6 / 100., 1.0); // Warning Beacon #D99B52
    let teal = hsla(180.0 / 360., 24.9 / 100., 41.8 / 100., 1.0); // Oxidation #508585
    let purple = hsla(268.8 / 360., 34.6 / 100., 58.6 / 100., 1.0); // Neural Parasite #9471BA
    let bright_green = hsla(100.6 / 360., 35.3 / 100., 60.6 / 100., 1.0); // Hive Moss #8EBE77
    let bright_red = hsla(0. / 360., 69.6 / 100., 60.0 / 100., 1.0); // Flamethrower #E05252
    let gold = hsla(34.9 / 360., 76.0 / 100., 57.5 / 100., 1.0); // Plasma Burn #E5A040
    let blue = hsla(207.4 / 360., 31.8 / 100., 64.9 / 100., 1.0); // Cryo Interface #89A8C2
    let green = hsla(142.9 / 360., 25.5 / 100., 51.6 / 100., 1.0); // Atmospheric #64A37C
    let navy = hsla(208.9 / 360., 22.3 / 100., 47.5 / 100., 1.0); // Weyland Blue #5E7A94
    let navigation = hsla(204.2 / 360., 24.3 / 100., 53.9 / 100., 1.0); // Navigation #6D8FA6
    let chitin = hsla(160.0 / 360., 17.9 / 100., 67.1 / 100., 1.0); // Chitin #9CBAB0
    let carapace = hsla(29.8 / 360., 57.4 / 100., 48.8 / 100., 1.0); // Carapace #C47C35
    let coolant = hsla(198.0 / 360., 28.8 / 100., 59.2 / 100., 1.0); // Coolant #79A3B5
    let _fossilized = hsla(37.7 / 360., 43.5 / 100., 57.6 / 100., 1.0); // Fossilized #C29F64
    let _vegetation = hsla(137.5 / 360., 20.7 / 100., 54.5 / 100., 1.0); // Vegetation #73A381
    let biofilm = hsla(146.9 / 360., 16.2 / 100., 35.1 / 100., 1.0); // Biofilm #4B6858
    let pig_iron = hsla(180.0 / 360., 7.5 / 100., 31.4 / 100., 1.0); // Pig Iron #4A5656
    let sensor_reading = hsla(202.7 / 360., 24.8 / 100., 70.8 / 100., 1.0); // Sensor Reading #A2B9C7
    let telemetry = hsla(203.8 / 360., 25.3 / 100., 48.8 / 100., 1.0); // Telemetry #5D839C
    let _derelict = hsla(36.0 / 360., 41.0 / 100., 35.9 / 100., 1.0); // Derelict #816336
    let dormant = hsla(164.6 / 360., 37.9 / 100., 20.2 / 100., 1.0); // Dormant #20473D
    let hyperdream = hsla(274.6 / 360., 35.3 / 100., 67.3 / 100., 1.0); // Hyperdream #B08EC9
    let acid_spray = hsla(91.8 / 360., 100.0 / 100., 64.5 / 100., 1.0); // Acid Spray #9FFF4A
    let rebreather = hsla(180.0 / 360., 27.4 / 100., 54.1 / 100., 1.0); // Rebreather #6AAAAA

    const ADDED_COLOR: Hsla = Hsla {
        h: 92.7 / 360.,
        s: 0.85,
        l: 0.49,
        a: 1.0,
    }; // Molecular Acid #74E813
    const WORD_ADDED_COLOR: Hsla = Hsla {
        h: 92.7 / 360.,
        s: 0.85,
        l: 0.49,
        a: 0.35,
    };
    const MODIFIED_COLOR: Hsla = Hsla {
        h: 207.4 / 360.,
        s: 0.25,
        l: 0.65,
        a: 1.0,
    }; // Cryo Interface #89A8C2
    const REMOVED_COLOR: Hsla = Hsla {
        h: 0. / 360.,
        s: 0.58,
        l: 0.48,
        a: 1.0,
    }; // Self-Destruct #C33434
    const WORD_DELETED_COLOR: Hsla = Hsla {
        h: 0. / 360.,
        s: 0.58,
        l: 0.48,
        a: 0.80,
    };

    let player = PlayerColors::dark();
    Theme {
        id: "xenomorph".to_string(),
        name: DEFAULT_DARK_THEME.into(),
        appearance: Appearance::Dark,
        styles: ThemeStyles {
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            system: SystemColors::default(),
            accents: AccentColors(Arc::from(vec![
                accent, bright_red, yellow, purple, teal, gold, bright_green,
            ])),
            colors: ThemeColors {
                border: hsla(0. / 360., 0. / 100., 6.7 / 100., 1.), // Ore Seam #111111
                border_variant: hsla(0. / 360., 0. / 100., 3.9 / 100., 1.), // Hibernation #0A0A0A
                border_focused: hsla(92.7 / 360., 84.9 / 100., 49.2 / 100., 1.), // Molecular Acid #74E813
                border_selected: hsla(98.0 / 360., 87.7 / 100., 15.9 / 100., 1.), // Biolume #1F4C05
                border_transparent: SystemColors::default().transparent,
                border_disabled: hsla(0. / 360., 0. / 100., 20.0 / 100., 1.), // Cast Iron #333333
                elevated_surface_background: elevated_surface,
                surface_background: bg,
                background: bg,
                element_background: elevated_surface,
                element_hover: hover,
                element_active: hsla(0. / 360., 0. / 100., 14.1 / 100., 1.), // Raw Steel #242424
                element_selected: hsla(0. / 360., 0. / 100., 10.2 / 100., 1.), // Sinter #1A1A1A
                element_disabled: bg,
                element_selection_background: player.local().selection.alpha(0.24),
                drop_target_background: accent.alpha(0.5),
                drop_target_border: hsla(98.0 / 360., 87.7 / 100., 15.9 / 100., 1.0), // Biolume
                ghost_element_background: SystemColors::default().transparent,
                ghost_element_hover: hover,
                ghost_element_active: hsla(0. / 360., 0. / 100., 14.1 / 100., 1.), // Raw Steel
                ghost_element_selected: hsla(0. / 360., 0. / 100., 10.2 / 100., 1.), // Sinter
                ghost_element_disabled: bg,
                text: hsla(144.0 / 360., 3.4 / 100., 71.6 / 100., 1.), // Titanium Alloy #B4B9B6
                text_muted: hsla(168.0 / 360., 2.2 / 100., 55.9 / 100., 1.), // Sensor Array #8C9190
                text_placeholder: hsla(168.0 / 360., 2.2 / 100., 55.9 / 100., 0.5), // Sensor Array 50%
                text_disabled: hsla(168.0 / 360., 2.2 / 100., 44.1 / 100., 1.), // Scrap Metal #6E7372
                text_accent: accent,
                icon: hsla(144.0 / 360., 3.4 / 100., 71.6 / 100., 1.), // Titanium Alloy
                icon_muted: hsla(168.0 / 360., 2.2 / 100., 55.9 / 100., 1.), // Sensor Array
                icon_disabled: hsla(168.0 / 360., 2.2 / 100., 44.1 / 100., 1.), // Scrap Metal
                icon_placeholder: hsla(168.0 / 360., 2.2 / 100., 55.9 / 100., 0.5), // Sensor Array 50%
                icon_accent: accent,
                debugger_accent: red,
                status_bar_background: bg,
                title_bar_background: bg,
                title_bar_inactive_background: bg,
                toolbar_background: bg,
                tab_bar_background: bg,
                tab_inactive_background: bg,
                tab_active_background: bg,
                search_match_background: hsla(0. / 360., 0. / 100., 9.4 / 100., 0.5), // Drill Core #181818
                search_active_match_background: hsla(0. / 360., 0. / 100., 14.1 / 100., 0.5), // Raw Steel #242424
                editor_background: editor,
                editor_gutter_background: editor,
                editor_subheader_background: elevated_surface,
                editor_active_line_background: hover.alpha(0.75),
                editor_highlighted_line_background: hover,
                editor_debugger_active_line_background: accent.alpha(0.2),
                editor_line_number: hsla(0. / 360., 0. / 100., 11.0 / 100., 1.), // Inactive line #1C1C1C
                editor_active_line_number: accent,
                editor_hover_line_number: accent.alpha(0.5),
                editor_invisible: hsla(0. / 360., 0. / 100., 16.5 / 100., 0.5), // Slag 50%
                editor_wrap_guide: hsla(0. / 360., 0. / 100., 6.7 / 100., 0.05), // Ore Seam
                editor_active_wrap_guide: hsla(0. / 360., 0. / 100., 6.7 / 100., 0.1), // Ore Seam
                editor_indent_guide: hsla(0. / 360., 0. / 100., 6.7 / 100., 1.), // Ore Seam #111111
                editor_indent_guide_active: hsla(0. / 360., 0. / 100., 16.5 / 100., 1.), // Slag #2A2A2A
                editor_document_highlight_read_background: accent.alpha(0.1),
                editor_document_highlight_write_background: accent.alpha(0.2),
                editor_document_highlight_bracket_background: accent.alpha(0.3),
                editor_diff_hunk_added_background: ADDED_COLOR.opacity(0.1),
                editor_diff_hunk_added_hollow_background: ADDED_COLOR.opacity(0.05),
                editor_diff_hunk_added_hollow_border: ADDED_COLOR.opacity(0.35),
                editor_diff_hunk_deleted_background: REMOVED_COLOR.opacity(0.1),
                editor_diff_hunk_deleted_hollow_background: REMOVED_COLOR.opacity(0.05),
                editor_diff_hunk_deleted_hollow_border: REMOVED_COLOR.opacity(0.35),
                terminal_background: editor,
                terminal_ansi_background: crate::black().dark().step_12(),
                terminal_foreground: hsla(144.0 / 360., 3.4 / 100., 71.6 / 100., 1.), // Titanium Alloy #B4B9B6
                terminal_bright_foreground: hsla(132.0 / 360., 6.8 / 100., 85.7 / 100., 1.), // Polished Titanium #D8DDD9
                terminal_dim_foreground: hsla(168.0 / 360., 2.2 / 100., 44.1 / 100., 1.), // Scrap Metal #6E7372
                terminal_ansi_black: editor,
                terminal_ansi_red: red,
                terminal_ansi_green: accent,
                terminal_ansi_yellow: yellow,
                terminal_ansi_blue: navy,
                terminal_ansi_magenta: purple,
                terminal_ansi_cyan: teal,
                terminal_ansi_white: hsla(144.0 / 360., 3.4 / 100., 71.6 / 100., 1.), // Titanium Alloy
                terminal_ansi_bright_black: hsla(0. / 360., 0. / 100., 16.5 / 100., 1.), // Slag #2A2A2A
                terminal_ansi_bright_red: bright_red,
                terminal_ansi_bright_green: acid_spray,
                terminal_ansi_bright_yellow: gold,
                terminal_ansi_bright_blue: blue,
                terminal_ansi_bright_magenta: hyperdream,
                terminal_ansi_bright_cyan: rebreather,
                terminal_ansi_bright_white: hsla(132.0 / 360., 6.8 / 100., 85.7 / 100., 1.), // Polished Titanium
                terminal_ansi_dim_black: bg,
                terminal_ansi_dim_red: hsla(0. / 360., 40.0 / 100., 22.0 / 100., 1.),
                terminal_ansi_dim_green: hsla(92.7 / 360., 50.0 / 100., 25.0 / 100., 1.),
                terminal_ansi_dim_yellow: hsla(32.4 / 360., 40.0 / 100., 30.0 / 100., 1.),
                terminal_ansi_dim_blue: hsla(208.9 / 360., 20.0 / 100., 30.0 / 100., 1.),
                terminal_ansi_dim_magenta: hsla(268.8 / 360., 25.0 / 100., 30.0 / 100., 1.),
                terminal_ansi_dim_cyan: hsla(180.0 / 360., 20.0 / 100., 25.0 / 100., 1.),
                terminal_ansi_dim_white: hsla(168.0 / 360., 2.2 / 100., 44.1 / 100., 1.), // Scrap Metal
                panel_background: elevated_surface,
                panel_focused_border: accent,
                panel_indent_guide: hsla(0. / 360., 0. / 100., 6.7 / 100., 1.), // Ore Seam #111111
                panel_indent_guide_hover: hsla(0. / 360., 0. / 100., 16.5 / 100., 1.), // Slag #2A2A2A
                panel_indent_guide_active: hsla(0. / 360., 0. / 100., 16.5 / 100., 1.), // Slag #2A2A2A
                panel_overlay_background: elevated_surface,
                panel_overlay_hover: hover,
                pane_focused_border: accent,
                pane_group_border: hsla(0. / 360., 0. / 100., 6.7 / 100., 1.), // Ore Seam
                scrollbar_thumb_background: hsla(0. / 360., 0. / 100., 7.8 / 100., 0.5), // Ore Vein #141414
                scrollbar_thumb_hover_background: hsla(0. / 360., 0. / 100., 20.0 / 100., 0.5), // Cast Iron #333333
                scrollbar_thumb_active_background: hover,
                scrollbar_thumb_border: hsla(0. / 360., 0. / 100., 6.7 / 100., 1.), // Ore Seam
                scrollbar_track_background: gpui::transparent_black(),
                scrollbar_track_border: hsla(0. / 360., 0. / 100., 6.7 / 100., 1.), // Ore Seam
                minimap_thumb_background: hover.alpha(0.7),
                minimap_thumb_hover_background: hover.alpha(0.7),
                minimap_thumb_active_background: hover.alpha(0.7),
                minimap_thumb_border: hsla(0. / 360., 0. / 100., 6.7 / 100., 1.), // Ore Seam
                editor_foreground: hsla(144.0 / 360., 3.4 / 100., 71.6 / 100., 1.), // Titanium Alloy #B4B9B6
                link_text_hover: navy,
                version_control_added: ADDED_COLOR,
                version_control_deleted: REMOVED_COLOR,
                version_control_modified: MODIFIED_COLOR,
                version_control_renamed: navigation,
                version_control_conflict: yellow,
                version_control_ignored: hsla(168.0 / 360., 2.2 / 100., 44.1 / 100., 1.), // Scrap Metal
                version_control_word_added: WORD_ADDED_COLOR,
                version_control_word_deleted: WORD_DELETED_COLOR,
                version_control_conflict_marker_ours: accent.alpha(0.1),
                version_control_conflict_marker_theirs: blue.alpha(0.1),
                vim_normal_background: SystemColors::default().transparent,
                vim_insert_background: SystemColors::default().transparent,
                vim_replace_background: SystemColors::default().transparent,
                vim_visual_background: SystemColors::default().transparent,
                vim_visual_line_background: SystemColors::default().transparent,
                vim_visual_block_background: SystemColors::default().transparent,
                vim_yank_background: accent.alpha(0.2),
                vim_helix_jump_label_foreground: red,
                vim_helix_normal_background: SystemColors::default().transparent,
                vim_helix_select_background: SystemColors::default().transparent,
                vim_normal_foreground: SystemColors::default().transparent,
                vim_insert_foreground: SystemColors::default().transparent,
                vim_replace_foreground: SystemColors::default().transparent,
                vim_visual_foreground: SystemColors::default().transparent,
                vim_visual_line_foreground: SystemColors::default().transparent,
                vim_visual_block_foreground: SystemColors::default().transparent,
                vim_helix_normal_foreground: SystemColors::default().transparent,
                vim_helix_select_foreground: SystemColors::default().transparent,
            },
            status: StatusColors {
                conflict: yellow,
                conflict_background: yellow,
                conflict_border: yellow,
                created: accent,
                created_background: accent,
                created_border: accent,
                deleted: red,
                deleted_background: red,
                deleted_border: red,
                error: red,
                error_background: red,
                error_border: red,
                hidden: dormant,
                hidden_background: dormant,
                hidden_border: dormant,
                hint: telemetry,
                hint_background: telemetry,
                hint_border: telemetry,
                ignored: hsla(168.0 / 360., 2.2 / 100., 44.1 / 100., 1.), // Scrap Metal
                ignored_background: hsla(168.0 / 360., 2.2 / 100., 44.1 / 100., 1.),
                ignored_border: hsla(0. / 360., 0. / 100., 6.7 / 100., 1.), // Ore Seam
                info: navy,
                info_background: navy,
                info_border: navy,
                modified: yellow,
                modified_background: yellow,
                modified_border: yellow,
                predictive: hsla(168.0 / 360., 2.2 / 100., 55.9 / 100., 1.), // Sensor Array
                predictive_background: hsla(168.0 / 360., 2.2 / 100., 55.9 / 100., 1.),
                predictive_border: hsla(0. / 360., 0. / 100., 6.7 / 100., 1.), // Ore Seam
                renamed: navigation,
                renamed_background: navigation,
                renamed_border: navigation,
                success: accent,
                success_background: accent,
                success_border: accent,
                unreachable: dormant,
                unreachable_background: dormant,
                unreachable_border: hsla(0. / 360., 0. / 100., 6.7 / 100., 1.), // Ore Seam
                warning: gold,
                warning_background: gold,
                warning_border: gold,
            },
            player,
            syntax: Arc::new(SyntaxTheme::new(vec![
                ("attribute".into(), carapace.into()), // Carapace #C47C35
                (
                    "boolean".into(),
                    gold.into(), // Plasma Burn #E5A040
                ),
                ("comment".into(), biofilm.into()), // Biofilm #4B6858
                ("comment.doc".into(), biofilm.into()), // Biofilm #4B6858
                ("constant".into(), gold.into()), // Plasma Burn #E5A040
                ("constructor".into(), bright_green.into()), // Hive Moss #8EBE77
                ("embedded".into(), HighlightStyle::default()),
                (
                    "emphasis".into(),
                    HighlightStyle {
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                (
                    "emphasis.strong".into(),
                    HighlightStyle {
                        font_weight: Some(FontWeight::BOLD),
                        ..HighlightStyle::default()
                    },
                ),
                ("enum".into(), yellow.into()), // Amber Resin #D99B52
                ("function".into(), bright_green.into()), // Hive Moss #8EBE77
                ("function.method".into(), bright_green.into()),
                ("function.definition".into(), bright_green.into()),
                ("hint".into(), telemetry.into()), // Telemetry #5D839C
                ("keyword".into(), blue.into()), // Cryo Interface #89A8C2
                ("label".into(), bright_green.into()), // Hive Moss #8EBE77
                ("link_text".into(), navy.into()), // Weyland Blue #5E7A94
                (
                    "link_uri".into(),
                    HighlightStyle {
                        color: Some(navigation),
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                ("number".into(), sensor_reading.into()), // Sensor Reading #A2B9C7
                ("operator".into(), teal.into()), // Oxidation #508585
                ("predictive".into(), HighlightStyle {
                    color: Some(hsla(168.0 / 360., 2.2 / 100., 55.9 / 100., 0.5)), // Sensor Array 50%
                    font_style: Some(FontStyle::Italic),
                    ..HighlightStyle::default()
                }),
                ("preproc".into(), blue.into()), // Cryo Interface #89A8C2
                ("primary".into(), HighlightStyle::default()),
                ("property".into(), chitin.into()), // Chitin #9CBAB0
                ("punctuation".into(), pig_iron.into()), // Pig Iron #4A5656
                ("punctuation.bracket".into(), pig_iron.into()),
                ("punctuation.delimiter".into(), pig_iron.into()),
                ("punctuation.list_marker".into(), pig_iron.into()),
                ("punctuation.special".into(), red.into()), // Self-Destruct #C33434
                ("string".into(), green.into()), // Atmospheric #64A37C
                ("string.escape".into(), purple.into()), // Neural Parasite #9471BA
                ("string.regex".into(), green.into()), // Atmospheric #64A37C
                ("string.special".into(), green.into()), // Atmospheric #64A37C
                ("string.special.symbol".into(), gold.into()), // Plasma Burn #E5A040
                ("tag".into(), coolant.into()), // Coolant #79A3B5
                ("text.literal".into(), green.into()), // Atmospheric #64A37C
                (
                    "title".into(),
                    HighlightStyle {
                        color: Some(blue), // Cryo Interface #89A8C2
                        font_weight: Some(FontWeight::NORMAL),
                        ..HighlightStyle::default()
                    },
                ),
                ("type".into(), yellow.into()), // Amber Resin #D99B52
                ("variable".into(), chitin.into()), // Chitin #9CBAB0
                ("variable.special".into(), gold.into()), // Plasma Burn #E5A040
                ("variant".into(), gold.into()), // Plasma Burn #E5A040
                ("diff.plus".into(), accent.into()), // Molecular Acid #74E813
                ("diff.minus".into(), red.into()), // Self-Destruct #C33434
            ])),
        },
    }
}

// If a theme customizes a foreground version of a status color, but does not
// customize the background color, then use a partly-transparent version of the
// foreground color for the background color.
/// Applies default status color backgrounds from their foreground counterparts.
pub fn apply_status_color_defaults(status: &mut StatusColorsRefinement) {
    for (fg_color, bg_color) in [
        (&status.deleted, &mut status.deleted_background),
        (&status.created, &mut status.created_background),
        (&status.modified, &mut status.modified_background),
        (&status.conflict, &mut status.conflict_background),
        (&status.error, &mut status.error_background),
        (&status.hidden, &mut status.hidden_background),
    ] {
        if bg_color.is_none()
            && let Some(fg_color) = fg_color
        {
            *bg_color = Some(fg_color.opacity(0.25));
        }
    }
}

/// Applies default theme color values derived from player colors.
pub fn apply_theme_color_defaults(
    theme_colors: &mut ThemeColorsRefinement,
    player_colors: &PlayerColors,
) {
    if theme_colors.element_selection_background.is_none() {
        let mut selection = player_colors.local().selection;
        if selection.a == 1.0 {
            selection.a = 0.25;
        }
        theme_colors.element_selection_background = Some(selection);
    }
}

pub(crate) fn zed_default_dark() -> Theme {
    let bg = hsla(215. / 360., 12. / 100., 15. / 100., 1.);
    let editor = hsla(220. / 360., 12. / 100., 18. / 100., 1.);
    let elevated_surface = hsla(225. / 360., 12. / 100., 17. / 100., 1.);
    let hover = hsla(225.0 / 360., 11.8 / 100., 26.7 / 100., 1.0);

    let blue = hsla(207.8 / 360., 81. / 100., 66. / 100., 1.0);
    let gray = hsla(218.8 / 360., 10. / 100., 40. / 100., 1.0);
    let green = hsla(95. / 360., 38. / 100., 62. / 100., 1.0);
    let orange = hsla(29. / 360., 54. / 100., 61. / 100., 1.0);
    let purple = hsla(286. / 360., 51. / 100., 64. / 100., 1.0);
    let red = hsla(355. / 360., 65. / 100., 65. / 100., 1.0);
    let teal = hsla(187. / 360., 47. / 100., 55. / 100., 1.0);
    let yellow = hsla(39. / 360., 67. / 100., 69. / 100., 1.0);

    const ADDED_COLOR: Hsla = Hsla {
        h: 134. / 360.,
        s: 0.55,
        l: 0.40,
        a: 1.0,
    };
    const WORD_ADDED_COLOR: Hsla = Hsla {
        h: 134. / 360.,
        s: 0.55,
        l: 0.40,
        a: 0.35,
    };
    const MODIFIED_COLOR: Hsla = Hsla {
        h: 48. / 360.,
        s: 0.76,
        l: 0.47,
        a: 1.0,
    };
    const REMOVED_COLOR: Hsla = Hsla {
        h: 350. / 360.,
        s: 0.88,
        l: 0.25,
        a: 1.0,
    };
    const WORD_DELETED_COLOR: Hsla = Hsla {
        h: 350. / 360.,
        s: 0.88,
        l: 0.25,
        a: 0.80,
    };

    let player = PlayerColors::dark();
    Theme {
        id: "one_dark".to_string(),
        name: DEFAULT_DARK_THEME.into(),
        appearance: Appearance::Dark,
        styles: ThemeStyles {
            window_background_appearance: WindowBackgroundAppearance::Opaque,
            system: SystemColors::default(),
            accents: AccentColors(Arc::from(vec![
                blue, orange, purple, teal, red, green, yellow,
            ])),
            colors: ThemeColors {
                border: hsla(225. / 360., 13. / 100., 12. / 100., 1.),
                border_variant: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                border_focused: hsla(223. / 360., 78. / 100., 65. / 100., 1.),
                border_selected: hsla(222.6 / 360., 77.5 / 100., 65.1 / 100., 1.0),
                border_transparent: SystemColors::default().transparent,
                border_disabled: hsla(222.0 / 360., 11.6 / 100., 33.7 / 100., 1.0),
                elevated_surface_background: elevated_surface,
                surface_background: bg,
                background: bg,
                element_background: hsla(223.0 / 360., 13. / 100., 21. / 100., 1.0),
                element_hover: hover,
                element_active: hsla(220.0 / 360., 11.8 / 100., 20.0 / 100., 1.0),
                element_selected: hsla(224.0 / 360., 11.3 / 100., 26.1 / 100., 1.0),
                element_disabled: SystemColors::default().transparent,
                element_selection_background: player.local().selection.alpha(0.25),
                drop_target_background: hsla(220.0 / 360., 8.3 / 100., 21.4 / 100., 1.0),
                drop_target_border: hsla(221. / 360., 11. / 100., 86. / 100., 1.0),
                ghost_element_background: SystemColors::default().transparent,
                ghost_element_hover: hover,
                ghost_element_active: hsla(220.0 / 360., 11.8 / 100., 20.0 / 100., 1.0),
                ghost_element_selected: hsla(224.0 / 360., 11.3 / 100., 26.1 / 100., 1.0),
                ghost_element_disabled: SystemColors::default().transparent,
                text: hsla(221. / 360., 11. / 100., 86. / 100., 1.0),
                text_muted: hsla(218.0 / 360., 7. / 100., 46. / 100., 1.0),
                text_placeholder: hsla(220.0 / 360., 6.6 / 100., 44.5 / 100., 1.0),
                text_disabled: hsla(220.0 / 360., 6.6 / 100., 44.5 / 100., 1.0),
                text_accent: hsla(222.6 / 360., 77.5 / 100., 65.1 / 100., 1.0),
                icon: hsla(222.9 / 360., 9.9 / 100., 86.1 / 100., 1.0),
                icon_muted: hsla(220.0 / 360., 12.1 / 100., 66.1 / 100., 1.0),
                icon_disabled: hsla(220.0 / 360., 6.4 / 100., 45.7 / 100., 1.0),
                icon_placeholder: hsla(220.0 / 360., 6.4 / 100., 45.7 / 100., 1.0),
                icon_accent: blue,
                debugger_accent: red,
                status_bar_background: bg,
                title_bar_background: bg,
                title_bar_inactive_background: bg,
                toolbar_background: editor,
                tab_bar_background: bg,
                tab_inactive_background: bg,
                tab_active_background: editor,
                search_match_background: bg,
                search_active_match_background: bg,

                editor_background: editor,
                editor_gutter_background: editor,
                editor_subheader_background: bg,
                editor_active_line_background: hsla(222.9 / 360., 13.5 / 100., 20.4 / 100., 1.0),
                editor_highlighted_line_background: hsla(207.8 / 360., 81. / 100., 66. / 100., 0.1),
                editor_debugger_active_line_background: hsla(
                    207.8 / 360.,
                    81. / 100.,
                    66. / 100.,
                    0.2,
                ),
                editor_line_number: hsla(222.0 / 360., 11.5 / 100., 34.1 / 100., 1.0),
                editor_active_line_number: hsla(216.0 / 360., 5.9 / 100., 49.6 / 100., 1.0),
                editor_hover_line_number: hsla(216.0 / 360., 5.9 / 100., 56.7 / 100., 1.0),
                editor_invisible: hsla(222.0 / 360., 11.5 / 100., 34.1 / 100., 1.0),
                editor_wrap_guide: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                editor_active_wrap_guide: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                editor_indent_guide: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                editor_indent_guide_active: hsla(225. / 360., 13. / 100., 12. / 100., 1.),
                editor_document_highlight_read_background: hsla(
                    207.8 / 360.,
                    81. / 100.,
                    66. / 100.,
                    0.2,
                ),
                editor_document_highlight_write_background: gpui::red(),
                editor_document_highlight_bracket_background: gpui::green(),
                editor_diff_hunk_added_background: ADDED_COLOR.opacity(0.12),
                editor_diff_hunk_added_hollow_background: ADDED_COLOR.opacity(0.06),
                editor_diff_hunk_added_hollow_border: ADDED_COLOR.opacity(0.36),
                editor_diff_hunk_deleted_background: REMOVED_COLOR.opacity(0.12),
                editor_diff_hunk_deleted_hollow_background: REMOVED_COLOR.opacity(0.06),
                editor_diff_hunk_deleted_hollow_border: REMOVED_COLOR.opacity(0.36),

                terminal_background: bg,
                // todo("Use one colors for terminal")
                terminal_ansi_background: crate::black().dark().step_12(),
                terminal_foreground: crate::white().dark().step_12(),
                terminal_bright_foreground: crate::white().dark().step_11(),
                terminal_dim_foreground: crate::white().dark().step_10(),
                terminal_ansi_black: crate::black().dark().step_12(),
                terminal_ansi_red: crate::red().dark().step_11(),
                terminal_ansi_green: crate::green().dark().step_11(),
                terminal_ansi_yellow: crate::yellow().dark().step_11(),
                terminal_ansi_blue: crate::blue().dark().step_11(),
                terminal_ansi_magenta: crate::violet().dark().step_11(),
                terminal_ansi_cyan: crate::cyan().dark().step_11(),
                terminal_ansi_white: crate::neutral().dark().step_12(),
                terminal_ansi_bright_black: crate::black().dark().step_11(),
                terminal_ansi_bright_red: crate::red().dark().step_10(),
                terminal_ansi_bright_green: crate::green().dark().step_10(),
                terminal_ansi_bright_yellow: crate::yellow().dark().step_10(),
                terminal_ansi_bright_blue: crate::blue().dark().step_10(),
                terminal_ansi_bright_magenta: crate::violet().dark().step_10(),
                terminal_ansi_bright_cyan: crate::cyan().dark().step_10(),
                terminal_ansi_bright_white: crate::neutral().dark().step_11(),
                terminal_ansi_dim_black: crate::black().dark().step_10(),
                terminal_ansi_dim_red: crate::red().dark().step_9(),
                terminal_ansi_dim_green: crate::green().dark().step_9(),
                terminal_ansi_dim_yellow: crate::yellow().dark().step_9(),
                terminal_ansi_dim_blue: crate::blue().dark().step_9(),
                terminal_ansi_dim_magenta: crate::violet().dark().step_9(),
                terminal_ansi_dim_cyan: crate::cyan().dark().step_9(),
                terminal_ansi_dim_white: crate::neutral().dark().step_10(),
                panel_background: bg,
                panel_focused_border: blue,
                panel_indent_guide: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                panel_indent_guide_hover: hsla(225. / 360., 13. / 100., 12. / 100., 1.),
                panel_indent_guide_active: hsla(225. / 360., 13. / 100., 12. / 100., 1.),
                panel_overlay_background: bg,
                panel_overlay_hover: hover,
                pane_focused_border: blue,
                pane_group_border: hsla(225. / 360., 13. / 100., 12. / 100., 1.),
                scrollbar_thumb_background: gpui::transparent_black(),
                scrollbar_thumb_hover_background: hover,
                scrollbar_thumb_active_background: hsla(
                    225.0 / 360.,
                    11.8 / 100.,
                    26.7 / 100.,
                    1.0,
                ),
                scrollbar_thumb_border: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                scrollbar_track_background: gpui::transparent_black(),
                scrollbar_track_border: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                minimap_thumb_background: hsla(225.0 / 360., 11.8 / 100., 26.7 / 100., 0.7),
                minimap_thumb_hover_background: hsla(225.0 / 360., 11.8 / 100., 26.7 / 100., 0.7),
                minimap_thumb_active_background: hsla(225.0 / 360., 11.8 / 100., 26.7 / 100., 0.7),
                minimap_thumb_border: hsla(228. / 360., 8. / 100., 25. / 100., 1.),
                editor_foreground: hsla(218. / 360., 14. / 100., 71. / 100., 1.),
                link_text_hover: blue,
                version_control_added: ADDED_COLOR,
                version_control_deleted: REMOVED_COLOR,
                version_control_modified: MODIFIED_COLOR,
                version_control_renamed: MODIFIED_COLOR,
                version_control_conflict: crate::orange().light().step_12(),
                version_control_ignored: crate::gray().light().step_12(),
                version_control_word_added: WORD_ADDED_COLOR,
                version_control_word_deleted: WORD_DELETED_COLOR,
                version_control_conflict_marker_ours: crate::green().light().step_12().alpha(0.5),
                version_control_conflict_marker_theirs: crate::blue().light().step_12().alpha(0.5),

                vim_normal_background: SystemColors::default().transparent,
                vim_insert_background: SystemColors::default().transparent,
                vim_replace_background: SystemColors::default().transparent,
                vim_visual_background: SystemColors::default().transparent,
                vim_visual_line_background: SystemColors::default().transparent,
                vim_visual_block_background: SystemColors::default().transparent,
                vim_yank_background: hsla(207.8 / 360., 81. / 100., 66. / 100., 0.2),
                vim_helix_jump_label_foreground: red,
                vim_helix_normal_background: SystemColors::default().transparent,
                vim_helix_select_background: SystemColors::default().transparent,
                vim_normal_foreground: SystemColors::default().transparent,
                vim_insert_foreground: SystemColors::default().transparent,
                vim_replace_foreground: SystemColors::default().transparent,
                vim_visual_foreground: SystemColors::default().transparent,
                vim_visual_line_foreground: SystemColors::default().transparent,
                vim_visual_block_foreground: SystemColors::default().transparent,
                vim_helix_normal_foreground: SystemColors::default().transparent,
                vim_helix_select_foreground: SystemColors::default().transparent,
            },
            status: StatusColors {
                conflict: yellow,
                conflict_background: yellow,
                conflict_border: yellow,
                created: green,
                created_background: green,
                created_border: green,
                deleted: red,
                deleted_background: red,
                deleted_border: red,
                error: red,
                error_background: red,
                error_border: red,
                hidden: gray,
                hidden_background: gray,
                hidden_border: gray,
                hint: blue,
                hint_background: blue,
                hint_border: blue,
                ignored: gray,
                ignored_background: gray,
                ignored_border: gray,
                info: blue,
                info_background: blue,
                info_border: blue,
                modified: yellow,
                modified_background: yellow,
                modified_border: yellow,
                predictive: gray,
                predictive_background: gray,
                predictive_border: gray,
                renamed: blue,
                renamed_background: blue,
                renamed_border: blue,
                success: green,
                success_background: green,
                success_border: green,
                unreachable: gray,
                unreachable_background: gray,
                unreachable_border: gray,
                warning: yellow,
                warning_background: yellow,
                warning_border: yellow,
            },
            player,
            syntax: Arc::new(SyntaxTheme::new(vec![
                ("attribute".into(), purple.into()),
                ("boolean".into(), orange.into()),
                ("comment".into(), gray.into()),
                ("comment.doc".into(), gray.into()),
                ("constant".into(), yellow.into()),
                ("constructor".into(), blue.into()),
                ("embedded".into(), HighlightStyle::default()),
                (
                    "emphasis".into(),
                    HighlightStyle {
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                (
                    "emphasis.strong".into(),
                    HighlightStyle {
                        font_weight: Some(FontWeight::BOLD),
                        ..HighlightStyle::default()
                    },
                ),
                ("enum".into(), teal.into()),
                ("function".into(), blue.into()),
                ("function.method".into(), blue.into()),
                ("function.definition".into(), blue.into()),
                ("hint".into(), blue.into()),
                ("keyword".into(), purple.into()),
                ("label".into(), HighlightStyle::default()),
                ("link_text".into(), blue.into()),
                (
                    "link_uri".into(),
                    HighlightStyle {
                        color: Some(teal),
                        font_style: Some(FontStyle::Italic),
                        ..HighlightStyle::default()
                    },
                ),
                ("number".into(), orange.into()),
                ("operator".into(), HighlightStyle::default()),
                ("predictive".into(), HighlightStyle::default()),
                ("preproc".into(), purple.into()),
                ("primary".into(), HighlightStyle::default()),
                ("property".into(), red.into()),
                ("punctuation".into(), HighlightStyle::default()),
                ("punctuation.bracket".into(), HighlightStyle::default()),
                ("punctuation.delimiter".into(), HighlightStyle::default()),
                ("punctuation.list_marker".into(), HighlightStyle::default()),
                ("punctuation.special".into(), HighlightStyle::default()),
                ("string".into(), green.into()),
                ("string.escape".into(), HighlightStyle::default()),
                ("string.regex".into(), red.into()),
                ("string.special".into(), HighlightStyle::default()),
                ("string.special.symbol".into(), HighlightStyle::default()),
                ("tag".into(), HighlightStyle::default()),
                ("text.literal".into(), HighlightStyle::default()),
                ("title".into(), HighlightStyle::default()),
                ("type".into(), teal.into()),
                ("variable".into(), HighlightStyle::default()),
                ("variable.special".into(), red.into()),
                ("variant".into(), HighlightStyle::default()),
                ("diff.plus".into(), green.into()),
                ("diff.minus".into(), red.into()),
            ])),
        },
    }
}
