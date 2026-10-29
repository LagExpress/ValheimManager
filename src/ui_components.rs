use crate::appearance::Palette;
use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Ui};

pub const TABLE_BREAKPOINT: f32 = 900.0;
pub const SYNC_BREAKPOINT: f32 = 1100.0;
pub const MAX_CONTENT_WIDTH: f32 = 1440.0;

/// Center page contents without narrowing the parent scroll viewport.
pub fn centered_page(ui: &mut Ui, contents: impl FnOnce(&mut Ui)) {
    let available = ui.available_rect_before_wrap();
    let width = (available.width() - 32.0).clamp(0.0, MAX_CONTENT_WIDTH);
    let rect = egui::Rect::from_min_size(
        egui::pos2(available.center().x - width / 2.0, available.top()),
        egui::vec2(width, available.height()),
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        contents,
    );
}

pub fn card(ui: &mut Ui, title: &str, content: impl FnOnce(&mut Ui)) {
    let palette = Palette::for_ui(ui);
    let glass = ui.ctx().data(|data| {
        data.get_temp::<bool>(egui::Id::unique("glass_ui"))
            .unwrap_or(false)
    });
    let mut frame = egui::Frame::new()
        .fill(ui.visuals().panel_fill)
        .stroke(Stroke::new(
            1.0,
            if glass {
                palette.border.lerp_to_gamma(palette.text, 0.12)
            } else {
                palette.border
            },
        ))
        .corner_radius(12)
        .inner_margin(16);
    if glass {
        frame = frame.shadow(egui::epaint::Shadow {
            offset: [0, 3],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(if ui.visuals().dark_mode { 36 } else { 16 }),
        });
    }
    frame.show(ui, |ui| {
        ui.set_min_width((ui.available_width() - 1.0).max(0.0));
        ui.label(RichText::new(title).size(17.0).strong());
        ui.add_space(4.0);
        content(ui);
    });
    ui.add_space(8.0);
}

pub fn field(ui: &mut Ui, label: &str, value: &mut String) -> egui::Response {
    ui.label(RichText::new(label).small().strong());
    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY))
}

pub fn path(ui: &mut Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).small().strong());
    ui.horizontal(|ui| {
        let spanish = ui.ctx().data(|data| {
            data.get_temp::<bool>(egui::Id::unique("spanish_ui"))
                .unwrap_or(false)
        });
        let copy_label = if spanish { "Copiar" } else { "Copy" };
        let button_width = ui
            .painter()
            .layout_no_wrap(
                copy_label.into(),
                egui::FontId::proportional(14.0),
                ui.visuals().text_color(),
            )
            .size()
            .x
            + ui.spacing().button_padding.x * 2.0
            + ui.spacing().item_spacing.x;
        let available = (ui.available_width() - button_width).max(40.0);
        left_label(
            ui,
            egui::vec2(available, 22.0),
            RichText::new(value).monospace(),
        )
        .on_hover_text(value);
        if ui.small_button(copy_label).clicked() {
            ui.ctx().copy_text(value.to_owned());
        }
    });
}

pub fn left_label(ui: &mut Ui, size: egui::Vec2, text: RichText) -> egui::Response {
    ui.allocate_ui_with_layout(
        size,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(size);
            ui.add(egui::Label::new(text).truncate())
        },
    )
    .inner
}

pub fn primary(ui: &mut Ui, label: &str) -> egui::Response {
    let p = Palette::for_ui(ui);
    ui.add(egui::Button::new(RichText::new(label).color(p.on_accent).strong()).fill(p.accent))
}

pub fn badge(ui: &mut Ui, label: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.10))
        .corner_radius(5)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(label).small().color(color).strong());
        });
}

#[derive(Clone, Copy)]
pub enum Icon {
    Shield,
    Sun,
    Moon,
    Search,
    Folder,
    Layers,
    Server,
    Check,
    Info,
    Build,
}

