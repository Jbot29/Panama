use anyhow::Result;
use rusqlite::Connection;

pub fn ensure_nodes_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS nodes (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            tutor_slug     TEXT NOT NULL,
            name           TEXT NOT NULL,
            description    TEXT NOT NULL DEFAULT '',
            mastery_score  REAL NOT NULL DEFAULT 0.5,
            times_reviewed INTEGER NOT NULL DEFAULT 0,
            last_reviewed  INTEGER,
            auto_generated INTEGER NOT NULL DEFAULT 0,
            quiz_file      TEXT,
            created_at     INTEGER NOT NULL,
            UNIQUE(tutor_slug, name)
        );
        CREATE TABLE IF NOT EXISTS edges (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            tutor_slug   TEXT NOT NULL,
            from_node    INTEGER NOT NULL REFERENCES nodes(id),
            to_node      INTEGER NOT NULL REFERENCES nodes(id),
            relationship TEXT NOT NULL DEFAULT 'subtopic'
        );
    "#,
    )?;
    Ok(())
}
