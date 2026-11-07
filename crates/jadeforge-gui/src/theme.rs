//! The `JadeForge` colour palettes, the active interface style and the Freya
//! theme built from them.
//!
//! Three complete palettes exist: the original dark jade theme, a light
//! parchment theme and a high-contrast theme. The user choice lives in the
//! settings file ([`crate::preferences::ThemeChoice`]); the root
//! component resolves it and installs the matching [`UiStyle`] for the render
//! pass. Views read colours and dimensions through the functions below, never
//! through hardcoded values, so a theme or text-scale change repaints the whole
//! window and the headless tests can render every variant.
//!
//! The active style is thread local on purpose: Freya renders one component
//! tree on the UI thread, and the headless suite can run several independent
//! windows without one test's theme leaking into another.

use std::cell::Cell;

use freya::prelude::*;

use crate::preferences::{MAX_TEXT_SCALE, MIN_TEXT_SCALE, ThemeChoice, normalize_text_scale};

/// Which palette a [`UiStyle`] uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaletteKind {
    /// The original dark jade-on-charcoal theme.
    #[default]
    Dark,
    /// A light parchment theme.
    Light,
    /// Maximum contrast, black background and bright accents.
    HighContrast,
}

impl PaletteKind {
    /// Stable name used in the Freya theme and in test diagnostics.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::HighContrast => "high-contrast",
        }
    }
}

/// Every colour the interface uses, for one theme.
///
/// Components resolve these through the accessor functions in this module so
/// switching the palette never leaves a hardcoded colour behind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// Which palette this is.
    pub kind: PaletteKind,
    /// Window background.
    pub background: Color,
    /// Regular card background.
    pub panel: Color,
    /// Raised surface inside a card (rows, inputs).
    pub surface: Color,
    /// Hovered surface.
    pub surface_hover: Color,
    /// Emphasised surface (active tab, selected row).
    pub surface_active: Color,
    /// Borders and dividers.
    pub border: Color,
    /// Border of an interactive control, at least 3:1 against every surface.
    pub control_border: Color,
    /// Primary text.
    pub text: Color,
    /// Secondary text.
    pub muted: Color,
    /// Disabled or very subtle text.
    pub dim: Color,
    /// Jade green, used for accents and primary actions.
    pub accent: Color,
    /// Hovered accent.
    pub accent_hover: Color,
    /// Text on top of [`Palette::accent`].
    pub on_accent: Color,
    /// Secondary jade for positive states and badges.
    pub jade: Color,
    /// Golden upgrade orb, matching the game's level-up screen.
    pub orb: Color,
    /// Hovered golden upgrade orb.
    pub orb_hover: Color,
    /// Error colour.
    pub danger: Color,
    /// Success colour.
    pub success: Color,
    /// Warning colour, kept amber so a warning never reads as the accent.
    pub warning: Color,
}

impl Palette {
    /// The original dark theme.
    pub const DARK: Self = Self {
        kind: PaletteKind::Dark,
        background: Color::from_rgb(13, 15, 19),
        panel: Color::from_rgb(20, 23, 29),
        surface: Color::from_rgb(26, 30, 38),
        surface_hover: Color::from_rgb(34, 39, 49),
        surface_active: Color::from_rgb(42, 48, 60),
        border: Color::from_rgb(44, 50, 62),
        control_border: Color::from_rgb(118, 128, 145),
        text: Color::from_rgb(233, 237, 244),
        muted: Color::from_rgb(170, 180, 195),
        dim: Color::from_rgb(145, 155, 170),
        accent: Color::from_rgb(60, 203, 143),
        accent_hover: Color::from_rgb(88, 222, 166),
        on_accent: Color::from_rgb(5, 28, 19),
        jade: Color::from_rgb(112, 176, 141),
        orb: Color::from_rgb(226, 179, 92),
        orb_hover: Color::from_rgb(241, 199, 121),
        danger: Color::from_rgb(232, 120, 130),
        success: Color::from_rgb(126, 190, 143),
        warning: Color::from_rgb(232, 180, 92),
    };

