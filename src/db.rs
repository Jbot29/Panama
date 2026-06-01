use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

use chrono::Utc;
use rs_fsrs::Card;

#[derive(Default)]
pub struct DBCard {
    pub id: u32,
    pub new_front: String,
    pub new_back: String,
    pub srs: Card,
}

impl DBCard {
    pub const SELECT_DUE_RANDOM: &str = r#"
SELECT id, front_text, back_text, scores
FROM cards
WHERE (due_date <= ?1 OR due_date IS NULL)
  AND hidden = 0
ORDER BY random()
LIMIT 1;
"#;

    pub const SELECT_DUE_COUNT: &str = r#"
    SELECT count(*)
    FROM cards
    WHERE due_date <= ?1  and hidden = 0
    ORDER BY random()
    LIMIT 1;
"#;


    pub fn random_next(conn: &rusqlite::Connection) -> anyhow::Result<DBCard> {
        // let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let now = Utc::now().timestamp(); // i64
        conn.query_row(Self::SELECT_DUE_RANDOM, [&now], |row| {
            // IMPORTANT: note the Option<String> here
            let scores_json: Option<String> = row.get("scores")?;

            let srs: Card = match scores_json {
                Some(ref json) if !json.trim().is_empty() => {
                    // Be defensive about bad JSON – don't crash the whole read
                    match serde_json::from_str::<Card>(json) {
                        Ok(card) => card,
                        Err(err) => {
                            eprintln!("Failed to parse scores_json for card: {err}");
                            Card::new()
                        }
                    }
                }
                _ => Card::new(),
            };

            Ok(DBCard {
                id: row.get("id")?,
                new_front: row.get("front_text")?,
                new_back: row.get("back_text")?,
                srs,
                // …any other fields…
            })
        })
        .map_err(|e| e.into())
    }

    pub fn due_count(conn: &rusqlite::Connection) -> Result<u32> {
        let now = Utc::now().timestamp(); // i64

        let count: u32 = conn.query_row(Self::SELECT_DUE_COUNT, [&now], |row| row.get(0))?;

        Ok(count)
    }
}

#[derive(Default, Clone)]
pub struct DBCardEdit {
    pub id: u32,
    pub front: String,
    pub back: String,
}
impl DBCardEdit {
    pub fn sql(&self) -> (&'static str, (&str, &str, u32)) {
        (
            "UPDATE cards SET front_text = ?1, back_text = ?2 WHERE id = ?3",
            (&self.front, &self.back, self.id),
        )
    }
}

#[derive(Default)]
pub struct DBCardInsert {
    pub new_front: String,
    pub new_back: String,
    pub srs: Card,
}

impl DBCardInsert {
    pub fn sql(&self) -> (&'static str, (&str, &str, String)) {
        let scores_json = serde_json::to_string(&self.srs).expect("serialize srs");
        (
            "INSERT INTO cards (front_text, back_text,scores) VALUES (?1, ?2, ?3)",
            (&self.new_front, &self.new_back, scores_json),
        )
    }
}

#[derive(Default)]
pub struct DBCardUpdateSRS {
    pub id: u32,
    pub srs: Card,
}

impl DBCardUpdateSRS {
    pub fn sql(&self) -> (&'static str, (String, i64, u32)) {
        let scores_json = serde_json::to_string(&self.srs).expect("serialize srs");

        let due_ts: i64 = self.srs.due.timestamp();
        println!("SRS: {:?} {:?} {:?}", self.id, self.srs.due, due_ts);
        (
            "UPDATE cards SET scores = ?1, due_date = ?2 WHERE id = ?3",
            (scores_json, due_ts, self.id),
        )
    }
}

#[derive(Default)]
pub struct DBCardUpdateHide {
    pub id: u32,
    pub hide: bool,
}

impl DBCardUpdateHide {
    pub fn sql(&self) -> (&'static str, (bool, u32)) {
        println!("Hide SRS: {:?} {:?}", self.id, self.hide);
        (
            "UPDATE cards SET hidden = ?1 WHERE id = ?2",
            (self.hide, self.id),
        )
    }
}

pub fn open_db(path: impl AsRef<Path>) -> Result<Connection> {
    Ok(Connection::open(path)?)
}

pub fn insert_card(conn: &rusqlite::Connection, card: &DBCardInsert) -> Result<i64> {
    let (sql, params) = card.sql();
    conn.execute(sql, params)?;
    Ok(conn.last_insert_rowid())
}

pub fn exec_sql<P>(conn: &Connection, sql: &str, params: P) -> Result<()>
where
    P: rusqlite::Params,
{
    conn.execute(sql, params)?;
    Ok(())
}
