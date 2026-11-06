//! The `JadeForge` colour palette and the Freya theme built from it.
//!
//! Every colour used by the views lives here. The component themes are taken
//! from Freya's dark theme and repainted with the palette below, so buttons,
//! inputs and scrollbars all follow the same look instead of the light
//! defaults.

use freya::prelude::*;

/// Window background.
pub const BACKGROUND: Color = Color::from_rgb(13, 15, 19);
/// Regular card background.
pub const PANEL: Color = Color::from_rgb(20, 23, 29);
/// Raised surface inside a card (rows, inputs).
pub const SURFACE: Color = Color::from_rgb(26, 30, 38);
/// Hovered surface.
pub const SURFACE_HOVER: Color = Color::from_rgb(34, 39, 49);
/// Emphasised surface (active tab, selected row).
pub const SURFACE_ACTIVE: Color = Color::from_rgb(42, 48, 60);
/// Borders and dividers.
pub const BORDER: Color = Color::from_rgb(44, 50, 62);
/// Primary text.
pub const TEXT: Color = Color::from_rgb(233, 237, 244);
/// Secondary text.
pub const MUTED: Color = Color::from_rgb(170, 180, 195);
/// Disabled or very subtle text.
pub const DIM: Color = Color::from_rgb(145, 155, 170);
/// Jade green, used for accents and primary actions.
pub const ACCENT: Color = Color::from_rgb(60, 203, 143);
/// Hovered accent.
pub const ACCENT_HOVER: Color = Color::from_rgb(88, 222, 166);
/// Text on top of [`ACCENT`].
pub const ON_ACCENT: Color = Color::from_rgb(5, 28, 19);
/// Secondary accent: a muted jade for positive states and badges.
pub const JADE: Color = Color::from_rgb(112, 176, 141);
/// Golden upgrade orb, matching the game's level-up screen.
pub const ORB: Color = Color::from_rgb(226, 179, 92);
/// Hovered golden upgrade orb.
pub const ORB_HOVER: Color = Color::from_rgb(241, 199, 121);
/// Error colour.
pub const DANGER: Color = Color::from_rgb(232, 120, 130);
/// Success colour.
pub const SUCCESS: Color = Color::from_rgb(126, 190, 143);
/// Warning colour, kept amber so a warning never reads as the accent.
pub const WARNING: Color = Color::from_rgb(232, 180, 92);

/// Named font sizes for the interface.
///
/// Views must use these instead of literal sizes. Keep the scale small and
/// semantic; add a token here rather than a one-off value in a view.
pub mod font {
    /// Hero name on the overview.
    pub const DISPLAY: f32 = 26.0;
    /// Application title.
    pub const TITLE: f32 = 22.0;
    /// Empty-state heading.
    pub const HEADING: f32 = 20.0;
    /// Statistic value and other large readouts.
    pub const SUBHEADING: f32 = 18.0;
    /// Card and sidebar title.
    pub const CARD_TITLE: f32 = 15.0;
    /// Default reading size.
    pub const BODY: f32 = 13.0;
    /// Form labels and row values.
    pub const LABEL: f32 = 12.0;
    /// Hints, status messages and secondary text.
    pub const CAPTION: f32 = 11.0;
    /// Uppercase section labels and row metadata.
    pub const MICRO: f32 = 10.0;
    /// Line box of one line of [`CAPTION`] text.
    pub const LINE_HEIGHT: f32 = 13.0;
}

/// Shared spacing scale in logical pixels.
///
/// The scale is even-valued and owns every gap, padding and wrap distance in
/// the GUI. Component-specific names are aliases with the same values.
pub mod space {
    /// No spacing.
    pub const NONE: f32 = 0.0;
    /// Tight spacing inside a badge or between two text lines.
    pub const XXS: f32 = 2.0;
    /// Small spacing between closely related controls.
    pub const XS: f32 = 4.0;
    /// Spacing inside a compact row.
    pub const SM: f32 = 6.0;
    /// Default spacing between inline controls.
    pub const MD: f32 = 8.0;
    /// Spacing between a label and its control.
    pub const LG: f32 = 10.0;
    /// Spacing between groups in a row.
    pub const XL: f32 = 12.0;
    /// Spacing between form fields.
    pub const FIELD: f32 = 14.0;
    /// Spacing between sections inside a card.
    pub const XXL: f32 = 16.0;
    /// Padding inside a card.
    pub const CARD: f32 = 18.0;
    /// Spacing between large content blocks.
    pub const HERO: f32 = 20.0;
    /// Spacing between overview statistics.
    pub const GRID: f32 = 24.0;
    /// Padding around an empty state.
    pub const PAGE: f32 = 28.0;
}

