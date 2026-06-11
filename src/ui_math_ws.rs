use crate::app::MyApp;
use crate::math_ws::{check_step, parse_line, MathStep, StepStatus};
use eframe::egui;

const SYNTAX_HELP: &str = "\
2*x  or  2x     multiplication
x^2             power
(x+1)/(x-1)     fraction
sqrt(x)         square root
sin(x) cos(x)   trig (radians)
ln(x)  exp(x)   natural log, eˣ
log(x)          log base 10
pi  e           constants
2x + 4 = 6      equations: each step
                must keep both sides
                equivalent";

pub fn ui_math_ws(app: &mut MyApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading("Workspace");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Clear").clicked() {
                app.math_ws_steps.clear();
            }
        });
    });
    ui.label(
        egui::RichText::new("First line is the given; every next line is checked against the last good one.")
            .small()
            .weak(),
    );

    egui::CollapsingHeader::new("Syntax").show(ui, |ui| {
        ui.label(egui::RichText::new(SYNTAX_HELP).monospace().small());
    });

    ui.separator();

    let input_height = 60.0;
    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .max_height(ui.available_height() - input_height)
        .show(ui, |ui| {
            for step in &app.math_ws_steps {
                let (mark, color, hover) = match &step.status {
                    StepStatus::Given => (
                        "●",
                        egui::Color32::GRAY,
                        "Given — starting expression".to_string(),
                    ),
                    StepStatus::Valid => (
                        "✓",
                        egui::Color32::from_rgb(80, 200, 100),
                        "Valid — matches at all sample points".to_string(),
                    ),
                    StepStatus::Invalid => (
                        "✗",
                        egui::Color32::from_rgb(230, 80, 80),
                        "Not equivalent to the previous line".to_string(),
                    ),
                    StepStatus::Error(e) => ("⚠", egui::Color32::from_rgb(230, 180, 60), e.clone()),
                };
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(mark).color(color))
                        .on_hover_text(hover);
                    ui.monospace(&step.text);
                });
            }
        });

    ui.separator();
    ui.horizontal(|ui| {
        let input_width = ui.available_width() - 60.0;
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.math_ws_input)
                .desired_width(input_width)
                .font(egui::TextStyle::Monospace)
                .hint_text(if app.math_ws_steps.is_empty() {
                    "given expression..."
                } else {
                    "next step..."
                }),
        );
        let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button("Add").clicked() || enter {
            submit(app);
            response.request_focus();
        }
    });
}

fn submit(app: &mut MyApp) {
    let text = app.math_ws_input.trim().to_string();
    if text.is_empty() {
        return;
    }
    let baseline = app.math_ws_steps.iter().rev().find(|s| s.status.is_good());
    let status = match baseline {
        None => match parse_line(&text) {
            Ok(_) => StepStatus::Given,
            Err(e) => StepStatus::Error(format!("parse error: {e}")),
        },
        Some(base) => check_step(&base.text, &text),
    };
    app.math_ws_steps.push(MathStep { text, status });
    app.math_ws_input.clear();
}
