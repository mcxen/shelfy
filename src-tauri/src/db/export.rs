use rusqlite::Result as SqliteResult;
use serde::{Deserialize, Serialize};

use crate::db::{
    folders::{
        add_watched_folder_record, delete_all_watched_folders, get_watched_folders, WatchedFolder,
    },
    rules::{add_rule_record, delete_all_rules, get_rules, Rule},
    settings::{get_settings, update_settings, AppSettings},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub settings: AppSettings,
    pub folders: Vec<WatchedFolder>,
    pub rules: Vec<Rule>,
}

pub fn get_config_snapshot() -> SqliteResult<ConfigSnapshot> {
    Ok(ConfigSnapshot {
        settings: get_settings()?,
        folders: get_watched_folders()?,
        rules: get_rules()?,
    })
}

pub fn import_config_snapshot(snapshot: &ConfigSnapshot, replace: bool) -> SqliteResult<()> {
    update_settings(&snapshot.settings)?;

    if replace {
        delete_all_rules()?;
        delete_all_watched_folders()?;
        for folder in &snapshot.folders {
            add_watched_folder_record(folder, true)?;
        }
        for rule in &snapshot.rules {
            add_rule_record(rule, true)?;
        }
    } else {
        for folder in &snapshot.folders {
            add_watched_folder_record(folder, false)?;
        }
        for rule in &snapshot.rules {
            let mut imported = rule.clone();
            imported.id = None;
            imported.folder_id = 0;
            imported.folder_path = imported
                .folder_path
                .filter(|value| !value.trim().is_empty());
            add_rule_record(&imported, false)?;
        }
    }

    Ok(())
}