/// Shared corner radii in logical pixels.
pub mod radius {
    /// Small controls and focus targets.
    pub const SMALL: f32 = 4.0;
    /// Compact list rows.
    pub const CONTROL: f32 = 6.0;
    /// Standard rows, tabs and inputs.
    pub const MEDIUM: f32 = 8.0;
    /// Images and medium surfaces.
    pub const LARGE: f32 = 10.0;
    /// Cards and portrait frames.
    pub const CARD: f32 = 12.0;
    /// Large panels such as the overview hero.
    pub const PANEL: f32 = 14.0;
    /// Pills, badges and circular orbs.
    pub const PILL: f32 = 99.0;
}

/// Shared stroke widths in logical pixels.
pub mod stroke {
    /// Dividers and hairlines.
    pub const HAIRLINE: f32 = 1.0;
    /// Focus rings and orb borders.
    pub const FOCUS: f32 = 1.5;
    /// Emphasised borders such as a focused orb.
    pub const EMPHASIS: f32 = 2.0;
}

/// Fixed component dimensions that are part of the layout contract.
pub mod layout {
    /// Sidebar width as a percentage of the window.
    pub const SIDEBAR_PERCENT: f32 = 24.0;
    /// Smallest sidebar width.
    pub const SIDEBAR_MIN_WIDTH: f32 = 230.0;
    /// Largest sidebar width.
    pub const SIDEBAR_MAX_WIDTH: f32 = 320.0;
    /// Square savegame preview slot.
    ///
    /// The placeholder and the loaded screenshot share this square, so choosing
    /// a savegame never reflows the sidebar and the save list stays usable at
    /// the minimum window size.
    pub const PREVIEW: f32 = 120.0;
    /// Square character portrait on the overview.
    pub const PORTRAIT: f32 = 164.0;
    /// Height of one savegame row.
    pub const SAVE_ROW_HEIGHT: f32 = 46.0;
    /// Width of the selection bar on a savegame row.
    pub const SELECTION_BAR_WIDTH: f32 = 3.0;
    /// Diameter of an upgrade orb.
    pub const ORB: f32 = 26.0;
    /// Width of a read-only statistic.
    pub const STAT_WIDTH: f32 = 120.0;
    /// Width of one label grid cell.
    pub const LABEL_GRID_WIDTH: f32 = 340.0;
    /// Width of a level stepper value.
    pub const STEPPER_WIDTH: f32 = 46.0;
    /// Width of an item count.
    pub const ITEM_COUNT_WIDTH: f32 = 30.0;
    /// Width of an upgrade family label.
    pub const UPGRADE_LABEL_WIDTH: f32 = 120.0;
    /// Narrow form field.
    pub const FIELD_NARROW: f32 = 100.0;
    /// Medium form field.
    pub const FIELD_MEDIUM: f32 = 150.0;
    /// Wide form field.
    pub const FIELD_WIDE: f32 = 220.0;
    /// Smallest width of a fluid form field.
    ///
    /// Fields fill their row and wrap once this width no longer fits, so the
    /// identity grid reflows from three columns to two instead of leaving one
    /// field alone on its own row.
    pub const FIELD_MIN_WIDTH: f32 = 132.0;
    /// Width of an overview label cell as a percentage of its row.
    ///
    /// Just under half, so two cells and their spacing share a row on the
    /// default window while the supported minimum shows one per row.
    pub const LABEL_CELL_PERCENT: f32 = 48.0;
    /// Smallest width of an overview label cell.
    pub const LABEL_CELL_MIN_WIDTH: f32 = 300.0;
    /// Smallest width of a fluid overview statistic.
    pub const STAT_MIN_WIDTH: f32 = 110.0;
    /// Largest width of the editor column.
    ///
    /// Cards stop growing on very wide windows, which keeps the reading column
    /// together instead of spreading a paragraph across the whole screen.
    pub const EDITOR_MAX_WIDTH: f32 = 1100.0;
    /// Largest width of a run of body text, about 75 characters at
    /// [`font::BODY`].
    pub const TEXT_MEASURE: f32 = 720.0;
    /// Width of the fixed sort-direction arrow slot.
    ///
    /// Every sort button reserves it, active or not, so changing the sort key
    /// or direction cannot move the neighbouring buttons.
    pub const SORT_ARROW_WIDTH: f32 = 16.0;
    /// Height of the fixed sort-direction arrow slot.
    pub const SORT_ARROW_HEIGHT: f32 = 12.0;
    /// Hairline divider height.
    pub const DIVIDER: f32 = 1.0;
    /// Width of the scrollbar.
    pub const SCROLLBAR: f32 = 9.0;
    /// Default window width.
    pub const WINDOW_WIDTH: f32 = 1180.0;
    /// Default window height.
    pub const WINDOW_HEIGHT: f32 = 760.0;
    /// Smallest supported window width.
    pub const MIN_WINDOW_WIDTH: f32 = 900.0;
    /// Smallest supported window height.
    pub const MIN_WINDOW_HEIGHT: f32 = 700.0;
}

