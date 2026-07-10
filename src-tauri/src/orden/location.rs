/// Default system-excluded files (mirrors `organize.location.DEFAULT_SYSTEM_EXCLUDE_FILES`).
pub const DEFAULT_SYSTEM_EXCLUDE_FILES: &[&str] =
    &["thumbs.db", "desktop.ini", "~$*", ".DS_Store", ".localized"];

/// Default system-excluded directories.
pub const DEFAULT_SYSTEM_EXCLUDE_DIRS: &[&str] = &[".git", ".svn"];

/// A search location for a rule.
///
/// Mirrors `organize.location.Location`.
#[derive(Debug, Clone)]
pub struct Location {
    pub paths: Vec<String>,
    pub min_depth: i32,
    pub max_depth: MaxDepth,
    pub search: SearchMethod,
    pub exclude_files: Vec<String>,
    pub exclude_dirs: Vec<String>,
    pub system_exclude_files: Vec<String>,
    pub system_exclude_dirs: Vec<String>,
    pub filter: Option<Vec<String>>,
    pub filter_dirs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxDepth {
    Inherit,
    Unlimited,
    Limited(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMethod {
    Breadth,
    Depth,
}

impl Default for Location {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            min_depth: 0,
            max_depth: MaxDepth::Inherit,
            search: SearchMethod::Breadth,
            exclude_files: Vec::new(),
            exclude_dirs: Vec::new(),
            system_exclude_files: DEFAULT_SYSTEM_EXCLUDE_FILES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            system_exclude_dirs: DEFAULT_SYSTEM_EXCLUDE_DIRS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            filter: None,
            filter_dirs: None,
        }
    }
}

impl Location {
    pub fn from_yaml(v: &serde_yaml::Value) -> Result<Self, String> {
        let mut loc = Location::default();
        match v {
            serde_yaml::Value::String(s) => loc.paths.push(s.clone()),
            serde_yaml::Value::Sequence(seq) => {
                for item in seq {
                    if let Some(s) = item.as_str() {
                        loc.paths.push(s.to_string());
                    }
                }
            }
            serde_yaml::Value::Mapping(m) => {
                // path can be str or list
                match m.get(&serde_yaml::Value::String("path".into())) {
                    Some(serde_yaml::Value::String(s)) => loc.paths.push(s.clone()),
                    Some(serde_yaml::Value::Sequence(seq)) => {
                        for item in seq {
                            if let Some(s) = item.as_str() {
                                loc.paths.push(s.to_string());
                            }
                        }
                    }
                    _ => {}
                }
                loc.min_depth = m
                    .get(&serde_yaml::Value::String("min_depth".into()))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                loc.max_depth = match m.get(&serde_yaml::Value::String("max_depth".into())) {
                    None => MaxDepth::Inherit,
                    Some(serde_yaml::Value::String(s)) if s == "inherit" => MaxDepth::Inherit,
                    Some(serde_yaml::Value::Null) => MaxDepth::Unlimited,
                    Some(n) => match n.as_i64() {
                        Some(d) => MaxDepth::Limited(d as i32),
                        None => MaxDepth::Inherit,
                    },
                };
                loc.search = match m
                    .get(&serde_yaml::Value::String("search".into()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("breadth")
                {
                    "breadth" => SearchMethod::Breadth,
                    "depth" => SearchMethod::Depth,
                    other => return Err(format!("Unknown search method: {}", other)),
                };
                loc.exclude_files = str_list(m, "exclude_files");
                loc.exclude_dirs = str_list(m, "exclude_dirs");
                loc.system_exclude_files =
                    match m.get(&serde_yaml::Value::String("system_exclude_files".into())) {
                        Some(_) => str_list(m, "system_exclude_files"),
                        None => loc.system_exclude_files.clone(),
                    };
                loc.system_exclude_dirs =
                    match m.get(&serde_yaml::Value::String("system_exclude_dirs".into())) {
                        Some(_) => str_list(m, "system_exclude_dirs"),
                        None => loc.system_exclude_dirs.clone(),
                    };
                loc.filter = opt_str_list(m, "filter");
                loc.filter_dirs = opt_str_list(m, "filter_dirs");
            }
            _ => return Err("Location must be a string or mapping".into()),
        }
        Ok(loc)
    }

    pub fn combined_exclude_files(&self) -> Vec<String> {
        let mut v = self.system_exclude_files.clone();
        v.extend(self.exclude_files.clone());
        v
    }
    pub fn combined_exclude_dirs(&self) -> Vec<String> {
        let mut v = self.system_exclude_dirs.clone();
        v.extend(self.exclude_dirs.clone());
        v
    }
}

fn str_list(m: &serde_yaml::Mapping, key: &str) -> Vec<String> {
    match m.get(&serde_yaml::Value::String(key.into())) {
        Some(serde_yaml::Value::String(s)) => vec![s.clone()],
        Some(serde_yaml::Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

fn opt_str_list(m: &serde_yaml::Mapping, key: &str) -> Option<Vec<String>> {
    match m.get(&serde_yaml::Value::String(key.into())) {
        Some(serde_yaml::Value::String(s)) => Some(vec![s.clone()]),
        Some(serde_yaml::Value::Sequence(seq)) => {
            let v: Vec<String> = seq
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            Some(v)
        }
        _ => None,
    }
}
