use crate::app::MyApp;
use crate::app_structs::View;
use crate::db::{DBCard, DBCardEdit, DBCardUpdateSRS, exec_sql};
use crate::draw::DrawStroke;
use crate::ui::{primary_color_picker, value_tinted};
use eframe::egui;
use egui::load::SizedTexture;
use egui::{Color32, Response, Sense, Vec2};
use rs_fsrs::Rating;

fn paint_stroke(painter: &egui::Painter, offset: egui::Vec2, stroke: &DrawStroke) {
    if stroke.points.len() < 2 {
        return;
    }
    let value_color = value_tinted(stroke.color, stroke.value);
    let egui_stroke = egui::Stroke::new(stroke.width, value_color);
    let pts: Vec<egui::Pos2> = stroke
        .points
        .iter()
        .map(|p| egui::pos2(p.x + offset.x, p.y + offset.y))
        .collect();
    painter.add(egui::Shape::line(pts, egui_stroke));
}

fn drawing_toolbar(app: &mut MyApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        primary_color_picker(ui, &mut app.current_color, app.brush_value);
        ui.add_space(8.0);
        ui.label("Shade");
        ui.add(egui::Slider::new(&mut app.brush_value, 0.0..=1.0).show_value(false));
        ui.add_space(8.0);
        ui.label("Size");
        ui.add(egui::Slider::new(&mut app.brush_width, 1.0..=20.0).show_value(false));
    });
}

pub fn drawing_surface_ui(
    ui: &mut egui::Ui,
    size: Vec2,
    strokes: &mut Vec<DrawStroke>,
    current_stroke: &mut Option<DrawStroke>,
    brush_value: f32,
    brush_width: f32,
    current_color: egui::Color32,
    background: Option<&egui::TextureHandle>,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::drag());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 6.0, Color32::from_gray(25));

    if let Some(tex) = background {
        painter.image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, Color32::from_gray(60)),
        egui::StrokeKind::Inside,
    );

    let offset = rect.min.to_vec2();
    for s in strokes.iter() {
        paint_stroke(&painter, offset, s);
    }
    if let Some(s) = current_stroke.as_ref() {
        paint_stroke(&painter, offset, s);
    }

    if response.drag_started() {
        let mut s = DrawStroke {
            points: Vec::new(),
            value: brush_value,
            width: brush_width,
            color: current_color,
        };
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            if rect.contains(pos) {
                let local = egui::pos2(pos.x - rect.min.x, pos.y - rect.min.y);
                s.points.push(local);
            }
        }
        *current_stroke = Some(s);
    }

    if response.dragged() {
        if let (Some(pos), Some(s)) = (
            ui.input(|i| i.pointer.interact_pos()),
            current_stroke.as_mut(),
        ) {
            if rect.contains(pos) {
                let local = egui::pos2(pos.x - rect.min.x, pos.y - rect.min.y);
                let push = s
                    .points
                    .last()
                    .map(|last| last.distance(local) > 0.5)
                    .unwrap_or(true);
                if push {
                    s.points.push(local);
                }
            }
        }
    }

    if ui.input(|i| i.pointer.primary_released()) {
        if let Some(s) = current_stroke.take() {
            if !s.points.is_empty() {
                strokes.push(s);
            }
        }
    }

    response
}

pub fn ui_review(app: &mut MyApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.heading(format!("Review: {}", app.due_count));
    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("Again (1)").clicked() {
            app.on_card_response(Rating::Again);
        }
        if ui.button("Hard (2)").clicked() {
            app.on_card_response(Rating::Hard);
        }
        if ui.button("Good (3)").clicked() {
            app.on_card_response(Rating::Good);
        }
        if ui.button("Easy (4)").clicked() {
            app.on_card_response(Rating::Easy);
        }
        if ui.button("Play (p)").clicked() {
            app.play_back_audio(ctx);
        }
        if ui.button("Edit (e)").clicked() {
            app.view = View::EditCard;
        }
        if ui.button("Hide (h)").clicked() {
            app.db_hide_card();
        }
    });

    ui.add_space(16.0);

    if let Some(tex) = &app.front_image {
        let sized = SizedTexture::from_handle(tex);
        let max_width = ui.available_width().min(400.0);
        let aspect = sized.size.y / sized.size.x;
        let size = egui::vec2(max_width, max_width * aspect);
        ui.add(egui::Image::from_texture(sized).fit_to_exact_size(size));
    }

    if let Some(card) = &app.current_card {
        if app.show_back {
            ui.heading(&card.new_back);
        } else {
            ui.heading(&card.new_front);
        }
    } else {
        ui.label("No card loaded.");
    }
}

