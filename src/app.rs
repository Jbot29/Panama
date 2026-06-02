use crate::config;
use crate::db::{
    DBCard, DBCardEdit, DBCardInsert, DBCardUpdateHide, exec_sql, insert_card, open_db,
};
use crate::draw::DrawStroke;
use anyhow::Result;
use chrono::Utc;
use eframe::egui;
use egui::ColorImage;
use rs_fsrs::{Card, FSRS, Rating};
use std::path::{Path, PathBuf};

use crate::audio::AudioPlayer;
use crate::helpers::TutorConfig;
use crate::quiz::{QuizQuestion, QuizSession};
use egui_commonmark::CommonMarkCache;

//mod app_structs;
use crate::app_structs::View;

#[derive(Debug, Clone, Default)]
pub struct GeneratedNode {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct GeneratedTutor {
    pub friendly_name: String,
    pub slug: String,
    pub system_prompt: String,
    pub nodes: Vec<GeneratedNode>,
}


#[derive(Default)]
pub struct MyApp {
    pub view: View,
    pub new_front: String,
    pub new_back: String,
    pub conn: Option<rusqlite::Connection>,
    pub current_card: Option<DBCard>,
    pub edit_card: Option<DBCardEdit>,
    pub refresh_card: bool,
    pub show_back: bool,
    pub front_image: Option<egui::TextureHandle>,
    pub due_count: u32,
    //    pub strokes: Vec<Vec<egui::Pos2>>,
    //    pub current_stroke: Vec<egui::Pos2>,
    pub strokes: Vec<DrawStroke>,
    pub current_stroke: Option<DrawStroke>,
    pub edit_background_image: Option<egui::TextureHandle>,
    pub brush_value: f32, // 0..=1
    pub brush_width: f32,
    pub current_color: egui::Color32,
    pub audio: Option<AudioPlayer>,
    pub markdown_cache: CommonMarkCache,
    pub quiz_questions: Vec<QuizQuestion>,
    pub quiz_idx: usize,
    pub quiz_selected: Option<usize>,
    pub quiz_correct_count: usize,
    pub quiz_history: Vec<QuizSession>,
    pub quiz_node_id: Option<i64>,
    pub quiz_node_name: String,
    pub quiz_current_file: String,
    pub quiz_active_tutor: Option<String>,
    // Generic tutor session
    pub active_tutor_slug: Option<String>,
    pub active_tutor_config: Option<TutorConfig>,
    pub tutor_current_node: Option<crate::tutor::TutorNode>,
    pub tutor_state: crate::tutor::TutorState,
    pub tutor_messages: Vec<crate::tutor::TutorMessage>,
    pub tutor_input: String,
    pub tutor_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    pub tutor_diagram_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    pub tutor_diagram: Option<egui::TextureHandle>,
    pub available_tutors: Vec<crate::tutor::TutorMeta>,
    // Tutor detail view
    pub tutor_detail_nodes: Vec<crate::tutor::TutorNode>,
    pub tutor_detail_selected: Option<i64>,
    pub tutor_detail_prereqs: Vec<crate::tutor::TutorNode>,
    pub tutor_detail_prereq_input: String,
    pub tutor_detail_new_node_name: String,
    pub tutor_detail_new_node_desc: String,
    pub tutor_pinned_node_id: Option<i64>,
    // When Some, the active session is an "office hours" chat (no node, free-form
    // diagnostic conversation) and this holds its system prompt.
    pub tutor_office_hours_prompt: Option<String>,
    // Create tutor flow
    pub create_tutor_subject: String,
    pub create_tutor_context: String,
    pub create_tutor_loading: bool,
    pub create_tutor_result: Option<GeneratedTutor>,
    pub create_tutor_error: Option<String>,
    pub create_tutor_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    // Card helper chat (new card view, throwaway per session)
    pub card_chat_messages: Vec<crate::tutor::TutorMessage>,
    pub card_chat_input: String,
    pub card_chat_loading: bool,
    pub card_chat_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    // Quiz generation from tutor detail
    pub tutor_detail_quiz_loading: bool,
    pub tutor_detail_quiz_node_id: Option<i64>,
    pub tutor_detail_quiz_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    // Image upload (new card flow — holds source path until card ID is assigned)
    pub new_card_upload_path: Option<PathBuf>,
}

fn color32_to_rgba(c: egui::Color32) -> image::Rgba<u8> {
    let [r, g, b, a] = c.to_array(); // RGBA u8
    image::Rgba([r, g, b, a])
}


impl MyApp {
    pub fn new_with_db() -> Result<Self> {
        config::ensure_dirs()?;
        let conn = open_db(config::db_path())?;
        crate::nodes::ensure_nodes_tables(&conn)?;
        crate::quiz::ensure_quiz_sessions_table(&conn)?;

        let audio = AudioPlayer::new().expect("audio init failed (no output device?)");

        Ok(Self {
            view: View::Base,
            conn: Some(conn),
            new_front: "".to_string(),
            new_back: "".to_string(),
            current_card: None,
            edit_card: None,
            refresh_card: true,
            show_back: false,
            front_image: None,
            strokes: Vec::new(),
            current_stroke: None,
            edit_background_image: None,
            brush_value: 0.9,
            brush_width: 3.0,
            current_color: egui::Color32::WHITE,
            audio: Some(audio),
            due_count: 0,
            markdown_cache: CommonMarkCache::default(),
            quiz_questions: Vec::new(),
            quiz_idx: 0,
            quiz_selected: None,
            quiz_correct_count: 0,
            quiz_history: Vec::new(),
            quiz_node_id: None,
            quiz_node_name: String::new(),
            quiz_current_file: String::new(),
            quiz_active_tutor: None,
            available_tutors: Vec::new(),
            active_tutor_slug: None,
            active_tutor_config: None,
            tutor_current_node: None,
            tutor_state: crate::tutor::TutorState::Idle,
            tutor_messages: Vec::new(),
            tutor_input: String::new(),
            tutor_rx: None,
            tutor_diagram_rx: None,
            tutor_diagram: None,
            create_tutor_subject: String::new(),
            create_tutor_context: String::new(),
            create_tutor_loading: false,
            create_tutor_result: None,
            create_tutor_error: None,
            create_tutor_rx: None,
            tutor_detail_nodes: Vec::new(),
            tutor_detail_selected: None,
            tutor_detail_prereqs: Vec::new(),
            tutor_detail_prereq_input: String::new(),
            tutor_detail_new_node_name: String::new(),
            tutor_detail_new_node_desc: String::new(),
            tutor_pinned_node_id: None,
            tutor_office_hours_prompt: None,
            card_chat_messages: Vec::new(),
            card_chat_input: String::new(),
            card_chat_loading: false,
            card_chat_rx: None,
            tutor_detail_quiz_loading: false,
            tutor_detail_quiz_node_id: None,
            tutor_detail_quiz_rx: None,
            new_card_upload_path: None,
        })
    }

