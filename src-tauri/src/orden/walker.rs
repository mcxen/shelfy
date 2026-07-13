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

#[derive(Debug, Clone)]
pub struct WalkError {
    pub path: PathBuf,
    pub message: String,
    pub permission_denied: bool,
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
        self.files_with_errors(path).0
    }

    pub fn files_with_errors(&self, path: &str) -> (Vec<PathBuf>, Vec<WalkError>) {
        let mut budget = usize::MAX;
        self.files_with_errors_budget(path, &mut budget)
    }

    pub fn files_with_errors_budget(
        &self,
        path: &str,
        scan_budget: &mut usize,
    ) -> (Vec<PathBuf>, Vec<WalkError>) {
        if *scan_budget == 0 {
            return (Vec::new(), Vec::new());
        }
        // a single file is emitted as-is
        if Path::new(path).is_file() {
            *scan_budget -= 1;
            return (vec![PathBuf::from(path)], Vec::new());
        }
        let mut out = Vec::new();
        let mut errors = Vec::new();
        self.walk(path, true, false, 0, scan_budget, &mut out, &mut errors);
        (out, errors)
    }

    pub fn dirs(&self, path: &str) -> Vec<PathBuf> {
        self.dirs_with_errors(path).0
    }

    pub fn dirs_with_errors(&self, path: &str) -> (Vec<PathBuf>, Vec<WalkError>) {
        let mut budget = usize::MAX;
        self.dirs_with_errors_budget(path, &mut budget)
    }

    pub fn dirs_with_errors_budget(
        &self,
        path: &str,
        scan_budget: &mut usize,
    ) -> (Vec<PathBuf>, Vec<WalkError>) {
        if *scan_budget == 0 {
            return (Vec::new(), Vec::new());
        }
        let mut out = Vec::new();
        let mut errors = Vec::new();
        self.walk(path, false, true, 0, scan_budget, &mut out, &mut errors);
        (out, errors)
    }

    fn walk(
        &self,
        top: &str,
        files: bool,
        dirs: bool,
        lvl: i32,
        scan_budget: &mut usize,
        out: &mut Vec<PathBuf>,
        errors: &mut Vec<WalkError>,
    ) {
        if *scan_budget == 0 {
            return;
        }
        let entries = match std::fs::read_dir(top) {
            Ok(e) => e,
            Err(error) => {
                errors.push(WalkError {
                    path: PathBuf::from(top),
                    message: format!("Cannot read directory: {}", error),
                    permission_denied: error.kind() == std::io::ErrorKind::PermissionDenied,
                });
                return;
            }
        };

        let mut dir_entries: Vec<PathBuf> = Vec::new();
        let mut file_entries: Vec<PathBuf> = Vec::new();

        for entry in entries.flatten() {
            if *scan_budget == 0 {
                break;
            }
            *scan_budget -= 1;
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
                    self.walk(
                        &d.to_string_lossy(),
                        files,
                        dirs,
                        lvl + 1,
                        scan_budget,
                        out,
                        errors,
                    );
                    if *scan_budget == 0 {
                        return;
                    }
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
                    self.walk(
                        &d.to_string_lossy(),
                        files,
                        dirs,
                        lvl + 1,
                        scan_budget,
                        out,
                        errors,
                    );
                    if *scan_budget == 0 {
                        return;
                    }
                }
                if files {
                    out.extend(file_entries);
                }
                if dirs && lvl >= self.min_depth {
                    out.extend(dir_entries.iter().cloned());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inaccessible_or_missing_roots_are_reported() {
        let path = std::env::temp_dir().join(format!(
            "shelfy-walker-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let (files, errors) = Walker::default().files_with_errors(&path.to_string_lossy());
        assert!(files.is_empty());
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path, path);
    }

    #[test]
    fn limited_walk_stops_after_requested_candidates() {
        let root = std::env::temp_dir().join(format!(
            "shelfy-walker-limit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        for index in 0..20 {
            std::fs::write(root.join(format!("{index:02}.txt")), b"test").unwrap();
        }

        let mut budget = 5;
        let (files, errors) =
            Walker::default().files_with_errors_budget(&root.to_string_lossy(), &mut budget);
        assert_eq!(files.len(), 5);
        assert_eq!(budget, 0);
        assert!(errors.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }
}
