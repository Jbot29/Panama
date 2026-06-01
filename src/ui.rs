use eframe::egui;

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)).round() as u8
}

pub fn value_tinted(base: egui::Color32, brush_value: f32) -> egui::Color32 {
    let t = brush_value.clamp(0.0, 1.0);

    // split around 0.5 so we can go darker or lighter than the base color
    if t < 0.5 {
        // 0.0..0.5 : black -> base
        let k = t / 0.5;
        let [r, g, b, a] = base.to_array();
        egui::Color32::from_rgba_unmultiplied(
            lerp_u8(0, r, k),
            lerp_u8(0, g, k),
            lerp_u8(0, b, k),
            a,
        )
    } else {
        // 0.5..1.0 : base -> white
        let k = (t - 0.5) / 0.5;
        let [r, g, b, a] = base.to_array();
        egui::Color32::from_rgba_unmultiplied(
            lerp_u8(r, 255, k),
            lerp_u8(g, 255, k),
            lerp_u8(b, 255, k),
            a,
        )
    }
}

//https://docs.rs/egui/latest/egui/struct.Color32.html
pub fn primary_color_picker(ui: &mut egui::Ui, current: &mut egui::Color32, brush_value: f32) {
    let colors = [
        egui::Color32::BLACK,
        egui::Color32::WHITE,
        egui::Color32::RED,
        egui::Color32::YELLOW,
        egui::Color32::BLUE,
        egui::Color32::ORANGE,
        egui::Color32::GREEN,
        egui::Color32::PURPLE,
    ];

    ui.horizontal(|ui| {
        for &color in &colors {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
            ui.painter().rect_filled(rect, 4.0, color);
            if *current == color {
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    egui::Stroke::new(2.5, egui::Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            }
            if response.clicked() {
                *current = color;
            }
        }

        // Live preview of the actual rendered colour (base + shade)
        let preview = value_tinted(*current, brush_value);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 4.0, preview);
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(120)),
            egui::StrokeKind::Outside,
        );
    });
}