pub fn ui_edit_card(app: &mut MyApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    let Some(card) = &app.current_card else { return };
    let edit_str = format!("Edit: {}", card.id);
    let init = app.edit_card.is_none();
    let (card_id, card_front, card_back) = (card.id, card.new_front.clone(), card.new_back.clone());

    ui.heading(edit_str);
    ui.separator();

    if init {
        app.edit_card = Some(DBCardEdit {
            id: card_id,
            front: card_front,
            back: card_back,
        });
        app.load_edit_background(ctx);
        app.load_strokes_from_sidecar();
    }

    if let Some(edit) = app.edit_card.as_mut() {
        ui.label("Front");
        ui.add(egui::TextEdit::multiline(&mut edit.front).desired_width(600.0));
        ui.label("Back");
        ui.add(egui::TextEdit::multiline(&mut edit.back).desired_width(600.0));
        ui.label("Scratch pad:");

        drawing_toolbar(app, ui);
    }

    let bg = app.edit_background_image.as_ref();
    drawing_surface_ui(
        ui,
        egui::vec2(500.0, 320.0),
        &mut app.strokes,
        &mut app.current_stroke,
        app.brush_value,
        app.brush_width,
        app.current_color,
        bg,
    );

    ui.horizontal(|ui| {
        if ui.button("Undo").clicked() {
            app.strokes.pop();
            app.save_strokes_to_sidecar();
        }
        if ui.button("Clear").clicked() {
            app.strokes.clear();
            app.current_stroke = None;
            app.save_strokes_to_sidecar();
        }
        if ui.button("Upload").clicked() {
            app.upload_for_edit_card(ctx);
        }
    });

    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            if let Some(card) = &app.current_card {
                let id = card.id;
                app.export_front_image(id.into());
            }
            app.strokes.clear();
            app.current_stroke = None;
            app.db_edit_card();
            app.edit_card = None;
            app.edit_background_image = None;
            app.view = View::Review;
        }
        if ui.button("Cancel").clicked() {
            app.edit_card = None;
            app.strokes.clear();
            app.current_stroke = None;
            app.edit_background_image = None;
            app.view = View::Review;
        }
    });
}

pub fn ui_cards_hub(app: &mut MyApp, ui: &mut egui::Ui) {
    ui.heading("Cards");
    ui.separator();
    ui.add_space(16.0);

    ui.label(format!("{} card(s) due", app.due_count));
    ui.add_space(12.0);

    if ui.add_sized([160.0, 40.0], egui::Button::new("Review")).clicked() {
        app.view = View::Review;
    }
    ui.add_space(8.0);
    if ui.add_sized([160.0, 40.0], egui::Button::new("New Card")).clicked() {
        app.strokes.clear();
        app.current_stroke = None;
        app.edit_background_image = None;
        app.new_card_upload_path = None;
        app.card_chat_messages.clear();
        app.card_chat_input.clear();
        app.card_chat_loading = false;
        app.card_chat_rx = None;
        app.view = View::NewCard;
    }
}

