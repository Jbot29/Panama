use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DrawStroke {
    pub points: Vec<egui::Pos2>, // canvas-local coords
    pub value: f32,              // 0.0 = black, 1.0 = white
    pub width: f32,              // in "pixels"
    pub color: egui::Color32,
}

