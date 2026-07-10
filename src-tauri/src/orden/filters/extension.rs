use std::collections::HashSet;

use crate::orden::filter::{set_var, Filter, FilterResult};
use crate::orden::resource::Resource;
use crate::orden::value::Value;

/// Filter by file extension.
///
/// Mirrors `organize.filters.extension.Extension`.
pub struct Extension {
    pub extensions: HashSet<String>,
}

impl Extension {
    pub fn new(extensions: Vec<String>) -> Self {
        let extensions: HashSet<String> = extensions
            .into_iter()
            .map(|e| normalize_extension(&e))
            .filter(|s| !s.is_empty())
            .collect();
        Self { extensions }
    }
}

fn normalize_extension(ext: &str) -> String {
    let ext = ext.trim_start_matches('.');
    ext.to_lowercase()
}

impl Filter for Extension {
    fn name(&self) -> &str {
        "extension"
    }
    fn supports_dirs(&self) -> bool {
        false
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        let path = res.path.as_ref().ok_or("extension: no path")?;
        if res.is_dir() {
            return Err("extension: dirs not supported".into());
        }
        let suffix = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let match_result = if self.extensions.is_empty() {
            true
        } else if suffix.is_empty() {
            false
        } else {
            self.extensions.contains(&suffix)
        };
        set_var(res, "extension", Value::Str(suffix));
        Ok(if match_result {
            FilterResult::Match
        } else {
            FilterResult::NoMatch
        })
    }
}
