use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::orden::action::{Level, Output};
use crate::orden::actions::{action_def_from_yaml, build_action, ActionDef};
use crate::orden::filter::{run_pipeline, FilterMode};
use crate::orden::filters::{build_filter, filter_def_from_yaml, parse_filter_mode, FilterDef};
use crate::orden::location::{Location, MaxDepth, SearchMethod};
use crate::orden::resource::Resource;
use crate::orden::walker::{WalkMethod, Walker};

pub mod action;
pub mod actions;
pub mod conflict;
pub mod filter;
pub mod filters;
pub mod location;
pub mod resource;
pub mod target_path;
pub mod template;
pub mod value;
pub mod walker;

pub use action::Action;
pub use filter::Filter;

/// A single organize rule.
pub struct Rule {
    pub name: Option<String>,
    pub enabled: bool,
    pub targets: Targets,
    pub locations: Vec<Location>,
    pub subfolders: bool,
    pub tags: HashSet<String>,
    pub filter_defs: Vec<FilterDef>,
    pub filter_mode: FilterMode,
    pub action_defs: Vec<ActionDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Targets {
    Files,
    Dirs,
}

/// A full organize config.
pub struct Config {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Default)]
pub struct ReportSummary {
    pub success: u64,
    pub errors: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct PreviewOptions {
    pub max_scan_entries: usize,
    pub max_matches: u64,
}

impl Default for PreviewOptions {
    fn default() -> Self {
        Self {
            max_scan_entries: 500,
            max_matches: 10,
        }
    }
}

/// Execution options for `Config::execute`.
#[derive(Clone)]
pub struct ExecuteOptions {
    pub simulate: bool,
    pub tags: HashSet<String>,
    pub skip_tags: HashSet<String>,
    pub working_dir: PathBuf,
    /// Bounded, side-effect-free GUI preview. CLI/MCP simulations remain exhaustive.
    pub preview: Option<PreviewOptions>,
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self {
            simulate: true,
            tags: HashSet::new(),
            skip_tags: HashSet::new(),
            working_dir: PathBuf::from("."),
            preview: None,
        }
    }
}

// ---- parsing ----

impl Config {
    pub fn from_string(yaml: &str) -> Result<Self, String> {
        let doc: serde_yaml::Value =
            serde_yaml::from_str(yaml).map_err(|e| format!("YAML error: {}", e))?;
        let mapping = doc
            .as_mapping()
            .ok_or("Config must be a mapping with a `rules` key")?;
        let rules_seq = mapping
            .get(&serde_yaml::Value::String("rules".into()))
            .and_then(|v| v.as_sequence())
            .ok_or("Config must have a `rules` list")?;
        let mut rules = Vec::new();
        for item in rules_seq {
            rules.push(Rule::from_yaml(item)?);
        }
        Ok(Self { rules })
    }
}

