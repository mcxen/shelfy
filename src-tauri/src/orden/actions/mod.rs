pub mod archive;
pub mod copy;
pub mod delete;
pub mod echo;
pub mod hardlink;
pub mod move_;
pub mod rename;
pub mod shell;
pub mod symlink;
pub mod trash;
pub mod write;

pub use archive::{ArchiveFormat, CompressArchive, ExtractArchive};
pub use copy::{ContinueWith, Copy};
pub use delete::Delete;
pub use echo::Echo;
pub use hardlink::Hardlink;
pub use move_::Move;
pub use rename::Rename;
pub use shell::Shell;
pub use symlink::Symlink;
pub use trash::Trash;
pub use write::{Write, WriteMode};

use crate::orden::conflict::ConflictMode;
use crate::orden::Action;
use chrono::Utc;
use std::path::Path;

/// A single action definition as parsed from YAML (before instantiation).
#[derive(Debug, Clone)]
pub struct ActionDef {
    pub name: String,
    pub value: serde_yaml::Value,
}

pub fn action_def_from_yaml(v: &serde_yaml::Value) -> Result<ActionDef, String> {
    if let serde_yaml::Value::String(s) = v {
        return Ok(ActionDef {
            name: s.clone(),
            value: serde_yaml::Value::Null,
        });
    }
    let mapping = v
        .as_mapping()
        .ok_or("Action must be a single-key mapping")?;
    if mapping.len() != 1 {
        return Err("Action definition must have only one key".into());
    }
    let (k, val) = mapping.iter().next().unwrap();
    let name = k.as_str().ok_or("Action key must be a string")?.to_string();
    Ok(ActionDef {
        name,
        value: val.clone(),
    })
}

pub fn build_action(def: &ActionDef) -> Result<Box<dyn Action>, String> {
    match def.name.as_str() {
        "echo" => {
            let msg = as_string_optional(&def.value, "");
            Ok(Box::new(Echo::new(msg)))
        }
        "move" => Ok(Box::new(Move::new(
            as_string(&def.value, "move", "dest")?,
            parse_conflict(&def.value, "rename_new")?,
            parse_str(&def.value, "rename_template", "{name} {counter}{extension}"),
            parse_bool(&def.value, "autodetect_folder", true),
        ))),
        "copy" => Ok(Box::new(Copy::new(
            as_destinations(&def.value, "copy")?,
            parse_conflict(&def.value, "rename_new")?,
            parse_str(&def.value, "rename_template", "{name} {counter}{extension}"),
            parse_bool(&def.value, "autodetect_folder", true),
            match parse_str(&def.value, "continue_with", "copy").as_str() {
                "original" => ContinueWith::Original,
                _ => ContinueWith::Copy,
            },
        ))),
        "rename" => Ok(Box::new(Rename::new(
            as_string(&def.value, "rename", "new_name")?,
            parse_conflict(&def.value, "rename_new")?,
            parse_str(&def.value, "rename_template", "{name} {counter}{extension}"),
        ))),
        "delete" => Ok(Box::new(Delete)),
        "trash" => Ok(Box::new(Trash)),
        "write" => Ok(Box::new(Write::new(
            as_string(&def.value, "write", "text")?,
            as_string(&def.value, "write", "outfile")?,
            match parse_str(&def.value, "mode", "append").as_str() {
                "prepend" => WriteMode::Prepend,
                "overwrite" => WriteMode::Overwrite,
                _ => WriteMode::Append,
            },
            parse_str(&def.value, "encoding", "utf-8"),
            parse_bool(&def.value, "newline", true),
            parse_bool(&def.value, "clear_before_first_write", false),
        ))),
        "symlink" => Ok(Box::new(Symlink::new(
            as_string(&def.value, "symlink", "dest")?,
            parse_conflict(&def.value, "rename_new")?,
            parse_str(&def.value, "rename_template", "{name} {counter}{extension}"),
            parse_bool(&def.value, "autodetect_folder", true),
        ))),
        "hardlink" => Ok(Box::new(Hardlink::new(
            as_string(&def.value, "hardlink", "dest")?,
            parse_conflict(&def.value, "rename_new")?,
            parse_str(&def.value, "rename_template", "{name} {counter}{extension}"),
            parse_bool(&def.value, "autodetect_folder", true),
        ))),
        "shell" => Ok(Box::new(Shell::new(
            as_string(&def.value, "shell", "cmd")?,
            parse_bool(&def.value, "run_in_simulation", false),
            parse_bool(&def.value, "ignore_errors", false),
            parse_str(&def.value, "simulation_output", "** simulation **"),
            parse_i64(&def.value, "simulation_returncode", 0),
        ))),
        "extract" | "unarchive" | "decompress" => Ok(Box::new(ExtractArchive::new(
            as_string(&def.value, &def.name, "dest")?,
            ArchiveFormat::parse(&parse_str(&def.value, "format", "zip"))?,
            parse_string_list(&def.value, "passwords", "password"),
            parse_bool(&def.value, "delete_original", false),
            parse_conflict(&def.value, "rename_new")?,
            parse_str(&def.value, "rename_template", "{name} {counter}{extension}"),
            parse_bool(&def.value, "autodetect_folder", true),
        ))),
        "compress" | "archive" => Ok(Box::new(CompressArchive::new(
            as_string(&def.value, &def.name, "dest")?,
            ArchiveFormat::parse(&parse_str(&def.value, "format", "zip"))?,
            parse_optional_str(&def.value, "password"),
            parse_bool(&def.value, "delete_original", false),
            parse_conflict(&def.value, "rename_new")?,
            parse_str(&def.value, "rename_template", "{name} {counter}{extension}"),
            parse_bool(&def.value, "autodetect_folder", true),
        ))),
        other => Err(format!("Unknown action: \"{}\"", other)),
    }
}

