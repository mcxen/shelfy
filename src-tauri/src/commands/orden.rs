use crate::db::*;

/// Resolve the Shelfy data directory (where shelfy.db / orden configs live).
fn data_dir() -> Result<std::path::PathBuf, String> {
    directories::ProjectDirs::from("cc", "shelfy", "shelfy")
        .map(|p| p.data_dir().to_path_buf())
        .ok_or_else(|| "Unable to resolve data directory".to_string())
}

/// A captured log entry from an orden run (mirrors `orden::action::LogEntry`).
#[derive(serde::Serialize)]
pub struct OrdenLog {
    level: String,
    sender: String,
    rule_nr: i64,
    path: String,
    msg: String,
}

#[derive(serde::Serialize)]
pub struct OrdenRunResult {
    success: u64,
    errors: u64,
    simulate: bool,
    logs: Vec<OrdenLog>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct OrdenVisualConfig {
    rules: Vec<OrdenVisualRule>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct OrdenVisualRule {
    id: String,
    name: String,
    enabled: bool,
    targets: String,
    location: String,
    subfolders: bool,
    extensions: String,
    #[serde(rename = "filterMode", default = "default_filter_mode")]
    filter_mode: String,
    tags: String,
    action: String,
    destination: String,
    #[serde(rename = "archiveFormat", default = "default_archive_format")]
    archive_format: String,
    #[serde(rename = "archivePassword", default)]
    archive_password: String,
    #[serde(rename = "archivePasswords", default)]
    archive_passwords: String,
    #[serde(rename = "deleteOriginal", default)]
    delete_original: bool,
    #[serde(rename = "onConflict", default = "default_on_conflict")]
    on_conflict: String,
}

fn default_filter_mode() -> String {
    "all".to_string()
}

fn default_archive_format() -> String {
    "auto".to_string()
}

fn default_on_conflict() -> String {
    "rename_new".to_string()
}

// ---------------------------------------------------------------------------
// Orden (advanced YAML rules engine)
// ---------------------------------------------------------------------------

/// List available orden config names.
#[tauri::command]
pub fn orden_list_cmd() -> Result<Vec<String>, String> {
    let mut names = list_orden_config_names().map_err(|e| e.to_string())?;
    for name in crate::orden::list_config_names(&data_dir()?) {
        if !names.contains(&name) {
            if let Ok(yaml) = crate::orden::load_config_text(&data_dir()?, &name) {
                let _ = upsert_orden_config(&name, &yaml);
            }
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

/// Load a config's YAML text by name.
#[tauri::command]
pub fn orden_load_cmd(name: String) -> Result<String, String> {
    if let Some(record) = get_orden_config(&name).map_err(|e| e.to_string())? {
        return Ok(record.yaml);
    }
    let yaml = crate::orden::load_config_text(&data_dir()?, &name)?;
    let _ = upsert_orden_config(&name, &yaml);
    Ok(yaml)
}

/// Save a config's YAML text by name (creates the orden dir if needed).
#[tauri::command]
pub fn orden_save_cmd(name: String, yaml: String) -> Result<(), String> {
    crate::orden::save_config_text(&data_dir()?, &name, &yaml)?;
    let clean = name
        .trim()
        .trim_end_matches(".yaml")
        .trim_end_matches(".yml")
        .to_string();
    upsert_orden_config(&clean, &yaml).map_err(|e| e.to_string())
}

/// Delete a config by name.
#[tauri::command]
pub fn orden_delete_cmd(name: String) -> Result<(), String> {
    let _ = crate::orden::delete_config(&data_dir()?, &name);
    delete_orden_config(&name).map_err(|e| e.to_string())
}

/// Validate a config's YAML text without executing it.
#[tauri::command]
pub fn orden_check_cmd(yaml: String) -> Result<(), String> {
    crate::orden::Config::from_string(&yaml).map(|_| ())
}

#[tauri::command]
pub fn orden_visual_from_yaml_cmd(yaml: String) -> Result<OrdenVisualConfig, String> {
    let value: serde_yaml::Value = serde_yaml::from_str(&yaml).map_err(|e| e.to_string())?;
    let rules = value
        .as_mapping()
        .and_then(|m| yaml_get(m, "rules"))
        .and_then(|v| v.as_sequence())
        .ok_or_else(|| "YAML must contain a rules list".to_string())?;

    let visual_rules = rules
        .iter()
        .enumerate()
        .map(|(idx, rule)| parse_visual_rule(idx, rule))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(OrdenVisualConfig {
        rules: visual_rules,
    })
}

/// Simulate or run an orden config from YAML text.
/// `simulate`: true = dry run, false = apply actions.
#[tauri::command]
pub fn orden_run_cmd(
    yaml: String,
    simulate: bool,
    tags: Vec<String>,
    skip_tags: Vec<String>,
) -> Result<OrdenRunResult, String> {
    let opts = crate::orden::ExecuteOptions {
        simulate,
        tags: tags.into_iter().collect(),
        skip_tags: skip_tags.into_iter().collect(),
        working_dir: std::env::current_dir().unwrap_or_default(),
    };
    let r = crate::orden::run_yaml(&yaml, &opts)?;
    let config_name =
        find_orden_config_name_for_yaml(&yaml).unwrap_or_else(|| "<ad-hoc>".to_string());
    let _ = log_orden_run(
        &config_name,
        simulate,
        r.success as i64,
        r.errors as i64,
        "manual",
        &serde_json::to_string(&r.logs).unwrap_or_else(|_| "[]".to_string()),
    );
    Ok(OrdenRunResult {
        success: r.success,
        errors: r.errors,
        simulate: r.simulate,
        logs: r
            .logs
            .into_iter()
            .map(|l| OrdenLog {
                level: l.level,
                sender: l.sender,
                rule_nr: l.rule_nr,
                path: l.path,
                msg: l.msg,
            })
            .collect(),
    })
}

fn parse_visual_rule(idx: usize, value: &serde_yaml::Value) -> Result<OrdenVisualRule, String> {
    let mapping = value
        .as_mapping()
        .ok_or_else(|| format!("Rule {} must be a YAML mapping", idx + 1))?;
    let action = parse_first_action_name(yaml_get(mapping, "actions"));
    Ok(OrdenVisualRule {
        id: format!("rule-{}", idx + 1),
        name: yaml_get(mapping, "name")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled rule")
            .to_string(),
        enabled: yaml_get(mapping, "enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        targets: yaml_get(mapping, "targets")
            .and_then(|v| v.as_str())
            .unwrap_or("files")
            .to_string(),
        location: parse_locations(yaml_get(mapping, "locations")),
        subfolders: yaml_get(mapping, "subfolders")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        extensions: parse_extension_filters(yaml_get(mapping, "filters")),
        filter_mode: yaml_get(mapping, "filter_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("all")
            .to_string(),
        tags: parse_string_list(yaml_get(mapping, "tags")),
        destination: parse_action_destinations(yaml_get(mapping, "actions"), &action),
        archive_format: parse_action_field(yaml_get(mapping, "actions"), &action, "format", "auto"),
        archive_password: parse_action_field(yaml_get(mapping, "actions"), &action, "password", ""),
        archive_passwords: parse_action_list_field(
            yaml_get(mapping, "actions"),
            &action,
            "passwords",
        ),
        delete_original: parse_action_bool_field(
            yaml_get(mapping, "actions"),
            &action,
            "delete_original",
            false,
        ),
        on_conflict: parse_action_field(
            yaml_get(mapping, "actions"),
            &action,
            "on_conflict",
            "rename_new",
        ),
        action,
    })
}

fn yaml_get<'a>(mapping: &'a serde_yaml::Mapping, key: &str) -> Option<&'a serde_yaml::Value> {
    mapping.get(serde_yaml::Value::String(key.to_string()))
}

fn parse_locations(value: Option<&serde_yaml::Value>) -> String {
    let Some(value) = value else {
        return "~/Downloads".to_string();
    };
    let mut locations = Vec::new();
    if let Some(seq) = value.as_sequence() {
        for item in seq {
            if let Some(path) = item.as_str() {
                locations.push(path.to_string());
            } else if let Some(map) = item.as_mapping() {
                if let Some(path) = yaml_get(map, "path").and_then(|v| v.as_str()) {
                    locations.push(path.to_string());
                }
            }
        }
    } else if let Some(path) = value.as_str() {
        locations.push(path.to_string());
    }
    if locations.is_empty() {
        "~/Downloads".to_string()
    } else {
        locations.join("\n")
    }
}

fn parse_extension_filters(value: Option<&serde_yaml::Value>) -> String {
    let mut extensions = Vec::new();
    let Some(filters) = value.and_then(|v| v.as_sequence()) else {
        return String::new();
    };
    for filter in filters {
        let Some(map) = filter.as_mapping() else {
            continue;
        };
        let Some(raw) = yaml_get(map, "extension") else {
            continue;
        };
        if let Some(ext) = raw.as_str() {
            extensions.push(ext.trim_start_matches('.').to_string());
        } else if let Some(seq) = raw.as_sequence() {
            for item in seq {
                if let Some(ext) = item.as_str() {
                    extensions.push(ext.trim_start_matches('.').to_string());
                }
            }
        }
    }
    extensions.join(", ")
}

fn parse_string_list(value: Option<&serde_yaml::Value>) -> String {
    let Some(seq) = value.and_then(|v| v.as_sequence()) else {
        return String::new();
    };
    seq.iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_first_action_name(value: Option<&serde_yaml::Value>) -> String {
    let Some(actions) = value.and_then(|v| v.as_sequence()) else {
        return "copy".to_string();
    };
    for action in actions {
        if let Some(map) = action.as_mapping() {
            if let Some(key) = map.keys().find_map(|k| k.as_str()) {
                return key.to_string();
            }
        }
    }
    "copy".to_string()
}

fn parse_action_destinations(value: Option<&serde_yaml::Value>, action_name: &str) -> String {
    let Some(actions) = value.and_then(|v| v.as_sequence()) else {
        return "~/Documents/Shelfy Backups/".to_string();
    };
    let mut destinations = Vec::new();
    for action in actions {
        let Some(map) = action.as_mapping() else {
            continue;
        };
        let Some((key, value)) = map.iter().next() else {
            continue;
        };
        if key.as_str() != Some(action_name) {
            continue;
        }
        destinations.extend(action_destination_values(value));
    }
    if destinations.is_empty() {
        "~/Documents/Shelfy Backups/".to_string()
    } else {
        destinations.join("\n")
    }
}

fn parse_action_mapping<'a>(
    value: Option<&'a serde_yaml::Value>,
    action_name: &str,
) -> Option<&'a serde_yaml::Mapping> {
    let actions = value.and_then(|v| v.as_sequence())?;
    for action in actions {
        let map = action.as_mapping()?;
        let (key, value) = map.iter().next()?;
        if key.as_str() == Some(action_name) {
            return value.as_mapping();
        }
    }
    None
}

fn parse_action_field(
    value: Option<&serde_yaml::Value>,
    action_name: &str,
    field: &str,
    default: &str,
) -> String {
    parse_action_mapping(value, action_name)
        .and_then(|m| yaml_get(m, field))
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

fn parse_action_bool_field(
    value: Option<&serde_yaml::Value>,
    action_name: &str,
    field: &str,
    default: bool,
) -> bool {
    parse_action_mapping(value, action_name)
        .and_then(|m| yaml_get(m, field))
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

fn parse_action_list_field(
    value: Option<&serde_yaml::Value>,
    action_name: &str,
    field: &str,
) -> String {
    let Some(raw) = parse_action_mapping(value, action_name).and_then(|m| yaml_get(m, field))
    else {
        return String::new();
    };
    if let Some(s) = raw.as_str() {
        return s.to_string();
    }
    raw.as_sequence()
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn action_destination_values(value: &serde_yaml::Value) -> Vec<String> {
    if let Some(dest) = value.as_str() {
        return vec![dest.to_string()];
    }
    if let Some(nested) = value.as_mapping() {
        if let Some(dest) = yaml_get(nested, "dest").and_then(|v| v.as_str()) {
            return vec![dest.to_string()];
        }
        if let Some(dest) = yaml_get(nested, "path").and_then(|v| v.as_str()) {
            return vec![dest.to_string()];
        }
        if let Some(seq) = yaml_get(nested, "dest").and_then(|v| v.as_sequence()) {
            return seq
                .iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect();
        }
    }
    Vec::new()
}

fn find_orden_config_name_for_yaml(yaml: &str) -> Option<String> {
    let names = list_orden_config_names().ok()?;
    names.into_iter().find(|name| {
        get_orden_config(name)
            .ok()
            .flatten()
            .map(|record| record.yaml == yaml)
            .unwrap_or(false)
    })
}

#[tauri::command]
pub fn orden_history_cmd(name: String, limit: i64) -> Result<Vec<OrdenRunLog>, String> {
    get_orden_run_logs(&name, limit.clamp(1, 200)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orden_jobs_cmd() -> Result<Vec<OrdenJob>, String> {
    list_orden_jobs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orden_save_job_cmd(job: OrdenJob) -> Result<i64, String> {
    if job.name.trim().is_empty() {
        return Err("Job name is required".into());
    }
    if job.config_name.trim().is_empty() {
        return Err("Orden config is required".into());
    }
    if job.mode == "cron" {
        crate::scheduler::validate_cron_expression(job.cron_expr.as_deref().unwrap_or(""))?;
    }
    upsert_orden_job(&job).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orden_delete_job_cmd(id: i64) -> Result<(), String> {
    delete_orden_job(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orden_run_job_cmd(job: OrdenJob) -> Result<OrdenRunResult, String> {
    let yaml = get_orden_config(&job.config_name)
        .map_err(|e| e.to_string())?
        .map(|record| record.yaml)
        .ok_or_else(|| format!("Orden config '{}' not found", job.config_name))?;
    let opts = crate::orden::ExecuteOptions {
        simulate: job.simulate,
        tags: split_csv(&job.tags).into_iter().collect(),
        skip_tags: split_csv(&job.skip_tags).into_iter().collect(),
        working_dir: std::env::current_dir().unwrap_or_default(),
    };
    let r = crate::orden::run_yaml(&yaml, &opts)?;
    let _ = log_orden_run(
        &job.config_name,
        job.simulate,
        r.success as i64,
        r.errors as i64,
        "manual-job",
        &serde_json::to_string(&r.logs).unwrap_or_else(|_| "[]".to_string()),
    );
    if let Some(id) = job.id {
        let _ = mark_orden_job_run(id);
    }
    Ok(OrdenRunResult {
        success: r.success,
        errors: r.errors,
        simulate: r.simulate,
        logs: r
            .logs
            .into_iter()
            .map(|l| OrdenLog {
                level: l.level,
                sender: l.sender,
                rule_nr: l.rule_nr,
                path: l.path,
                msg: l.msg,
            })
            .collect(),
    })
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}