impl Rule {
    pub fn from_yaml(v: &serde_yaml::Value) -> Result<Self, String> {
        let m = v.as_mapping().ok_or("Rule must be a mapping")?;
        let name = m
            .get(&serde_yaml::Value::String("name".into()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let enabled = m
            .get(&serde_yaml::Value::String("enabled".into()))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let targets = match m
            .get(&serde_yaml::Value::String("targets".into()))
            .and_then(|v| v.as_str())
            .unwrap_or("files")
        {
            "files" => Targets::Files,
            "dirs" => Targets::Dirs,
            other => return Err(format!("Unknown targets: {}", other)),
        };
        let locations = match m.get(&serde_yaml::Value::String("locations".into())) {
            None => Vec::new(),
            Some(serde_yaml::Value::Null) => Vec::new(),
            Some(serde_yaml::Value::String(s)) => {
                vec![Location::from_yaml(&serde_yaml::Value::String(s.clone()))?]
            }
            Some(serde_yaml::Value::Sequence(seq)) => {
                let mut out = Vec::new();
                for item in seq {
                    out.push(Location::from_yaml(item)?);
                }
                out
            }
            Some(other) => vec![Location::from_yaml(other)?],
        };
        let subfolders = m
            .get(&serde_yaml::Value::String("subfolders".into()))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let tags = match m.get(&serde_yaml::Value::String("tags".into())) {
            Some(serde_yaml::Value::Sequence(seq)) => seq
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => HashSet::new(),
        };
        let filter_defs = match m.get(&serde_yaml::Value::String("filters".into())) {
            None => Vec::new(),
            Some(serde_yaml::Value::Sequence(seq)) => {
                let mut out = Vec::new();
                for item in seq {
                    out.push(filter_def_from_yaml(item)?);
                }
                out
            }
            _ => Vec::new(),
        };
        let filter_mode = parse_filter_mode(
            m.get(&serde_yaml::Value::String("filter_mode".into()))
                .and_then(|v| v.as_str())
                .unwrap_or("all"),
        )?;
        let actions_val = m
            .get(&serde_yaml::Value::String("actions".into()))
            .ok_or("Rule requires an `actions` list")?;
        let action_defs = match actions_val {
            serde_yaml::Value::Sequence(seq) => {
                let mut out = Vec::new();
                for item in seq {
                    out.push(action_def_from_yaml(item)?);
                }
                out
            }
            _ => return Err("`actions` must be a list".into()),
        };
        Ok(Self {
            name,
            enabled,
            targets,
            locations,
            subfolders,
            tags,
            filter_defs,
            filter_mode,
            action_defs,
        })
    }
}

// ---- tags / execution gating (mirrors config.should_execute) ----

pub fn should_execute(
    rule_tags: &HashSet<String>,
    tags: &HashSet<String>,
    skip_tags: &HashSet<String>,
) -> bool {
    if rule_tags.contains("always") && !skip_tags.contains("always") {
        return true;
    }
    if rule_tags.contains("never") && !tags.contains("never") {
        return false;
    }
    if tags.is_empty() && skip_tags.is_empty() {
        return true;
    }
    if rule_tags.is_empty() && !tags.is_empty() {
        return false;
    }
    let should_run =
        rule_tags.iter().any(|t| tags.contains(t)) || tags.is_empty() || rule_tags.is_empty();
    let should_skip = rule_tags.iter().any(|t| skip_tags.contains(t));
    should_run && !should_skip
}

// ---- execution ----

impl Rule {
    pub fn execute(
        &self,
        rule_nr: i64,
        opts: &ExecuteOptions,
        output: &dyn Output,
    ) -> ReportSummary {
        let rule_resource = Resource::standalone(rule_nr, self.name.clone());
        if !self.enabled {
            output.msg(
                &rule_resource,
                "Rule disabled; skipped",
                "rule",
                Level::Info,
            );
            return ReportSummary::default();
        }
        output.msg(
            &rule_resource,
            &format!(
                "Rule started ({} filters, {} actions, simulate={})",
                self.filter_defs.len(),
                self.action_defs.len(),
                opts.simulate
            ),
            "rule",
            Level::Info,
        );
        let mut summary = ReportSummary::default();

        // standalone mode (no locations)
        if self.locations.is_empty() {
            let mut res = Resource::standalone(rule_nr, self.name.clone());
            if let Err(e) = self.run_actions(&mut res, opts, &mut summary, output) {
                output.msg(&res, &e, "rule", Level::Error);
                summary.errors += 1;
            }
            output.msg(
                &res,
                &format!(
                    "Rule finished: {} matched, {} errors",
                    summary.success, summary.errors
                ),
                "rule",
                Level::Info,
            );
            return summary;
        }

        // build filters and actions once per rule
        let mut filters: Vec<Box<dyn Filter>> = match self
            .filter_defs
            .iter()
            .map(build_filter)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(v) => v,
            Err(e) => {
                output.msg(
                    &Resource::standalone(rule_nr, self.name.clone()),
                    &e,
                    "rule",
                    Level::Error,
                );
                summary.errors += 1;
                return summary;
            }
        };
        let mut actions: Vec<Box<dyn Action>> = match self
            .action_defs
            .iter()
            .map(build_action)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(v) => v,
            Err(e) => {
                output.msg(
                    &Resource::standalone(rule_nr, self.name.clone()),
                    &e,
                    "rule",
                    Level::Error,
                );
                summary.errors += 1;
                return summary;
            }
        };

        let mut skip_pathes: HashSet<PathBuf> = HashSet::new();
        let mut scan_budget = opts
            .preview
            .map(|preview| preview.max_scan_entries)
            .unwrap_or(usize::MAX);
        let mut scanned_entries = 0usize;
        let mut preview_truncated = false;
        'locations: for loc in &self.locations {
            let max_depth = match loc.max_depth {
                MaxDepth::Inherit => {
                    if self.subfolders {
                        None
                    } else {
                        Some(0)
                    }
                }
                MaxDepth::Unlimited => None,
                MaxDepth::Limited(d) => Some(d),
            };
            let walker = Walker {
                min_depth: loc.min_depth,
                max_depth,
                method: match loc.search {
                    SearchMethod::Breadth => WalkMethod::Breadth,
                    SearchMethod::Depth => WalkMethod::Depth,
                },
                filter_dirs: loc.filter_dirs.clone(),
                filter_files: loc.filter.clone(),
                exclude_dirs: loc.combined_exclude_dirs(),
                exclude_files: loc.combined_exclude_files(),
            };
            for path_str in &loc.paths {
                if scan_budget == 0 {
                    preview_truncated = true;
                    break 'locations;
                }
                let expanded = match template::render(path_str, &template::map_from(vec![])) {
                    Ok(p) => p,
                    Err(e) => {
                        summary.errors += 1;
                        let res = Resource::new(
                            PathBuf::from(path_str),
                            opts.working_dir.clone(),
                            rule_nr,
                            self.name.clone(),
                        );
                        output.msg(
                            &res,
                            &format!("Cannot expand location: {}", e),
                            "walker",
                            Level::Error,
                        );
                        continue;
                    }
                };
                let base = resolve_location_path(&expanded, &opts.working_dir);
                let location_resource =
                    Resource::new(base.clone(), base.clone(), rule_nr, self.name.clone());
                output.msg(
                    &location_resource,
                    &format!("Scanning {}", base.display()),
                    "walker",
                    Level::Info,
                );
                let budget_before_walk = scan_budget;
                let (entries, walk_errors) =
                    match self.targets {
                        Targets::Files => walker
                            .files_with_errors_budget(&base.to_string_lossy(), &mut scan_budget),
                        Targets::Dirs => walker
                            .dirs_with_errors_budget(&base.to_string_lossy(), &mut scan_budget),
                    };
                scanned_entries += budget_before_walk.saturating_sub(scan_budget);
                if opts.preview.is_some() && scan_budget == 0 {
                    preview_truncated = true;
                }
                output.msg(
                    &location_resource,
                    &format!("Found {} candidate items", entries.len()),
                    "walker",
                    Level::Info,
                );
                for error in walk_errors {
                    let res = Resource::new(error.path, base.clone(), rule_nr, self.name.clone());
                    let message = if error.permission_denied {
                        format!(
                            "{}; permission denied. On macOS, grant Files and Folders or Full Disk Access.",
                            error.message
                        )
                    } else {
                        error.message
                    };
                    output.msg(&res, &message, "walker", Level::Error);
                    summary.errors += 1;
                }
                for p in entries {
                    if skip_pathes.contains(&p) {
                        continue;
                    }
                    let mut res = Resource::new(p, base.clone(), rule_nr, self.name.clone());
                    let matched = match run_pipeline(&mut filters, self.filter_mode, &mut res) {
                        Ok(b) => b,
                        Err(e) => {
                            output.msg(&res, &e, "filter", Level::Error);
                            summary.errors += 1;
                            continue;
                        }
                    };
                    if !matched {
                        continue;
                    }
                    output.msg(&res, "Filters matched", "filter", Level::Info);
                    match self.run_actions_with(&mut res, opts, &mut actions, output) {
                        Ok(()) => {
                            for sp in &res.walker_skip_pathes {
                                skip_pathes.insert(sp.clone());
                            }
                            summary.success += 1;
                            if opts
                                .preview
                                .is_some_and(|preview| summary.success >= preview.max_matches)
                            {
                                preview_truncated = true;
                                break 'locations;
                            }
                        }
                        Err(e) => {
                            output.msg(&res, &e, "action", Level::Error);
                            summary.errors += 1;
                        }
                    }
                }
            }
        }
        if preview_truncated {
            output.msg(
                &rule_resource,
                &format!(
                    "Preview stopped after scanning {} entries / {} matches",
                    scanned_entries, summary.success
                ),
                "preview",
                Level::Info,
            );
        }
        output.msg(
            &rule_resource,
            &format!(
                "Rule finished: {} matched, {} errors",
                summary.success, summary.errors
            ),
            "rule",
            if summary.errors > 0 {
                Level::Warn
            } else {
                Level::Info
            },
        );
        summary
    }

    fn run_actions(
        &self,
        res: &mut Resource,
        opts: &ExecuteOptions,
        _summary: &mut ReportSummary,
        output: &dyn Output,
    ) -> Result<(), String> {
        let mut actions: Vec<Box<dyn Action>> = self
            .action_defs
            .iter()
            .map(build_action)
            .collect::<Result<Vec<_>, _>>()?;
        self.run_actions_with(res, opts, &mut actions, output)
    }

    fn run_actions_with(
        &self,
        res: &mut Resource,
        opts: &ExecuteOptions,
        actions: &mut [Box<dyn Action>],
        output: &dyn Output,
    ) -> Result<(), String> {
        for action in actions.iter_mut() {
            if opts.preview.is_some() && action.name() == "shell" {
                output.msg(
                    res,
                    "Shell command skipped in preview",
                    action.name(),
                    Level::Info,
                );
                continue;
            }
            match action.pipeline_with_output(res, opts.simulate, output) {
                Ok(()) => {
                    output.msg(
                        res,
                        &format!("{}: ok", action.name()),
                        action.name(),
                        Level::Info,
                    );
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}

impl Config {
    pub fn execute(&self, opts: &ExecuteOptions, output: &dyn Output) -> ReportSummary {
        let mut summary = ReportSummary::default();
        for (i, rule) in self.rules.iter().enumerate() {
            if opts
                .preview
                .is_some_and(|preview| summary.success >= preview.max_matches)
            {
                break;
            }
            if !should_execute(&rule.tags, &opts.tags, &opts.skip_tags) {
                output.msg(
                    &Resource::standalone(i as i64, rule.name.clone()),
                    "Rule skipped by tags/skip-tags selection",
                    "rule",
                    Level::Info,
                );
                continue;
            }
            let mut rule_opts = opts.clone();
            if let Some(preview) = rule_opts.preview.as_mut() {
                preview.max_matches = preview.max_matches.saturating_sub(summary.success);
            }
            let s = rule.execute(i as i64, &rule_opts, output);
            summary.success += s.success;
            summary.errors += s.errors;
        }
        summary
    }

    /// Convenience: run with a `CollectingOutput` and return the captured logs + summary.
    pub fn execute_collect(&self, opts: &ExecuteOptions) -> (ReportSummary, Vec<action::LogEntry>) {
        let out = action::CollectingOutput::new();
        let s = self.execute(opts, &out);
        (s, out.take())
    }
}

fn resolve_location_path(path: &str, working_dir: &Path) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        working_dir.join(p)
    }
}

// ---- config file management (stored under <data_dir>/orden/*.yaml) ----

/// The filename stem is used as the config name. A config named "main" maps to
/// `<data_dir>/orden/main.yaml`.
pub fn configs_dir(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("orden")
}

pub fn normalize_config_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    let lower = trimmed.to_lowercase();
    let clean = if lower.ends_with(".yaml") {
        &trimmed[..trimmed.len() - ".yaml".len()]
    } else if lower.ends_with(".yml") {
        &trimmed[..trimmed.len() - ".yml".len()]
    } else {
        trimmed
    };
    if clean.is_empty() {
        return Err("Config name cannot be empty".into());
    }
    if clean == "."
        || clean == ".."
        || clean.contains('/')
        || clean.contains('\\')
        || clean.contains("..")
    {
        return Err("Config name cannot contain path separators".into());
    }
    if !clean
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err("Config name can only contain Unicode letters, numbers, '-' and '_'".into());
    }
    Ok(clean.to_string())
}