    /// The light parchment theme.
    pub const LIGHT: Self = Self {
        kind: PaletteKind::Light,
        background: Color::from_rgb(246, 244, 238),
        panel: Color::from_rgb(255, 255, 255),
        surface: Color::from_rgb(240, 237, 229),
        surface_hover: Color::from_rgb(230, 226, 216),
        surface_active: Color::from_rgb(220, 215, 202),
        border: Color::from_rgb(201, 196, 184),
        control_border: Color::from_rgb(111, 106, 96),
        text: Color::from_rgb(27, 32, 29),
        muted: Color::from_rgb(58, 70, 63),
        dim: Color::from_rgb(79, 90, 83),
        accent: Color::from_rgb(10, 92, 61),
        accent_hover: Color::from_rgb(14, 112, 72),
        on_accent: Color::from_rgb(255, 255, 255),
        jade: Color::from_rgb(47, 107, 83),
        orb: Color::from_rgb(140, 92, 15),
        orb_hover: Color::from_rgb(166, 112, 24),
        danger: Color::from_rgb(170, 34, 54),
        success: Color::from_rgb(30, 100, 59),
        warning: Color::from_rgb(122, 82, 0),
    };

    /// The high-contrast theme: black, white and a bright yellow focus ring.
    pub const HIGH_CONTRAST: Self = Self {
        kind: PaletteKind::HighContrast,
        background: Color::from_rgb(0, 0, 0),
        panel: Color::from_rgb(8, 8, 8),
        surface: Color::from_rgb(14, 14, 14),
        surface_hover: Color::from_rgb(24, 24, 24),
        surface_active: Color::from_rgb(34, 34, 34),
        border: Color::from_rgb(255, 255, 255),
        control_border: Color::from_rgb(255, 255, 255),
        text: Color::from_rgb(255, 255, 255),
        muted: Color::from_rgb(232, 232, 232),
        dim: Color::from_rgb(212, 212, 212),
        accent: Color::from_rgb(102, 255, 178),
        accent_hover: Color::from_rgb(153, 255, 204),
        on_accent: Color::from_rgb(0, 0, 0),
        jade: Color::from_rgb(102, 255, 178),
        orb: Color::from_rgb(255, 215, 0),
        orb_hover: Color::from_rgb(255, 235, 102),
        danger: Color::from_rgb(255, 138, 154),
        success: Color::from_rgb(125, 255, 176),
        warning: Color::from_rgb(255, 213, 77),
    };

    /// The palette for a resolved theme choice.
    #[must_use]
    pub fn for_choice(choice: ThemeChoice) -> Self {
        match choice {
            ThemeChoice::Light => Self::LIGHT,
            ThemeChoice::HighContrast => Self::HIGH_CONTRAST,
            ThemeChoice::Dark | ThemeChoice::System => Self::DARK,
        }
    }
}

/// The style the current render pass uses: a palette plus a text scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiStyle {
    /// Colours to paint with.
    pub palette: Palette,
    /// Layout and type multiplier; `1.0` is 100 %.
    pub text_scale: f32,
}

impl UiStyle {
    /// The default style: the dark palette at 100 %.
    pub const DEFAULT: Self = Self {
        palette: Palette::DARK,
        text_scale: 1.0,
    };

    /// Builds a style from a theme choice and a percentage.
    #[must_use]
    pub fn new(choice: ThemeChoice, percent: u16) -> Self {
        Self {
            palette: Palette::for_choice(choice),
            text_scale: f32::from(normalize_text_scale(percent)) / 100.0,
        }
    }

    /// Builds a style without clamping the scale, for layout unit tests.
    #[must_use]
    pub fn with_scale(palette: Palette, text_scale: f32) -> Self {
        Self {
            palette,
            text_scale: text_scale.clamp(0.5, 5.0),
        }
    }
}

thread_local! {
    /// Active style for this render thread.
    ///
    /// The root component installs it before building the tree; anything that
    /// renders without a root (unit tests) sees [`UiStyle::DEFAULT`].
    static CURRENT_STYLE: Cell<UiStyle> = const { Cell::new(UiStyle::DEFAULT) };
}

/// Installs the style used by the following render pass.
pub fn install(style: UiStyle) {
    CURRENT_STYLE.with(|current| current.set(style));
}

/// The active style.
#[must_use]
pub fn current() -> UiStyle {
    CURRENT_STYLE.with(Cell::get)
}

/// Resolves a theme choice against the operating system preference.
///
/// Freya 0.5 reports light/dark as [`PreferredTheme`]; it has no forced-colours
/// signal, so high contrast can only be selected explicitly.
#[must_use]
pub fn resolved_choice(choice: ThemeChoice, preferred: PreferredTheme) -> ThemeChoice {
    let light = preferred == PreferredTheme::Light;
    choice.resolve(light)
}

/// Multiplier applied to the type, spacing and fixed row dimensions.
#[must_use]
pub fn text_scale() -> f32 {
    current().text_scale
}

fn scaled(value: f32) -> f32 {
    value * text_scale()
}

/// Window background.
#[must_use]
pub fn background() -> Color {
    current().palette.background
}

/// Regular card background.
#[must_use]
pub fn panel() -> Color {
    current().palette.panel
}

