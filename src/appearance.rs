use eframe::egui::{self, Color32, FontFamily, FontId, Stroke, TextStyle};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    Light,
    Dark,
    GlassLight,
    GlassDark,
}

impl ThemePreset {
    pub const ALL: [Self; 4] = [Self::Light, Self::Dark, Self::GlassLight, Self::GlassDark];

    pub fn dark(self) -> bool {
        matches!(self, Self::Dark | Self::GlassDark)
    }

    pub fn glass(self) -> bool {
        matches!(self, Self::GlassLight | Self::GlassDark)
    }

    pub fn label(self, spanish: bool) -> &'static str {
        match (self, spanish) {
            (Self::Light, false) => "Light",
            (Self::Dark, false) => "Dark",
            (Self::GlassLight, false) => "Glass Light",
            (Self::GlassDark, false) => "Glass Dark",
            (Self::Light, true) => "Claro",
            (Self::Dark, true) => "Oscuro",
            (Self::GlassLight, true) => "Cristal claro",
            (Self::GlassDark, true) => "Cristal oscuro",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UiPreferences {
    pub theme: Option<ThemePreset>,
}

impl UiPreferences {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("valheim-modpack-studio/appearance.json")
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }

    pub fn resolve(&self, ctx: &egui::Context) -> ThemePreset {
        self.theme.unwrap_or_else(|| {
            if ctx.system_theme() == Some(egui::Theme::Light) {
                ThemePreset::Light
            } else {
                ThemePreset::Dark
            }
        })
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub surface: Color32,
    pub elevated: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub border: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub info: Color32,
}

impl Palette {
    pub fn new(theme: ThemePreset, native_glass: bool) -> Self {
        let dark = theme.dark();
        let mut p = if dark {
            Self {
                background: Color32::from_rgb(16, 22, 29),
                surface: Color32::from_rgb(27, 38, 49),
                elevated: Color32::from_rgb(37, 51, 64),
                text: Color32::from_rgb(239, 243, 246),
                muted: Color32::from_rgb(168, 180, 193),
                accent: Color32::from_rgb(214, 173, 112),
                on_accent: Color32::from_rgb(26, 23, 18),
                border: Color32::from_rgb(62, 78, 91),
                success: Color32::from_rgb(111, 208, 165),
                warning: Color32::from_rgb(232, 191, 114),
                danger: Color32::from_rgb(240, 140, 140),
                info: Color32::from_rgb(141, 187, 232),
            }
        } else {
            Self {
                background: Color32::from_rgb(244, 242, 237),
                surface: Color32::from_rgb(255, 254, 250),
                elevated: Color32::from_rgb(234, 231, 223),
                text: Color32::from_rgb(23, 35, 45),
                muted: Color32::from_rgb(81, 95, 108),
                accent: Color32::from_rgb(135, 87, 32),
                on_accent: Color32::WHITE,
                border: Color32::from_rgb(199, 201, 197),
                success: Color32::from_rgb(28, 112, 78),
                warning: Color32::from_rgb(138, 91, 0),
                danger: Color32::from_rgb(177, 45, 59),
                info: Color32::from_rgb(39, 93, 152),
            }
        };
        if theme.glass() {
            if native_glass {
                p.background = p.background.gamma_multiply(if dark { 0.72 } else { 0.80 });
            }
            p.surface = p.surface.gamma_multiply(if dark { 0.88 } else { 0.92 });
        }
        p
    }

    pub fn for_ui(ui: &egui::Ui) -> Self {
        Self::new(
            if ui.visuals().dark_mode {
                ThemePreset::Dark
            } else {
                ThemePreset::Light
            },
            false,
        )
    }

    pub fn apply(self, ctx: &egui::Context, theme: ThemePreset) {
        ctx.set_theme(if theme.dark() {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        let mut visuals = if theme.dark() {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.override_text_color = None;
        visuals.weak_text_color = Some(self.muted);
        visuals.panel_fill = self.surface;
        visuals.window_fill = self.surface;
        visuals.extreme_bg_color = self.background;
        visuals.faint_bg_color = self.elevated.gamma_multiply(0.30);
        visuals.window_corner_radius = 12.into();
        visuals.window_stroke = Stroke::new(1.0, self.border);
        visuals.selection.bg_fill = self.accent;
        visuals.selection.stroke = Stroke::new(1.5, self.on_accent);
        visuals.hyperlink_color = self.info;
        visuals.warn_fg_color = self.warning;
        visuals.error_fg_color = self.danger;
        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.corner_radius = 7.into();
            widget.bg_stroke = Stroke::new(1.0, self.border);
            widget.fg_stroke = Stroke::new(1.0, self.text);
        }
        visuals.widgets.noninteractive.bg_fill = self.surface;
        visuals.widgets.inactive.bg_fill = self.elevated;
        visuals.widgets.inactive.weak_bg_fill = self.elevated;
        visuals.widgets.hovered.bg_fill = self.elevated.lerp_to_gamma(self.accent, 0.15);
        visuals.widgets.hovered.weak_bg_fill = visuals.widgets.hovered.bg_fill;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.5, self.accent);
        visuals.widgets.active.bg_fill = self.elevated.lerp_to_gamma(self.accent, 0.25);
        visuals.widgets.active.weak_bg_fill = visuals.widgets.active.bg_fill;
        visuals.widgets.active.bg_stroke = Stroke::new(2.0, self.accent);
        visuals.widgets.open.bg_stroke = Stroke::new(2.0, self.accent);
        ctx.set_visuals(visuals);
        ctx.style_mut_of(
            if theme.dark() {
                egui::Theme::Dark
            } else {
                egui::Theme::Light
            },
            |style| {
                style.spacing.item_spacing = egui::vec2(8.0, 8.0);
                style.spacing.button_padding = egui::vec2(12.0, 7.0);
                style.spacing.interact_size.y = 32.0;
                style.spacing.window_margin = egui::Margin::same(16);
                style
                    .text_styles
                    .insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
                style.text_styles.insert(
                    TextStyle::Button,
                    FontId::new(14.0, FontFamily::Proportional),
                );
                style.text_styles.insert(
                    TextStyle::Heading,
                    FontId::new(24.0, FontFamily::Proportional),
                );
                style.text_styles.insert(
                    TextStyle::Small,
                    FontId::new(12.0, FontFamily::Proportional),
                );
                style.text_styles.insert(
                    TextStyle::Monospace,
                    FontId::new(13.0, FontFamily::Monospace),
                );
                // Let each layout choose wrapping: prose wraps vertically while
                // buttons in horizontal action rows retain a single line.
                style.wrap_mode = None;
            },
        );
    }
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Inter".into(),
        egui::FontData::from_static(include_bytes!("assets/InterVariable.ttf")).into(),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Inter".into());
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_roundtrip_and_invalid_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("appearance.json");
        assert!(UiPreferences::load(&path).theme.is_none());
        for theme in ThemePreset::ALL {
            UiPreferences { theme: Some(theme) }.save(&path).unwrap();
            assert_eq!(UiPreferences::load(&path).theme, Some(theme));
        }
        std::fs::write(&path, r#"{"theme":"future_theme"}"#).unwrap();
        assert!(UiPreferences::load(&path).theme.is_none());
        std::fs::write(&path, "broken").unwrap();
        assert!(UiPreferences::load(&path).theme.is_none());
    }

    #[test]
    fn bundled_fonts_cover_localized_interface_glyphs() {
        let ctx = egui::Context::default();
        install_fonts(&ctx);
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.fonts_mut(|fonts| {
                let font = FontId::proportional(15.0);
                for source in [include_str!("i18n.rs"), include_str!("app/ui.rs"), "📦⚙"] {
                    for character in source.chars().filter(|c| !c.is_ascii()) {
                        assert!(
                            fonts.has_glyph(&font, character),
                            "Missing glyph {character:?}"
                        );
                    }
                }
            });
        });
        output.drop_without_applying_deltas();
    }

    #[test]
    fn all_themes_keep_text_and_semantic_colors_readable() {
        fn over(foreground: Color32, background: Color32) -> Color32 {
            let [r, g, b, a] = foreground.to_srgba_unmultiplied();
            let alpha = a as f32 / 255.0;
            let mix = |f: u8, b: u8| (f as f32 * alpha + b as f32 * (1.0 - alpha)).round() as u8;
            Color32::from_rgb(
                mix(r, background.r()),
                mix(g, background.g()),
                mix(b, background.b()),
            )
        }
        fn luminance(color: Color32) -> f32 {
            let channel = |v: u8| {
                let v = v as f32 / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            channel(color.r()) * 0.2126 + channel(color.g()) * 0.7152 + channel(color.b()) * 0.0722
        }
        fn contrast(a: Color32, b: Color32) -> f32 {
            let a = luminance(a);
            let b = luminance(b);
            (a.max(b) + 0.05) / (a.min(b) + 0.05)
        }
        for theme in ThemePreset::ALL {
            for native in [false, true] {
                let p = Palette::new(theme, native);
                for desktop in [Color32::BLACK, Color32::WHITE] {
                    let background = over(p.background, desktop);
                    let panel = over(p.surface, background);
                    for color in [p.text, p.muted, p.success, p.warning, p.danger, p.info] {
                        assert!(
                            contrast(color, panel) >= 4.5,
                            "{theme:?}: {:?} on {:?}",
                            color,
                            panel
                        );
                    }
                }
                assert!(contrast(p.accent, p.on_accent) >= 4.5);
            }
        }
    }
}