    pub fn db_insert_card(&self, front: &str, back: &str) -> Result<i64> {
        let conn = self.conn.as_ref().expect("DB not connected");
        let card = DBCardInsert {
            new_front: front.to_string(),
            new_back: back.to_string(),
            srs: Card::new(),
        };
        insert_card(conn, &card)
    }

    pub fn load_due_count(&mut self, _ctx: &egui::Context) {
        let conn = self.conn.as_ref().unwrap();

        match DBCard::due_count(conn) {
            Ok(count) => {
                self.due_count = count;
            }
            Err(e) => {
                eprintln!("failed to load due count: {e}");
                self.due_count = 0; // or leave unchanged
            }
        }
    }

    pub fn db_hide_card(&mut self) {
        println!("Hide card");

        if let Some(db_card) = &self.current_card {
            let conn = self.conn.as_ref().expect("DB not connected");
            let update = DBCardUpdateHide {
                id: db_card.id,
                hide: true,
            };
            let (sql, params) = update.sql();
            exec_sql(&conn, sql, params).expect("BAD");
        }
    }

    pub fn update_card_response(card: &Card, rating: Rating) -> Card {
        println!("TEST CARDS");
        let fsrs = FSRS::default();
        let now = Utc::now();

        // if you actually need `repeat` with the existing card:
        let _record_log = fsrs.repeat(card.clone(), now);

        let card0 = Card::new();
        let card_next = fsrs.next(card0.clone(), now, rating);
        card_next.card
    }

    pub fn db_edit_card(&mut self) {
        println!("EDIT CARD, save updates");
        if let Some(edit) = self.edit_card.as_ref() {
            let conn = self.conn.as_ref().expect("DB not connected");
            let (sql, params) = edit.sql();
            exec_sql(&conn, sql, params).expect("BAD");

            // Keep the in-memory card in sync so Review shows the edits
            // immediately instead of the stale pre-edit row.
            if let Some(card) = self.current_card.as_mut() {
                if card.id == edit.id {
                    card.new_front = edit.front.clone();
                    card.new_back = edit.back.clone();
                }
            }
            // Drop the cached front texture so the updated image reloads.
            self.front_image = None;
        }
    }

    fn image_png_path(card_id: u32) -> String {
        config::media_images()
            .join(format!("front_{}.png", card_id))
            .to_string_lossy()
            .into_owned()
    }

    fn image_json_path(card_id: u32) -> String {
        config::media_images()
            .join(format!("front_{}.json", card_id))
            .to_string_lossy()
            .into_owned()
    }

    /// Load existing strokes from the JSON sidecar (if any) into self.strokes.
    pub fn load_strokes_from_sidecar(&mut self) {
        let id = match &self.current_card {
            Some(c) => c.id,
            None => return,
        };
        let path = Self::image_json_path(id);
        if let Ok(data) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<Vec<DrawStroke>>(&data) {
                Ok(strokes) => self.strokes = strokes,
                Err(e) => eprintln!("Failed to parse stroke sidecar: {e}"),
            }
        }
    }