/// Raised surface inside a card.
#[must_use]
pub fn surface() -> Color {
    current().palette.surface
}

/// Hovered surface.
#[must_use]
pub fn surface_hover() -> Color {
    current().palette.surface_hover
}

/// Emphasised surface such as the active tab.
#[must_use]
pub fn surface_active() -> Color {
    current().palette.surface_active
}

/// Borders and dividers.
#[must_use]
pub fn border() -> Color {
    current().palette.border
}

/// Border of an interactive control.
#[must_use]
pub fn control_border() -> Color {
    current().palette.control_border
}

/// Primary text.
#[must_use]
pub fn text() -> Color {
    current().palette.text
}

/// Secondary text.
#[must_use]
pub fn muted() -> Color {
    current().palette.muted
}

/// Disabled or very subtle text.
#[must_use]
pub fn dim() -> Color {
    current().palette.dim
}

/// Jade accent.
#[must_use]
pub fn accent() -> Color {
    current().palette.accent
}

/// Hovered accent.
#[must_use]
pub fn accent_hover() -> Color {
    current().palette.accent_hover
}

/// Text painted on the accent.
#[must_use]
pub fn on_accent() -> Color {
    current().palette.on_accent
}

/// Secondary jade.
#[must_use]
pub fn jade() -> Color {
    current().palette.jade
}

/// Golden upgrade orb.
#[must_use]
pub fn orb() -> Color {
    current().palette.orb
}

/// Hovered golden upgrade orb.
#[must_use]
pub fn orb_hover() -> Color {
    current().palette.orb_hover
}

/// Error colour.
#[must_use]
pub fn danger() -> Color {
    current().palette.danger
}

/// Success colour.
#[must_use]
pub fn success() -> Color {
    current().palette.success
}

/// Warning colour.
#[must_use]
pub fn warning() -> Color {
    current().palette.warning
}

/// Named font sizes for the interface.
///
/// Views must use these instead of literal sizes. The values are multiplied by
/// the user's text scale, so 200 % never needs a second layout.
pub mod font {
    use super::scaled;

    /// Hero name on the overview.
    #[must_use]
    pub fn display() -> f32 {
        scaled(26.0)
    }
    /// Application title.
    #[must_use]
    pub fn title() -> f32 {
        scaled(22.0)
    }
    /// Empty-state heading.
    #[must_use]
    pub fn heading() -> f32 {
        scaled(20.0)
    }
    /// Statistic value and other large readouts.
    #[must_use]
    pub fn subheading() -> f32 {
        scaled(18.0)
    }
    /// Card and sidebar title.
    #[must_use]
    pub fn card_title() -> f32 {
        scaled(15.0)
    }
    /// Default reading size.
    #[must_use]
    pub fn body() -> f32 {
        scaled(14.0)
    }
    /// Form labels and row values.
    #[must_use]
    pub fn label() -> f32 {
        scaled(13.0)
    }
    /// Hints, status messages and secondary text.
    #[must_use]
    pub fn caption() -> f32 {
        scaled(12.0)
    }
    /// Uppercase section labels and row metadata.
    #[must_use]
    pub fn micro() -> f32 {
        scaled(11.0)
    }
    /// Line box of one line of [`caption`] text.
    #[must_use]
    pub fn line_height() -> f32 {
        scaled(14.0)
    }
}

/// Shared spacing scale in logical pixels.
pub mod space {
    use super::scaled;

    /// No spacing.
    #[must_use]
    pub fn none() -> f32 {
        0.0
    }
    /// Tight spacing inside a badge or between two text lines.
    #[must_use]
    pub fn xxs() -> f32 {
        scaled(2.0)
    }
    /// Small spacing between closely related controls.
    #[must_use]
    pub fn xs() -> f32 {
        scaled(4.0)
    }
    /// Spacing inside a compact row.
    #[must_use]
    pub fn sm() -> f32 {
        scaled(6.0)
    }
    /// Default spacing between inline controls.
    #[must_use]
    pub fn md() -> f32 {
        scaled(8.0)
    }
    /// Spacing between a label and its control.
    #[must_use]
    pub fn lg() -> f32 {
        scaled(10.0)
    }
    /// Spacing between groups in a row.
    #[must_use]
    pub fn xl() -> f32 {
        scaled(12.0)
    }
    /// Spacing between form fields.
    #[must_use]
    pub fn field() -> f32 {
        scaled(14.0)
    }
    /// Spacing between sections inside a card.
    #[must_use]
    pub fn xxl() -> f32 {
        scaled(16.0)
    }
    /// Padding inside a card.
    #[must_use]
    pub fn card() -> f32 {
        scaled(18.0)
    }
    /// Spacing between large content blocks.
    #[must_use]
    pub fn hero() -> f32 {
        scaled(20.0)
    }
    /// Spacing between overview statistics.
    #[must_use]
    pub fn grid() -> f32 {
        scaled(24.0)
    }
    /// Padding around an empty state.
    #[must_use]
    pub fn page() -> f32 {
        scaled(28.0)
    }
}

