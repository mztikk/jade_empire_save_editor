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
}