fn config_path(data_dir: &std::path::Path, name: &str) -> Result<std::path::PathBuf, String> {
    let clean = normalize_config_name(name)?;
    Ok(configs_dir(data_dir).join(format!("{}.yaml", clean)))
}

/// List available orden config names (file stems) under the data dir.
pub fn list_config_names(data_dir: &std::path::Path) -> Vec<String> {
    let dir = configs_dir(data_dir);
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

/// Load a config by name. Returns the raw YAML text.
pub fn load_config_text(data_dir: &std::path::Path, name: &str) -> Result<String, String> {
    let path = config_path(data_dir, name)?;
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))
}

/// Save a config's YAML text by name (creates the orden dir if needed).
pub fn save_config_text(data_dir: &std::path::Path, name: &str, yaml: &str) -> Result<(), String> {
    let dir = configs_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = config_path(data_dir, name)?;
    std::fs::write(&path, yaml).map_err(|e| e.to_string())
}

pub fn rename_config_text(
    data_dir: &std::path::Path,
    old_name: &str,
    new_name: &str,
    yaml: &str,
) -> Result<(), String> {
    let old_path = config_path(data_dir, old_name)?;
    let new_path = config_path(data_dir, new_name)?;
    if old_path == new_path {
        return std::fs::write(old_path, yaml).map_err(|e| e.to_string());
    }
    if new_path.exists() {
        return Err(format!(
            "Orden config '{}' already exists",
            normalize_config_name(new_name)?
        ));
    }
    std::fs::write(&new_path, yaml).map_err(|e| e.to_string())?;
    if old_path.exists() {
        if let Err(error) = std::fs::remove_file(&old_path) {
            let _ = std::fs::remove_file(&new_path);
            return Err(error.to_string());
        }
    }
    Ok(())
}