pub fn ui_new_card(app: &mut MyApp, ui: &mut egui::Ui, ctx: &egui::Context) {
    ui.columns(2, |cols| {
        // ── Left: card helper chat ────────────────────────────
        let ui = &mut cols[0];
        ui.heading("Card Helper");
        ui.separator();

        let input_height = 70.0;
        let chat_height = (ui.available_height() - input_height - 20.0).max(100.0);

        egui::ScrollArea::vertical()
            .id_salt("card_chat_scroll")
            .max_height(chat_height)
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if app.card_chat_messages.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "Paste a word, sentence, or passage and I'll help you make a good card.",
                        )
                        .color(egui::Color32::from_gray(140)),
                    );
                }
                let messages = app.card_chat_messages.clone();
                for msg in &messages {
                    match msg.role.as_str() {
                        "assistant" => {
                            ui.label(
                                egui::RichText::new("Assistant:")
                                    .color(egui::Color32::from_rgb(120, 200, 120))
                                    .strong(),
                            );
                            egui_commonmark::CommonMarkViewer::new().show(
                                ui,
                                &mut app.markdown_cache,
                                &msg.content,
                            );
                        }
                        "system" => {
                            ui.label(
                                egui::RichText::new(&msg.content)
                                    .color(egui::Color32::from_rgb(200, 170, 60)),
                            );
                        }
                        _ => {
                            ui.label(
                                egui::RichText::new(format!("You: {}", msg.content))
                                    .color(egui::Color32::from_gray(190)),
                            );
                        }
                    }
                    ui.add_space(6.0);
                }
                if app.card_chat_loading {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Assistant: ")
                                .color(egui::Color32::from_rgb(120, 200, 120))
                                .strong(),
                        );
                        ui.spinner();
                    });
                }
            });

        ui.separator();
        ui.horizontal(|ui| {
            let send_enabled = !app.card_chat_loading;
            let input_width = ui.available_width() - 60.0;
            let response = ui.add_enabled(
                send_enabled,
                egui::TextEdit::multiline(&mut app.card_chat_input)
                    .desired_width(input_width)
                    .desired_rows(3)
                    .hint_text("Word, sentence, or passage..."),
            );
            let enter = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter));
            if ui.add_enabled(send_enabled, egui::Button::new("Send")).clicked() || enter {
                app.card_chat_send();
                response.request_focus();
            }
        });

        // ── Right: card form ──────────────────────────────────
        let ui = &mut cols[1];
        ui.heading("New Card");
        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("new_card_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label("Front");
                ui.add(
                    egui::TextEdit::multiline(&mut app.new_front)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Heading),
                );
                ui.label("Back");
                ui.add(
                    egui::TextEdit::multiline(&mut app.new_back)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Heading),
                );

                ui.add_space(8.0);
                ui.label("Scratch pad:");
                drawing_toolbar(app, ui);
                let bg = app.edit_background_image.as_ref();
                drawing_surface_ui(
                    ui,
                    egui::vec2(460.0, 280.0),
                    &mut app.strokes,
                    &mut app.current_stroke,
                    app.brush_value,
                    app.brush_width,
                    app.current_color,
                    bg,
                );
                ui.horizontal(|ui| {
                    if ui.button("Undo").clicked() { app.strokes.pop(); }
                    if ui.button("Clear").clicked() {
                        app.strokes.clear();
                        app.current_stroke = None;
                    }
                    if ui.button("Upload").clicked() {
                        app.upload_for_new_card(ctx);
                    }
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        let front = app.new_front.clone();
                        let back = app.new_back.clone();
                        match app.db_insert_card(&front, &back) {
                            Ok(new_id) => {
                                app.apply_new_card_upload(new_id);
                                if !app.strokes.is_empty() {
                                    app.export_front_image(new_id);
                                }
                                app.strokes.clear();
                                app.current_stroke = None;
                                app.edit_background_image = None;
                                app.new_front.clear();
                                app.new_back.clear();
                                app.card_chat_messages.clear();
                                app.view = View::Cards;
                            }
                            Err(e) => eprintln!("insert failed: {e}"),
                        }
                        let _ = ctx;
                    }
                    if ui.button("Cancel").clicked() {
                        app.strokes.clear();
                        app.current_stroke = None;
                        app.view = View::Cards;
                    }
                });
            });
    });
}

impl MyApp {
    pub fn load_next_card(&mut self, _ctx: &egui::Context) {
        if !self.refresh_card {
            return;
        }

        let conn = self.conn.as_ref().unwrap();

        match DBCard::random_next(conn) {
            Ok(card) => {
                println!("{:?}", card.id);
                self.current_card = Some(card);
            }
            Err(e) => {
                eprintln!("Error in DBCard::read_by_random: {e:?}");
                self.current_card = None;
            }
        }

        if self.current_card.is_some() {
            self.refresh_card = false;
        } else {
            eprintln!("Error");
        }
        self.show_back = false;
        self.front_image = None;
    }

    pub fn play_back_audio(&mut self, _ctx: &egui::Context) {
        let card_audio_path = self.get_back_audio_path();
        if let Some(path) = &card_audio_path {
            println!("audio path: {}", path.display());
            if let Some(player) = self.audio.as_mut() {
                if let Err(e) = player.play_file(&path) {
                    eprintln!("audio play failed: {e}");
                }
            }
        } else {
            println!("no audio file found for this card");
        }
    }

    pub fn on_card_response(&mut self, rating: Rating) {
        if let Some(db_card) = &self.current_card {
            let updated_srs = MyApp::update_card_response(&db_card.srs, rating);
            let conn = self.conn.as_ref().expect("DB not connected");
            let update = DBCardUpdateSRS {
                id: db_card.id,
                srs: updated_srs,
            };
            let (sql, params) = update.sql();
            exec_sql(&conn, sql, params).expect("BAD");
        } else {
            println!("Error");
        }
        self.refresh_card = true;
    }

    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::Key;

        if self.view != View::Review {
            return;
        }
        ctx.input(|i| {
            if i.key_pressed(Key::Num1) {
                self.on_card_response(Rating::Again);
                self.refresh_card = true;
            }
            if i.key_pressed(Key::Num2) {
                self.on_card_response(Rating::Hard);
                self.refresh_card = true;
            }
            if i.key_pressed(Key::Num3) {
                self.on_card_response(Rating::Good);
                self.refresh_card = true;
            }
            if i.key_pressed(Key::Num4) {
                self.on_card_response(Rating::Easy);
                self.refresh_card = true;
            }
            if i.key_pressed(Key::H) {
                self.db_hide_card();
                self.refresh_card = true;
            }
            if i.key_pressed(Key::E) {
                self.view = View::EditCard;
            }
            if i.key_pressed(Key::Space) {
                self.show_back = !self.show_back;
            }
            if i.key_pressed(Key::P) {
                self.play_back_audio(ctx);
            }
        });
    }
}
