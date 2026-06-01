use anyhow::{Context, Result};
use std::fs;

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TutorNodeDef {
    pub name: String,
    pub description: String,
    pub quiz_file: Option<String>,
    pub material: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TutorConfig {
    pub friendly_name: String,
    pub system_prompt: String,
    pub nodes: Vec<TutorNodeDef>,
}

pub fn load_tutor_config(slug: &str) -> Result<TutorConfig> {
    let path = crate::config::tutors_dir().join(slug).join("config.toml");
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}


