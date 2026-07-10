use crate::db::{add_rule, delete_all_rules, delete_rule, get_rules, update_rule, Rule};

#[tauri::command]
pub fn get_rules_cmd() -> Result<Vec<Rule>, String> {
    get_rules().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_rule_cmd(rule: Rule) -> Result<i64, String> {
    add_rule(&rule).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_rule_cmd(rule: Rule) -> Result<(), String> {
    update_rule(&rule).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_rule_cmd(id: i64) -> Result<(), String> {
    delete_rule(id).map_err(|e| e.to_string())
}

/// Export all rules to a JSON file at the given path.
#[tauri::command]
pub fn export_rules_cmd(path: String) -> Result<(), String> {
    let rules = get_rules().map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&rules).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Import rules from a JSON file at the given path.
/// If `replace` is true, all existing rules are removed before inserting.
/// Each imported rule is inserted with `id` set to None to avoid collisions.
#[tauri::command]
pub fn import_rules_cmd(path: String, replace: bool) -> Result<usize, String> {
    let data = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let rules: Vec<Rule> = serde_json::from_str(&data).map_err(|e| e.to_string())?;

    if replace {
        delete_all_rules().map_err(|e| e.to_string())?;
    }

    let mut count = 0;
    for mut rule in rules {
        rule.id = None;
        add_rule(&rule).map_err(|e| e.to_string())?;
        count += 1;
    }
    Ok(count)
}
