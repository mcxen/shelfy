use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write as IoWrite;

use crate::orden::action::{DefaultOutput, Level, Output};
use crate::orden::resource::Resource;
use crate::orden::template;
use crate::orden::Action;

/// Write text to a file (append / prepend / overwrite).
///
/// Mirrors `organize.actions.write.Write`.
pub struct Write {
    pub text: String,
    pub outfile: String,
    pub mode: WriteMode,
    pub encoding: String,
    pub newline: bool,
    pub clear_before_first_write: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    Append,
    Prepend,
    Overwrite,
}

impl Write {
    pub fn new(
        text: String,
        outfile: String,
        mode: WriteMode,
        encoding: String,
        newline: bool,
        clear_before_first_write: bool,
    ) -> Self {
        Self {
            text,
            outfile,
            mode,
            encoding,
            newline,
            clear_before_first_write,
        }
    }
}

impl Action for Write {
    fn name(&self) -> &str {
        "write"
    }
    fn standalone(&self) -> bool {
        true
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource, simulate: bool) -> Result<(), String> {
        let mut text = template::render(&self.text, &res.dict())?;
        let path = std::path::PathBuf::from(template::render(&self.outfile, &res.dict())?);

        if self.newline {
            text.push('\n');
        }

        let verb = match self.mode {
            WriteMode::Append => "append",
            WriteMode::Prepend => "prepend",
            WriteMode::Overwrite => "overwrite",
        };
        DefaultOutput.msg(
            res,
            &format!("{}: {} \"{}\"", path.display(), verb, text.trim_end()),
            "write",
            Level::Info,
        );

        if simulate {
            return Ok(());
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // only support utf-8 in Rust port
        let _ = self.encoding;

        match self.mode {
            WriteMode::Append => {
                let mut f = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .map_err(|e| e.to_string())?;
                IoWrite::write_all(&mut f, text.as_bytes()).map_err(|e| e.to_string())?;
                let _ = IoWrite::flush(&mut f);
            }
            WriteMode::Prepend => {
                let mut content = String::new();
                if path.exists() {
                    let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
                    f.read_to_string(&mut content).map_err(|e| e.to_string())?;
                }
                std::fs::write(&path, format!("{}{}", text, content)).map_err(|e| e.to_string())?;
            }
            WriteMode::Overwrite => {
                if self.clear_before_first_write && path.exists() {
                    std::fs::write(&path, "").map_err(|e| e.to_string())?;
                }
                std::fs::write(&path, &text).map_err(|e| e.to_string())?;
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
        let path = template::render(&self.outfile, &res.dict())?;
        let result = self.pipeline(res, simulate);
        if result.is_ok() {
            output.msg(res, &format!("Write to {}", path), "write", Level::Info);
        }
        result
    }
}