/// Shared corner radii in logical pixels.
pub mod radius {
    use super::scaled;

    /// Small controls and focus targets.
    #[must_use]
    pub fn small() -> f32 {
        scaled(4.0)
    }
    /// Compact list rows.
    #[must_use]
    pub fn control() -> f32 {
        scaled(6.0)
    }
    /// Standard rows, tabs and inputs.
    #[must_use]
    pub fn medium() -> f32 {
        scaled(8.0)
    }
    /// Images and medium surfaces.
    #[must_use]
    pub fn large() -> f32 {
        scaled(10.0)
    }
    /// Cards and portrait frames.
    #[must_use]
    pub fn card() -> f32 {
        scaled(12.0)
    }
    /// Large panels such as the overview hero.
    #[must_use]
    pub fn panel() -> f32 {
        scaled(14.0)
    }
    /// Pills, badges and circular orbs.
    #[must_use]
    pub fn pill() -> f32 {
        scaled(99.0)
    }
}

/// Shared stroke widths in logical pixels.
pub mod stroke {
    use super::scaled;

    /// Dividers and hairlines.
    #[must_use]
    pub fn hairline() -> f32 {
        scaled(1.0)
    }
    /// Focus rings and orb borders.
    #[must_use]
    pub fn focus() -> f32 {
        scaled(1.5)
    }
    /// Emphasised borders such as a focused orb.
    #[must_use]
    pub fn emphasis() -> f32 {
        scaled(2.0)
    }
}

/// Fixed component dimensions that are part of the layout contract.
///
/// Type-related dimensions scale with the text scale so fixed rows and badges
/// grow with their text. Window geometry, the sidebar percentage and the image
/// slots deliberately stay at 100 %: they describe the window and the monitor,
/// not the type, and scaling them would push the minimum window off a 1366x768
/// screen at 200 %.
pub mod layout {
    use super::scaled;