/// The colour sheet used by every component theme.
#[must_use]
pub fn palette() -> ColorsSheet {
    ColorsSheet {
        primary: ACCENT,
        secondary: JADE,
        tertiary: SURFACE_ACTIVE,
        success: SUCCESS,
        warning: WARNING,
        error: DANGER,
        info: JADE,
        background: BACKGROUND,
        surface_primary: PANEL,
        surface_secondary: SURFACE,
        surface_tertiary: SURFACE_ACTIVE,
        surface_inverse: TEXT,
        surface_inverse_secondary: MUTED,
        surface_inverse_tertiary: DIM,
        border: BORDER,
        border_focus: ACCENT,
        border_disabled: DIM,
        text_primary: TEXT,
        text_secondary: MUTED,
        text_placeholder: DIM,
        text_inverse: ON_ACCENT,
        text_highlight: ACCENT,
        focus: ACCENT,
        active: SURFACE_ACTIVE,
        disabled: DIM,
        overlay: Color::from_argb(180, 0, 0, 0),
        shadow: Color::TRANSPARENT,
    }
}

/// Builds the application theme.
#[must_use]
pub fn theme() -> Theme {
    let mut theme = dark_theme();
    theme.name = "jadeforge";
    theme.colors = palette();

    theme.set(
        "button",
        ButtonColorsThemePreference {
            background: Preference::Specific(SURFACE),
            hover_background: Preference::Specific(SURFACE_HOVER),
            border_fill: Preference::Specific(BORDER),
            focus_border_fill: Preference::Specific(ACCENT),
            color: Preference::Specific(TEXT),
        },
    );
    theme.set(
        "filled_button",
        ButtonColorsThemePreference {
            background: Preference::Specific(ACCENT),
            hover_background: Preference::Specific(ACCENT_HOVER),
            border_fill: Preference::Specific(Color::TRANSPARENT),
            focus_border_fill: Preference::Specific(TEXT),
            color: Preference::Specific(ON_ACCENT),
        },
    );
    theme.set(
        "outline_button",
        ButtonColorsThemePreference {
            background: Preference::Specific(Color::TRANSPARENT),
            hover_background: Preference::Specific(SURFACE_ACTIVE),
            border_fill: Preference::Specific(BORDER),
            focus_border_fill: Preference::Specific(ACCENT),
            color: Preference::Specific(TEXT),
        },
    );
    theme.set(
        "flat_button",
        ButtonColorsThemePreference {
            background: Preference::Specific(Color::TRANSPARENT),
            hover_background: Preference::Specific(SURFACE_ACTIVE),
            border_fill: Preference::Specific(Color::TRANSPARENT),
            focus_border_fill: Preference::Specific(ACCENT),
            color: Preference::Specific(TEXT),
        },
    );
    theme.set(
        "input",
        InputColorsThemePreference {
            background: Preference::Specific(SURFACE),
            focus_background: Preference::Specific(SURFACE_HOVER),
            color: Preference::Specific(TEXT),
            placeholder_color: Preference::Specific(DIM),
            border_fill: Preference::Specific(BORDER),
            focus_border_fill: Preference::Specific(ACCENT),
        },
    );
    theme.set(
        "flat_input",
        InputColorsThemePreference {
            background: Preference::Specific(Color::TRANSPARENT),
            focus_background: Preference::Specific(SURFACE),
            color: Preference::Specific(TEXT),
            placeholder_color: Preference::Specific(DIM),
            border_fill: Preference::Specific(Color::TRANSPARENT),
            focus_border_fill: Preference::Specific(BORDER),
        },
    );
    theme.set(
        "scrollbar",
        ScrollBarThemePreference {
            background: Preference::Specific(Color::TRANSPARENT),
            thumb_background: Preference::Specific(BORDER),
            hover_thumb_background: Preference::Specific(SURFACE_ACTIVE),
            active_thumb_background: Preference::Specific(ACCENT),
            // Freya 0.5 replaced the single `size` field with a resting and an
            // expanded width plus their opacities and gaps. JadeForge keeps the
            // 0.4 look: a slim, always visible 9 px bar that does not grow on
            // hover, which the scroll views pair with `auto_hide_scrollbars(false)`.
            thumb_cross_size: Preference::Specific(layout::SCROLLBAR),
            expanded_thumb_cross_size: Preference::Specific(layout::SCROLLBAR),
            opacity: Preference::Specific(255.0),
            expanded_opacity: Preference::Specific(255.0),
            cross_gap: Preference::Specific(0.0),
            expanded_cross_gap: Preference::Specific(0.0),
        },
    );
    theme.set(
        "tooltip",
        TooltipThemePreference {
            background: Preference::Specific(SURFACE_ACTIVE),
            border_fill: Preference::Specific(BORDER),
            color: Preference::Specific(TEXT),
            font_size: Preference::Specific(font::CAPTION),
        },
    );
    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(value: f32) -> f32 {
        let value = value / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    fn luminance(color: Color) -> f32 {
        0.2126 * channel(f32::from(color.r()))
            + 0.7152 * channel(f32::from(color.g()))
            + 0.0722 * channel(f32::from(color.b()))
    }

    fn contrast(foreground: Color, background: Color) -> f32 {
        let (light, dark) = {
            let foreground = luminance(foreground) + 0.05;
            let background = luminance(background) + 0.05;
            if foreground > background {
                (foreground, background)
            } else {
                (background, foreground)
            }
        };
        light / dark
    }

    #[test]
    fn surfaces_are_dark() {
        let colors = palette();
        for (name, color) in [
            ("background", colors.background),
            ("surface_primary", colors.surface_primary),
            ("surface_secondary", colors.surface_secondary),
            ("surface_tertiary", colors.surface_tertiary),
            ("border", colors.border),
        ] {
            assert!(
                luminance(color) < 0.25,
                "{name} is too bright for a dark theme"
            );
        }
    }

    #[test]
    fn text_stands_out_from_the_background() {
        let colors = palette();
        assert!(luminance(colors.text_primary) > 0.75);
        assert!(luminance(colors.text_primary) - luminance(colors.background) > 0.7);
    }

    #[test]
    fn accent_is_jade_green() {
        let colors = palette();
        let primary = colors.primary;
        assert!(
            primary.g() > primary.r() && primary.g() > primary.b(),
            "the accent should be green"
        );
        assert!(
            primary.b() > primary.r(),
            "jade leans cool instead of yellow"
        );
        assert!(
            primary.g() > 150,
            "the accent must stay bright on the dark surfaces"
        );
    }

    #[test]
    fn readable_text_meets_wcag_aa_on_every_surface() {
        for background in [BACKGROUND, PANEL, SURFACE, SURFACE_ACTIVE] {
            assert!(
                contrast(TEXT, background) >= 4.5,
                "primary text fails on {background:?}"
            );
            assert!(
                contrast(MUTED, background) >= 4.5,
                "muted text fails on {background:?}"
            );
            assert!(
                contrast(DIM, background) >= 4.5,
                "dim text fails on {background:?}"
            );
            assert!(
                contrast(DANGER, background) >= 4.5,
                "danger text fails on {background:?}"
            );
            assert!(
                contrast(SUCCESS, background) >= 4.5,
                "success text fails on {background:?}"
            );
            assert!(
                contrast(WARNING, background) >= 4.5,
                "warning text fails on {background:?}"
            );
        }
    }

    #[test]
    fn accent_text_meets_wcag_aa_on_every_surface() {
        for background in [BACKGROUND, PANEL, SURFACE, SURFACE_HOVER, SURFACE_ACTIVE] {
            assert!(
                contrast(ACCENT, background) >= 4.5,
                "accent text fails on {background:?}"
            );
        }
    }

    #[test]
    fn text_on_the_accent_is_readable() {
        assert!(
            contrast(ON_ACCENT, ACCENT) >= 4.5,
            "text on the accent fails"
        );
        assert!(
            contrast(ON_ACCENT, ACCENT_HOVER) >= 4.5,
            "text on the hovered accent fails"
        );
        assert!(
            luminance(ACCENT_HOVER) > luminance(ACCENT),
            "hover must lighten the accent"
        );
    }

    #[test]
    fn upgrade_orbs_keep_the_game_gold() {
        assert!(ORB.r() > ORB.b(), "the orbs should stay warm");
        assert!(ORB.r() > 180 && ORB.b() < 150);
        assert_ne!(ORB, ACCENT, "the orbs must not follow the accent");
        for background in [BACKGROUND, PANEL, SURFACE, SURFACE_HOVER, SURFACE_ACTIVE] {
            assert!(
                contrast(ORB, background) >= 3.0,
                "the orb ring is too faint on {background:?}"
            );
        }
        assert!(
            luminance(ORB_HOVER) > luminance(ORB),
            "hover must lighten the orb"
        );
    }

    #[test]
    fn no_component_surface_is_white() {
        for color in [
            SURFACE,
            SURFACE_HOVER,
            SURFACE_ACTIVE,
            PANEL,
            BORDER,
            BACKGROUND,
        ] {
            assert!(luminance(color) < 0.25, "{color:?} is too bright");
        }
        assert_ne!(ON_ACCENT, Color::WHITE);
        assert_ne!(TEXT, Color::WHITE);
    }

    #[test]
    fn font_scale_is_ordered_and_semantic() {
        let sizes = [
            font::MICRO,
            font::CAPTION,
            font::LABEL,
            font::BODY,
            font::CARD_TITLE,
            font::SUBHEADING,
            font::HEADING,
            font::TITLE,
            font::DISPLAY,
        ];
        assert!(
            sizes.windows(2).all(|pair| pair[0] < pair[1]),
            "font tokens must stay strictly ordered"
        );
        let line_heights = [font::CAPTION, font::LINE_HEIGHT];
        assert!(line_heights.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn spacing_scale_is_even_and_ordered() {
        let spaces = [
            space::NONE,
            space::XXS,
            space::XS,
            space::SM,
            space::MD,
            space::LG,
            space::XL,
            space::FIELD,
            space::XXL,
            space::CARD,
            space::HERO,
            space::GRID,
            space::PAGE,
        ];
        assert!(
            spaces.windows(2).all(|pair| pair[0] < pair[1]),
            "spacing tokens must stay strictly ordered"
        );
        assert!(spaces.iter().all(|value| value % 2.0 == 0.0));
    }

    #[test]
    fn radius_and_stroke_tokens_are_positive() {
        let radii = [
            radius::SMALL,
            radius::CONTROL,
            radius::MEDIUM,
            radius::LARGE,
            radius::CARD,
            radius::PANEL,
            radius::PILL,
        ];
        assert!(radii.windows(2).all(|pair| pair[0] < pair[1]));
        let strokes = [stroke::HAIRLINE, stroke::FOCUS, stroke::EMPHASIS];
        assert!(strokes.iter().all(|value| *value > 0.0));
        assert!(strokes.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn layout_tokens_are_positive_and_ordered() {
        for value in [
            layout::SIDEBAR_PERCENT,
            layout::SIDEBAR_MIN_WIDTH,
            layout::SIDEBAR_MAX_WIDTH,
            layout::PREVIEW,
            layout::PORTRAIT,
            layout::SAVE_ROW_HEIGHT,
            layout::SELECTION_BAR_WIDTH,
            layout::ORB,
            layout::STAT_WIDTH,
            layout::LABEL_GRID_WIDTH,
            layout::STEPPER_WIDTH,
            layout::ITEM_COUNT_WIDTH,
            layout::UPGRADE_LABEL_WIDTH,
            layout::FIELD_NARROW,
            layout::FIELD_MEDIUM,
            layout::FIELD_WIDE,
            layout::SORT_ARROW_WIDTH,
            layout::SORT_ARROW_HEIGHT,
            layout::DIVIDER,
            layout::SCROLLBAR,
            layout::WINDOW_WIDTH,
            layout::WINDOW_HEIGHT,
            layout::MIN_WINDOW_WIDTH,
            layout::MIN_WINDOW_HEIGHT,
        ] {
            assert!(value > 0.0, "layout token must be positive");
        }
        let sidebar_widths = [layout::SIDEBAR_MIN_WIDTH, layout::SIDEBAR_MAX_WIDTH];
        assert!(sidebar_widths.windows(2).all(|pair| pair[0] < pair[1]));
        let window_widths = [layout::MIN_WINDOW_WIDTH, layout::WINDOW_WIDTH];
        assert!(window_widths.windows(2).all(|pair| pair[0] < pair[1]));
        let window_heights = [layout::MIN_WINDOW_HEIGHT, layout::WINDOW_HEIGHT];
        assert!(window_heights.windows(2).all(|pair| pair[0] < pair[1]));
        let field_widths = [
            layout::FIELD_NARROW,
            layout::FIELD_MEDIUM,
            layout::FIELD_WIDE,
        ];
        assert!(field_widths.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
