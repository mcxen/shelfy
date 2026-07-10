use crate::orden::action::{DefaultOutput, Level, Output};
use crate::orden::conflict::{resolve_conflict, ConflictMode};
use crate::orden::resource::Resource;
use crate::orden::target_path::prepare_target_path;
use crate::orden::template;
use crate::orden::Action;

/// Move a file to a new location.
///
/// Mirrors `organize.actions.move.Move`.
pub struct Move {
    pub dest: String,
    pub on_conflict: ConflictMode,
    pub rename_template: String,
    pub autodetect_folder: bool,
}

impl Move {
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

    fn execute(
        &mut self,
        res: &mut Resource,
        simulate: bool,
        output: &dyn Output,
    ) -> Result<(), String> {
        let src = res.path.clone().ok_or("move: no source path")?;
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
            output.msg(
                res,
                &format!("Skipped existing {}", dst.display()),
                "move",
                Level::Warn,
            );
            return Ok(());
        }
        let dst = r.use_dst;
        output.msg(
            res,
            &format!("Move to {}", dst.display()),
            "move",
            Level::Info,
        );
        res.walker_skip_pathes.insert(dst.clone());
        if !simulate {
            if let Err(error) = std::fs::rename(&src, &dst) {
                std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
                std::fs::remove_file(&src).map_err(|e| e.to_string())?;
                output.msg(
                    res,
                    &format!("Atomic move unavailable ({}); used copy + remove", error),
                    "move",
                    Level::Warn,
                );
            }
            crate::orden::actions::log_history("move", &src, &dst, res.rule_name.clone());
        }
        res.path = Some(dst);
        Ok(())
    }
}

impl Action for Move {
    fn name(&self) -> &str {
        "move"
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