    /// Sidebar width as a percentage of the window.
    #[must_use]
    pub fn sidebar_percent() -> f32 {
        24.0
    }
    /// Smallest sidebar width.
    #[must_use]
    pub fn sidebar_min_width() -> f32 {
        230.0
    }
    /// Largest sidebar width.
    #[must_use]
    pub fn sidebar_max_width() -> f32 {
        320.0
    }
    /// Square savegame preview slot.
    #[must_use]
    pub fn preview() -> f32 {
        120.0
    }
    /// Square character portrait on the overview.
    #[must_use]
    pub fn portrait() -> f32 {
        164.0
    }
    /// Height of one savegame row.
    #[must_use]
    pub fn save_row_height() -> f32 {
        scaled(46.0)
    }
    /// Width of the selection bar on a savegame row.
    #[must_use]
    pub fn selection_bar_width() -> f32 {
        scaled(3.0)
    }
    /// Diameter of an upgrade orb.
    #[must_use]
    pub fn orb() -> f32 {
        scaled(26.0)
    }
    /// Width of a read-only statistic.
    #[must_use]
    pub fn stat_width() -> f32 {
        scaled(120.0)
    }
    /// Width of a level stepper value.
    #[must_use]
    pub fn stepper_width() -> f32 {
        scaled(46.0)
    }
    /// Width of the placeholder bar in a sidebar loading row.
    #[must_use]
    pub fn skeleton_bar_percent() -> f32 {
        62.0
    }
    /// Width of a style row's `Level N` readout.
    #[must_use]
    pub fn level_width() -> f32 {
        scaled(84.0)
    }
    /// Width of an item count.
    #[must_use]
    pub fn item_count_width() -> f32 {
        scaled(30.0)
    }
    /// Width of an upgrade family label.
    #[must_use]
    pub fn upgrade_label_width() -> f32 {
        scaled(120.0)
    }
    /// Narrow form field.
    #[must_use]
    pub fn field_narrow() -> f32 {
        scaled(100.0)
    }
    /// Medium form field.
    #[must_use]
    pub fn field_medium() -> f32 {
        scaled(150.0)
    }
    /// Wide form field.
    #[must_use]
    pub fn field_wide() -> f32 {
        scaled(220.0)
    }
    /// Smallest width of a fluid form field.
    #[must_use]
    pub fn field_min_width() -> f32 {
        scaled(132.0)
    }
    /// Smallest width of a fluid overview statistic.
    #[must_use]
    pub fn stat_min_width() -> f32 {
        scaled(110.0)
    }
    /// Largest width of the editor column.
    #[must_use]
    pub fn editor_max_width() -> f32 {
        scaled(1100.0)
    }
    /// Largest width of a run of body text.
    #[must_use]
    pub fn text_measure() -> f32 {
        scaled(720.0)
    }
    /// Height of one row in a virtualized picker.
    ///
    /// Two text lines plus the action button fit, and the virtual list uses it
    /// as its fixed item size.
    #[must_use]
    pub fn picker_row_height() -> f32 {
        scaled(56.0)
    }
    /// Smallest click target of a row control.
    #[must_use]
    pub fn target_min() -> f32 {
        scaled(28.0)
    }
    /// Width of the disclosure chevron slot on a row.
    #[must_use]
    pub fn chevron_width() -> f32 {
        scaled(12.0)
    }
    /// Width of the reserved scan-status slot in the tab bar.
    #[must_use]
    pub fn busy_slot() -> f32 {
        scaled(118.0)
    }
    /// Width of the fixed sort-direction arrow slot.
    #[must_use]
    pub fn sort_arrow_width() -> f32 {
        scaled(12.0)
    }
    /// Height of the fixed sort-direction arrow slot.
    #[must_use]
    pub fn sort_arrow_height() -> f32 {
        scaled(12.0)
    }
    /// Hairline divider height.
    #[must_use]
    pub fn divider() -> f32 {
        scaled(1.0)
    }
    /// Width of the scrollbar.
    #[must_use]
    pub fn scrollbar() -> f32 {
        9.0
    }
    /// Default window width.
    #[must_use]
    pub fn window_width() -> f32 {
        1180.0
    }
    /// Default window height.
    #[must_use]
    pub fn window_height() -> f32 {
        760.0
    }
    /// Smallest supported window width.
    #[must_use]
    pub fn min_window_width() -> f32 {
        900.0
    }
    /// Smallest supported window height.
    #[must_use]
    pub fn min_window_height() -> f32 {
        700.0
    }
    /// Free space kept between a window and the monitor edges.
    #[must_use]
    pub fn window_margin() -> f32 {
        16.0
    }
}

/// The colour sheet used by every Freya component theme.
#[must_use]
pub fn palette() -> ColorsSheet {
    let colors = current().palette;
    ColorsSheet {
        primary: colors.accent,
        secondary: colors.jade,
        tertiary: colors.surface_active,
        success: colors.success,
        warning: colors.warning,
        error: colors.danger,
        info: colors.jade,
        background: colors.background,
        surface_primary: colors.panel,
        surface_secondary: colors.surface,
        surface_tertiary: colors.surface_active,
        surface_inverse: colors.text,
        surface_inverse_secondary: colors.muted,
        surface_inverse_tertiary: colors.dim,
        border: colors.border,
        border_focus: colors.accent,
        border_disabled: colors.dim,
        text_primary: colors.text,
        text_secondary: colors.muted,
        text_placeholder: colors.dim,
        text_inverse: colors.on_accent,
        text_highlight: colors.accent,
        focus: colors.accent,
        active: colors.surface_active,
        disabled: colors.dim,
        overlay: Color::from_argb(180, 0, 0, 0),
        shadow: Color::TRANSPARENT,
    }
}

