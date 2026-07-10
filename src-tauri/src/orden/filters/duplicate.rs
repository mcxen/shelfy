use std::collections::HashMap;
use std::path::PathBuf;

use crate::orden::filter::{set_var, Filter, FilterResult};
use crate::orden::filters::hash::{hash_file, hash_first_chunk};
use crate::orden::resource::Resource;
use crate::orden::value::Value;

/// A fast duplicate file finder.
///
/// Mirrors `organize.filters.duplicate.Duplicate`. Three-stage detection:
/// 1. group by file size
/// 2. within same size, hash the first 1024-byte chunk
/// 3. within same chunk hash, hash the full file
///
/// `detect_original_by` decides which of a (known, new) pair is the original:
/// first_seen / last_seen / name / created / lastmodified (prefix `-` to reverse).
pub struct Duplicate {
    pub detect_original_by: DetectMethod,
    pub hash_algorithm: String,
    files_for_size: HashMap<u64, Vec<PathBuf>>,
    files_for_chunk: HashMap<String, Vec<PathBuf>>,
    file_for_hash: HashMap<String, PathBuf>,
    seen_files: std::collections::HashSet<PathBuf>,
    first_chunk_known: std::collections::HashSet<PathBuf>,
    hash_known: std::collections::HashSet<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectMethod {
    FirstSeen,
    LastSeen,
    Name,
    Created,
    LastModified,
}

impl DetectMethod {
    pub fn parse(s: &str) -> Result<(Self, bool), String> {
        let (s, reverse) = if let Some(rest) = s.strip_prefix('-') {
            (rest, true)
        } else {
            (s, false)
        };
        let m = match s {
            "first_seen" => DetectMethod::FirstSeen,
            "last_seen" => DetectMethod::LastSeen,
            "name" => DetectMethod::Name,
            "created" => DetectMethod::Created,
            "lastmodified" => DetectMethod::LastModified,
            other => return Err(format!("Unknown detection method: {}", other)),
        };
        Ok((m, reverse))
    }
}

impl Duplicate {
    pub fn new(detect_original_by: DetectMethod, hash_algorithm: String) -> Self {
        Self {
            detect_original_by,
            hash_algorithm,
            files_for_size: HashMap::new(),
            files_for_chunk: HashMap::new(),
            file_for_hash: HashMap::new(),
            seen_files: Default::default(),
            first_chunk_known: Default::default(),
            hash_known: Default::default(),
        }
    }

    fn detect_original(&self, known: &PathBuf, new: &PathBuf) -> (PathBuf, PathBuf) {
        let (original, duplicate) = match self.detect_original_by {
            DetectMethod::FirstSeen => (known.clone(), new.clone()),
            DetectMethod::LastSeen => (new.clone(), known.clone()),
            DetectMethod::Name => {
                if known.file_name() <= new.file_name() {
                    (known.clone(), new.clone())
                } else {
                    (new.clone(), known.clone())
                }
            }
            DetectMethod::Created | DetectMethod::LastModified => {
                let key = |p: &PathBuf| {
                    let meta = std::fs::metadata(p).ok();
                    let t = match self.detect_original_by {
                        DetectMethod::Created => meta.and_then(|m| m.created().ok()),
                        _ => meta.and_then(|m| m.modified().ok()),
                    };
                    t.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                };
                if key(known) <= key(new) {
                    (known.clone(), new.clone())
                } else {
                    (new.clone(), known.clone())
                }
            }
        };
        (original, duplicate)
    }
}

impl Filter for Duplicate {
    fn name(&self) -> &str {
        "duplicate"
    }
    fn supports_dirs(&self) -> bool {
        false
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        let path = res.path.clone().ok_or("duplicate: no path")?;
        if path.is_symlink() {
            return Ok(FilterResult::NoMatch);
        }
        if self.seen_files.contains(&path) {
            return Ok(FilterResult::NoMatch);
        }
        self.seen_files.insert(path.clone());

        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let same_size = self.files_for_size.entry(file_size).or_default();
        same_size.push(path.clone());
        if same_size.len() == 1 {
            return Ok(FilterResult::NoMatch);
        }

        // ensure chunk hashes for earlier files
        let knowns: Vec<PathBuf> = same_size[..same_size.len() - 1].to_vec();
        for f in knowns {
            if !self.first_chunk_known.contains(&f) {
                let chunk = hash_first_chunk(&f, &self.hash_algorithm)?;
                self.first_chunk_known.insert(f.clone());
                self.files_for_chunk.entry(chunk).or_default().push(f);
            }
        }

        let chunk = hash_first_chunk(&path, &self.hash_algorithm)?;
        let same_chunk = self.files_for_chunk.entry(chunk.clone()).or_default();
        same_chunk.push(path.clone());
        self.first_chunk_known.insert(path.clone());
        if same_chunk.len() == 1 {
            return Ok(FilterResult::NoMatch);
        }

        // ensure full hashes for earlier files with same chunk
        let knowns: Vec<PathBuf> = same_chunk[..same_chunk.len() - 1].to_vec();
        for f in knowns {
            if !self.hash_known.contains(&f) {
                let h = hash_file(&f, &self.hash_algorithm)?;
                self.hash_known.insert(f.clone());
                self.file_for_hash.insert(h, f);
            }
        }

        let h = hash_file(&path, &self.hash_algorithm)?;
        self.hash_known.insert(path.clone());
        if let Some(known) = self.file_for_hash.get(&h).cloned() {
            let (original, duplicate) = self.detect_original(&known, &path);
            if known != original {
                self.file_for_hash.insert(h, original.clone());
            }
            res.path = Some(duplicate.clone());
            let mut m = std::collections::BTreeMap::new();
            m.insert(
                "original".to_string(),
                Value::Str(original.to_string_lossy().to_string()),
            );
            set_var(res, "duplicate", Value::Map(m));
            Ok(FilterResult::Match)
        } else {
            self.file_for_hash.insert(h, path);
            Ok(FilterResult::NoMatch)
        }
    }
}
