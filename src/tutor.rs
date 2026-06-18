use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use serde_json;
use std::sync::mpsc;

use crate::helpers::{TutorNodeDef, load_tutor_config};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TutorNode {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub mastery_score: f64,
    pub times_reviewed: i64,
    pub quiz_file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TutorMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum TutorState {
    #[default]
    Idle,
    Loading,
}

#[derive(Debug, Clone)]
pub struct TutorMeta {
    pub slug: String,
    pub friendly_name: String,
    pub node_count: usize,
    pub avg_mastery: f64,
    pub quiz_node_count: usize,
}

pub fn load_available_tutors(conn: &Connection) -> Vec<TutorMeta> {
    let tutors_dir = crate::config::tutors_dir();
    let mut metas = Vec::new();

    let Ok(entries) = std::fs::read_dir(&tutors_dir) else {
        return metas;
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        let Ok(config) = load_tutor_config(&slug) else {
            continue;
        };

        let (node_count, avg_mastery) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(AVG(mastery_score), 0.5) FROM nodes WHERE tutor_slug = ?1",
                rusqlite::params![slug],
                |row| Ok((row.get::<_, usize>(0)?, row.get::<_, f64>(1)?)),
            )
            .unwrap_or((0, 0.5));

        let quiz_node_count = conn
            .query_row(
                "SELECT COUNT(*) FROM nodes WHERE tutor_slug = ?1 AND quiz_file IS NOT NULL",
                rusqlite::params![slug],
                |row| row.get::<_, usize>(0),
            )
            .unwrap_or(0);

        metas.push(TutorMeta {
            slug,
            friendly_name: config.friendly_name,
            node_count,
            avg_mastery,
            quiz_node_count,
        });
    }

    metas.sort_by(|a, b| a.friendly_name.cmp(&b.friendly_name));
    metas
}