/// Builds the Freya theme for the active palette.
#[must_use]
pub fn theme() -> Theme {
    let colors = current().palette;
    let mut theme = dark_theme();
    theme.name = match colors.kind {
        PaletteKind::Dark => "jadeforge-dark",
        PaletteKind::Light => "jadeforge-light",
        PaletteKind::HighContrast => "jadeforge-high-contrast",
    };
    theme.colors = palette();

    theme.set(
        "button",
        ButtonColorsThemePreference {
            background: Preference::Specific(colors.surface),
            hover_background: Preference::Specific(colors.surface_hover),
            border_fill: Preference::Specific(colors.border),
            focus_border_fill: Preference::Specific(colors.accent),
            color: Preference::Specific(colors.text),
        },
    );
    theme.set(
        "filled_button",
        ButtonColorsThemePreference {
            background: Preference::Specific(colors.accent),
            hover_background: Preference::Specific(colors.accent_hover),
            border_fill: Preference::Specific(Color::TRANSPARENT),
            focus_border_fill: Preference::Specific(colors.text),
            color: Preference::Specific(colors.on_accent),
        },
    );
    theme.set(
        "outline_button",
        ButtonColorsThemePreference {
            background: Preference::Specific(Color::TRANSPARENT),
            hover_background: Preference::Specific(colors.surface_active),
            border_fill: Preference::Specific(colors.control_border),
            focus_border_fill: Preference::Specific(colors.accent),
            color: Preference::Specific(colors.text),
        },
    );
    theme.set(
        "flat_button",
        ButtonColorsThemePreference {
            background: Preference::Specific(Color::TRANSPARENT),
            hover_background: Preference::Specific(colors.surface_active),
            border_fill: Preference::Specific(Color::TRANSPARENT),
            focus_border_fill: Preference::Specific(colors.accent),
            color: Preference::Specific(colors.text),
        },
    );
    theme.set(
        "input",
        InputColorsThemePreference {
            background: Preference::Specific(colors.surface),
            focus_background: Preference::Specific(colors.surface_hover),
            color: Preference::Specific(colors.text),
            placeholder_color: Preference::Specific(colors.dim),
            border_fill: Preference::Specific(colors.control_border),
            focus_border_fill: Preference::Specific(colors.accent),
        },
    );
    theme.set(
        "flat_input",
        InputColorsThemePreference {
            background: Preference::Specific(Color::TRANSPARENT),
            focus_background: Preference::Specific(colors.surface),
            color: Preference::Specific(colors.text),
            placeholder_color: Preference::Specific(colors.dim),
            border_fill: Preference::Specific(Color::TRANSPARENT),
            focus_border_fill: Preference::Specific(colors.control_border),
        },
    );
    theme.set(
        "scrollbar",
        ScrollBarThemePreference {
            background: Preference::Specific(Color::TRANSPARENT),
            thumb_background: Preference::Specific(colors.control_border),
            hover_thumb_background: Preference::Specific(colors.surface_active),
            active_thumb_background: Preference::Specific(colors.accent),
            // Freya 0.5 replaced the single `size` field with a resting and an
            // expanded width plus their opacities and gaps. JadeForge keeps the
            // 0.4 look: a slim, always visible 9 px bar that does not grow on
            // hover, which the scroll views pair with `auto_hide_scrollbars(false)`.
            thumb_cross_size: Preference::Specific(layout::scrollbar()),
            expanded_thumb_cross_size: Preference::Specific(layout::scrollbar()),
            opacity: Preference::Specific(255.0),
            expanded_opacity: Preference::Specific(255.0),
            cross_gap: Preference::Specific(0.0),
            expanded_cross_gap: Preference::Specific(0.0),
        },
    );
    theme.set(
        "tooltip",
        TooltipThemePreference {
            background: Preference::Specific(colors.surface_active),
            border_fill: Preference::Specific(colors.border),
            color: Preference::Specific(colors.text),
            font_size: Preference::Specific(font::caption()),
        },
    );
    theme
}

/// Installs the palette for `choice` at `percent` and returns the Freya theme.
///
/// The root component calls this on every render; keeping the two steps
/// together stops the direct colour tokens and the component themes from ever
/// disagreeing.
pub fn apply(choice: ThemeChoice, percent: u16) -> Theme {
    install(UiStyle::new(choice, percent));
    theme()
}

