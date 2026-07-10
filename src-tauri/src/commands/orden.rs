use crate::db::*;

/// Resolve the Shelfy data directory (where shelfy.db / orden configs live).
fn data_dir() -> Result<std::path::PathBuf, String> {
    directories::ProjectDirs::from("cc", "shelfy", "shelfy")
        .map(|p| p.data_dir().to_path_buf())
        .ok_or_else(|| "Unable to resolve data directory".to_string())
}

fn orden_templates_dir(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("orden").join("templates")
}

fn valid_template_name(name: &str) -> Result<String, String> {
    // Strip a trailing .yaml/.yml extension (case-insensitive) so that
    // custom template ids returned to the frontend match the client-side
    // normalization (which also strips .ya?ml$/i). Otherwise something like
    // "foo.YAML" would be saved as "foo.YAML.yaml" and surface the mismatch.
    let trimmed = name.trim();
    let lower = trimmed.to_lowercase();
    let clean = if lower.ends_with(".yaml") {
        &trimmed[..trimmed.len() - ".yaml".len()]
    } else if lower.ends_with(".yml") {
        &trimmed[..trimmed.len() - ".yml".len()]
    } else {
        trimmed
    }
    .to_string();
    if clean.is_empty()
        || clean == "."
        || clean == ".."
        || clean.contains('/')
        || clean.contains('\\')
        || clean.chars().any(|ch| ch.is_control())
    {
        return Err("Template name must be a safe file name".to_string());
    }
    Ok(clean)
}

struct SystemOrdenTemplate {
    id: &'static str,
    title_key: &'static str,
    description_key: &'static str,
    category_key: &'static str,
    icon: &'static str,
    tone: &'static str,
    yaml: &'static str,
}

const SYSTEM_ORDEN_TEMPLATES: &[SystemOrdenTemplate] = &[
    SystemOrdenTemplate {
        id: "sort-images",
        title_key: "settings.orden.templates.system.images.title",
        description_key: "settings.orden.templates.system.images.description",
        category_key: "settings.orden.templates.categories.organize",
        icon: "image",
        tone: "organize",
        yaml: "rules:\n  - name: \"整理图片\"\n    locations:\n      - ~/Downloads\n    subfolders: true\n    filters:\n      - extension: [jpg, jpeg, png, gif, webp]\n    actions:\n      - move: ~/Pictures/Inbox/\n",
    },
    SystemOrdenTemplate {
        id: "sort-documents",
        title_key: "settings.orden.templates.system.documents.title",
        description_key: "settings.orden.templates.system.documents.description",
        category_key: "settings.orden.templates.categories.organize",
        icon: "file",
        tone: "organize",
        yaml: "rules:\n  - name: \"整理文档\"\n    locations:\n      - ~/Downloads\n    subfolders: true\n    filters:\n      - extension: [pdf, doc, docx, xls, xlsx, ppt, pptx]\n    actions:\n      - move: ~/Documents/Inbox/\n",
    },
    SystemOrdenTemplate {
        id: "sort-archives",
        title_key: "settings.orden.templates.system.archives.title",
        description_key: "settings.orden.templates.system.archives.description",
        category_key: "settings.orden.templates.categories.organize",
        icon: "archive",
        tone: "organize",
        yaml: "rules:\n  - name: \"整理压缩包\"\n    locations:\n      - ~/Downloads\n    filters:\n      - extension: [zip, 7z, rar, tar, gz]\n    actions:\n      - move: ~/Downloads/Archives/\n",
    },
    SystemOrdenTemplate {
        id: "extract-downloads",
        title_key: "settings.orden.templates.system.extract.title",
        description_key: "settings.orden.templates.system.extract.description",
        category_key: "settings.orden.templates.categories.automation",
        icon: "folder-archive",
        tone: "automation",
        yaml: "rules:\n  - name: \"自动解压下载的压缩包\"\n    locations:\n      - ~/Downloads\n    filters:\n      - extension: [zip, tar, gz]\n    actions:\n      - extract:\n          dest: ~/Downloads/Unpacked/\n          delete_original: false\n",
    },
    SystemOrdenTemplate {
        id: "backup-pdfs",
        title_key: "settings.orden.templates.system.backup.title",
        description_key: "settings.orden.templates.system.backup.description",
        category_key: "settings.orden.templates.categories.backup",
        icon: "layers",
        tone: "backup",
        yaml: "rules:\n  - name: \"备份重要 PDF\"\n    locations:\n      - ~/Documents\n    filters:\n      - extension: pdf\n    actions:\n      - copy:\n          dest: ~/Documents/Shelfy Backups/PDF/\n          continue_with: original\n",
    },
    SystemOrdenTemplate {
        id: "empty-downloads",
        title_key: "settings.orden.templates.system.cleanup.title",
        description_key: "settings.orden.templates.system.cleanup.description",
        category_key: "settings.orden.templates.categories.maintenance",
        icon: "sparkles",
        tone: "maintenance",
        yaml: "rules:\n  - name: \"清理临时文件\"\n    locations:\n      - ~/Downloads\n    filters:\n      - extension: [tmp, log, part]\n    actions:\n      - trash\n",
    },
];

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct OrdenTemplateInfo {
    id: String,
    name: String,
    yaml: String,
    is_system: bool,
    title_key: Option<String>,
    description_key: Option<String>,
    category_key: Option<String>,
    icon: String,
    tone: String,
}

