use crate::orden::action::{DefaultOutput, Level, Output};
use crate::orden::resource::Resource;

/// Delete a file or dir from disk (no recovery).
///
/// Mirrors `organize.actions.delete.Delete`.
pub struct Delete;

impl crate::orden::Action for Delete {
    fn name(&self) -> &str {
        "delete"
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource, simulate: bool) -> Result<(), String> {
        let path = res.path.clone().ok_or("delete: no path")?;
        DefaultOutput.msg(
            res,
            &format!("Deleting {}", path.display()),
            "delete",
            Level::Info,
        );
        if !simulate {
            if path.is_dir() {
                std::fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
            } else {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }
        res.path = None;
        Ok(())
    }
}
