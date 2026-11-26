//! Apariencia: paleta clara/oscura, tarjetas y etiquetas de estado.

use egui::{Color32, CornerRadius, Frame, Margin, RichText, Stroke, Ui};

#[derive(Clone, Copy)]
pub struct Pal {
    pub bg: Color32,
    pub panel: Color32,
    pub card: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub primary_bg: Color32,
    pub reviewed_bg: Color32,
    pub green: Color32,
    pub amber: Color32,
    pub red: Color32,
    pub gray: Color32,
    pub nav: Color32,
    pub nav_text: Color32,
}

pub fn pal(dark: bool) -> Pal {
    if dark {
        Pal {
            bg: Color32::from_rgb(17, 20, 27),
            panel: Color32::from_rgb(24, 28, 37),
            card: Color32::from_rgb(30, 35, 46),
            border: Color32::from_rgb(48, 55, 70),
            text: Color32::from_rgb(230, 234, 242),
            muted: Color32::from_rgb(150, 160, 178),
            accent: Color32::from_rgb(96, 145, 255),
            accent_soft: Color32::from_rgb(36, 50, 84),
            primary_bg: Color32::from_rgb(37, 99, 235),
            reviewed_bg: Color32::from_rgb(16, 122, 74),
            green: Color32::from_rgb(70, 190, 130),
            amber: Color32::from_rgb(235, 170, 60),
            red: Color32::from_rgb(240, 100, 100),
            gray: Color32::from_rgb(120, 128, 142),
            nav: Color32::from_rgb(13, 16, 22),
            nav_text: Color32::from_rgb(200, 207, 220),
        }
    } else {
        Pal {
            bg: Color32::from_rgb(244, 246, 250),
            panel: Color32::from_rgb(255, 255, 255),
            card: Color32::from_rgb(255, 255, 255),
            border: Color32::from_rgb(225, 229, 238),
            text: Color32::from_rgb(28, 33, 45),
            muted: Color32::from_rgb(92, 101, 117),
            accent: Color32::from_rgb(37, 99, 235),
            accent_soft: Color32::from_rgb(226, 236, 254),
            primary_bg: Color32::from_rgb(37, 99, 235),
            reviewed_bg: Color32::from_rgb(15, 118, 70),
            green: Color32::from_rgb(15, 118, 70),
            amber: Color32::from_rgb(161, 98, 7),
            red: Color32::from_rgb(210, 55, 55),
            gray: Color32::from_rgb(130, 138, 152),
            nav: Color32::from_rgb(22, 30, 48),
            nav_text: Color32::from_rgb(205, 213, 230),
        }
    }
}

pub fn aplicar(ctx: &egui::Context, dark: bool) {
    let p = pal(dark);
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.bg;
    v.window_fill = p.panel;
    v.extreme_bg_color = if dark { Color32::from_rgb(20, 24, 32) } else { Color32::from_rgb(250, 251, 253) };
    v.faint_bg_color = if dark { Color32::from_rgb(27, 31, 41) } else { Color32::from_rgb(247, 249, 252) };
    v.selection.bg_fill = p.accent_soft;
    v.selection.stroke = Stroke::new(1.0_f32, p.accent);
    v.hyperlink_color = p.accent;
    v.window_corner_radius = CornerRadius::same(10);
    v.override_text_color = None;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = CornerRadius::same(6);
    }
    v.widgets.inactive.bg_fill = if dark { Color32::from_rgb(40, 46, 60) } else { Color32::from_rgb(238, 241, 247) };
    v.widgets.inactive.weak_bg_fill = v.widgets.inactive.bg_fill;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
    v.widgets.hovered.bg_fill = p.accent_soft;
    v.widgets.hovered.weak_bg_fill = p.accent_soft;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.button_padding = egui::vec2(10.0, 5.0);
        s.spacing.interact_size.y = 26.0;
        use egui::{FontId, TextStyle};
        s.text_styles.insert(TextStyle::Body, FontId::proportional(14.0));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(14.0));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(12.0));
        s.text_styles.insert(TextStyle::Heading, FontId::proportional(21.0));
        s.text_styles.insert(TextStyle::Monospace, FontId::monospace(13.0));
    });
}

pub fn card(p: &Pal) -> Frame {
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14))
}

pub fn chip(ui: &mut Ui, text: &str, color: Color32, dark: bool) {
    let fill = if dark { color.gamma_multiply(0.25) } else { color.gamma_multiply(0.14) };
    Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(12.0).color(color).strong());
        });
}

pub fn primary(ui: &mut Ui, p: &Pal, text: &str) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(text).color(Color32::WHITE).strong()).fill(p.primary_bg).corner_radius(CornerRadius::same(7)))
}

pub fn titulo(ui: &mut Ui, p: &Pal, t: &str, sub: &str) {
    ui.label(RichText::new(t).size(22.0).strong().color(p.text));
    if !sub.is_empty() {
        ui.label(RichText::new(sub).size(13.0).color(p.muted));
    }
    ui.add_space(6.0);
}

pub fn seccion(ui: &mut Ui, p: &Pal, t: &str) {
    ui.label(RichText::new(t.to_uppercase()).size(11.5).strong().color(p.muted));
}

#[cfg(test)]
mod pruebas_contraste {
    use super::*;

    fn luminancia(c: Color32) -> f64 {
        let lineal = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * lineal(c.r()) + 0.7152 * lineal(c.g()) + 0.0722 * lineal(c.b())
    }

    #[test]
    fn cinco_pares_aa_sin_redondear() {
        let claro = pal(false);
        let oscuro = pal(true);
        for color in [claro.amber, claro.green, claro.muted, oscuro.primary_bg, oscuro.reviewed_bg] {
            let ratio = (luminancia(Color32::WHITE) + 0.05) / (luminancia(color) + 0.05);
            assert!(ratio >= 4.5, "{color:?}: {ratio}");
        }
    }
}
