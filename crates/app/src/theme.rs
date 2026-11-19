//! Interface colors: light and dark themes plus a high-contrast variant of each.
//!
//! Pure functions that build `egui::Visuals`, so the contrast of the palette can be unit tested.

use eframe::egui::{self, Color32, Stroke, Theme as EguiTheme, ThemePreference, Visuals};
use scopeview_core::Theme;

#[cfg(test)]
/// Smallest contrast ratio (WCAG AA) for normal text.
pub const MIN_TEXT_CONTRAST: f32 = 4.5;
#[cfg(test)]
/// Smallest contrast ratio (WCAG AAA) the high-contrast variant must reach.
pub const MIN_HIGH_CONTRAST: f32 = 7.0;

#[cfg(test)]
fn channel(value: u8) -> f32 {
    let c = f32::from(value) / 255.0;
    if c <= 0.039_28 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
/// Relative luminance as defined by WCAG 2.
fn luminance(color: Color32) -> f32 {
    0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
}

#[cfg(test)]
/// WCAG contrast ratio between two opaque colors, from 1.0 (identical) to 21.0 (black on white).
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// The visuals for one theme. `high_contrast` uses pure black or white backgrounds, full-strength
/// text and thick outlines.
pub fn visuals(theme: EguiTheme, high_contrast: bool) -> Visuals {
    let dark = theme == EguiTheme::Dark;
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    // egui's stock warning and error colors are too faint on the light theme.
    if dark {
        v.warn_fg_color = Color32::from_rgb(255, 200, 80);
        v.error_fg_color = Color32::from_rgb(255, 120, 120);
        v.hyperlink_color = Color32::from_rgb(120, 175, 255);
    } else {
        v.warn_fg_color = Color32::from_rgb(130, 75, 0);
        v.error_fg_color = Color32::from_rgb(175, 0, 0);
        v.hyperlink_color = Color32::from_rgb(0, 70, 170);
    }
    if !high_contrast {
        return v;
    }

    let (bg, fg, raised, hover, accent) = if dark {
        (
            Color32::BLACK,
            Color32::WHITE,
            Color32::from_gray(28),
            Color32::from_gray(64),
            Color32::from_rgb(255, 235, 59),
        )
    } else {
        (
            Color32::WHITE,
            Color32::BLACK,
            Color32::from_gray(232),
            Color32::from_gray(200),
            Color32::from_rgb(0, 50, 160),
        )
    };
    v.override_text_color = Some(fg);
    v.panel_fill = bg;
    v.window_fill = bg;
    v.extreme_bg_color = bg;
    v.faint_bg_color = raised;
    v.window_stroke = Stroke::new(2.0, fg);
    v.warn_fg_color = if dark {
        Color32::from_rgb(255, 214, 10)
    } else {
        Color32::from_rgb(110, 60, 0)
    };
    v.error_fg_color = if dark {
        Color32::from_rgb(255, 140, 140)
    } else {
        Color32::from_rgb(150, 0, 0)
    };
    v.hyperlink_color = accent;
    v.selection.bg_fill = if dark {
        Color32::from_rgb(0, 70, 190)
    } else {
        Color32::from_rgb(0, 50, 160)
    };
    v.selection.stroke = Stroke::new(2.0, Color32::WHITE);

    v.widgets.noninteractive.bg_fill = bg;
    v.widgets.noninteractive.weak_bg_fill = bg;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, fg);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, fg);
    for widget in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        widget.bg_fill = raised;
        widget.weak_bg_fill = raised;
        widget.bg_stroke = Stroke::new(1.5, fg);
        widget.fg_stroke = Stroke::new(1.5, fg);
    }
    v.widgets.hovered.bg_fill = hover;
    v.widgets.hovered.weak_bg_fill = hover;
    v.widgets.hovered.bg_stroke = Stroke::new(2.5, accent);
    v.widgets.active.bg_fill = hover;
    v.widgets.active.weak_bg_fill = hover;
    v.widgets.active.bg_stroke = Stroke::new(2.5, accent);
    v
}