pub fn select_weakest_quiz_node(conn: &Connection, slug: &str) -> Result<Option<TutorNode>> {
    let mut stmt = conn.prepare(
        "SELECT n.id, n.name, n.description, n.mastery_score, n.times_reviewed, n.quiz_file
         FROM nodes n
         WHERE n.tutor_slug = ?1 AND n.quiz_file IS NOT NULL
         ORDER BY (
             SELECT s.score FROM quiz_sessions s
             WHERE s.quiz_file = n.quiz_file AND s.tutor = ?1
             ORDER BY s.taken_at DESC LIMIT 1
         ) ASC NULLS FIRST,
         n.mastery_score ASC
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map(rusqlite::params![slug], |row| {
        Ok(TutorNode {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            mastery_score: row.get(3)?,
            times_reviewed: row.get(4)?,
            quiz_file: row.get(5)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

pub fn select_weakest_node(conn: &Connection, slug: &str) -> Result<Option<TutorNode>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, mastery_score, times_reviewed, quiz_file
         FROM nodes
         WHERE tutor_slug = ?1
         ORDER BY mastery_score ASC, last_reviewed ASC NULLS FIRST
         LIMIT 1",
    )?;
    let node = stmt
        .query_map(rusqlite::params![slug], |row| {
            Ok(TutorNode {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                mastery_score: row.get(3)?,
                times_reviewed: row.get(4)?,
                quiz_file: row.get(5)?,
            })
        })?
        .next()
        .transpose()?;

    let Some(node) = node else { return Ok(None) };

    // If any prerequisite is below mastery threshold, study that first
    let mut prereq_stmt = conn.prepare(
        "SELECT n.id, n.name, n.description, n.mastery_score, n.times_reviewed, n.quiz_file
         FROM edges e
         JOIN nodes n ON n.id = e.to_node
         WHERE e.from_node = ?1 AND e.relationship = 'prerequisite'
           AND n.mastery_score < 0.6
         ORDER BY n.mastery_score ASC, n.last_reviewed ASC NULLS FIRST
         LIMIT 1",
    )?;
    let prereq = prereq_stmt
        .query_map([node.id], |row| {
            Ok(TutorNode {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                mastery_score: row.get(3)?,
                times_reviewed: row.get(4)?,
                quiz_file: row.get(5)?,
            })
        })?
        .next()
        .transpose()?;

    Ok(Some(prereq.unwrap_or(node)))
}

pub fn update_node_mastery(conn: &Connection, id: i64, delta: f64) -> Result<()> {
    let now = Utc::now().timestamp();
    conn.execute(
        "UPDATE nodes SET
         mastery_score  = MAX(0.0, MIN(1.0, mastery_score + ?1)),
         times_reviewed = times_reviewed + 1,
         last_reviewed  = ?2
         WHERE id = ?3",
        rusqlite::params![delta, now, id],
    )?;
    Ok(())
}

pub fn load_tutor_nodes(conn: &Connection, slug: &str) -> Result<Vec<TutorNode>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, mastery_score, times_reviewed, quiz_file
         FROM nodes WHERE tutor_slug = ?1
         ORDER BY mastery_score ASC, name ASC",
    )?;
    stmt.query_map(rusqlite::params![slug], |row| {
        Ok(TutorNode {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            mastery_score: row.get(3)?,
            times_reviewed: row.get(4)?,
            quiz_file: row.get(5)?,
        })
    })?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(Into::into)
}

pub fn get_node_by_id(conn: &Connection, id: i64) -> Result<Option<TutorNode>> {
    match conn.query_row(
        "SELECT id, name, description, mastery_score, times_reviewed, quiz_file
         FROM nodes WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(TutorNode {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                mastery_score: row.get(3)?,
                times_reviewed: row.get(4)?,
                quiz_file: row.get(5)?,
            })
        },
    ) {
        Ok(n) => Ok(Some(n)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn get_node_prereqs(conn: &Connection, node_id: i64) -> Result<Vec<TutorNode>> {
    let mut stmt = conn.prepare(
        "SELECT n.id, n.name, n.description, n.mastery_score, n.times_reviewed, n.quiz_file
         FROM edges e
         JOIN nodes n ON n.id = e.to_node
         WHERE e.from_node = ?1 AND e.relationship = 'prerequisite'
         ORDER BY n.mastery_score ASC",
    )?;
    stmt.query_map([node_id], |row| {
        Ok(TutorNode {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            mastery_score: row.get(3)?,
            times_reviewed: row.get(4)?,
            quiz_file: row.get(5)?,
        })
    })?
    .collect::<rusqlite::Result<Vec<_>>>()
    .map_err(Into::into)
}

pub fn remove_prereq(conn: &Connection, child_id: i64, parent_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM edges WHERE from_node = ?1 AND to_node = ?2 AND relationship = 'prerequisite'",
        rusqlite::params![child_id, parent_id],
    )?;
    Ok(())
}

// Inserts any nodes not already present; also applies quiz_file if set and missing.
pub fn seed_nodes(conn: &Connection, slug: &str, nodes: &[TutorNodeDef]) -> Result<()> {
    let now = Utc::now().timestamp();
    for node in nodes {
        conn.execute(
            "INSERT INTO nodes (tutor_slug, name, description, created_at)
             SELECT ?1, ?2, ?3, ?4
             WHERE NOT EXISTS (SELECT 1 FROM nodes WHERE tutor_slug = ?1 AND name = ?2)",
            rusqlite::params![slug, node.name, node.description, now],
        )?;
        if let Some(quiz_file) = &node.quiz_file {
            conn.execute(
                "UPDATE nodes SET quiz_file = ?1
                 WHERE tutor_slug = ?2 AND name = ?3 AND quiz_file IS NULL",
                rusqlite::params![quiz_file, slug, node.name],
            )?;
        }
    }
    Ok(())
}

pub fn ask_claude_raw(
    system_prompt: String,
    messages: Vec<serde_json::Value>,
    max_tokens: u32,
    tx: mpsc::Sender<Result<String, String>>,
) {
    std::thread::spawn(move || {
        let api_key = match std::env::var("ANTHROPIC_API_KEY") {
            Ok(k) => k,
            Err(_) => {
                let _ = tx.send(Err("ANTHROPIC_API_KEY not set".to_string()));
                return;
            }
        };

        let body = serde_json::json!({
            "model": "claude-opus-4-8",
            // "model": "claude-haiku-4-5-20251001",
            "max_tokens": max_tokens,
            "system": system_prompt,
            "messages": messages,
        });

        match ureq::post("https://api.anthropic.com/v1/messages")
            .set("x-api-key", &api_key)
            .set("anthropic-version", "2023-06-01")
            .send_json(&body)
        {
            Ok(resp) => match resp.into_json::<serde_json::Value>() {
                Ok(json) => {
                    let text = json["content"][0]["text"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    let _ = tx.send(Ok(text));
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                }
            },
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                let _ = tx.send(Err(format!("API error {code}: {body}")));
            }
            Err(e) => {
                let _ = tx.send(Err(e.to_string()));
            }
        }
    });
}

pub fn ask_claude(
    node: TutorNode,
    system_prompt: String,
    api_messages: Vec<serde_json::Value>,
    tx: mpsc::Sender<Result<String, String>>,
) {
    let system = system_prompt
        .replace("{node_name}", &node.name)
        .replace("{node_description}", &node.description);

    std::thread::spawn(move || {
        let api_key = match std::env::var("ANTHROPIC_API_KEY") {
            Ok(k) => k,
            Err(_) => {
                let _ = tx.send(Err("ANTHROPIC_API_KEY not set".to_string()));
                return;
            }
        };

        let body = serde_json::json!({
            "model": "claude-opus-4-8",
            // "model": "claude-haiku-4-5-20251001",
            "max_tokens": 2048,
            "system": system,
            "messages": api_messages,
        });

        match ureq::post("https://api.anthropic.com/v1/messages")
            .set("x-api-key", &api_key)
            .set("anthropic-version", "2023-06-01")
            .send_json(&body)
        {
            Ok(resp) => match resp.into_json::<serde_json::Value>() {
                Ok(json) => {
                    let text = json["content"][0]["text"]
                        .as_str()
                        .unwrap_or("(no response)")
                        .to_string();
                    let _ = tx.send(Ok(text));
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                }
            },
            Err(ureq::Error::Status(code, resp)) => {
                let body = resp.into_string().unwrap_or_default();
                eprintln!("Anthropic API {code}: {body}");
                let _ = tx.send(Err(format!("API error {code}: {body}")));
            }
            Err(e) => {
                let _ = tx.send(Err(e.to_string()));
            }
        }
    });
}
