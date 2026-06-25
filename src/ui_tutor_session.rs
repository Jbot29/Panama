use crate::app::MyApp;
use crate::app_structs::View;
use crate::tutor::TutorState;
use eframe::egui;
use egui::{Color32, RichText, ScrollArea};

// Number of visible rows in the answer input box. The chat scroll area
// reserves height for this so the input never spills past the window.
const INPUT_ROWS: usize = 5;

pub fn ui_tutor_session(app: &mut MyApp, ui: &mut egui::Ui) {
    // ── top bar ──────────────────────────────────────────────
    ui.horizontal(|ui| {
        if ui.button("< Tutors").clicked() {
            app.tutor_office_hours_prompt = None;
            app.tutor_project_design = false;
            app.load_available_tutors();
            app.view = View::Tutors;
        }
        let office_hours = app.tutor_office_hours_prompt.is_some();
        let project_design = app.tutor_project_design;
        if let Some(node) = &app.tutor_current_node {
            let mastery_pct = (node.mastery_score * 100.0).round() as u32;
            ui.label(
                RichText::new(format!("{}  (mastery {}%)", node.name, mastery_pct))
                    .size(16.0)
                    .color(Color32::from_rgb(180, 200, 255)),
            );
        } else if project_design {
            ui.label(
                RichText::new("🛠 Design a Project")
                    .size(16.0)
                    .color(Color32::from_rgb(210, 190, 140)),
            );
        } else if office_hours {
            ui.label(
                RichText::new("☕ Office Hours")
                    .size(16.0)
                    .color(Color32::from_rgb(210, 190, 140)),
            );
        } else {
            ui.label("Picking topic...");
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if project_design {
                // Free-form design chat: leave, or write the brief to a file.
                if ui.button("Done").clicked() {
                    app.tutor_office_hours_prompt = None;
                    app.tutor_project_design = false;
                    app.load_available_tutors();
                    app.view = View::Tutors;
                }
                let can_save = app.tutor_state == TutorState::Idle
                    && app.tutor_project_save_rx.is_none()
                    && app.tutor_messages.iter().any(|m| m.role == "assistant");
                if ui
                    .add_enabled(can_save, egui::Button::new("💾 Save as Project"))
                    .clicked()
                {
                    app.save_project();
                }
            } else if office_hours {
                // No mastery rating in office hours — it isn't a graded node.
                if ui.button("Done").clicked() {
                    app.tutor_office_hours_prompt = None;
                    app.load_available_tutors();
                    app.view = View::Tutors;
                }
            } else {
                if ui.button("Next topic").clicked() {
                    app.load_available_tutors();
                    app.view = View::Tutors;
                }
                if ui.button("Still struggling").clicked() {
                    app.tutor_rate(-0.1);
                    app.load_available_tutors();
                    app.view = View::Tutors;
                }
                if ui.button("Got it").clicked() {
                    app.tutor_rate(0.15);
                    app.load_available_tutors();
                    app.view = View::Tutors;
                }
            }
        });
    });
    ui.separator();

    // ── chat history ─────────────────────────────────────────
    // Reserve room for the input bar so it never spills past the window.
    // Derived from the actual row count + font height so it stays correct
    // if INPUT_ROWS changes.
    let row_h = ui.text_style_height(&egui::TextStyle::Body);
    let input_height = row_h * INPUT_ROWS as f32 + 24.0;
    let chat_height = ui.available_height() - input_height - 12.0;
    ScrollArea::vertical()
        .max_height(chat_height)
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            let messages = app.tutor_messages.clone();
            for msg in &messages {
                if msg.role == "assistant" {
                    ui.label(
                        RichText::new("Tutor:")
                            .size(17.0)
                            .color(Color32::from_rgb(120, 200, 120))
                            .strong(),
                    );
                    egui_commonmark::CommonMarkViewer::new().show(
                        ui,
                        &mut app.markdown_cache,
                        &msg.content,
                    );
                    ui.add_space(4.0);
                } else if msg.role == "system" {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("> ")
                                .color(Color32::from_rgb(200, 170, 60)),
                        );
                        ui.label(
                            RichText::new(&msg.content)
                                .color(Color32::from_rgb(200, 170, 60)),
                        );
                    });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("You: ")
                                .color(Color32::from_rgb(140, 180, 255))
                                .strong(),
                        );
                        ui.label(
                            RichText::new(&msg.content)
                                .color(Color32::from_gray(190)),
                        );
                    });
                }
                ui.add_space(10.0);
            }
            if app.tutor_state == TutorState::Loading {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Tutor: ")
                            .color(Color32::from_rgb(120, 200, 120))
                            .strong(),
                    );
                    ui.spinner();
                });
            }

            // ── diagram (inline, after messages) ─────────────
            if app.tutor_diagram_rx.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("Generating diagram...").color(Color32::from_gray(160)));
                });
            } else if app.tutor_diagram.is_some() {
                ui.add_space(8.0);
                let clear = ui.horizontal(|ui| {
                    ui.label(RichText::new("Diagram").strong());
                    ui.small_button("✕ Clear").clicked()
                }).inner;
                if clear {
                    app.tutor_diagram = None;
                } else if let Some(texture) = &app.tutor_diagram {
                    let size = texture.size_vec2();
                    let available = ui.available_width();
                    let scale = (available / size.x).min(1.0);
                    let display_size = egui::vec2(size.x * scale, size.y * scale);
                    ui.image((texture.id(), display_size));
                }
            }
        });

    // ── input bar ────────────────────────────────────────────
    ui.separator();
    ui.horizontal(|ui| {
        let send_enabled = app.tutor_state == TutorState::Idle;
        let input_width = ui.available_width() - 70.0;
        let response = ui.add_enabled(
            send_enabled,
            egui::TextEdit::multiline(&mut app.tutor_input)
                .desired_width(input_width)
                .desired_rows(INPUT_ROWS)
                .hint_text("Your answer..."),
        );
        let enter_pressed = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter));
        if ui
            .add_enabled(send_enabled, egui::Button::new("Send"))
            .clicked()
            || enter_pressed
        {
            app.tutor_send_message();
            response.request_focus();
        }
    });
}
