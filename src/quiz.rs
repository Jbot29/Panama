use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Clone, Debug)]
pub struct QuizQuestion {
    pub question: String,
    pub choices: Vec<String>,
    pub correct: usize,
    pub topic: String,
}

#[derive(Deserialize)]
struct QuizFile {
    topic: String,
    questions: Vec<QuestionEntry>,
}

#[derive(Deserialize)]
struct QuestionEntry {
    question: String,
    choices: Vec<String>,
    correct: usize,
}

pub fn load_quiz(path: &Path) -> Result<Vec<QuizQuestion>> {
    let content = std::fs::read_to_string(path)?;
    let qf: QuizFile = toml::from_str(&content)?;
    Ok(qf
        .questions
        .into_iter()
        .map(|q| QuizQuestion {
            question: q.question,
            choices: q.choices,
            correct: q.correct,
            topic: qf.topic.clone(),
        })
        .collect())
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct QuizSession {
    pub tutor: String,
    pub quiz_file: String,
    pub node_id: Option<i64>,
    pub taken_at: i64,
    pub correct: i64,
    pub total: i64,
    pub score: f64,
}

pub fn ensure_quiz_sessions_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS quiz_sessions (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            tutor     TEXT NOT NULL,
            quiz_file TEXT NOT NULL,
            node_id   INTEGER,
            taken_at  INTEGER NOT NULL,
            correct   INTEGER NOT NULL,
            total     INTEGER NOT NULL,
            score     REAL NOT NULL
        );",
    )?;
    Ok(())
}

pub fn save_quiz_session(
    conn: &Connection,
    tutor: &str,
    quiz_file: &str,
    node_id: Option<i64>,
    correct: i64,
    total: i64,
) -> Result<()> {
    let now = Utc::now().timestamp();
    let score = if total > 0 {
        correct as f64 / total as f64
    } else {
        0.0
    };
    conn.execute(
        "INSERT INTO quiz_sessions (tutor, quiz_file, node_id, taken_at, correct, total, score)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![tutor, quiz_file, node_id, now, correct, total, score],
    )?;
    Ok(())
}

pub fn load_quiz_sessions(conn: &Connection, tutor: &str, limit: i64) -> Result<Vec<QuizSession>> {
    let mut stmt = conn.prepare(
        "SELECT tutor, quiz_file, node_id, taken_at, correct, total, score
         FROM quiz_sessions
         WHERE tutor = ?1
         ORDER BY taken_at DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![tutor, limit], |row| {
        Ok(QuizSession {
            tutor: row.get(0)?,
            quiz_file: row.get(1)?,
            node_id: row.get(2)?,
            taken_at: row.get(3)?,
            correct: row.get(4)?,
            total: row.get(5)?,
            score: row.get(6)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub struct GeneratedQuestion {
    pub question: String,
    pub choices: Vec<String>,
    pub correct: usize,
}

pub fn parse_quiz_response(text: &str) -> Option<(String, Vec<GeneratedQuestion>)> {
    let lower = text.trim();
    let json_str = if lower.starts_with("```") {
        let after = lower.trim_start_matches('`');
        let inner = after.find('\n').map(|i| &after[i + 1..]).unwrap_or(after);
        inner.trim_end_matches('`').trim_end_matches('\n')
    } else {
        lower
    };
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let topic = v["topic"].as_str()?.to_string();
    let questions = v["questions"].as_array()?
        .iter()
        .filter_map(|q| {
            let question = q["question"].as_str()?.to_string();
            let choices = q["choices"].as_array()?
                .iter()
                .filter_map(|c| c.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>();
            let correct = q["correct"].as_u64()? as usize;
            Some(GeneratedQuestion { question, choices, correct })
        })
        .collect();
    Some((topic, questions))
}

pub fn write_quiz_toml(path: &std::path::Path, topic: &str, questions: &[GeneratedQuestion]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut content = String::new();
    let escaped_topic = topic.replace('"', "\\\"");
    content.push_str(&format!("topic = \"{escaped_topic}\"\n"));
    for q in questions {
        content.push_str("\n[[questions]]\n");
        let eq = q.question.replace('"', "\\\"");
        content.push_str(&format!("question = \"{eq}\"\n"));
        let choices_str = q.choices.iter()
            .map(|c| format!("\"{}\"", c.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(", ");
        content.push_str(&format!("choices = [{choices_str}]\n"));
        content.push_str(&format!("correct = {}\n", q.correct));
    }
    std::fs::write(path, content)?;
    Ok(())
}
