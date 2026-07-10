use crate::orden::action::{DefaultOutput, Level, Output};
use crate::orden::resource::Resource;
use crate::orden::Action;

/// Move a file or dir into the trash.
///
/// Mirrors `organize.actions.trash.Trash`. Uses the `trash` crate.
pub struct Trash;

impl Action for Trash {
    fn name(&self) -> &str {
        "trash"
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource, simulate: bool) -> Result<(), String> {
        let path = res.path.clone().ok_or("trash: no path")?;
        DefaultOutput.msg(
            res,
            &format!("Trash \"{}\"", path.display()),
            "trash",
            Level::Info,
        );
        if !simulate {
            trash::delete(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
