use std::path::{Path, PathBuf};

use crate::orden::resource::Resource;
use crate::orden::template;

/// Conflict resolution modes (mirrors `organize.actions.common.conflict.ConflictMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictMode {
    Skip,
    Overwrite,
    Trash,
    RenameNew,
    RenameExisting,
    Deduplicate,
}

impl ConflictMode {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "skip" => Ok(Self::Skip),
            "overwrite" => Ok(Self::Overwrite),
            "trash" => Ok(Self::Trash),
            "rename_new" => Ok(Self::RenameNew),
            "rename_existing" => Ok(Self::RenameExisting),
            "deduplicate" => Ok(Self::Deduplicate),
            other => Err(format!("Unknown conflict mode: {}", other)),
        }
    }
    pub fn default_name() -> &'static str {
        "rename_new"
    }
}

pub struct ConflictResult {
    pub skip_action: bool,
    pub use_dst: PathBuf,
}

/// Increment `{counter}` in `template` until the dst path is free.
///
/// Mirrors `organize.actions.common.conflict.next_free_name`.
pub fn next_free_name(
    dst: &Path,
    rename_template: &str,
    _res: &Resource,
) -> Result<PathBuf, String> {
    if !dst.exists() {
        return Ok(dst.to_path_buf());
    }
    let stem = dst
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = dst
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    let mut counter = 2i64;
    let mut prev: Option<PathBuf> = None;
    loop {
        let vars = crate::orden::template::map_from(vec![
            ("name", crate::orden::value::Value::Str(stem.clone())),
            ("extension", crate::orden::value::Value::Str(ext.clone())),
            ("counter", crate::orden::value::Value::Int(counter)),
        ]);
        let rendered = template::render(rename_template, &vars)?;
        let candidate = dst.with_file_name(rendered);
        if !candidate.exists() {
            return Ok(candidate);
        }
        if let Some(p) = &prev {
            if p == &candidate {
                return Err(
                    "Could not find a free filename. Maybe you forgot the {counter} placeholder?"
                        .into(),
                );
            }
        }
        prev = Some(candidate.clone());
        counter += 1;
    }
}

/// Resolve a conflict when `dst` already exists.
pub fn resolve_conflict(
    dst: &Path,
    res: &Resource,
    mode: ConflictMode,
    rename_template: &str,
    simulate: bool,
) -> Result<ConflictResult, String> {
    if !dst.exists() {
        return Ok(ConflictResult {
            skip_action: false,
            use_dst: dst.to_path_buf(),
        });
    }

    let src = res.path.as_ref().ok_or("conflict: no source path")?;
    if src.canonicalize().ok() == dst.canonicalize().ok() {
        return Ok(ConflictResult {
            skip_action: true,
            use_dst: src.clone(),
        });
    }

    Ok(match mode {
        ConflictMode::Skip => ConflictResult {
            skip_action: true,
            use_dst: src.clone(),
        },
        ConflictMode::Overwrite => {
            if !simulate {
                if dst.is_dir() {
                    std::fs::remove_dir_all(dst).map_err(|e| e.to_string())?;
                } else {
                    std::fs::remove_file(dst).map_err(|e| e.to_string())?;
                }
            }
            ConflictResult {
                skip_action: false,
                use_dst: dst.to_path_buf(),
            }
        }
        ConflictMode::Trash => {
            if !simulate {
                trash::delete(dst).map_err(|e| e.to_string())?;
            }
            ConflictResult {
                skip_action: false,
                use_dst: dst.to_path_buf(),
            }
        }
        ConflictMode::RenameNew => {
            let new_path = next_free_name(dst, rename_template, res)?;
            ConflictResult {
                skip_action: false,
                use_dst: new_path,
            }
        }
        ConflictMode::RenameExisting => {
            let new_path = next_free_name(dst, rename_template, res)?;
            if !simulate {
                std::fs::rename(dst, &new_path).map_err(|e| e.to_string())?;
            }
            ConflictResult {
                skip_action: false,
                use_dst: dst.to_path_buf(),
            }
        }
        ConflictMode::Deduplicate => {
            let same = file_content_equal(src, dst)?;
            if same {
                ConflictResult {
                    skip_action: true,
                    use_dst: src.clone(),
                }
            } else {
                let new_path = next_free_name(dst, rename_template, res)?;
                ConflictResult {
                    skip_action: false,
                    use_dst: new_path,
                }
            }
        }
    })
}

/// Compare two files byte-by-byte (like python filecmp.cmp shallow=False).
fn file_content_equal(a: &Path, b: &Path) -> Result<bool, String> {
    let ma = std::fs::metadata(a).map_err(|e| e.to_string())?;
    let mb = std::fs::metadata(b).map_err(|e| e.to_string())?;
    if ma.len() != mb.len() {
        return Ok(false);
    }
    let mut fa = std::fs::File::open(a).map_err(|e| e.to_string())?;
    let mut fb = std::fs::File::open(b).map_err(|e| e.to_string())?;
    let mut ba = [0u8; 65536];
    let mut bb = [0u8; 65536];
    loop {
        use std::io::Read;
        let na = fa.read(&mut ba).map_err(|e| e.to_string())?;
        let nb = fb.read(&mut bb).map_err(|e| e.to_string())?;
        if na != nb {
            return Ok(false);
        }
        if na == 0 {
            return Ok(true);
        }
        if ba[..na] != bb[..na] {
            return Ok(false);
        }
    }
}
