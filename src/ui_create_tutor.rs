use crate::app::{GeneratedTutor, GeneratedNode, MyApp};
use crate::app_structs::View;
use eframe::egui;
use egui::{Color32, RichText, ScrollArea};

// ── Helpers ──────────────────────────────────────────────────────────────────

pub fn parse_tutor_response(text: &str) -> Option<GeneratedTutor> {
    // Strip markdown code fences if present
    let lower = text.trim();
    let json_str = if lower.starts_with("```") {
        let after = lower.trim_start_matches('`');
        let inner = after.find('\n').map(|i| &after[i + 1..]).unwrap_or(after);
        inner.trim_end_matches('`').trim_end_matches('\n')
    } else {
        lower
    };

    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let friendly_name = v["friendly_name"].as_str()?.to_string();
    let system_prompt = v["system_prompt"].as_str()?.to_string();
    let nodes = v["nodes"]
        .as_array()?
        .iter()
        .filter_map(|n| {
            Some(GeneratedNode {
                name: n["name"].as_str()?.to_string(),
                description: n["description"].as_str()?.to_string(),
            })
        })
        .collect();
    let slug = slugify(&friendly_name);
    Some(GeneratedTutor { friendly_name, slug, system_prompt, nodes })
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn toml_quoted(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn toml_multiline(s: &str) -> String {
    // TOML basic multi-line: """ ... """
    // Only escape sequences inside are \\ and \"\"\"
    let escaped = s.replace("\"\"\"", "\"\"\\\"");
    format!("\"\"\"\n{escaped}\n\"\"\"")
}

fn write_config_toml(tutor: &GeneratedTutor) -> anyhow::Result<()> {
    let dir = crate::config::tutors_dir().join(&tutor.slug);
    std::fs::create_dir_all(&dir)?;

    let mut content = String::new();
    content.push_str(&format!("friendly_name = {}\n\n", toml_quoted(&tutor.friendly_name)));
    content.push_str(&format!("system_prompt = {}\n", toml_multiline(&tutor.system_prompt)));
    for node in &tutor.nodes {
        content.push_str("\n[[nodes]]\n");
        content.push_str(&format!("name = {}\n", toml_quoted(&node.name)));
        content.push_str(&format!("description = {}\n", toml_quoted(&node.description)));
    }

    std::fs::write(dir.join("config.toml"), content)?;
    Ok(())
}

fn fire_generate(app: &mut MyApp) {
    let subject = app.create_tutor_subject.trim().to_string();
    if subject.is_empty() {
        return;
    }
    let context = app.create_tutor_context.trim().to_string();

    let user_msg = format!(
        "Create a tutor configuration for learning: \"{subject}\"\n\
         Learner context: {context}\n\n\
         Return a JSON object — no markdown, no code fences, just JSON — with exactly this structure:\n\
         {{\n\
           \"friendly_name\": \"Short display name (2-4 words)\",\n\
           \"system_prompt\": \"Socratic tutor prompt. Must contain the literal placeholders {{node_name}} and {{node_description}}. \
         Ask exactly one question at a time. Give brief feedback. Use Markdown. Use Unicode math symbols, never LaTeX $$. \
         Suggest flashcards as /new-card \\\"front\\\" \\\"back\\\". \
         Suggest /flag \\\"subtopic\\\" \\\"description\\\" when a student repeatedly struggles with one thing. \
         Keep responses concise (2-4 sentences).\",\n\
           \"nodes\": [\n\
             {{\"name\": \"Topic Name\", \"description\": \"What this topic covers in 1-2 sentences\"}},\n\
             ... 12-15 nodes ordered from foundational to advanced ...\n\
           ]\n\
         }}"
    );

    let system = "You are a learning curriculum designer. \
        Return ONLY a valid JSON object — no markdown, no code fences, no explanation. \
        The system_prompt field must contain the literal strings {node_name} and {node_description} as placeholders."
        .to_string();

    let messages = vec![serde_json::json!({"role": "user", "content": user_msg})];
    let (tx, rx) = std::sync::mpsc::channel();
    crate::tutor::ask_claude_raw(system, messages, 3000, tx);
    app.create_tutor_rx = Some(rx);
    app.create_tutor_loading = true;
    app.create_tutor_result = None;
    app.create_tutor_error = None;
}

// ── View ─────────────────────────────────────────────────────────────────────

pub fn ui_create_tutor(app: &mut MyApp, ui: &mut egui::Ui) {
    // ── top bar ──────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        if ui.button("< Tutors").clicked() {
            app.view = View::Tutors;
        }
        ui.label(RichText::new("Create Tutor").size(18.0).strong());
    });
    ui.separator();
    ui.add_space(8.0);

    ScrollArea::vertical()
        .id_salt("create_tutor_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ── Input form ───────────────────────────────────────────────────
            ui.label(RichText::new("Subject").strong());
            ui.add(
                egui::TextEdit::singleline(&mut app.create_tutor_subject)
                    .desired_width(400.0)
                    .hint_text("e.g. Photography, Ancient History, German C1 Grammar"),
            );
            ui.add_space(8.0);

            ui.label(RichText::new("Context").strong());
            ui.label(
                RichText::new("Your level, focus areas, what you want to get out of it")
                    .color(Color32::from_gray(150)),
            );
            ui.add(
                egui::TextEdit::multiline(&mut app.create_tutor_context)
                    .desired_width(400.0)
                    .desired_rows(3)
                    .hint_text("e.g. Complete beginner, focus on street photography and exposure triangle"),
            );
            ui.add_space(12.0);

            let can_generate = !app.create_tutor_subject.trim().is_empty()
                && !app.create_tutor_loading;

            if app.create_tutor_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new("Generating tutor...").color(Color32::from_gray(160)),
                    );
                });
            } else if ui
                .add_enabled(can_generate, egui::Button::new("Generate"))
                .clicked()
            {
                fire_generate(app);
            }

            // ── Error ────────────────────────────────────────────────────────
            if let Some(err) = &app.create_tutor_error.clone() {
                ui.add_space(8.0);
                ui.label(RichText::new(err).color(Color32::from_rgb(220, 80, 80)));
            }

            // ── Result preview ───────────────────────────────────────────────
            let result = app.create_tutor_result.clone();
            if let Some(tutor) = &result {
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(&tutor.friendly_name).size(18.0).strong());
                    ui.label(
                        RichText::new(format!("  tutors/{}/", tutor.slug))
                            .color(Color32::from_gray(140))
                            .monospace(),
                    );
                });
                ui.add_space(8.0);

                ui.label(RichText::new("System Prompt").strong());
                egui::Frame::new()
                    .fill(Color32::from_gray(22))
                    .inner_margin(egui::Margin::same(8i8))
                    .show(ui, |ui| {
                        let preview = if tutor.system_prompt.len() > 400 {
                            format!("{}…", &tutor.system_prompt[..400])
                        } else {
                            tutor.system_prompt.clone()
                        };
                        ui.label(
                            RichText::new(preview)
                                .color(Color32::from_gray(190))
                                .size(12.0),
                        );
                    });

                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("Topics ({})", tutor.nodes.len())).strong(),
                );
                for (i, node) in tutor.nodes.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{}.", i + 1))
                                .color(Color32::from_gray(120)),
                        );
                        ui.label(RichText::new(&node.name).strong());
                        ui.label(
                            RichText::new(format!("— {}", node.description))
                                .color(Color32::from_gray(160)),
                        );
                    });
                }

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(
                            RichText::new("Save Tutor")
                                .color(Color32::from_rgb(100, 200, 120)),
                        ))
                        .clicked()
                    {
                        match write_config_toml(tutor) {
                            Ok(()) => {
                                app.load_available_tutors();
                                app.view = View::Tutors;
                            }
                            Err(e) => {
                                app.create_tutor_error =
                                    Some(format!("Failed to save: {e}"));
                            }
                        }
                    }
                    if ui.button("Regenerate").clicked() {
                        fire_generate(app);
                    }
                });
            }
        });
}