/// Delete a config by name.
pub fn delete_config(data_dir: &std::path::Path, name: &str) -> Result<(), String> {
    let path = config_path(data_dir, name)?;
    std::fs::remove_file(&path).map_err(|e| e.to_string())
}

/// A serialized result of running an orden config (sim or run).
#[derive(Debug, serde::Serialize)]
pub struct RunResult {
    pub success: u64,
    pub errors: u64,
    pub simulate: bool,
    pub logs: Vec<action::LogEntry>,
}

/// Parse, optionally validate, and execute a config from YAML text.
pub fn run_yaml(yaml: &str, opts: &ExecuteOptions) -> Result<RunResult, String> {
    let cfg = Config::from_string(yaml)?;
    let (s, logs) = cfg.execute_collect(opts);
    Ok(RunResult {
        success: s.success,
        errors: s.errors,
        simulate: opts.simulate,
        logs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_execute() {
        let empty = HashSet::new();
        let mut tags = HashSet::new();
        tags.insert("release".to_string());
        let mut skip = HashSet::new();
        skip.insert("release".to_string());

        assert!(should_execute(&empty, &empty, &empty));
        // rule with no tags, tags selected → skip
        assert!(!should_execute(&empty, &tags, &empty));
        // matching tag runs
        let mut rule_tags = HashSet::new();
        rule_tags.insert("release".to_string());
        assert!(should_execute(&rule_tags, &tags, &empty));
        // skip-tags skips
        assert!(!should_execute(&rule_tags, &empty, &skip));
    }

    #[test]
    fn test_parse_multiple_sources_copy_destinations_and_filter_mode() {
        let yaml = r#"
rules:
  - name: "Fan out"
    locations:
      - /source/one
      - path:
          - /source/two
          - /source/three
    filter_mode: any
    filters:
      - extension: pdf
      - name: "report-*"
    actions:
      - copy:
          dest:
            - /backup/one/
            - /backup/two/
          continue_with: original
"#;
        let cfg = Config::from_string(yaml).unwrap();
        let rule = &cfg.rules[0];
        assert_eq!(rule.locations.len(), 2);
        assert_eq!(
            rule.locations.iter().map(|l| l.paths.len()).sum::<usize>(),
            3
        );
        assert_eq!(rule.filter_mode, FilterMode::Any);
        assert_eq!(rule.filter_defs.len(), 2);
        let action = build_action(&rule.action_defs[0]).unwrap();
        assert_eq!(action.name(), "copy");
    }

    #[test]
    fn test_copy_fans_out_to_multiple_destinations() {
        let root =
            std::env::temp_dir().join(format!("shelfy-orden-multi-dest-{}", std::process::id()));
        let source = root.join("source");
        let first = root.join("first");
        let second = root.join("second");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("report.pdf"), b"test").unwrap();
        let yaml = format!(
            r#"
rules:
  - locations:
      - "{}"
    filter_mode: all
    filters:
      - extension: pdf
    actions:
      - copy:
          dest:
            - "{}/"
            - "{}/"
          continue_with: original
"#,
            source.display(),
            first.display(),
            second.display()
        );
        let config = Config::from_string(&yaml).unwrap();
        let result = config.execute(
            &ExecuteOptions {
                simulate: false,
                ..ExecuteOptions::default()
            },
            &action::CollectingOutput::new(),
        );
        assert_eq!(result.success, 1);
        assert_eq!(result.errors, 0);
        assert!(first.join("report.pdf").exists());
        assert!(second.join("report.pdf").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn preview_limits_matches_and_never_runs_shell_commands() {
        let root = std::env::temp_dir().join(format!(
            "shelfy-orden-preview-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = root.join("source");
        let marker = root.join("shell-ran");
        std::fs::create_dir_all(&source).unwrap();
        for index in 0..20 {
            std::fs::write(source.join(format!("{index:02}.txt")), b"test").unwrap();
        }
        let yaml = format!(
            r#"
rules:
  - locations: "{}"
    actions:
      - shell:
          cmd: "touch '{}'"
          run_in_simulation: true
"#,
            source.display(),
            marker.display()
        );

        let result = run_yaml(
            &yaml,
            &ExecuteOptions {
                simulate: true,
                preview: Some(PreviewOptions {
                    max_scan_entries: 20,
                    max_matches: 3,
                }),
                ..ExecuteOptions::default()
            },
        )
        .unwrap();

        assert_eq!(result.success, 3);
        assert!(!marker.exists());
        assert!(result
            .logs
            .iter()
            .any(|log| log.msg.contains("Shell command skipped in preview")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn test_parse_simple_config() {
        let yaml = r#"
rules:
  - name: "Test"
    locations:
      - ~/Downloads
    filters:
      - extension: pdf
    actions:
      - echo: "found a pdf"
"#;
        let cfg = Config::from_string(yaml).unwrap();
        assert_eq!(cfg.rules.len(), 1);
        assert_eq!(cfg.rules[0].name.as_deref(), Some("Test"));
        assert_eq!(cfg.rules[0].filter_defs.len(), 1);
        assert_eq!(cfg.rules[0].action_defs.len(), 1);
    }

    #[test]
    fn unicode_config_names_round_trip_on_disk() {
        let root = std::env::temp_dir().join(format!(
            "shelfy-orden-unicode-name-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let yaml = "rules: []\n";

        save_config_text(&root, "整理下载.YAML", yaml).unwrap();
        assert_eq!(load_config_text(&root, "整理下载").unwrap(), yaml);
        assert_eq!(list_config_names(&root), vec!["整理下载"]);
        delete_config(&root, "整理下载.yml").unwrap();
        assert!(list_config_names(&root).is_empty());
        assert!(normalize_config_name("../整理下载").is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn unicode_config_names_can_be_renamed_without_leaving_a_copy() {
        let root = std::env::temp_dir().join(format!(
            "shelfy-orden-unicode-rename-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let yaml = "rules: []\n";
        save_config_text(&root, "整理下载", yaml).unwrap();
        rename_config_text(&root, "整理下载", "整理文档", yaml).unwrap();
        assert_eq!(list_config_names(&root), vec!["整理文档"]);
        assert!(load_config_text(&root, "整理下载").is_err());
        assert_eq!(load_config_text(&root, "整理文档").unwrap(), yaml);
        let _ = std::fs::remove_dir_all(root);
    }
}
