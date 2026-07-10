use rusqlite::{params, Result as SqliteResult};
use serde::{Deserialize, Serialize};

use crate::db::folders::get_watched_folders;
use crate::db::get_db;

// ---------------------------------------------------------------------------
// Rule
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: Option<i64>,
    pub name: String,
    pub priority: i32,
    pub enabled: bool,
    pub extensions: Vec<String>,
    pub pattern: Option<String>,
    pub destination: String,
    pub action: String, // "move", "rename", "delete", "ignore"
    pub folder_id: i64,
    #[serde(default)]
    pub folder_path: Option<String>,
}

pub fn migrate_rules_to_relative() -> SqliteResult<()> {
    let folders = get_watched_folders();
    // If watched folders can't be retrieved, skip migration
    let folders = match folders {
        Ok(f) => f,
        Err(_) => return Ok(()),
    };
    let db = get_db();
    let conn = db.lock().unwrap();
    for folder in folders {
        let folder_norm = folder.path.trim_end_matches('/').trim_end_matches('\\');
        if folder_norm.is_empty() {
            continue;
        }
        let mut stmt =
            conn.prepare("SELECT id, destination FROM rules WHERE destination LIKE ?1")?;
        let rows: Vec<(i64, String)> = stmt
            .query_map([format!("{}%", folder_norm)], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<SqliteResult<Vec<_>>>()?;
        for (id, dest) in rows {
            let relative = if dest.starts_with(&folder_norm) {
                dest[folder_norm.len()..]
                    .trim_start_matches('/')
                    .trim_start_matches('\\')
                    .to_string()
            } else {
                dest.clone()
            };
            if !relative.is_empty() && relative != dest {
                conn.execute(
                    "UPDATE rules SET destination = ?1 WHERE id = ?2",
                    params![relative, id],
                )?;
            }
        }
    }
    Ok(())
}

pub fn insert_default_rules(_folder_path: &str) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();

    // Only insert defaults if no rules exist yet
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM rules", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(());
    }

    let defaults = vec![
        (
            "Images",
            1,
            vec!["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg", "ico"],
            "Images",
        ),
        (
            "Documents",
            2,
            vec![
                "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "rtf", "odt",
            ],
            "Documents",
        ),
        (
            "Archives",
            3,
            vec!["zip", "rar", "7z", "tar", "gz", "bz2"],
            "Archives",
        ),
        (
            "Installers",
            4,
            vec!["exe", "msi", "msix", "appx"],
            "Installers",
        ),
        (
            "Music",
            5,
            vec!["mp3", "wav", "flac", "aac", "ogg", "wma", "m4a"],
            "Music",
        ),
        (
            "Videos",
            6,
            vec!["mp4", "avi", "mkv", "mov", "wmv", "flv", "webm"],
            "Videos",
        ),
        ("Others", 99, vec!["*"], "Others"),
    ];

    for (name, priority, exts, dest) in defaults {
        let extensions = exts.join(",");
        let destination = dest.to_string();
        conn.execute(
            "INSERT INTO rules (name, priority, extensions, destination, action, folder_id, folder_path) VALUES (?1, ?2, ?3, ?4, 'move', 0, NULL)",
            params![name, priority, extensions, destination],
        )?;
    }

    Ok(())
}

pub fn get_rules() -> SqliteResult<Vec<Rule>> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, priority, enabled, extensions, pattern, destination, action, folder_id, folder_path FROM rules ORDER BY priority"
    )?;

    let rules = stmt
        .query_map([], |row| {
            let exts_str: String = row.get(4)?;
            Ok(Rule {
                id: row.get(0)?,
                name: row.get(1)?,
                priority: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                extensions: exts_str
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .collect(),
                pattern: row.get(5)?,
                destination: row.get(6)?,
                action: row.get(7)?,
                folder_id: row.get(8)?,
                folder_path: row.get(9)?,
            })
        })?
        .collect::<SqliteResult<Vec<_>>>()?;

    Ok(rules)
}

pub fn add_rule(rule: &Rule) -> SqliteResult<i64> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let exts = rule.extensions.join(",");
    conn.execute(
        "INSERT INTO rules (name, priority, enabled, extensions, pattern, destination, action, folder_id, folder_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![rule.name, rule.priority, rule.enabled as i32, exts, rule.pattern, rule.destination, rule.action, rule.folder_id, rule.folder_path],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_rule(rule: &Rule) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let exts = rule.extensions.join(",");
    conn.execute(
        "UPDATE rules SET name=?1, priority=?2, enabled=?3, extensions=?4, pattern=?5, destination=?6, action=?7, folder_id=?8, folder_path=?9 WHERE id=?10",
        params![rule.name, rule.priority, rule.enabled as i32, exts, rule.pattern, rule.destination, rule.action, rule.folder_id, rule.folder_path, rule.id],
    )?;
    Ok(())
}

pub fn delete_rule(id: i64) -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM rules WHERE id=?1", params![id])?;
    Ok(())
}

pub fn delete_all_rules() -> SqliteResult<()> {
    let db = get_db();
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM rules", [])?;
    Ok(())
}

pub fn add_rule_record(rule: &Rule, preserve_id: bool) -> SqliteResult<i64> {
    let db = get_db();
    let conn = db.lock().unwrap();
    let exts = rule.extensions.join(",");
    if preserve_id {
        conn.execute(
            "INSERT OR REPLACE INTO rules (id, name, priority, enabled, extensions, pattern, destination, action, folder_id, folder_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![rule.id, rule.name, rule.priority, rule.enabled as i32, exts, rule.pattern, rule.destination, rule.action, rule.folder_id, rule.folder_path],
        )?;
    } else {
        conn.execute(
            "INSERT INTO rules (name, priority, enabled, extensions, pattern, destination, action, folder_id, folder_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![rule.name, rule.priority, rule.enabled as i32, exts, rule.pattern, rule.destination, rule.action, rule.folder_id, rule.folder_path],
        )?;
    }
    Ok(conn.last_insert_rowid())
}