pub(crate) fn log_history(action: &str, src: &Path, dst: &Path, rule_label: Option<String>) {
    let Some(file_name) = src.file_name().map(|n| n.to_string_lossy().to_string()) else {
        return;
    };
    let log = crate::db::ActionLog {
        id: None,
        timestamp: Utc::now(),
        source_path: src.to_string_lossy().to_string(),
        destination_path: Some(dst.to_string_lossy().to_string()),
        action: action.to_string(),
        file_name,
        file_type: "Orden".to_string(),
        engine: "orden".to_string(),
        rule_label,
        undone: false,
    };
    let _ = crate::db::log_action_if_initialized(&log);
}

// ---------- helpers ----------

/// If the action value is a plain string, return it for the required key `key`
/// (organize allows `- move: ~/path/` as shorthand for `dest`).
fn as_string(v: &serde_yaml::Value, action: &str, key: &str) -> Result<String, String> {
    match v {
        serde_yaml::Value::String(s) => Ok(s.clone()),
        serde_yaml::Value::Mapping(m) => m
            .get(&serde_yaml::Value::String(key.into()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| format!("Action \"{}\" requires \"{}\"", action, key)),
        serde_yaml::Value::Null => Err(format!("Action \"{}\" requires \"{}\"", action, key)),
        _ => Err(format!("Action \"{}\" expects a string or mapping", action)),
    }
}

fn as_destinations(v: &serde_yaml::Value, action: &str) -> Result<Vec<String>, String> {
    let value = match v {
        serde_yaml::Value::Mapping(m) => m
            .get(&serde_yaml::Value::String("dest".into()))
            .ok_or_else(|| format!("Action \"{}\" requires \"dest\"", action))?,
        other => other,
    };
    let destinations = match value {
        serde_yaml::Value::String(s) if !s.trim().is_empty() => vec![s.clone()],
        serde_yaml::Value::Sequence(seq) => seq
            .iter()
            .map(|item| {
                item.as_str()
                    .filter(|s| !s.trim().is_empty())
                    .map(ToString::to_string)
                    .ok_or_else(|| format!("Action \"{}\" destinations must be strings", action))
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => Vec::new(),
    };
    if destinations.is_empty() {
        Err(format!(
            "Action \"{}\" requires at least one destination",
            action
        ))
    } else {
        Ok(destinations)
    }
}

fn as_string_optional(v: &serde_yaml::Value, default: &str) -> String {
    match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Mapping(m) => m
            .get(&serde_yaml::Value::String("msg".into()))
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_string(),
        _ => default.to_string(),
    }
}

fn parse_str(v: &serde_yaml::Value, key: &str, default: &str) -> String {
    if let serde_yaml::Value::Mapping(m) = v {
        if let Some(s) = m
            .get(&serde_yaml::Value::String(key.into()))
            .and_then(|v| v.as_str())
        {
            return s.to_string();
        }
    }
    default.to_string()
}

fn parse_bool(v: &serde_yaml::Value, key: &str, default: bool) -> bool {
    if let serde_yaml::Value::Mapping(m) = v {
        if let Some(b) = m
            .get(&serde_yaml::Value::String(key.into()))
            .and_then(|v| v.as_bool())
        {
            return b;
        }
    }
    default
}

fn parse_i64(v: &serde_yaml::Value, key: &str, default: i64) -> i64 {
    if let serde_yaml::Value::Mapping(m) = v {
        if let Some(n) = m
            .get(&serde_yaml::Value::String(key.into()))
            .and_then(|v| v.as_i64())
        {
            return n;
        }
    }
    default
}

fn parse_optional_str(v: &serde_yaml::Value, key: &str) -> Option<String> {
    if let serde_yaml::Value::Mapping(m) = v {
        return m
            .get(&serde_yaml::Value::String(key.into()))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }
    None
}

fn parse_string_list(v: &serde_yaml::Value, list_key: &str, single_key: &str) -> Vec<String> {
    let serde_yaml::Value::Mapping(m) = v else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(s) = m
        .get(&serde_yaml::Value::String(single_key.into()))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        out.push(s.to_string());
    }
    if let Some(value) = m.get(&serde_yaml::Value::String(list_key.into())) {
        match value {
            serde_yaml::Value::String(s) if !s.is_empty() => out.push(s.clone()),
            serde_yaml::Value::Sequence(seq) => {
                out.extend(
                    seq.iter()
                        .filter_map(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string()),
                );
            }
            _ => {}
        }
    }
    out
}

fn parse_conflict(v: &serde_yaml::Value, default: &str) -> Result<ConflictMode, String> {
    let s = parse_str(v, "on_conflict", default);
    ConflictMode::parse(&s)
}
