use crate::app::MyApp;
use crate::app_structs::View;
use crate::tutor::TutorState;
use eframe::egui;
use egui::{Color32, RichText, ScrollArea};

fn mastery_color(score: f64) -> Color32 {
    let t = score.clamp(0.0, 1.0) as f32;
    if t < 0.5 {
        let k = t / 0.5;
        Color32::from_rgb(
            200,
            (200.0 * k) as u8,
            0,
        )
    } else {
        let k = (t - 0.5) / 0.5;
        Color32::from_rgb(
            (200.0 * (1.0 - k)) as u8,
            200,
            0,
        )
    }
}

pub fn ui_tutor_detail(app: &mut MyApp, ui: &mut egui::Ui) {
    let tutor_name = app
        .active_tutor_config
        .as_ref()
        .map(|c| c.friendly_name.clone())
        .unwrap_or_default();

    // ── top bar ──────────────────────────────────────────────
    ui.horizontal(|ui| {
        if ui.button("< Tutors").clicked() {
            app.load_available_tutors();
            app.view = View::Tutors;
        }
        ui.label(RichText::new(&tutor_name).size(18.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(RichText::new("Start Session >").size(15.0)).clicked() {
                app.tutor_current_node = None;
                app.tutor_messages.clear();
                app.tutor_state = TutorState::Idle;
                app.init_tutor_session();
                app.view = View::TutorSession;
            }
            if ui.button(RichText::new("☕ Office Hours").size(15.0)).clicked() {
                app.tutor_state = TutorState::Idle;
                app.init_office_hours();
                app.view = View::TutorSession;
            }
            if ui.button("Refresh").clicked() {
                app.load_tutor_detail();
            }
        });
    });
    ui.separator();

    let node_count = app.tutor_detail_nodes.len();

    ui.columns(2, |cols| {
        // ── Left: node list ──────────────────────────────────
        let ui = &mut cols[0];
        ui.label(
            RichText::new(format!("{node_count} topics — sorted weakest first"))
                .color(Color32::from_gray(150)),
        );
        ui.add_space(4.0);

        // ── Add a new top-level topic ────────────────────────
        ui.add(
            egui::TextEdit::singleline(&mut app.tutor_detail_new_node_name)
                .desired_width(f32::INFINITY)
                .hint_text("new topic name"),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.tutor_detail_new_node_desc)
                .desired_width(f32::INFINITY)
                .hint_text("optional description"),
        );
        if ui.button("+ Add Topic").clicked() {
            let name = app.tutor_detail_new_node_name.trim().to_string();
            let desc = app.tutor_detail_new_node_desc.trim().to_string();
            if !name.is_empty() {
                if let Ok(id) = app.add_tutor_node(&name, &desc) {
                    app.tutor_detail_new_node_name.clear();
                    app.tutor_detail_new_node_desc.clear();
                    app.select_detail_node(id);
                }
            }
        }
        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        ScrollArea::vertical()
            .id_salt("detail_node_list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let nodes = app.tutor_detail_nodes.clone();
                for node in &nodes {
                    let is_selected = app.tutor_detail_selected == Some(node.id);
                    let color = mastery_color(node.mastery_score);

                    ui.horizontal(|ui| {
                        // Colored mastery dot
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(10.0, 10.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().circle_filled(rect.center(), 5.0, color);

                        // Selectable name with id
                        if ui
                            .selectable_label(
                                is_selected,
                                format!("[{}] {}", node.id, node.name),
                            )
                            .clicked()
                        {
                            let id = node.id;
                            app.select_detail_node(id);
                        }

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    RichText::new(format!("{:.0}%", node.mastery_score * 100.0))
                                        .color(Color32::from_gray(140)),
                                );
                            },
                        );
                    });
                }
            });

        // ── Right: node detail ───────────────────────────────
        let ui = &mut cols[1];

        let selected_id = app.tutor_detail_selected;

        let Some(node) = app
            .tutor_detail_nodes
            .iter()
            .find(|n| Some(n.id) == selected_id)
            .cloned()
        else {
            ui.label(
                RichText::new("Select a topic on the left to see details.")
                    .color(Color32::from_gray(140)),
            );
            return;
        };

        ScrollArea::vertical()
            .id_salt("detail_node_detail")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(RichText::new(&node.name).size(20.0).strong());
                ui.add_space(4.0);

                if !node.description.is_empty() {
                    ui.label(RichText::new(&node.description).color(Color32::from_gray(190)));
                    ui.add_space(8.0);
                }

                // Mastery bar
                let mastery_pct = node.mastery_score as f32;
                ui.horizontal(|ui| {
                    ui.label("Mastery:");
                    ui.add(
                        egui::ProgressBar::new(mastery_pct)
                            .desired_width(120.0)
                            .fill(mastery_color(node.mastery_score)),
                    );
                    ui.label(
                        RichText::new(format!("{:.0}%", node.mastery_score * 100.0))
                            .color(mastery_color(node.mastery_score)),
                    );
                });
                ui.label(
                    RichText::new(format!("Reviewed {} time(s)", node.times_reviewed))
                        .color(Color32::from_gray(150)),
                );

                ui.add_space(12.0);
                ui.separator();

                // ── Actions ──────────────────────────────────
                ui.label(RichText::new("Actions").strong());
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("Study This Now").clicked() {
                        let slug = app.active_tutor_slug.clone();
                        let config = app.active_tutor_config.clone();
                        if let (Some(slug), Some(config)) = (slug, config) {
                            app.active_tutor_slug = Some(slug);
                            app.active_tutor_config = Some(config);
                            app.tutor_pinned_node_id = Some(node.id);
                            app.tutor_current_node = None;
                            app.tutor_messages.clear();
                            app.tutor_state = TutorState::Idle;
                            app.init_tutor_session();
                            app.view = View::TutorSession;
                        }
                    }
                    if ui
                        .button(
                            RichText::new("Mark Struggling")
                                .color(Color32::from_rgb(220, 120, 80)),
                        )
                        .clicked()
                    {
                        let conn = app.conn.as_ref().expect("DB not connected");
                        let _ = crate::tutor::update_node_mastery(conn, node.id, -0.1);
                        let slug = app.active_tutor_slug.clone().unwrap_or_default();
                        app.tutor_detail_nodes =
                            crate::tutor::load_tutor_nodes(conn, &slug).unwrap_or_default();
                    }
                });

                // ── Quiz ─────────────────────────────────────
                ui.add_space(12.0);
                ui.separator();
                ui.label(RichText::new("Quiz").strong());
                ui.add_space(4.0);

                if let Some(ref qf) = node.quiz_file {
                    ui.label(
                        RichText::new(format!("Quiz file: {qf}"))
                            .color(Color32::from_gray(150))
                            .monospace(),
                    );
                    ui.add_space(4.0);
                }

                if app.tutor_detail_quiz_loading
                    && app.tutor_detail_quiz_node_id == Some(node.id)
                {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(
                            RichText::new("Generating quiz...").color(Color32::from_gray(160)),
                        );
                    });
                } else {
                    let label = if node.quiz_file.is_some() {
                        "Regenerate Quiz"
                    } else {
                        "Generate Quiz"
                    };
                    if ui
                        .add_enabled(
                            !app.tutor_detail_quiz_loading,
                            egui::Button::new(label),
                        )
                        .clicked()
                    {
                        let prompt = format!(
                            "Generate a quiz for the topic: \"{name}\".\n\
                             Topic description: {desc}\n\n\
                             Return a JSON object — no markdown, no code fences, just JSON — with exactly this structure:\n\
                             {{\"topic\": \"{name}\", \"questions\": [\
                             {{\"question\": \"...\", \"choices\": [\"A\", \"B\", \"C\", \"D\"], \"correct\": 0}}, ...]}}\n\
                             Generate exactly 6 multiple-choice questions. Each question must have exactly 4 choices. \
                             correct is the 0-based index of the right answer. \
                             Questions should range from basic recall to application.",
                            name = node.name,
                            desc = if node.description.is_empty() { "No description provided.".to_string() } else { node.description.clone() },
                        );
                        let system = "You are a quiz generator. Return ONLY a valid JSON object \
                            — no markdown, no code fences, no explanation."
                            .to_string();
                        let messages = vec![serde_json::json!({"role": "user", "content": prompt})];
                        let (tx, rx) = std::sync::mpsc::channel();
                        crate::tutor::ask_claude_raw(system, messages, 2048, tx);
                        app.tutor_detail_quiz_rx = Some(rx);
                        app.tutor_detail_quiz_loading = true;
                        app.tutor_detail_quiz_node_id = Some(node.id);
                    }
                }

                ui.add_space(12.0);
                ui.separator();

                // ── Prerequisites ─────────────────────────────
                ui.label(RichText::new("Prerequisites").strong());
                ui.add_space(4.0);

                let prereqs = app.tutor_detail_prereqs.clone();
                if prereqs.is_empty() {
                    ui.label(
                        RichText::new("None").color(Color32::from_gray(140)),
                    );
                } else {
                    for prereq in &prereqs {
                        let color = mastery_color(prereq.mastery_score);
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(8.0, 8.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().circle_filled(rect.center(), 4.0, color);
                            ui.label(&prereq.name);
                            ui.label(
                                RichText::new(format!("{:.0}%", prereq.mastery_score * 100.0))
                                    .color(Color32::from_gray(140)),
                            );
                            if ui
                                .small_button(RichText::new("×").color(Color32::from_gray(160)))
                                .clicked()
                            {
                                let conn = app.conn.as_ref().expect("DB not connected");
                                let _ = crate::tutor::remove_prereq(conn, node.id, prereq.id);
                                app.tutor_detail_prereqs = crate::tutor::get_node_prereqs(conn, node.id)
                                    .unwrap_or_default();
                            }
                        });
                    }
                }

                // Add prereq
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Add prereq:");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut app.tutor_detail_prereq_input)
                            .desired_width(140.0)
                            .hint_text("name or id"),
                    );
                    let enter = response.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Add").clicked() || enter {
                        let input = app.tutor_detail_prereq_input.trim().to_string();
                        if !input.is_empty() {
                            let slug = app
                                .active_tutor_slug
                                .clone()
                                .unwrap_or_default();
                            let conn = app.conn.as_ref().expect("DB not connected");
                            // Resolve by ID or name
                            let parent_id: Option<i64> = if let Ok(id) = input.parse::<i64>() {
                                conn.query_row(
                                    "SELECT id FROM nodes WHERE id = ?1 AND tutor_slug = ?2",
                                    rusqlite::params![id, slug],
                                    |row| row.get(0),
                                )
                                .ok()
                            } else {
                                conn.query_row(
                                    "SELECT id FROM nodes WHERE name = ?1 AND tutor_slug = ?2",
                                    rusqlite::params![input, slug],
                                    |row| row.get(0),
                                )
                                .ok()
                            };
                            if let Some(parent_id) = parent_id {
                                let _ = conn.execute(
                                    "INSERT OR IGNORE INTO edges \
                                     (tutor_slug, from_node, to_node, relationship) \
                                     VALUES (?1, ?2, ?3, 'prerequisite')",
                                    rusqlite::params![slug, node.id, parent_id],
                                );
                                app.tutor_detail_prereqs =
                                    crate::tutor::get_node_prereqs(conn, node.id)
                                        .unwrap_or_default();
                            }
                            app.tutor_detail_prereq_input.clear();
                        }
                    }
                });
            });
    });
}
