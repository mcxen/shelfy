use std::collections::HashSet;
use std::path::PathBuf;

use crate::orden::value::Value;

/// A resource is created for each handled file (or folder) and passed through the
/// filter and action pipeline.
///
/// Mirrors `organize.resource.Resource`.
#[derive(Debug, Clone)]
pub struct Resource {
    /// The path to the current file or folder (None in standalone mode).
    pub path: Option<PathBuf>,
    /// The search location as given in rule.locations.
    pub basedir: Option<PathBuf>,
    /// Filter / action variables, available as `{name}` placeholders in templates.
    pub vars: Value,
    /// Paths to skip for the rest of the rule (populated by duplicate detection, move...).
    pub walker_skip_pathes: HashSet<PathBuf>,
    /// The index of the rule in the config file.
    pub rule_nr: i64,
    /// Human-readable rule name for history / previews.
    pub rule_name: Option<String>,
}

impl Resource {
    pub fn new(path: PathBuf, basedir: PathBuf, rule_nr: i64, rule_name: Option<String>) -> Self {
        Self {
            path: Some(path),
            basedir: Some(basedir),
            vars: Value::Map(Default::default()),
            walker_skip_pathes: HashSet::new(),
            rule_nr,
            rule_name,
        }
    }

    pub fn standalone(rule_nr: i64, rule_name: Option<String>) -> Self {
        Self {
            path: None,
            basedir: None,
            vars: Value::Map(Default::default()),
            walker_skip_pathes: HashSet::new(),
            rule_nr,
            rule_name,
        }
    }

    pub fn is_file(&self) -> bool {
        self.path.as_ref().map(|p| p.is_file()).unwrap_or(false)
    }

    pub fn is_dir(&self) -> bool {
        self.path.as_ref().map(|p| p.is_dir()).unwrap_or(false)
    }

    pub fn is_empty(&self) -> bool {
        if let Some(path) = &self.path {
            if path.is_file() {
                return std::fs::metadata(path)
                    .map(|m| m.len() == 0)
                    .unwrap_or(false);
            } else if path.is_dir() {
                return std::fs::read_dir(path)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false);
            }
        }
        false
    }

    pub fn relative_path(&self) -> Option<PathBuf> {
        match (&self.basedir, &self.path) {
            (Some(base), Some(path)) => path.strip_prefix(base).ok().map(|p| p.to_path_buf()),
            (None, Some(path)) => Some(path.clone()),
            _ => None,
        }
    }

    /// Build the variable context available to templates.
    pub fn dict(&self) -> Value {
        let mut m = if let Value::Map(ref existing) = self.vars {
            existing.clone()
        } else {
            Default::default()
        };
        m.insert(
            "path".to_string(),
            Value::Str(
                self.path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
        );
        m.insert(
            "basedir".to_string(),
            Value::Str(
                self.basedir
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
        );
        m.insert(
            "location".to_string(),
            Value::Str(
                self.basedir
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
        );
        m.insert(
            "relative_path".to_string(),
            Value::Str(
                self.relative_path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
        );
        Value::Map(m)
    }
}