fn ensure_orden_templates(data_dir: &std::path::Path) -> Result<(), String> {
    let dir = orden_templates_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for template in SYSTEM_ORDEN_TEMPLATES {
        let path = dir.join(format!("{}.yaml", template.id));
        let is_current = std::fs::read_to_string(&path)
            .map(|yaml| yaml == template.yaml)
            .unwrap_or(false);
        if !is_current {
            std::fs::write(&path, template.yaml).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn find_system_template(name: &str) -> Option<&'static SystemOrdenTemplate> {
    SYSTEM_ORDEN_TEMPLATES
        .iter()
        .find(|template| template.id == name)
}

/// A captured log entry from an orden run (mirrors `orden::action::LogEntry`).
#[derive(serde::Serialize, Clone)]
pub struct OrdenLog {
    level: String,
    sender: String,
    rule_nr: i64,
    path: String,
    msg: String,
}

#[derive(serde::Serialize, Clone)]
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
    #[serde(rename = "filterSteps", default)]
    filter_steps: Vec<OrdenVisualStep>,
    #[serde(rename = "actionSteps", default)]
    action_steps: Vec<OrdenVisualStep>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct OrdenVisualStep {
    id: String,
    kind: String,
    value: String,
    #[serde(default)]
    inverted: bool,
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

#[tauri::command]
pub fn orden_template_list_cmd() -> Result<Vec<OrdenTemplateInfo>, String> {
    let root = data_dir()?;
    ensure_orden_templates(&root)?;
    let dir = orden_templates_dir(&root);
    let mut templates = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| "Invalid template file name".to_string())?
            .to_string();
        let yaml = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        if let Some(system) = find_system_template(&name) {
            templates.push(OrdenTemplateInfo {
                id: system.id.to_string(),
                name: system.id.to_string(),
                yaml,
                is_system: true,
                title_key: Some(system.title_key.to_string()),
                description_key: Some(system.description_key.to_string()),
                category_key: Some(system.category_key.to_string()),
                icon: system.icon.to_string(),
                tone: system.tone.to_string(),
            });
        } else {
            templates.push(OrdenTemplateInfo {
                id: format!("custom-{}", name),
                name,
                yaml,
                is_system: false,
                title_key: None,
                description_key: None,
                category_key: None,
                icon: "sparkles".to_string(),
                tone: "custom".to_string(),
            });
        }
    }
    templates.sort_by(|a, b| {
        a.is_system
            .cmp(&b.is_system)
            .reverse()
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(templates)
}

#[tauri::command]
pub fn orden_template_load_cmd(name: String) -> Result<String, String> {
    let root = data_dir()?;
    ensure_orden_templates(&root)?;
    let clean = valid_template_name(&name)?;
    std::fs::read_to_string(orden_templates_dir(&root).join(format!("{}.yaml", clean)))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orden_template_save_cmd(name: String, yaml: String) -> Result<(), String> {
    let root = data_dir()?;
    ensure_orden_templates(&root)?;
    let clean = valid_template_name(&name)?;
    if find_system_template(&clean).is_some() {
        return Err("System templates cannot be overwritten".to_string());
    }
    crate::orden::Config::from_string(&yaml)?;
    std::fs::write(
        orden_templates_dir(&root).join(format!("{}.yaml", clean)),
        yaml,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orden_template_delete_cmd(name: String) -> Result<(), String> {
    let root = data_dir()?;
    ensure_orden_templates(&root)?;
    let clean = valid_template_name(&name)?;
    if find_system_template(&clean).is_some() {
        return Err("System templates cannot be deleted".to_string());
    }
    let path = orden_templates_dir(&root).join(format!("{}.yaml", clean));
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
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

fn map_run_result(result: crate::orden::RunResult) -> OrdenRunResult {
    OrdenRunResult {
        success: result.success,
        errors: result.errors,
        simulate: result.simulate,
        logs: result
            .logs
            .into_iter()
            .map(|log| OrdenLog {
                level: log.level,
                sender: log.sender,
                rule_nr: log.rule_nr,
                path: log.path,
                msg: log.msg,
            })
            .collect(),
    }
}

/// Start a simulated or real Orden execution on a dedicated worker thread.
#[tauri::command]
pub fn orden_run_cmd(
    yaml: String,
    simulate: bool,
    tags: Vec<String>,
    skip_tags: Vec<String>,
) -> Result<crate::orden_runtime::OrdenTaskHandle, String> {
    let task = crate::orden_runtime::spawn(move || {
        let config_name =
            find_orden_config_name_for_yaml(&yaml).unwrap_or_else(|| "<ad-hoc>".to_string());
        let opts = crate::orden::ExecuteOptions {
            simulate,
            tags: tags.into_iter().collect(),
            skip_tags: skip_tags.into_iter().collect(),
            working_dir: std::env::current_dir().unwrap_or_default(),
        };
        let result = match crate::orden::run_yaml(&yaml, &opts) {
            Ok(result) => result,
            Err(error) => {
                log_orden_failure(&config_name, simulate, "manual", &error);
                return Err(error);
            }
        };
        let _ = log_orden_run(
            &config_name,
            simulate,
            result.success as i64,
            result.errors as i64,
            "manual",
            &serde_json::to_string(&result.logs).unwrap_or_else(|_| "[]".to_string()),
        );
        serde_json::to_value(map_run_result(result)).map_err(|error| error.to_string())
    });
    Ok(task)
}

#[tauri::command]
pub fn orden_task_status_cmd(
    task_id: String,
) -> Result<crate::orden_runtime::OrdenTaskStatus, String> {
    crate::orden_runtime::status(&task_id)
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
        filter_steps: parse_visual_steps(yaml_get(mapping, "filters"), "filter", true),
        action_steps: parse_visual_steps(yaml_get(mapping, "actions"), "action", false),
        action,
    })
}

fn parse_visual_steps(
    value: Option<&serde_yaml::Value>,
    id_prefix: &str,
    allow_inverted: bool,
) -> Vec<OrdenVisualStep> {
    let Some(sequence) = value.and_then(|value| value.as_sequence()) else {
        return Vec::new();
    };
    sequence
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let (raw_kind, value) = if let Some(kind) = item.as_str() {
                (kind.to_string(), serde_yaml::Value::Null)
            } else {
                let mapping = item.as_mapping()?;
                let (key, value) = mapping.iter().next()?;
                (key.as_str()?.to_string(), value.clone())
            };
            let (kind, inverted) = if allow_inverted {
                raw_kind
                    .strip_prefix("not ")
                    .map(|kind| (kind.to_string(), true))
                    .unwrap_or((raw_kind, false))
            } else {
                (raw_kind, false)
            };
            Some(OrdenVisualStep {
                id: format!("{}-{}", id_prefix, index + 1),
                kind,
                value: yaml_value_for_editor(&value),
                inverted,
            })
        })
        .collect()
}

fn yaml_value_for_editor(value: &serde_yaml::Value) -> String {
    if value.is_null() {
        return String::new();
    }
    let serialized = serde_yaml::to_string(value).unwrap_or_default();
    let trimmed = serialized.trim();
    trimmed
        .strip_prefix("---\n")
        .unwrap_or(trimmed)
        .trim()
        .to_string()
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
pub fn orden_delete_history_cmd(id: i64) -> Result<(), String> {
    delete_orden_run_log(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn orden_clear_history_cmd(name: Option<String>) -> Result<(), String> {
    clear_orden_run_logs(name.as_deref()).map_err(|e| e.to_string())
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
pub fn orden_run_job_cmd(job: OrdenJob) -> Result<crate::orden_runtime::OrdenTaskHandle, String> {
    let task = crate::orden_runtime::spawn(move || {
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
        let result = match crate::orden::run_yaml(&yaml, &opts) {
            Ok(result) => result,
            Err(error) => {
                log_orden_failure(&job.config_name, job.simulate, "manual-job", &error);
                return Err(error);
            }
        };
        let _ = log_orden_run(
            &job.config_name,
            job.simulate,
            result.success as i64,
            result.errors as i64,
            "manual-job",
            &serde_json::to_string(&result.logs).unwrap_or_else(|_| "[]".to_string()),
        );
        if let Some(id) = job.id {
            let _ = mark_orden_job_run(id);
        }
        serde_json::to_value(map_run_result(result)).map_err(|error| error.to_string())
    });
    Ok(task)
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn log_orden_failure(config_name: &str, simulate: bool, trigger: &str, error: &str) {
    let logs = serde_json::json!([{
        "level": "error",
        "sender": "orden",
        "rule_nr": -1,
        "path": "<config>",
        "msg": error,
    }]);
    let _ = log_orden_run(config_name, simulate, 0, 1, trigger, &logs.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("shelfy-{name}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn ensure_templates_refreshes_system_yaml_and_preserves_custom_files() {
        let root = temp_data_dir("templates");
        let templates_dir = orden_templates_dir(&root);
        std::fs::create_dir_all(&templates_dir).unwrap();
        std::fs::write(templates_dir.join("backup-pdfs.yaml"), "stale").unwrap();
        std::fs::write(templates_dir.join("my-template.yaml"), "custom").unwrap();

        ensure_orden_templates(&root).unwrap();

        let backup = find_system_template("backup-pdfs").unwrap();
        assert_eq!(
            std::fs::read_to_string(templates_dir.join("backup-pdfs.yaml")).unwrap(),
            backup.yaml
        );
        assert_eq!(
            std::fs::read_to_string(templates_dir.join("my-template.yaml")).unwrap(),
            "custom"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn visual_parser_keeps_complete_filter_and_action_pipelines() {
        let yaml = r#"
rules:
  - name: pipeline
    locations: [~/Downloads]
    filters:
      - extension: [pdf, docx]
      - not size: "> 10 MB"
      - created:
          days: 30
          mode: older
    actions:
      - copy:
          dest: [~/Backup/A, ~/Backup/B]
          continue_with: original
      - shell:
          cmd: "echo {path}"
          run_in_simulation: false
"#;

        let visual = orden_visual_from_yaml_cmd(yaml.to_string()).unwrap();
        let rule = &visual.rules[0];
        assert_eq!(rule.filter_steps.len(), 3);
        assert_eq!(rule.filter_steps[0].kind, "extension");
        assert_eq!(rule.filter_steps[1].kind, "size");
        assert!(rule.filter_steps[1].inverted);
        assert!(rule.filter_steps[2].value.contains("days: 30"));
        assert_eq!(rule.action_steps.len(), 2);
        assert_eq!(rule.action_steps[0].kind, "copy");
        assert!(rule.action_steps[0]
            .value
            .contains("continue_with: original"));
        assert_eq!(rule.action_steps[1].kind, "shell");
    }

    #[test]
    fn valid_template_name_strips_yaml_extension_case_insensitively() {
        // matches the frontend regex /\\.ya?ml$/i so custom-{{name}} ids stay
        // in sync between client and server (see orden_template_save_cmd).
        assert_eq!(valid_template_name("foo").unwrap(), "foo");
        assert_eq!(valid_template_name("foo.yaml").unwrap(), "foo");
        assert_eq!(valid_template_name("foo.yml").unwrap(), "foo");
        assert_eq!(valid_template_name("foo.YAML").unwrap(), "foo");
        assert_eq!(valid_template_name("foo.Yml").unwrap(), "foo");
        assert_eq!(valid_template_name("  foo.YAML ").unwrap(), "foo");
        assert_eq!(valid_template_name("foo.tar.gz").unwrap(), "foo.tar.gz");
        assert!(valid_template_name("").is_err());
        assert!(valid_template_name("../etc").is_err());
        assert!(valid_template_name("a/b").is_err());
    }
}
