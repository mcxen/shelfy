use std::path::{Path, PathBuf};

/// Whether the given destination string means a folder target.
///
/// Mirrors `organize.actions.common.target_path.user_wants_a_folder`.
pub fn user_wants_a_folder(path: &str, autodetect: bool) -> bool {
    if path.ends_with('/') || path.ends_with('\\') {
        return true;
    }
    if autodetect {
        // treat names without a "." in the last segment as folders
        return !Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().contains('.'))
            .unwrap_or(false);
    }
    false
}

/// Fully resolve the destination for folder targets and create the folder structure.
///
/// Mirrors `organize.actions.common.target_path.prepare_target_path`.
pub fn prepare_target_path(
    src_name: &str,
    dst: &str,
    autodetect_folder: bool,
    simulate: bool,
) -> Result<PathBuf, String> {
    let result = PathBuf::from(dst);
    let wants_folder = user_wants_a_folder(dst, autodetect_folder);

    if result.exists() {
        if result.is_dir() {
            return Ok(result.join(src_name));
        } else if wants_folder {
            return Err(format!(
                "Expected \"{}\" to be a folder, but it's not!",
                dst
            ));
        }
    }

    if wants_folder {
        if !simulate {
            std::fs::create_dir_all(&result).map_err(|e| e.to_string())?;
        }
        Ok(result.join(src_name))
    } else {
        if let Some(parent) = result.parent() {
            if !simulate && !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        Ok(result)
    }
}
