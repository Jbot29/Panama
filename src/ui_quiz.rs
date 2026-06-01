use crate::app::MyApp;
use chrono::TimeZone;
use eframe::egui;
use egui::{Color32, RichText, ScrollArea};

pub fn ui_quiz(app: &mut MyApp, ui: &mut egui::Ui) {
    let in_session = !app.quiz_questions.is_empty();
    let finished = in_session && app.quiz_idx >= app.quiz_questions.len();

    if app.quiz_active_tutor.is_none() {
        ui_tutor_list(app, ui);
    } else if finished {
        ui_results(app, ui);
    } else if in_session {
        ui_question(app, ui);
    } else {
        ui_tutor_quiz_page(app, ui);
    }
}

fn ui_tutor_list(app: &mut MyApp, ui: &mut egui::Ui) {
    ui.add_space(16.0);
    ui.heading(RichText::new("Quiz").size(22.0));
    ui.add_space(16.0);

    ScrollArea::vertical().show(ui, |ui| {
        for meta in app.available_tutors.clone() {
            egui::Frame::new()
                .fill(Color32::from_gray(28))
                .inner_margin(egui::Margin::same(12))
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&meta.friendly_name).size(15.0).strong());
                            if meta.quiz_node_count > 0 {
                                ui.label(
                                    RichText::new(format!("{} quiz topics", meta.quiz_node_count))
                                        .size(12.0)
                                        .color(Color32::from_gray(150)),
                                );
                            } else {
                                ui.label(
                                    RichText::new("No quizzes yet")
                                        .size(12.0)
                                        .color(Color32::from_gray(100)),
                                );
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if meta.quiz_node_count > 0
                                && ui.button(RichText::new("Open >").size(14.0)).clicked()
                            {
                                app.quiz_active_tutor = Some(meta.slug.clone());
                                app.quiz_load_history();
                            }
                        });
                    });
                });
            ui.add_space(8.0);
        }
    });
}

fn ui_results(app: &mut MyApp, ui: &mut egui::Ui) {
    let total = app.quiz_questions.len();
    let correct = app.quiz_correct_count;
    let pct = (correct as f32 / total as f32 * 100.0).round() as u32;
    let score_color = if pct >= 80 {
        Color32::from_rgb(100, 220, 100)
    } else if pct >= 60 {
        Color32::from_rgb(220, 200, 80)
    } else {
        Color32::from_rgb(220, 100, 100)
    };

    ui.add_space(16.0);
    ui.label(
        RichText::new(&app.quiz_node_name)
            .size(15.0)
            .color(Color32::from_rgb(140, 170, 220)),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!("{correct} / {total}  ({pct}%)"))
            .size(32.0)
            .color(score_color)
            .strong(),
    );
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        if ui.button(RichText::new("Next Quiz >").size(15.0)).clicked() {
            app.quiz_questions.clear();
            app.quiz_start();
        }
        ui.add_space(8.0);
        if ui.button(RichText::new("< Back").size(15.0)).clicked() {
            app.quiz_questions.clear();
        }
    });
}

fn ui_question(app: &mut MyApp, ui: &mut egui::Ui) {
    let q = app.quiz_questions[app.quiz_idx].clone();
    let total = app.quiz_questions.len();
    let idx = app.quiz_idx;
    let selected = app.quiz_selected;

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("Question {} / {}", idx + 1, total))
                .size(13.0)
                .color(Color32::from_gray(140)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(&q.topic)
                    .size(13.0)
                    .color(Color32::from_rgb(140, 170, 220)),
            );
        });
    });
    ui.add_space(4.0);

    let progress = idx as f32 / total as f32;
    let bar_width = ui.available_width().min(700.0);
    let bar_height = 6.0;
    let (bar_rect, _) =
        ui.allocate_exact_size(egui::vec2(bar_width, bar_height), egui::Sense::hover());
    ui.painter()
        .rect_filled(bar_rect, 3.0, Color32::from_gray(50));
    let filled = egui::Rect::from_min_size(
        bar_rect.min,
        egui::vec2(bar_width * progress, bar_height),
    );
    ui.painter()
        .rect_filled(filled, 3.0, Color32::from_rgb(80, 140, 220));

    ui.add_space(20.0);
    ui.add(
        egui::Label::new(RichText::new(&q.question).size(18.0).color(Color32::from_gray(230)))
            .wrap(),
    );
    ui.add_space(20.0);

    let labels = ["A", "B", "C", "D"];
    for (i, choice) in q.choices.iter().enumerate() {
        let btn_color = match selected {
            None => Color32::from_gray(55),
            Some(_) if i == q.correct => Color32::from_rgb(40, 120, 60),
            Some(sel) if i == sel => Color32::from_rgb(120, 40, 40),
            _ => Color32::from_gray(40),
        };
        let text_color = match selected {
            None => Color32::from_gray(220),
            Some(sel) if i == q.correct || i == sel => Color32::from_gray(240),
            _ => Color32::from_gray(130),
        };
        let btn = egui::Button::new(
            RichText::new(format!("{}  {}", labels[i], choice))
                .size(15.0)
                .color(text_color),
        )
        .fill(btn_color)
        .min_size(egui::vec2(ui.available_width().min(650.0), 36.0));

        if ui.add(btn).clicked() && selected.is_none() {
            app.quiz_select(i);
        }
        ui.add_space(4.0);
    }

    if selected.is_some() {
        ui.add_space(16.0);
        let next_label = if idx + 1 >= total { "Done" } else { "Next >" };
        if ui.button(RichText::new(next_label).size(15.0)).clicked() {
            app.quiz_next();
        }
    }
}

fn ui_tutor_quiz_page(app: &mut MyApp, ui: &mut egui::Ui) {
    let tutor_name = app
        .available_tutors
        .iter()
        .find(|m| Some(&m.slug) == app.quiz_active_tutor.as_ref())
        .map(|m| m.friendly_name.clone())
        .unwrap_or_default();

    ui.add_space(8.0);
    if ui.button("< Quizzes").clicked() {
        app.quiz_active_tutor = None;
        app.quiz_history.clear();
    }
    ui.add_space(12.0);
    ui.heading(RichText::new(&tutor_name).size(22.0));
    ui.add_space(16.0);
    if ui.button(RichText::new("Start Quiz >").size(15.0)).clicked() {
        app.quiz_start();
    }

    if !app.quiz_history.is_empty() {
        ui.add_space(24.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(
            RichText::new("Recent results")
                .size(13.0)
                .color(Color32::from_gray(150)),
        );
        ui.add_space(6.0);
        for session in app.quiz_history.clone() {
            let pct = (session.correct as f32 / session.total as f32 * 100.0).round() as u32;
            let dt = chrono::Utc
                .timestamp_opt(session.taken_at, 0)
                .single()
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "?".to_string());
            let color = if pct >= 80 {
                Color32::from_rgb(100, 220, 100)
            } else if pct >= 60 {
                Color32::from_rgb(220, 200, 80)
            } else {
                Color32::from_rgb(220, 100, 100)
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(&dt).size(13.0).color(Color32::from_gray(140)));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(&session.quiz_file)
                        .size(13.0)
                        .color(Color32::from_rgb(140, 170, 220)),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("{}/{} ({}%)", session.correct, session.total, pct))
                        .size(13.0)
                        .color(color),
                );
            });
        }
    }
}
