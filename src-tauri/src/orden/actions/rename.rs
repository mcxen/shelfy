use crate::orden::action::{DefaultOutput, Level, Output};
use crate::orden::conflict::{resolve_conflict, ConflictMode};
use crate::orden::resource::Resource;
use crate::orden::template;
use crate::orden::Action;

/// Renames a file (same directory, new name, no slashes allowed).
///
/// Mirrors `organize.actions.rename.Rename`.
pub struct Rename {
    pub new_name: String,
    pub on_conflict: ConflictMode,
    pub rename_template: String,
}

impl Rename {
    pub fn new(new_name: String, on_conflict: ConflictMode, rename_template: String) -> Self {
        Self {
            new_name,
            on_conflict,
            rename_template,
        }
    }

    fn execute(
        &mut self,
        res: &mut Resource,
        simulate: bool,
        output: &dyn Output,
    ) -> Result<(), String> {
        let src = res.path.clone().ok_or("rename: no source path")?;
        let new_name = template::render(&self.new_name, &res.dict())?;
        if new_name.contains('/') || new_name.contains('\\') {
            return Err("The new name cannot contain slashes. To move files use `move`.".into());
        }
        let dst = src.with_file_name(new_name);
        let r = resolve_conflict(&dst, res, self.on_conflict, &self.rename_template, simulate)?;
        if r.skip_action {
            output.msg(
                res,
                &format!("Skipped existing {}", dst.display()),
                "rename",
                Level::Warn,
            );
            return Ok(());
        }
        let dst = r.use_dst;
        output.msg(
            res,
            &format!("Rename to {}", dst.display()),
            "rename",
            Level::Info,
        );
        if !simulate {
            std::fs::rename(&src, &dst).map_err(|e| e.to_string())?;
            crate::orden::actions::log_history("rename", &src, &dst, res.rule_name.clone());
        }
        res.path = Some(dst.clone());
        res.walker_skip_pathes.insert(dst);
        Ok(())
    }
}

impl Action for Rename {
    fn name(&self) -> &str {
        "rename"
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource, simulate: bool) -> Result<(), String> {
        self.execute(res, simulate, &DefaultOutput)
    }

    fn pipeline_with_output(
        &mut self,
        res: &mut Resource,
        simulate: bool,
        output: &dyn Output,
    ) -> Result<(), String> {
        self.execute(res, simulate, output)
    }
}
