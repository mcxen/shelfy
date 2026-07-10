use crate::orden::action::{DefaultOutput, Level, Output};
use crate::orden::conflict::{resolve_conflict, ConflictMode};
use crate::orden::resource::Resource;
use crate::orden::target_path::prepare_target_path;
use crate::orden::template;
use crate::orden::Action;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinueWith {
    Copy,
    Original,
}

/// Copy a file or dir to a new location.
///
/// Mirrors `organize.actions.copy.Copy`.
pub struct Copy {
    pub destinations: Vec<String>,
    pub on_conflict: ConflictMode,
    pub rename_template: String,
    pub autodetect_folder: bool,
    pub continue_with: ContinueWith,
}

impl Copy {
    pub fn new(
        destinations: Vec<String>,
        on_conflict: ConflictMode,
        rename_template: String,
        autodetect_folder: bool,
        continue_with: ContinueWith,
    ) -> Self {
        Self {
            destinations,
            on_conflict,
            rename_template,
            autodetect_folder,
            continue_with,
        }
    }
}

impl Action for Copy {
    fn name(&self) -> &str {
        "copy"
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource, simulate: bool) -> Result<(), String> {
        let src = res.path.clone().ok_or("copy: no source path")?;
        if self.destinations.is_empty() {
            return Err("copy: at least one destination is required".into());
        }

        let source_is_dir = src.is_dir();
        let mut last_destination = None;
        for destination in &self.destinations {
            // Render every destination against the original resource. This makes a
            // destination list a fan-out operation instead of copying destination
            // one into destination two.
            res.path = Some(src.clone());
            let rendered = template::render(destination, &res.dict())?;
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
                continue;
            }
            let dst = r.use_dst;

            DefaultOutput.msg(
                res,
                &format!("Copy to {}", dst.display()),
                "copy",
                Level::Info,
            );
            res.walker_skip_pathes.insert(dst.clone());
            if !simulate {
                if source_is_dir {
                    copy_dir(&src, &dst)?;
                } else {
                    std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
                }
            }
            last_destination = Some(dst);
        }

        res.path = match (self.continue_with, last_destination) {
            (ContinueWith::Copy, Some(dst)) => Some(dst),
            _ => Some(src),
        };
        Ok(())
    }
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let p = entry.path();
        let target = dst.join(entry.file_name());
        if p.is_dir() {
            copy_dir(&p, &target)?;
        } else {
            std::fs::copy(&p, &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