/// Apply the theme choice to the whole UI. Cheap enough to call whenever a setting changes.
pub fn apply(ctx: &egui::Context, theme: Theme, high_contrast: bool) {
    ctx.set_theme(match theme {
        Theme::System => ThemePreference::System,
        Theme::Light => ThemePreference::Light,
        Theme::Dark => ThemePreference::Dark,
    });
    ctx.set_visuals_of(EguiTheme::Dark, visuals(EguiTheme::Dark, high_contrast));
    ctx.set_visuals_of(EguiTheme::Light, visuals(EguiTheme::Light, high_contrast));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_ratio_matches_the_wcag_extremes() {
        let max = contrast_ratio(Color32::BLACK, Color32::WHITE);
        assert!((max - 21.0).abs() < 0.01, "{max}");
        assert!((contrast_ratio(Color32::RED, Color32::RED) - 1.0).abs() < 1e-6);
        // Order does not matter.
        assert_eq!(
            contrast_ratio(Color32::BLACK, Color32::from_gray(128)),
            contrast_ratio(Color32::from_gray(128), Color32::BLACK)
        );
        // Mid gray on white is the classic 3.95:1.
        let gray = contrast_ratio(Color32::from_gray(128), Color32::WHITE);
        assert!((gray - 3.95).abs() < 0.05, "{gray}");
    }

    /// Foreground and background pairs that carry text in the UI.
    fn text_pairs(v: &Visuals) -> Vec<(&'static str, Color32, Color32)> {
        vec![
            ("text on panel", v.text_color(), v.panel_fill),
            ("text on window", v.text_color(), v.window_fill),
            ("text in fields", v.text_color(), v.extreme_bg_color),
            ("warning on panel", v.warn_fg_color, v.panel_fill),
            ("error on panel", v.error_fg_color, v.panel_fill),
            ("link on panel", v.hyperlink_color, v.panel_fill),
            (
                "idle button",
                v.widgets.inactive.fg_stroke.color,
                v.widgets.inactive.weak_bg_fill,
            ),
            (
                "hovered button",
                v.widgets.hovered.fg_stroke.color,
                v.widgets.hovered.weak_bg_fill,
            ),
            (
                "pressed button",
                v.widgets.active.fg_stroke.color,
                v.widgets.active.weak_bg_fill,
            ),
            (
                "selected item",
                v.selection.stroke.color,
                v.selection.bg_fill,
            ),
        ]
    }

    #[test]
    fn every_theme_reaches_aa_contrast_for_text() {
        for theme in [EguiTheme::Dark, EguiTheme::Light] {
            let v = visuals(theme, false);
            for (what, fg, bg) in text_pairs(&v) {
                let ratio = contrast_ratio(fg, bg);
                assert!(
                    ratio >= MIN_TEXT_CONTRAST,
                    "{theme:?} {what}: {ratio:.2} < {MIN_TEXT_CONTRAST}"
                );
            }
        }
    }

    #[test]
    fn high_contrast_reaches_aaa() {
        for theme in [EguiTheme::Dark, EguiTheme::Light] {
            let v = visuals(theme, true);
            for (what, fg, bg) in text_pairs(&v) {
                let ratio = contrast_ratio(fg, bg);
                assert!(
                    ratio >= MIN_HIGH_CONTRAST,
                    "{theme:?} high contrast {what}: {ratio:.2} < {MIN_HIGH_CONTRAST}"
                );
            }
        }
    }

    #[test]
    fn high_contrast_outlines_are_thicker() {
        for theme in [EguiTheme::Dark, EguiTheme::Light] {
            let normal = visuals(theme, false);
            let high = visuals(theme, true);
            assert!(
                high.widgets.inactive.bg_stroke.width > normal.widgets.inactive.bg_stroke.width
            );
            assert!(high.widgets.hovered.bg_stroke.width >= 2.0);
        }
    }
}
