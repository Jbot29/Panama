use crate::app::MyApp;
use crate::app_structs::View;
use crate::tutor::TutorState;
use eframe::egui;
use egui::{Color32, RichText, ScrollArea};

pub fn ui_tutors(app: &mut MyApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading("Tutors");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh").clicked() {
                app.load_available_tutors();
            }
            if ui.button("+ Create Tutor").clicked() {
                app.create_tutor_subject.clear();
                app.create_tutor_context.clear();
                app.create_tutor_result = None;
                app.create_tutor_error = None;
                app.create_tutor_loading = false;
                app.view = View::CreateTutor;
            }
        });
    });
    ui.separator();

    if app.available_tutors.is_empty() {
        ui.label(
            RichText::new("No tutors found in tutors/ directory.")
                .color(Color32::from_gray(160)),
        );
    }

    ScrollArea::vertical().show(ui, |ui| {
        let tutors = app.available_tutors.clone();
        for meta in &tutors {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_min_width(ui.available_width() - 16.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&meta.friendly_name).size(18.0).strong());
                        ui.label(
                            RichText::new(format!(
                                "{} topics | avg mastery {:.0}%",
                                meta.node_count,
                                meta.avg_mastery * 100.0
                            ))
                            .color(Color32::from_gray(170)),
                        );
                        let progress = meta.avg_mastery as f32;
                        ui.add(egui::ProgressBar::new(progress).desired_width(220.0));
                    });
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            let slug = meta.slug.clone();

                            if ui.button(RichText::new("Start >").size(15.0)).clicked() {
                                if let Ok(config) = crate::helpers::load_tutor_config(&slug) {
                                    if let Some(conn) = &app.conn {
                                        let _ = crate::tutor::seed_nodes(conn, &slug, &config.nodes);
                                    }
                                    app.active_tutor_slug = Some(slug.clone());
                                    app.active_tutor_config = Some(config);
                                    app.tutor_current_node = None;
                                    app.tutor_messages.clear();
                                    app.tutor_state = TutorState::Idle;
                                    app.init_tutor_session();
                                    app.view = View::TutorSession;
                                }
                            }

                            if ui.button(RichText::new("View").size(15.0)).clicked() {
                                if let Ok(config) = crate::helpers::load_tutor_config(&slug) {
                                    if let Some(conn) = &app.conn {
                                        let _ = crate::tutor::seed_nodes(conn, &slug, &config.nodes);
                                    }
                                    app.active_tutor_slug = Some(slug);
                                    app.active_tutor_config = Some(config);
                                    app.load_tutor_detail();
                                    app.view = View::TutorDetail;
                                }
                            }
                        },
                    );
                });
            });
            ui.add_space(8.0);
        }
    });
}
