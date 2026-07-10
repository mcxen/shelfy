use crate::orden::action::{DefaultOutput, Level, Output};
use crate::orden::conflict::{resolve_conflict, ConflictMode};
use crate::orden::resource::Resource;
use crate::orden::target_path::prepare_target_path;
use crate::orden::template;
use crate::orden::Action;

/// Create a symbolic link.
///
/// Mirrors `organize.actions.symlink.Symlink`.
pub struct Symlink {
    pub dest: String,
    pub on_conflict: ConflictMode,
    pub rename_template: String,
    pub autodetect_folder: bool,
}

impl Symlink {
    pub fn new(
        dest: String,
        on_conflict: ConflictMode,
        rename_template: String,
        autodetect_folder: bool,
    ) -> Self {
        Self {
            dest,
            on_conflict,
            rename_template,
            autodetect_folder,
        }
    }
}

impl Action for Symlink {
    fn name(&self) -> &str {
        "symlink"
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource, simulate: bool) -> Result<(), String> {
        let src = res.path.clone().ok_or("symlink: no source")?;
        let rendered = template::render(&self.dest, &res.dict())?;
        let dst = prepare_target_path(
            &src.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            &rendered,
            self.autodetect_folder,
            simulate,
        )?;
        let r = resolve_conflict(&dst, res, self.on_conflict, &self.rename_template, simulate)?;
        if r.skip_action {
            return Ok(());
        }
        let dst = r.use_dst;

        DefaultOutput.msg(
            res,
            &format!("Creating symlink at {}", dst.display()),
            "symlink",
            Level::Info,
        );
        res.walker_skip_pathes.insert(dst.clone());
        if !simulate {
            #[cfg(unix)]
            std::os::unix::fs::symlink(&src, &dst).map_err(|e| e.to_string())?;
            #[cfg(windows)]
            {
                if res.is_dir() {
                    std::os::windows::fs::symlink_dir(&src, &dst).map_err(|e| e.to_string())?;
                } else {
                    std::os::windows::fs::symlink_file(&src, &dst).map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(())
    }

    fn pipeline_with_output(
        &mut self,
        res: &mut Resource,
        simulate: bool,
        output: &dyn Output,
    ) -> Result<(), String> {
        let destination = template::render(&self.dest, &res.dict())?;
        let result = self.pipeline(res, simulate);
        if result.is_ok() {
            output.msg(
                res,
                &format!("Create symlink at {}", destination),
                "symlink",
                Level::Info,
            );
        }
        result
    }
}
