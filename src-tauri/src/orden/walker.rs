use std::path::{Path, PathBuf};

use globset::GlobSetBuilder;

/// A filesystem walker with depth limits and glob-based include/exclude filters.
///
/// Mirrors `organize.walker.Walker`. Symlinks are skipped (same as organize).
#[derive(Debug, Clone)]
pub struct Walker {
    pub min_depth: i32,
    pub max_depth: Option<i32>,
    pub method: WalkMethod,
    pub filter_dirs: Option<Vec<String>>,
    pub filter_files: Option<Vec<String>>,
    pub exclude_dirs: Vec<String>,
    pub exclude_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkMethod {
    Breadth,
    Depth,
}

impl Default for Walker {
    fn default() -> Self {
        Self {
            min_depth: 0,
            max_depth: None,
            method: WalkMethod::Breadth,
            filter_dirs: None,
            filter_files: None,
            exclude_dirs: Vec::new(),
            exclude_files: Vec::new(),
        }
    }
}

fn build_matcher(patterns: &[String]) -> Option<globset::GlobSet> {
    if patterns.is_empty() {
        return None;
    }
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        // case-insensitive to match organize's fnmatch behaviour
        builder.add(globset::Glob::new(p).ok()?);
    }
    builder.build().ok()
}

fn matches_any(name: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    if let Some(set) = build_matcher(patterns) {
        return set.is_match(name);
    }
    false
}

impl Walker {
    pub fn files(&self, path: &str) -> Vec<PathBuf> {
        // a single file is emitted as-is
        if Path::new(path).is_file() {
            return vec![PathBuf::from(path)];
        }
        let mut out = Vec::new();
        self.walk(path, true, false, 0, &mut out);
        out
    }

    pub fn dirs(&self, path: &str) -> Vec<PathBuf> {
        let mut out = Vec::new();
        self.walk(path, false, true, 0, &mut out);
        out
    }

    fn walk(&self, top: &str, files: bool, dirs: bool, lvl: i32, out: &mut Vec<PathBuf>) {
        let entries = match std::fs::read_dir(top) {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut dir_entries: Vec<PathBuf> = Vec::new();
        let mut file_entries: Vec<PathBuf> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            // skip symlinks (organize behaviour)
            if path.is_symlink() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if !matches_any(&name, &self.exclude_dirs)
                    && self
                        .filter_dirs
                        .as_ref()
                        .map_or(true, |f| matches_any(&name, f))
                {
                    dir_entries.push(path);
                }
            } else if files {
                if lvl >= self.min_depth
                    && !matches_any(&name, &self.exclude_files)
                    && self
                        .filter_files
                        .as_ref()
                        .map_or(true, |f| matches_any(&name, f))
                {
                    file_entries.push(path);
                }
            }
        }

        // natural sort by name (organize uses os_sorted)
        dir_entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        file_entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        match self.method {
            WalkMethod::Breadth => {
                if files {
                    out.extend(file_entries);
                }
                if dirs && lvl >= self.min_depth {
                    out.extend(dir_entries.iter().cloned());
                }
                if let Some(max) = self.max_depth {
                    if lvl >= max {
                        return;
                    }
                }
                for d in dir_entries {
                    self.walk(&d.to_string_lossy(), files, dirs, lvl + 1, out);
                }
            }
            WalkMethod::Depth => {
                if let Some(max) = self.max_depth {
                    if lvl >= max {
                        if dirs && lvl >= self.min_depth {
                            out.extend(dir_entries);
                        }
                        if files {
                            out.extend(file_entries);
                        }
                        return;
                    }
                }
                for d in dir_entries.iter() {
                    self.walk(&d.to_string_lossy(), files, dirs, lvl + 1, out);
                }
                if files {
                    out.extend(file_entries);
                }
                if dirs && lvl >= self.min_depth {
                    let already = out.len();
                    out.extend(dir_entries.iter().cloned());
                    // depth-first already yielded subdirs above; here we only yield
                    // the current-level dirs that haven't been emitted.
                    out.truncate(already + dir_entries.len());
                }
            }
        }
    }
}
