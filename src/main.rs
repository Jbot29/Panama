use eframe::egui;
use std::sync::Arc;
mod db;
mod helpers;
mod audio;

mod app_structs;
use app_structs::View;

mod app;
use app::MyApp;

mod config;
mod draw;
mod svg_diagram;
mod ui;
mod nodes;
mod quiz;
mod tutor;
mod ui_cards;
mod ui_quiz;
mod ui_tutor_session;
mod ui_tutors;
mod ui_tutor_detail;
mod ui_create_tutor;


impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);
        self.load_next_card(ctx);
        self.load_front_image(ctx);
        self.load_due_count(ctx);
        self.poll_tutor(ctx);
        self.poll_diagram(ctx);
        self.poll_card_chat(ctx);
        self.poll_create_tutor(ctx);
        self.poll_detail_quiz(ctx);

        egui::SidePanel::left("left_controls")
            .resizable(true) // user can drag to resize
            .min_width(220.0)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Panama");
                ui.separator();

                if ui.button("Cards").clicked() {
                    self.view = View::Cards;
                }

                if ui.button("Tutors").clicked() {
                    self.load_available_tutors();
                    self.view = View::Tutors;
                }

                if ui.button("Quiz").clicked() {
                    self.load_available_tutors();
                    self.quiz_active_tutor = None;
                    self.quiz_questions.clear();
                    self.view = View::Quiz;
                }

            });
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.view {
                View::Base => {
                    ui.vertical_centered(|ui| {
                        ui.heading("Panama — Digital Aristotle");
                    });
                }
                View::Cards => {
                    ui_cards::ui_cards_hub(self, ui);
                }
                View::Review => {
                    ui_cards::ui_review(self, ui, ctx);
                }
                View::EditCard => {
                    ui_cards::ui_edit_card(self, ui, ctx);
                }
                View::Quiz => {
                    ui_quiz::ui_quiz(self, ui);
                }

                View::NewCard => {
                    ui_cards::ui_new_card(self, ui, ctx);
                }

                View::Tutors => {
                    ui_tutors::ui_tutors(self, ui);
                }
                View::CreateTutor => {
                    ui_create_tutor::ui_create_tutor(self, ui);
                }
                View::TutorDetail => {
                    ui_tutor_detail::ui_tutor_detail(self, ui);
                }

                View::TutorSession => {
                    ui_tutor_session::ui_tutor_session(self, ui);
                }
            }
        });
    }
}

fn main() {
    // Load ANTHROPIC_API_KEY (and any other vars) from a `.env` file at the
    // project root if present. Real environment variables take precedence.
    let _ = dotenvy::dotenv();

    let icon = eframe::icon_data::from_png_bytes(include_bytes!(".././thumbnail.png"))
        .expect("Failed to load icon.png");

    let options = eframe::NativeOptions {
        // for eframe ≥ 0.30:
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_icon(Arc::new(icon)),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Panama",
        options,
        Box::new(|cc| {
            use egui::{FontFamily, FontId, TextStyle};
            let mut style = (*cc.egui_ctx.style()).clone();
            style.text_styles = [
                (
                    TextStyle::Heading,
                    FontId::new(24.0, FontFamily::Proportional),
                ),
                (TextStyle::Body, FontId::new(19.0, FontFamily::Proportional)),
                (
                    TextStyle::Button,
                    FontId::new(16.0, FontFamily::Proportional),
                ),
                (
                    TextStyle::Monospace,
                    FontId::new(15.0, FontFamily::Monospace),
                ),
                (
                    TextStyle::Small,
                    FontId::new(13.0, FontFamily::Proportional),
                ),
            ]
            .into();
            cc.egui_ctx.set_style(style);

            // Bundle a font with broad Unicode coverage (superscripts like ˣ, math
            // symbols, checkmarks, arrows, etc.) as a fallback for glyphs missing
            // from egui's built-in font. Embedded in the binary so coverage is
            // identical on every platform — no reliance on OS-specific font paths.
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "unicode_fallback".to_owned(),
                egui::FontData::from_static(include_bytes!(
                    "../assets/fonts/DejaVuSans.ttf"
                ))
                .into(),
            );
            for family in fonts.families.values_mut() {
                family.push("unicode_fallback".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);

            let app = MyApp::new_with_db().expect("Failed to init MyApp with DB");
            Ok::<Box<dyn eframe::App>, _>(Box::new(app))
        }),
    );
}