    /// Load the existing PNG (if any) as the edit-canvas background texture.
    pub fn load_edit_background(&mut self, ctx: &egui::Context) {
        let id = match &self.current_card {
            Some(c) => c.id,
            None => return,
        };
        let path_str = Self::image_png_path(id);
        let path = Path::new(&path_str);
        if !path.is_file() {
            return;
        }

        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("load_edit_background read error: {e}");
                return;
            }
        };
        let img = match image::load_from_memory(&bytes) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                eprintln!("load_edit_background decode error: {e}");
                return;
            }
        };
        let (w, h) = (img.width() as usize, img.height() as usize);
        let color_image = ColorImage::from_rgba_unmultiplied([w, h], &img.into_raw());
        self.edit_background_image =
            Some(ctx.load_texture("edit_bg", color_image, egui::TextureOptions::LINEAR));
    }

    fn copy_image_as_png(src: &Path, dest: &Path) -> anyhow::Result<()> {
        let img = image::open(src)?.to_rgba8();
        img.save(dest)?;
        Ok(())
    }

    /// Open a file picker and load the chosen image as the new-card upload preview.
    pub fn upload_for_new_card(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg", "webp"])
            .pick_file()
        else {
            return;
        };
        self.edit_background_image = Self::load_png_as_texture(ctx, &path, "edit_bg");
        self.new_card_upload_path = Some(path);
    }

    /// Copy the pending upload to `front_{id}.png` after the card has been saved.
    pub fn apply_new_card_upload(&mut self, card_id: i64) {
        let Some(src) = self.new_card_upload_path.take() else { return };
        let dest_str = Self::image_png_path(card_id as u32);
        if let Err(e) = Self::copy_image_as_png(&src, Path::new(&dest_str)) {
            eprintln!("apply_new_card_upload failed: {e}");
        }
    }

    /// Open a file picker and immediately copy the chosen image to this card's PNG slot.
    pub fn upload_for_edit_card(&mut self, ctx: &egui::Context) {
        let id = match &self.current_card {
            Some(c) => c.id,
            None => return,
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg", "webp"])
            .pick_file()
        else {
            return;
        };
        let dest_str = Self::image_png_path(id as u32);
        if let Err(e) = Self::copy_image_as_png(&path, Path::new(&dest_str)) {
            eprintln!("upload_for_edit_card failed: {e}");
            return;
        }
        self.load_edit_background(ctx);
    }

    /// Composite strokes onto the existing PNG (or a blank canvas) and save.
    /// Also writes remaining strokes to the JSON sidecar (empty after a full save).
    pub fn export_front_image(&mut self, id: i64) {
        let png_path_str = Self::image_png_path(id as u32);
        let png_path = Path::new(&png_path_str);

        let canvas_size = egui::vec2(500.0, 320.0);
        let w = canvas_size.x as u32;
        let h = canvas_size.y as u32;

        // Start from existing PNG if present, otherwise transparent background.
        let mut base: image::RgbaImage = if png_path.is_file() {
            match std::fs::read(png_path).and_then(|b| {
                image::load_from_memory(&b)
                    .map(|i| i.to_rgba8())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            }) {
                Ok(img) => {
                    // Resize to canvas if needed (old images may differ).
                    if img.width() == w && img.height() == h {
                        img
                    } else {
                        image::imageops::resize(&img, w, h, image::imageops::FilterType::Lanczos3)
                    }
                }
                Err(e) => {
                    eprintln!("export_front_image: could not load existing PNG: {e}");
                    image::RgbaImage::from_pixel(w, h, image::Rgba([0, 0, 0, 0]))
                }
            }
        } else {
            image::RgbaImage::from_pixel(w, h, image::Rgba([0, 0, 0, 0]))
        };

        // Render new strokes on top.
        for stroke in &self.strokes {
            let rgba = color32_to_rgba(crate::ui::value_tinted(stroke.color, stroke.value));
            let radius = (stroke.width / 2.0).max(1.0).round() as i32;
            for p in &stroke.points {
                imageproc::drawing::draw_filled_circle_mut(
                    &mut base,
                    (p.x.round() as i32, p.y.round() as i32),
                    radius,
                    rgba,
                );
            }
            for pair in stroke.points.windows(2) {
                imageproc::drawing::draw_line_segment_mut(
                    &mut base,
                    (pair[0].x, pair[0].y),
                    (pair[1].x, pair[1].y),
                    rgba,
                );
            }
        }

        if let Err(e) = base.save(png_path) {
            eprintln!("export_front_image save failed: {e}");
        }

        // Clear the JSON sidecar — strokes are now baked into the PNG.
        let json_path = Self::image_json_path(id as u32);
        if let Err(e) = std::fs::write(&json_path, "[]") {
            eprintln!("Failed to clear stroke sidecar: {e}");
        }
    }

    /// Persist current in-memory strokes to the JSON sidecar without flattening.
    pub fn save_strokes_to_sidecar(&self) {
        let id = match &self.current_card {
            Some(c) => c.id,
            None => return,
        };
        let path = Self::image_json_path(id);
        match serde_json::to_string(&self.strokes) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    eprintln!("Failed to write stroke sidecar: {e}");
                }
            }
            Err(e) => eprintln!("Failed to serialise strokes: {e}"),
        }
    }

    pub fn get_back_audio_path(&self) -> Option<PathBuf> {
        let id = self.current_card.as_ref()?.id;
        let path = config::media_audio().join(format!("back_{}.mp3", id));
        path.is_file().then_some(path)
    }

    fn load_png_as_texture(
        ctx: &egui::Context,
        path: &Path,
        label: &str,
    ) -> Option<egui::TextureHandle> {
        let bytes = std::fs::read(path).ok()?;
        let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let (w, h) = (img.width() as usize, img.height() as usize);
        let color_image = ColorImage::from_rgba_unmultiplied([w, h], &img.into_raw());
        Some(ctx.load_texture(label, color_image, egui::TextureOptions::LINEAR))
    }

    pub fn load_front_image(&mut self, ctx: &egui::Context) {
        if self.front_image.is_some() {
            return;
        }
        let id = match &self.current_card {
            Some(c) => c.id,
            None => return,
        };
        let path_str = Self::image_png_path(id);
        let path = Path::new(&path_str);
        if path.is_file() {
            self.front_image = Self::load_png_as_texture(ctx, path, "front_image");
        }
    }

    fn parse_quoted_args(s: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut in_quote = false;
        let mut current = String::new();
        for c in s.chars() {
            match c {
                '"' if !in_quote => in_quote = true,
                '"' if in_quote => {
                    result.push(std::mem::take(&mut current));
                    in_quote = false;
                }
                c if in_quote => current.push(c),
                _ => {}
            }
        }
        result
    }

    // Returns Some(feedback) if input was a command, None if it should go to the chat.
    fn process_chat_command(&mut self, input: &str, tutor_slug: &str) -> Option<String> {
        if !input.starts_with('/') {
            return None;
        }
        let without_slash = input.trim_start_matches('/');
        let mut parts = without_slash.splitn(2, char::is_whitespace);
        let cmd = parts.next().unwrap_or("").to_lowercase();
        let rest = parts.next().unwrap_or("").trim();

        Some(match cmd.as_str() {
            "new-card" => {
                let args = Self::parse_quoted_args(rest);
                if args.len() >= 2 {
                    match self.db_insert_card(&args[0], &args[1]) {
                        Ok(_) => format!("Card created - front: \"{}\"", args[0]),
                        Err(e) => format!("Failed to create card: {e}"),
                    }
                } else {
                    "/new-card usage: /new-card \"front text\" \"back text\"".to_string()
                }
            }
            "flag" => {
                let args = Self::parse_quoted_args(rest);
                if let Some(name) = args.first() {
                    let desc = args.get(1).map(|s| s.as_str()).unwrap_or("");
                    let now = chrono::Utc::now().timestamp();
                    let conn = self.conn.as_ref().expect("DB not connected");
                    match conn.execute(
                        "INSERT OR IGNORE INTO nodes (tutor_slug, name, description, mastery_score, created_at) \
                         VALUES (?1, ?2, ?3, 0.3, ?4)",
                        rusqlite::params![tutor_slug, name, desc, now],
                    ) {
                        Ok(_) => format!(
                            "Flagged \"{}\" - added to review queue at low mastery",
                            name
                        ),
                        Err(e) => format!("Failed to flag: {e}"),
                    }
                } else {
                    "/flag usage: /flag \"topic name\" [\"optional description\"]".to_string()
                }
            }
            "flag-prereq" => {
                // Flag a new topic AND make it a prerequisite of the current session node.
                let Some(current) = self.tutor_current_node.clone() else {
                    return Some(
                        "/flag-prereq only works inside a node session — there is no current node to attach the prerequisite to."
                            .to_string(),
                    );
                };
                let args = Self::parse_quoted_args(rest);
                if let Some(name) = args.first() {
                    let desc = args.get(1).map(|s| s.as_str()).unwrap_or("");
                    let now = chrono::Utc::now().timestamp();
                    let conn = self.conn.as_ref().expect("DB not connected");
                    let result: rusqlite::Result<String> = (|| {
                        conn.execute(
                            "INSERT OR IGNORE INTO nodes (tutor_slug, name, description, mastery_score, created_at) \
                             VALUES (?1, ?2, ?3, 0.3, ?4)",
                            rusqlite::params![tutor_slug, name, desc, now],
                        )?;
                        // Resolve the node id whether it was just inserted or already existed.
                        let prereq_id: i64 = conn.query_row(
                            "SELECT id FROM nodes WHERE tutor_slug = ?1 AND name = ?2",
                            rusqlite::params![tutor_slug, name],
                            |row| row.get(0),
                        )?;
                        conn.execute(
                            "INSERT OR IGNORE INTO edges (tutor_slug, from_node, to_node, relationship) \
                             VALUES (?1, ?2, ?3, 'prerequisite')",
                            rusqlite::params![tutor_slug, current.id, prereq_id],
                        )?;
                        Ok(format!(
                            "Flagged \"{}\" and set it as a prerequisite of \"{}\"",
                            name, current.name
                        ))
                    })();
                    result.unwrap_or_else(|e| format!("Failed to flag prerequisite: {e}"))
                } else {
                    "/flag-prereq usage: /flag-prereq \"topic name\" [\"optional description\"]".to_string()
                }
            }
            "nodes" => {
                let conn = self.conn.as_ref().expect("DB not connected");
                let search = rest.trim().to_string();
                let slug = tutor_slug.to_string();
                let result: rusqlite::Result<Vec<(i64, String, f64)>> = (|| {
                    if search.is_empty() {
                        let mut stmt = conn.prepare(
                            "SELECT id, name, mastery_score FROM nodes WHERE tutor_slug = ?1 ORDER BY mastery_score ASC",
                        )?;
                        stmt.query_map(rusqlite::params![slug], |row| {
                            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?))
                        })?.collect()
                    } else {
                        let pattern = format!("%{}%", search);
                        let mut stmt = conn.prepare(
                            "SELECT id, name, mastery_score FROM nodes WHERE tutor_slug = ?1 AND name LIKE ?2 ORDER BY mastery_score ASC",
                        )?;
                        stmt.query_map(rusqlite::params![slug, pattern], |row| {
                            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?))
                        })?.collect()
                    }
                })();
                match result {
                    Ok(nodes) if nodes.is_empty() => "No nodes found".to_string(),
                    Ok(nodes) => {
                        let lines: Vec<String> = nodes
                            .iter()
                            .map(|(id, name, score)| format!("  [{}] {} ({:.0}%)", id, name, score * 100.0))
                            .collect();
                        format!("{} node(s):\n{}", nodes.len(), lines.join("\n"))
                    }
                    Err(e) => format!("Failed to list nodes: {e}"),
                }
            }
            "prereq" => {
                // Parse two tokens: each is either a bare integer (node id) or a quoted string (name)
                let tokens: Option<(String, String)> = {
                    let mut result = Vec::new();
                    let mut chars = rest.trim().chars().peekable();
                    while result.len() < 2 {
                        while chars.peek() == Some(&' ') { chars.next(); }
                        match chars.peek() {
                            None => break,
                            Some(&'"') => {
                                chars.next();
                                let mut tok = String::new();
                                for c in chars.by_ref() { if c == '"' { break; } tok.push(c); }
                                result.push(tok);
                            }
                            _ => {
                                let mut tok = String::new();
                                for c in chars.by_ref() { if c == ' ' { break; } tok.push(c); }
                                result.push(tok);
                            }
                        }
                    }
                    if result.len() >= 2 { Some((result.remove(0), result.remove(0))) } else { None }
                };
                if let Some((child_tok, parent_tok)) = tokens {
                    let slug = tutor_slug.to_string();
                    let conn = self.conn.as_ref().expect("DB not connected");
                    // Resolve a token to (id, name): by numeric id or by name
                    let resolve = |tok: &str| -> rusqlite::Result<Option<(i64, String)>> {
                        if let Ok(id) = tok.parse::<i64>() {
                            match conn.query_row(
                                "SELECT id, name FROM nodes WHERE id = ?1 AND tutor_slug = ?2",
                                rusqlite::params![id, slug],
                                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                            ) {
                                Ok(r) => Ok(Some(r)),
                                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                                Err(e) => Err(e),
                            }
                        } else {
                            match conn.query_row(
                                "SELECT id, name FROM nodes WHERE tutor_slug = ?1 AND name = ?2",
                                rusqlite::params![slug, tok],
                                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                            ) {
                                Ok(r) => Ok(Some(r)),
                                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                                Err(e) => Err(e),
                            }
                        }
                    };
                    let result: rusqlite::Result<String> = (|| {
                        let (child_id, child_name) = match resolve(&child_tok)? {
                            Some(r) => r,
                            None => return Ok(format!("Node not found: \"{}\"", child_tok)),
                        };
                        let (parent_id, parent_name) = match resolve(&parent_tok)? {
                            Some(r) => r,
                            None => return Ok(format!("Node not found: \"{}\"", parent_tok)),
                        };
                        conn.execute(
                            "INSERT OR IGNORE INTO edges (tutor_slug, from_node, to_node, relationship) \
                             VALUES (?1, ?2, ?3, 'prerequisite')",
                            rusqlite::params![slug, child_id, parent_id],
                        )?;
                        Ok(format!("\"{}\" now requires \"{}\" as a prerequisite", child_name, parent_name))
                    })();
                    result.unwrap_or_else(|e| format!("Failed to add prerequisite: {e}"))
                } else {
                    "/prereq usage: /prereq \"child\" \"prereq\"  or  /prereq 3 4".to_string()
                }
            }
            "diagram" => {
                // Strip surrounding quotes from the user-supplied focus if present
                let user_focus = if rest.is_empty() {
                    None
                } else {
                    Some(rest.trim_matches('"').to_string())
                };
                let (topic, description) = if let Some(node) = &self.tutor_current_node {
                    (node.name.clone(), node.description.clone())
                } else {
                    ("the current topic".to_string(), String::new())
                };
                let prompt = if let Some(focus) = user_focus {
                    // User provided a specific focus — use it, with the node as context
                    format!(
                        "Generate a simple SVG diagram illustrating: \"{focus}\" \
                         (in the context of {topic}). \
                         Return ONLY valid SVG code — no markdown, no explanation, no code fences. \
                         Start with <svg and end with </svg>. \
                         Use width=\"600\" height=\"400\", white background (#ffffff), \
                         simple shapes, and clear readable labels in a sans-serif font."
                    )
                } else if description.is_empty() {
                    format!(
                        "Generate a simple SVG diagram illustrating: \"{topic}\". \
                         Return ONLY valid SVG code — no markdown, no explanation, no code fences. \
                         Start with <svg and end with </svg>. \
                         Use width=\"600\" height=\"400\", white background (#ffffff), \
                         simple shapes, and clear readable labels in a sans-serif font."
                    )
                } else {
                    format!(
                        "Generate a simple SVG diagram illustrating: \"{topic}\" — {description}. \
                         Return ONLY valid SVG code — no markdown, no explanation, no code fences. \
                         Start with <svg and end with </svg>. \
                         Use width=\"600\" height=\"400\", white background (#ffffff), \
                         simple shapes, and clear readable labels in a sans-serif font."
                    )
                };
                let messages = vec![serde_json::json!({"role": "user", "content": prompt})];
                let system = "You are an SVG diagram generator. \
                    Return only valid SVG markup, nothing else — no markdown, no code fences. \
                    Use simple geometric shapes (rectangles, circles, lines, arrows) \
                    and text labels to illustrate concepts clearly. \
                    IMPORTANT: escape all XML special characters in text content: \
                    use &amp;amp; for &, &amp;lt; for <, &amp;gt; for >."
                    .to_string();
                let (tx, rx) = std::sync::mpsc::channel();
                eprintln!("[diagram] firing request for topic: {topic:?}");
                crate::tutor::ask_claude_raw(system, messages, 4096, tx);
                self.tutor_diagram_rx = Some(rx);
                self.tutor_diagram = None;
                "→ Generating diagram...".to_string()
            }
            "help" => "/new-card \"front\" \"back\" - create a flashcard\n\
		 /flag \"topic\" [\"description\"] - flag a weakness, adds it to the review queue\n\
		 /flag-prereq \"topic\" [\"description\"] - flag a topic and make it a prerequisite of the current node\n\
		 /nodes [search] - list nodes; optionally filter by name\n\
		 /prereq \"child\" \"parent\" - mark parent as a prerequisite of child\n\
		 /diagram [\"focus\"] - generate an SVG diagram; optionally specify a focus area\n\
		 /summary - summarize the session and suggest flashcards to memorize\n\
		 /help - show this message"
                .to_string(),
            _ => format!("Unknown command: /{cmd} - type /help for available commands"),
        })
    }

    // ── Generic tutor ────────────────────────────────────────────────────────

    pub fn load_available_tutors(&mut self) {
        let conn = self.conn.as_ref().expect("DB not connected");
        self.available_tutors = crate::tutor::load_available_tutors(conn);
    }

    pub fn load_tutor_detail(&mut self) {
        let Some(slug) = self.active_tutor_slug.as_ref() else { return };
        let conn = self.conn.as_ref().expect("DB not connected");
        self.tutor_detail_nodes = crate::tutor::load_tutor_nodes(conn, slug).unwrap_or_default();
        self.tutor_detail_selected = None;
        self.tutor_detail_prereqs.clear();
        self.tutor_detail_prereq_input.clear();
    }

    pub fn select_detail_node(&mut self, node_id: i64) {
        self.tutor_detail_selected = Some(node_id);
        self.tutor_detail_prereq_input.clear();
        let conn = self.conn.as_ref().expect("DB not connected");
        self.tutor_detail_prereqs = crate::tutor::get_node_prereqs(conn, node_id).unwrap_or_default();
    }

    /// Add a new top-level topic node for the active tutor and refresh the
    /// detail list. Unlike `/flag` (which seeds at 0.3 to surface quickly),
    /// this is a normal new topic at the table-default mastery. Returns the
    /// new node's id, or the existing id if a node with that name already exists.
    pub fn add_tutor_node(&mut self, name: &str, desc: &str) -> Result<i64, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Topic name is required".into());
        }
        let slug = self.active_tutor_slug.clone().unwrap_or_default();
        let now = chrono::Utc::now().timestamp();
        let conn = self.conn.as_ref().expect("DB not connected");
        conn.execute(
            "INSERT OR IGNORE INTO nodes (tutor_slug, name, description, created_at) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![slug, name, desc.trim(), now],
        )
        .map_err(|e| e.to_string())?;
        let id: i64 = conn
            .query_row(
                "SELECT id FROM nodes WHERE tutor_slug = ?1 AND name = ?2",
                rusqlite::params![slug, name],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        self.tutor_detail_nodes = crate::tutor::load_tutor_nodes(conn, &slug).unwrap_or_default();
        Ok(id)
    }

    /// Start an "office hours" session: a free-form, no-objective conversation
    /// where the tutor chats like a grad student, probes gently for gaps, and
    /// recommends flagging them. It is pull, not push — only ever launched by an
    /// explicit button, never auto-served by node selection. The system prompt is
    /// built from the tutor's subject plus a live snapshot of node mastery.
    pub fn init_office_hours(&mut self) {
        if self.tutor_state == crate::tutor::TutorState::Loading {
            return;
        }
        let Some(slug) = self.active_tutor_slug.clone() else { return };
        let Some(config) = self.active_tutor_config.clone() else { return };
        let conn = self.conn.as_ref().expect("DB not connected");
        let nodes = crate::tutor::load_tutor_nodes(conn, &slug).unwrap_or_default();

        // Snapshot of where the learner stands, struggling topics first.
        let mut state = String::new();
        if nodes.is_empty() {
            state.push_str("They haven't tracked any specific topics yet.");
        } else {
            let mut sorted = nodes.clone();
            sorted.sort_by(|a, b| {
                a.mastery_score
                    .partial_cmp(&b.mastery_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            state.push_str("Where they currently stand (mastery 0–100%):\n");
            for n in &sorted {
                let tag = if n.mastery_score < 0.6 { "STRUGGLING" } else { "solid" };
                state.push_str(&format!(
                    "- {tag}: {} ({:.0}%)\n",
                    n.name,
                    n.mastery_score * 100.0
                ));
            }
        }

        let system = format!(
            "You are an approachable tutor holding informal office hours for a learner studying \"{subject}\".\n\n\
             {state}\n\n\
             This is NOT a structured lesson or a quiz. Talk to them the way a friendly grad student would at \
             office hours: relaxed, conversational, curious. Open by asking how it's going, then follow their \
             interests and just discuss. As you talk, probe gently where they seem shaky — but keep it natural, \
             never an interrogation. When you spot a concrete gap worth tracking, suggest they flag it with a \
             copy-pasteable command on its own line: `/flag \"topic name\" \"short description\"`. Recommend \
             resources, next steps, or things to try. Keep replies fairly short and human. Use Markdown and \
             Unicode math; never LaTeX.",
            subject = config.friendly_name,
            state = state,
        );

        self.tutor_office_hours_prompt = Some(system.clone());
        self.tutor_current_node = None;
        self.tutor_pinned_node_id = None;
        self.tutor_messages.clear();

        let kickoff = vec![serde_json::json!({"role": "user", "content": "Start"})];
        let (tx, rx) = std::sync::mpsc::channel();
        crate::tutor::ask_claude_raw(system, kickoff, 1024, tx);
        self.tutor_rx = Some(rx);
        self.tutor_state = crate::tutor::TutorState::Loading;
    }

    pub fn init_tutor_session(&mut self) {
        if self.tutor_state == crate::tutor::TutorState::Loading {
            return;
        }
        // A normal node session is never office hours.
        self.tutor_office_hours_prompt = None;
        let Some(slug) = self.active_tutor_slug.clone() else { return };
        let Some(config) = self.active_tutor_config.clone() else { return };
        let conn = self.conn.as_ref().expect("DB not connected");
        let node_result = if let Some(pinned_id) = self.tutor_pinned_node_id.take() {
            match crate::tutor::get_node_by_id(conn, pinned_id) {
                Ok(Some(n)) => Ok(Some(n)),
                Ok(None) => crate::tutor::select_weakest_node(conn, &slug),
                Err(e) => Err(e),
            }
        } else {
            crate::tutor::select_weakest_node(conn, &slug)
        };
        match node_result {
            Ok(Some(node)) => {
                let conn = self.conn.as_ref().expect("DB not connected");
                let now = chrono::Utc::now().timestamp();
                let _ = conn.execute(
                    "UPDATE nodes SET last_reviewed = ?1 WHERE id = ?2",
                    rusqlite::params![now, node.id],
                );
                self.tutor_current_node = Some(node.clone());
                self.tutor_messages.clear();

                // If this node references a material, load its manifest live from
                // the tutor folder (like quizzes) and append it to the system
                // prompt. Content stays in the folder; the DB holds only progress.
                let mut system_prompt = config.system_prompt;
                if let Some(material) = config
                    .nodes
                    .iter()
                    .find(|n| n.name == node.name)
                    .and_then(|n| n.material.clone())
                {
                    let path = config::tutors_dir()
                        .join(&slug)
                        .join("materials")
                        .join(format!("{material}.toml"));
                    match std::fs::read_to_string(&path) {
                        Ok(text) => {
                            system_prompt.push_str(
                                "\n\n---\nMATERIAL FOR THIS NODE\nThe learner is looking at the image described by this manifest. Grade their coverage against it and steer them toward regions they have not yet mentioned. Never name an element before they do.\n\n",
                            );
                            system_prompt.push_str(&text);
                        }
                        Err(e) => eprintln!("failed to load material {}: {e}", path.display()),
                    }
                }

                let kickoff = vec![serde_json::json!({"role": "user", "content": "Start"})];
                let (tx, rx) = std::sync::mpsc::channel();
                crate::tutor::ask_claude(node, system_prompt, kickoff, tx);
                self.tutor_rx = Some(rx);
                self.tutor_state = crate::tutor::TutorState::Loading;
            }
            Ok(None) => {}
            Err(e) => eprintln!("init_tutor_session failed: {e}"),
        }
    }

    pub fn poll_tutor(&mut self, ctx: &egui::Context) {
        use std::sync::mpsc::TryRecvError;
        if let Some(rx) = &self.tutor_rx {
            match rx.try_recv() {
                Ok(Ok(text)) => {
                    self.tutor_messages.push(crate::tutor::TutorMessage {
                        role: "assistant".into(),
                        content: text,
                    });
                    self.tutor_state = crate::tutor::TutorState::Idle;
                    self.tutor_rx = None;
                }
                Ok(Err(e)) => {
                    self.tutor_messages.push(crate::tutor::TutorMessage {
                        role: "assistant".into(),
                        content: format!("Error: {e}"),
                    });
                    self.tutor_state = crate::tutor::TutorState::Idle;
                    self.tutor_rx = None;
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                Err(TryRecvError::Disconnected) => {
                    self.tutor_state = crate::tutor::TutorState::Idle;
                    self.tutor_rx = None;
                }
            }
        }
    }

    pub fn poll_diagram(&mut self, ctx: &egui::Context) {
        use std::sync::mpsc::TryRecvError;
        if let Some(rx) = &self.tutor_diagram_rx {
            match rx.try_recv() {
                Ok(Ok(text)) => {
                    eprintln!("[diagram] got response ({} chars)", text.len());
                    eprintln!("[diagram] first 200 chars: {:?}", &text[..text.len().min(200)]);
                    match crate::svg_diagram::extract_svg(&text) {
                        Some(svg) => {
                            eprintln!("[diagram] extracted SVG ({} chars)", svg.len());
                            match crate::svg_diagram::render_svg(&svg) {
                                Some(image) => {
                                    eprintln!("[diagram] rendered {}x{}", image.size[0], image.size[1]);
                                    self.tutor_diagram = Some(ctx.load_texture(
                                        "tutor_diagram",
                                        image,
                                        egui::TextureOptions::LINEAR,
                                    ));
                                }
                                None => {
                                    eprintln!("[diagram] render_svg returned None");
                                    self.tutor_messages.push(crate::tutor::TutorMessage {
                                        role: "system".into(),
                                        content: "→ Diagram error: failed to render SVG".into(),
                                    });
                                }
                            }
                        }
                        None => {
                            eprintln!("[diagram] no <svg> found in response");
                            self.tutor_messages.push(crate::tutor::TutorMessage {
                                role: "system".into(),
                                content: "→ Diagram error: response contained no SVG".into(),
                            });
                        }
                    }
                    self.tutor_diagram_rx = None;
                }
                Ok(Err(e)) => {
                    eprintln!("[diagram] API error: {e}");
                    self.tutor_messages.push(crate::tutor::TutorMessage {
                        role: "system".into(),
                        content: format!("→ Diagram error: {e}"),
                    });
                    self.tutor_diagram_rx = None;
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                Err(TryRecvError::Disconnected) => {
                    eprintln!("[diagram] channel disconnected");
                    self.tutor_diagram_rx = None;
                }
            }
        }
    }

    pub fn poll_create_tutor(&mut self, ctx: &egui::Context) {
        use std::sync::mpsc::TryRecvError;
        if let Some(rx) = &self.create_tutor_rx {
            match rx.try_recv() {
                Ok(Ok(text)) => {
                    match crate::ui_create_tutor::parse_tutor_response(&text) {
                        Some(tutor) => {
                            self.create_tutor_result = Some(tutor);
                            self.create_tutor_error = None;
                        }
                        None => {
                            self.create_tutor_error =
                                Some("Could not parse response — try again.".into());
                        }
                    }
                    self.create_tutor_loading = false;
                    self.create_tutor_rx = None;
                }
                Ok(Err(e)) => {
                    self.create_tutor_error = Some(format!("API error: {e}"));
                    self.create_tutor_loading = false;
                    self.create_tutor_rx = None;
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                Err(TryRecvError::Disconnected) => {
                    self.create_tutor_loading = false;
                    self.create_tutor_rx = None;
                }
            }
        }
    }

    pub fn poll_card_chat(&mut self, ctx: &egui::Context) {
        use std::sync::mpsc::TryRecvError;
        if let Some(rx) = &self.card_chat_rx {
            match rx.try_recv() {
                Ok(Ok(text)) => {
                    self.card_chat_messages.push(crate::tutor::TutorMessage {
                        role: "assistant".into(),
                        content: text,
                    });
                    self.card_chat_loading = false;
                    self.card_chat_rx = None;
                }
                Ok(Err(e)) => {
                    self.card_chat_messages.push(crate::tutor::TutorMessage {
                        role: "system".into(),
                        content: format!("Error: {e}"),
                    });
                    self.card_chat_loading = false;
                    self.card_chat_rx = None;
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                Err(TryRecvError::Disconnected) => {
                    self.card_chat_loading = false;
                    self.card_chat_rx = None;
                }
            }
        }
    }

    pub fn poll_detail_quiz(&mut self, ctx: &egui::Context) {
        use std::sync::mpsc::TryRecvError;
        if let Some(rx) = &self.tutor_detail_quiz_rx {
            match rx.try_recv() {
                Ok(Ok(text)) => {
                    if let Some((topic, questions)) = crate::quiz::parse_quiz_response(&text) {
                        if let Some(node_id) = self.tutor_detail_quiz_node_id {
                            let slug = self.active_tutor_slug.clone().unwrap_or_default();
                            let quiz_file = node_id.to_string();
                            let path = config::tutors_dir()
                                .join(&slug)
                                .join("quizzes")
                                .join(format!("{}.toml", node_id));
                            match crate::quiz::write_quiz_toml(&path, &topic, &questions) {
                                Ok(()) => {
                                    let conn = self.conn.as_ref().expect("DB not connected");
                                    let _ = conn.execute(
                                        "UPDATE nodes SET quiz_file = ?1 WHERE id = ?2",
                                        rusqlite::params![quiz_file, node_id],
                                    );
                                    self.tutor_detail_nodes =
                                        crate::tutor::load_tutor_nodes(conn, &slug).unwrap_or_default();
                                }
                                Err(e) => eprintln!("write_quiz_toml failed: {e}"),
                            }
                        }
                    }
                    self.tutor_detail_quiz_loading = false;
                    self.tutor_detail_quiz_rx = None;
                }
                Ok(Err(e)) => {
                    eprintln!("quiz generation error: {e}");
                    self.tutor_detail_quiz_loading = false;
                    self.tutor_detail_quiz_rx = None;
                }
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                Err(TryRecvError::Disconnected) => {
                    self.tutor_detail_quiz_loading = false;
                    self.tutor_detail_quiz_rx = None;
                }
            }
        }
    }

    pub fn card_chat_send(&mut self) {
        if self.card_chat_input.is_empty() || self.card_chat_loading {
            return;
        }
        let input = std::mem::take(&mut self.card_chat_input);
        self.card_chat_messages.push(crate::tutor::TutorMessage {
            role: "user".into(),
            content: input,
        });
        let api_messages: Vec<serde_json::Value> = self.card_chat_messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();
        let system = "You are a German language assistant helping an intermediate-to-advanced \
            learner make cloze deletion flashcards. When given a target word or phrase, produce a card in this exact format:\n\
            - FRONT: a natural, idiomatic German sentence that uses the target word, but with the target word itself replaced by a blank written as a run of underscores (__________). Pick a sentence where context makes the meaning clear.\n\
            - BACK: the full German sentence with the target word filled in (bold the target word), then on a new line the English translation of the complete sentence.\n\
            Also give a one-line definition of the target word and flag any tricky gender/plural/case or collocation worth knowing. \
            Then offer the finished card as /new-card \"front\" \"back\" so it can be added directly. \
            If the word has multiple meanings, clarify which applies and note the others briefly. \
            Keep responses tight and practical. Use Markdown for formatting. Respond in English unless the user writes in German."
            .to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        crate::tutor::ask_claude_raw(system, api_messages, 1024, tx);
        self.card_chat_rx = Some(rx);
        self.card_chat_loading = true;
    }

    pub fn tutor_rate(&mut self, delta: f64) {
        if let Some(node) = &self.tutor_current_node {
            let conn = self.conn.as_ref().expect("DB not connected");
            let _ = crate::tutor::update_node_mastery(conn, node.id, delta);
        }
    }

    pub fn tutor_send_message(&mut self) {
        if self.tutor_input.is_empty() || self.tutor_state == crate::tutor::TutorState::Loading {
            return;
        }
        let input = std::mem::take(&mut self.tutor_input);
        let Some(slug) = self.active_tutor_slug.clone() else { return };
        let Some(config) = self.active_tutor_config.clone() else { return };

        if input.trim() == "/summary" {
            self.tutor_messages.push(crate::tutor::TutorMessage {
                role: "user".into(),
                content: input,
            });
            // System prompt: office-hours override if active, else node-derived.
            let system = if let Some(oh) = self.tutor_office_hours_prompt.clone() {
                oh
            } else {
                let Some(node) = self.tutor_current_node.clone() else { return };
                config
                    .system_prompt
                    .replace("{node_name}", &node.name)
                    .replace("{node_description}", &node.description)
            };
            let mut api_messages = vec![serde_json::json!({"role": "user", "content": "Start"})];
            let msg_count = self.tutor_messages.len();
            for m in self.tutor_messages.iter().take(msg_count - 1) {
                if m.role != "system" {
                    api_messages.push(serde_json::json!({"role": m.role, "content": m.content}));
                }
            }
            api_messages.push(serde_json::json!({
                "role": "user",
                "content": "Please summarize our tutoring session with two sections:\n\n\
                    **Key Takeaways** - the main points and concepts we covered that are worth remembering.\n\n\
                    **Flashcard Suggestions** - recommend 4-6 cards that are good to memorize. \
                    Include a mix of two types:\n\
                    - **Conceptual cards**: the goal, motivation, or intuition behind an idea \
                    (e.g. \"What problem does X solve?\", \"Why do we care about Y?\", \"What's the key insight of Z?\"). \
                    These should outnumber the factual ones.\n\
                    - **Factual cards**: specific formulas, definitions, or facts worth recalling verbatim.\n\
                    Each card should be self-contained. Format each as the exact command so it can be copy-pasted: \
                    `/new-card \"front text\" \"back text\"`"
            }));
            let (tx, rx) = std::sync::mpsc::channel();
            crate::tutor::ask_claude_raw(system, api_messages, 2048, tx);
            self.tutor_rx = Some(rx);
            self.tutor_state = crate::tutor::TutorState::Loading;
            return;
        }

        if let Some(feedback) = self.process_chat_command(&input, &slug) {
            self.tutor_messages.push(crate::tutor::TutorMessage {
                role: "user".into(),
                content: input,
            });
            self.tutor_messages.push(crate::tutor::TutorMessage {
                role: "system".into(),
                content: feedback,
            });
            return;
        }

        self.tutor_messages.push(crate::tutor::TutorMessage {
            role: "user".into(),
            content: input,
        });

        let mut api_messages = vec![serde_json::json!({"role": "user", "content": "Start"})];
        for m in &self.tutor_messages {
            if m.role != "system" {
                api_messages.push(serde_json::json!({"role": m.role, "content": m.content}));
            }
        }

        let (tx, rx) = std::sync::mpsc::channel();
        if let Some(oh) = self.tutor_office_hours_prompt.clone() {
            crate::tutor::ask_claude_raw(oh, api_messages, 1024, tx);
        } else {
            let Some(node) = self.tutor_current_node.clone() else { return };
            crate::tutor::ask_claude(node, config.system_prompt, api_messages, tx);
        }
        self.tutor_rx = Some(rx);
        self.tutor_state = crate::tutor::TutorState::Loading;
    }

    pub fn quiz_start(&mut self) {
        let slug = match self.quiz_active_tutor.clone() {
            Some(s) => s,
            None => return,
        };
        let conn = self.conn.as_ref().expect("DB not connected");
        match crate::tutor::select_weakest_quiz_node(conn, &slug) {
            Ok(Some(node)) => {
                let quiz_file = node.quiz_file.clone().unwrap();
                let path = config::tutors_dir()
                    .join(&slug)
                    .join("quizzes")
                    .join(format!("{}.toml", quiz_file));
                match crate::quiz::load_quiz(&path) {
                    Ok(qs) => {
                        self.quiz_questions = qs;
                        self.quiz_idx = 0;
                        self.quiz_selected = None;
                        self.quiz_correct_count = 0;
                        self.quiz_node_id = Some(node.id);
                        self.quiz_node_name = node.name;
                        self.quiz_current_file = quiz_file;
                    }
                    Err(e) => eprintln!("Failed to load quiz file: {e}"),
                }
            }
            Ok(None) => eprintln!("No quiz nodes available"),
            Err(e) => eprintln!("Failed to select quiz node: {e}"),
        }
    }

    pub fn quiz_select(&mut self, choice: usize) {
        if self.quiz_selected.is_some() {
            return;
        }
        self.quiz_selected = Some(choice);
        if let Some(q) = self.quiz_questions.get(self.quiz_idx) {
            if choice == q.correct {
                self.quiz_correct_count += 1;
            }
        }
    }

    pub fn quiz_next(&mut self) {
        self.quiz_idx += 1;
        self.quiz_selected = None;
        if self.quiz_idx >= self.quiz_questions.len() && !self.quiz_questions.is_empty() {
            let conn = self.conn.as_ref().expect("DB not connected");
            let total = self.quiz_questions.len() as i64;
            let correct = self.quiz_correct_count as i64;
            if let Some(slug) = self.quiz_active_tutor.as_deref() {
                if let Err(e) = crate::quiz::save_quiz_session(
                    conn,
                    slug,
                    &self.quiz_current_file,
                    self.quiz_node_id,
                    correct,
                    total,
                ) {
                    eprintln!("Failed to save quiz session: {e}");
                }
                self.quiz_history =
                    crate::quiz::load_quiz_sessions(conn, slug, 8).unwrap_or_default();
            }
            // Update node mastery: linear delta from -0.15 (0%) to +0.15 (100%)
            if let Some(node_id) = self.quiz_node_id {
                let score = correct as f64 / total as f64;
                let delta = (score - 0.5) * 0.3;
                let _ = crate::tutor::update_node_mastery(conn, node_id, delta);
            }
        }
    }

    pub fn quiz_load_history(&mut self) {
        let slug = match self.quiz_active_tutor.as_deref() {
            Some(s) => s.to_string(),
            None => return,
        };
        let conn = self.conn.as_ref().expect("DB not connected");
        self.quiz_history =
            crate::quiz::load_quiz_sessions(conn, &slug, 8).unwrap_or_default();
    }

}