/// Clamps a percentage to the range the layout is tested at.
#[must_use]
pub fn clamp_scale_percent(percent: u16) -> u16 {
    percent.clamp(MIN_TEXT_SCALE, MAX_TEXT_SCALE)
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
        let foreground = luminance(foreground) + 0.05;
        let background = luminance(background) + 0.05;
        if foreground > background {
            foreground / background
        } else {
            background / foreground
        }
    }

    fn with_palette(palette: Palette, test: impl FnOnce()) {
        install(UiStyle::with_scale(palette, 1.0));
        test();
        install(UiStyle::DEFAULT);
    }

    fn all_palettes() -> [Palette; 3] {
        [Palette::DARK, Palette::LIGHT, Palette::HIGH_CONTRAST]
    }

    #[test]
    fn dark_surfaces_are_dark() {
        with_palette(Palette::DARK, || {
            for color in [
                background(),
                panel(),
                surface(),
                surface_hover(),
                surface_active(),
                border(),
            ] {
                assert!(luminance(color) < 0.25, "{color:?} is too bright");
            }
            assert!(luminance(text()) > 0.75);
            assert!(luminance(text()) - luminance(background()) > 0.7);
            assert_ne!(text(), Color::WHITE);
        });
    }

    #[test]
    fn light_surfaces_are_light_and_text_is_dark() {
        with_palette(Palette::LIGHT, || {
            for color in [background(), panel(), surface()] {
                assert!(luminance(color) > 0.75, "{color:?} is too dark");
            }
            assert!(luminance(text()) < 0.1);
            assert!(luminance(background()) - luminance(text()) > 0.5);
        });
    }

    #[test]
    fn every_palette_keeps_text_readable_on_every_surface() {
        for palette in all_palettes() {
            with_palette(palette, || {
                for background in [
                    background(),
                    panel(),
                    surface(),
                    surface_hover(),
                    surface_active(),
                ] {
                    for (name, color) in [
                        ("text", text()),
                        ("muted", muted()),
                        ("dim", dim()),
                        ("danger", danger()),
                        ("success", success()),
                        ("warning", warning()),
                    ] {
                        assert!(
                            contrast(color, background) >= 4.5,
                            "{} {name} fails on {background:?}",
                            palette.kind.key()
                        );
                    }
                }
            });
        }
    }

    #[test]
    fn accent_text_and_focus_rings_stay_visible() {
        for palette in all_palettes() {
            with_palette(palette, || {
                let minimum = if palette.kind == PaletteKind::HighContrast {
                    7.0
                } else {
                    4.5
                };
                for background in [background(), panel(), surface(), surface_active()] {
                    assert!(
                        contrast(accent(), background) >= minimum,
                        "{} accent fails on {background:?}",
                        palette.kind.key()
                    );
                }
                for background in [background(), panel(), surface()] {
                    assert!(
                        contrast(control_border(), background) >= 3.0,
                        "{} control border fails on {background:?}",
                        palette.kind.key()
                    );
                }
                assert!(
                    contrast(accent(), background()) >= 3.0,
                    "{} focus ring fails on the background",
                    palette.kind.key()
                );
            });
        }
    }

    #[test]
    fn high_contrast_text_meets_wcag_aaa() {
        with_palette(Palette::HIGH_CONTRAST, || {
            for background in [background(), panel(), surface(), surface_active()] {
                for color in [text(), muted(), dim(), danger(), success(), warning()] {
                    assert!(
                        contrast(color, background) >= 7.0,
                        "{color:?} does not meet AAA on {background:?}"
                    );
                }
            }
            assert!(contrast(text(), background()) >= 20.0);
        });
    }

    #[test]
    fn text_on_the_accent_is_readable_in_every_theme() {
        for palette in all_palettes() {
            with_palette(palette, || {
                assert!(contrast(on_accent(), accent()) >= 4.5);
                assert!(contrast(on_accent(), accent_hover()) >= 4.5);
                assert!(
                    luminance(accent_hover()) > luminance(accent()),
                    "hover must lighten the accent"
                );
            });
        }
    }

    #[test]
    fn accent_stays_green_in_every_theme() {
        for palette in all_palettes() {
            with_palette(palette, || {
                let primary = accent();
                assert!(
                    primary.g() > primary.r() && primary.g() > primary.b(),
                    "{} accent should be green",
                    palette.kind.key()
                );
                assert!(primary.b() > primary.r(), "jade leans cool");
            });
        }
    }

    #[test]
    fn upgrade_orbs_keep_the_game_gold() {
        for palette in all_palettes() {
            with_palette(palette, || {
                assert!(orb().r() > orb().b(), "the orbs should stay warm");
                assert_ne!(orb(), accent(), "the orbs must not follow the accent");
                for background in [background(), panel(), surface(), surface_active()] {
                    assert!(
                        contrast(orb(), background) >= 3.0,
                        "{} orb is too faint on {background:?}",
                        palette.kind.key()
                    );
                }
                assert!(
                    luminance(orb_hover()) > luminance(orb()),
                    "hover must lighten the orb"
                );
            });
        }
    }

    #[test]
    fn the_freya_colour_sheet_follows_the_active_palette() {
        // Building the full `Theme` needs a Freya render context (it resolves
        // component defaults); the colour sheet is the palette contract that
        // the UI tests render through the real root.
        for colors in all_palettes() {
            install(UiStyle::with_scale(colors, 1.0));
            let sheet = palette();
            assert_eq!(sheet.primary, colors.accent);
            assert_eq!(sheet.background, colors.background);
            assert_eq!(sheet.text_primary, colors.text);
        }
        install(UiStyle::DEFAULT);
    }

    #[test]
    fn system_preference_resolves_to_light_or_dark() {
        assert_eq!(
            resolved_choice(ThemeChoice::System, PreferredTheme::Light),
            ThemeChoice::Light
        );
        assert_eq!(
            resolved_choice(ThemeChoice::System, PreferredTheme::Dark),
            ThemeChoice::Dark
        );
        assert_eq!(
            resolved_choice(ThemeChoice::HighContrast, PreferredTheme::Light),
            ThemeChoice::HighContrast
        );
    }

    #[test]
    fn text_scale_multiplies_type_space_and_rows() {
        install(UiStyle::new(ThemeChoice::Dark, 200));
        assert!((font::body() - 28.0).abs() < f32::EPSILON);
        assert!((space::md() - 16.0).abs() < f32::EPSILON);
        assert!((radius::card() - 24.0).abs() < f32::EPSILON);
        assert!((layout::save_row_height() - 92.0).abs() < f32::EPSILON);
        assert!((font::line_height() * 3.0 - 84.0).abs() < f32::EPSILON);
        assert!((layout::field_min_width() - 264.0).abs() < f32::EPSILON);
        assert!((layout::target_min() - 56.0).abs() < f32::EPSILON);
        install(UiStyle::DEFAULT);
    }

    #[test]
    fn window_and_image_tokens_do_not_follow_the_text_scale() {
        install(UiStyle::new(ThemeChoice::Dark, 200));
        assert!((layout::window_width() - 1180.0).abs() < f32::EPSILON);
        assert!((layout::min_window_height() - 700.0).abs() < f32::EPSILON);
        assert!((layout::preview() - 120.0).abs() < f32::EPSILON);
        assert!((layout::sidebar_min_width() - 230.0).abs() < f32::EPSILON);
        assert!((layout::scrollbar() - 9.0).abs() < f32::EPSILON);
        install(UiStyle::DEFAULT);
    }

    #[test]
    fn scale_percent_is_clamped_to_the_tested_range() {
        assert_eq!(clamp_scale_percent(50), 100);
        assert_eq!(clamp_scale_percent(200), 200);
        assert_eq!(clamp_scale_percent(400), 200);
    }

    #[test]
    fn font_scale_is_ordered_and_semantic() {
        let sizes = [
            font::micro(),
            font::caption(),
            font::label(),
            font::body(),
            font::card_title(),
            font::subheading(),
            font::heading(),
            font::title(),
            font::display(),
        ];
        assert!(
            sizes.windows(2).all(|pair| pair[0] < pair[1]),
            "font tokens must stay strictly ordered"
        );
        assert!(font::line_height() > font::caption());
    }

    #[test]
    fn the_smallest_type_stays_readable() {
        assert!(font::body() >= 12.0, "body text must be at least 12 px");
        assert!(font::micro() >= 11.0, "micro labels must be at least 11 px");
    }

    #[test]
    fn spacing_scale_is_even_and_ordered() {
        let spaces = [
            space::none(),
            space::xxs(),
            space::xs(),
            space::sm(),
            space::md(),
            space::lg(),
            space::xl(),
            space::field(),
            space::xxl(),
            space::card(),
            space::hero(),
            space::grid(),
            space::page(),
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
            radius::small(),
            radius::control(),
            radius::medium(),
            radius::large(),
            radius::card(),
            radius::panel(),
            radius::pill(),
        ];
        assert!(radii.windows(2).all(|pair| pair[0] < pair[1]));
        let strokes = [stroke::hairline(), stroke::focus(), stroke::emphasis()];
        assert!(strokes.iter().all(|value| *value > 0.0));
        assert!(strokes.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn layout_tokens_are_positive_and_ordered() {
        for value in [
            layout::sidebar_percent(),
            layout::sidebar_min_width(),
            layout::sidebar_max_width(),
            layout::preview(),
            layout::portrait(),
            layout::save_row_height(),
            layout::selection_bar_width(),
            layout::orb(),
            layout::stat_width(),
            layout::busy_slot(),
            layout::skeleton_bar_percent(),
            layout::stepper_width(),
            layout::level_width(),
            layout::item_count_width(),
            layout::upgrade_label_width(),
            layout::picker_row_height(),
            layout::field_narrow(),
            layout::field_medium(),
            layout::field_wide(),
            layout::target_min(),
            layout::chevron_width(),
            layout::sort_arrow_width(),
            layout::sort_arrow_height(),
            layout::divider(),
            layout::scrollbar(),
            layout::window_width(),
            layout::window_height(),
            layout::min_window_width(),
            layout::min_window_height(),
        ] {
            assert!(value > 0.0, "layout token must be positive");
        }
        assert!(layout::sidebar_min_width() < layout::sidebar_max_width());
        assert!(layout::min_window_width() < layout::window_width());
        assert!(layout::min_window_height() < layout::window_height());
        assert!(layout::field_narrow() < layout::field_medium());
        assert!(layout::field_medium() < layout::field_wide());
    }
}