pub fn icon(ui: &mut Ui, icon: Icon, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let p = ui.painter();
    let point = |x: f32, y: f32| Pos2::new(rect.left() + x * size, rect.top() + y * size);
    let stroke = Stroke::new(1.6, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([point(a.0, a.1), point(b.0, b.1)], stroke);
    };
    match icon {
        Icon::Shield => {
            p.add(egui::Shape::closed_line(
                vec![
                    point(0.18, 0.14),
                    point(0.82, 0.14),
                    point(0.78, 0.62),
                    point(0.5, 0.9),
                    point(0.22, 0.62),
                ],
                stroke,
            ));
            line((0.5, 0.28), (0.5, 0.65));
            line((0.35, 0.43), (0.65, 0.43));
        }
        Icon::Sun => {
            p.circle_stroke(rect.center(), size * 0.22, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let v = egui::vec2(a.cos(), a.sin());
                p.line_segment(
                    [
                        rect.center() + v * size * 0.33,
                        rect.center() + v * size * 0.44,
                    ],
                    stroke,
                );
            }
        }
        Icon::Moon => {
            p.add(egui::Shape::line(
                (0..18)
                    .map(|i| {
                        let a = 0.7 + i as f32 / 17.0 * 4.6;
                        point(0.52 + a.cos() * 0.35, 0.48 + a.sin() * 0.35)
                    })
                    .collect(),
                stroke,
            ));
            line((0.79, 0.24), (0.63, 0.48));
            line((0.63, 0.48), (0.79, 0.77));
        }
        Icon::Search => {
            p.circle_stroke(point(0.43, 0.43), size * 0.27, stroke);
            line((0.64, 0.64), (0.88, 0.88));
        }
        Icon::Folder => {
            p.add(egui::Shape::closed_line(
                vec![
                    point(0.1, 0.25),
                    point(0.4, 0.25),
                    point(0.5, 0.35),
                    point(0.9, 0.35),
                    point(0.9, 0.8),
                    point(0.1, 0.8),
                ],
                stroke,
            ));
        }
        Icon::Layers => {
            for y in [0.3, 0.5, 0.7] {
                line((0.15, y), (0.5, y + 0.15));
                line((0.5, y + 0.15), (0.85, y));
            }
        }
        Icon::Server => {
            for y in [0.15, 0.55] {
                p.rect_stroke(
                    egui::Rect::from_min_max(point(0.13, y), point(0.87, y + 0.28)),
                    3,
                    stroke,
                    egui::StrokeKind::Inside,
                );
                p.circle_filled(point(0.26, y + 0.14), size * 0.035, color);
            }
        }
        Icon::Check => {
            line((0.16, 0.52), (0.4, 0.76));
            line((0.4, 0.76), (0.85, 0.23));
        }
        Icon::Info => {
            p.circle_stroke(rect.center(), size * 0.38, stroke);
            line((0.5, 0.45), (0.5, 0.72));
            p.circle_filled(point(0.5, 0.28), size * 0.04, color);
        }
        Icon::Build => {
            line((0.2, 0.2), (0.8, 0.8));
            line((0.8, 0.2), (0.2, 0.8));
            line((0.12, 0.12), (0.35, 0.12));
            line((0.88, 0.12), (0.88, 0.35));
        }
    }
}

pub fn backdrop(ui: &Ui, background: Color32, accent: Color32, glass: bool) {
    let rect = ui.max_rect();
    let mut mesh = egui::Mesh::default();
    let right = if glass {
        background.lerp_to_gamma(Palette::for_ui(ui).info, 0.22)
    } else {
        background
    };
    mesh.colored_vertex(rect.left_top(), background);
    mesh.colored_vertex(rect.right_top(), right);
    mesh.colored_vertex(rect.right_bottom(), background);
    mesh.colored_vertex(
        rect.left_bottom(),
        if glass {
            background.lerp_to_gamma(accent, 0.18)
        } else {
            background
        },
    );
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(egui::Shape::mesh(mesh));
}
