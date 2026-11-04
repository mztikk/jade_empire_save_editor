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
pub const MUTED: Color = Color::from_rgb(148, 157, 173);
/// Disabled or very subtle text.
pub const DIM: Color = Color::from_rgb(103, 112, 128);
/// Jade Empire gold, used for accents and primary actions.
pub const ACCENT: Color = Color::from_rgb(226, 179, 92);
/// Hovered accent.
pub const ACCENT_HOVER: Color = Color::from_rgb(241, 199, 121);
/// Text on top of [`ACCENT`].
pub const ON_ACCENT: Color = Color::from_rgb(26, 20, 7);
/// Secondary accent (jade green).
pub const JADE: Color = Color::from_rgb(112, 176, 141);
/// Error colour.
pub const DANGER: Color = Color::from_rgb(226, 106, 116);
/// Success colour.
pub const SUCCESS: Color = Color::from_rgb(126, 190, 143);

/// The colour sheet used by every component theme.
#[must_use]
pub fn palette() -> ColorsSheet {
    ColorsSheet {
        primary: ACCENT,
        secondary: JADE,
        tertiary: SURFACE_ACTIVE,
        success: SUCCESS,
        warning: ACCENT,
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
            hover_background: Preference::Specific(SURFACE),
            border_fill: Preference::Specific(BORDER),
            focus_border_fill: Preference::Specific(ACCENT),
            color: Preference::Specific(TEXT),
        },
    );
    theme.set(
        "flat_button",
        ButtonColorsThemePreference {
            background: Preference::Specific(Color::TRANSPARENT),
            hover_background: Preference::Specific(SURFACE),
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
            size: Preference::Specific(9.),
        },
    );
    theme.set(
        "tooltip",
        TooltipThemePreference {
            background: Preference::Specific(SURFACE_ACTIVE),
            border_fill: Preference::Specific(BORDER),
            color: Preference::Specific(TEXT),
            font_size: Preference::Specific(11.),
        },
    );
    theme
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(color: Color) -> f32 {
        (0.2126 * f32::from(color.r())
            + 0.7152 * f32::from(color.g())
            + 0.0722 * f32::from(color.b()))
            / 255.0
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
    fn accent_is_gold_and_not_blue() {
        let colors = palette();
        let primary = colors.primary;
        assert!(primary.r() > primary.b(), "the accent should be warm");
        assert!(primary.r() > 180 && primary.b() < 150);
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
}
