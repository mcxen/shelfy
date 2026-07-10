use crate::orden::action::{DefaultOutput, Level, Output};
use crate::orden::resource::Resource;
use crate::orden::template;
use crate::orden::Action;

/// Prints the given message (supports templates).
///
/// Mirrors `organize.actions.echo.Echo`.
pub struct Echo {
    pub msg: String,
}

impl Echo {
    pub fn new(msg: String) -> Self {
        Self { msg }
    }
}

impl Action for Echo {
    fn name(&self) -> &str {
        "echo"
    }
    fn standalone(&self) -> bool {
        true
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource, _simulate: bool) -> Result<(), String> {
        let dict = res.dict();
        let full = template::render(&self.msg, &dict)?;
        DefaultOutput.msg(res, &full, "echo", Level::Info);
        Ok(())
    }
}
