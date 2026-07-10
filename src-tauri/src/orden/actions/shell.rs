use crate::orden::action::{DefaultOutput, Level, Output};
use crate::orden::resource::Resource;
use crate::orden::template;
use crate::orden::template::map_from;
use crate::orden::value::Value;
use crate::orden::Action;

/// Executes a shell command.
///
/// Mirrors `organize.actions.shell.Shell`. Returns `{shell.output}` / `{shell.returncode}`.
pub struct Shell {
    pub cmd: String,
    pub run_in_simulation: bool,
    pub ignore_errors: bool,
    pub simulation_output: String,
    pub simulation_returncode: i64,
}

impl Shell {
    pub fn new(
        cmd: String,
        run_in_simulation: bool,
        ignore_errors: bool,
        simulation_output: String,
        simulation_returncode: i64,
    ) -> Self {
        Self {
            cmd,
            run_in_simulation,
            ignore_errors,
            simulation_output,
            simulation_returncode,
        }
    }
}

impl Action for Shell {
    fn name(&self) -> &str {
        "shell"
    }
    fn standalone(&self) -> bool {
        true
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource, simulate: bool) -> Result<(), String> {
        let full_cmd = template::render(&self.cmd, &res.dict())?;

        if !simulate || self.run_in_simulation {
            DefaultOutput.msg(res, &format!("$ {}", full_cmd), "shell", Level::Info);
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&full_cmd)
                .output();
            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let rc = out.status.code().unwrap_or(-1) as i64;
                    if !out.status.success() && !self.ignore_errors {
                        return Err(format!(
                            "shell command failed (rc={}): {}",
                            rc,
                            String::from_utf8_lossy(&out.stderr)
                        ));
                    }
                    let m = map_from(vec![
                        ("output", Value::Str(stdout)),
                        ("returncode", Value::Int(rc)),
                    ]);
                    res.vars.deep_merge_into("shell", m);
                }
                Err(e) => {
                    if !self.ignore_errors {
                        return Err(format!("shell command failed: {}", e));
                    }
                    let m = map_from(vec![
                        ("output", Value::Str(String::new())),
                        ("returncode", Value::Int(-1)),
                    ]);
                    res.vars.deep_merge_into("shell", m);
                }
            }
        } else {
            DefaultOutput.msg(
                res,
                &format!("** not run in simulation ** $ {}", full_cmd),
                "shell",
                Level::Info,
            );
            let sim_out = template::render(&self.simulation_output, &res.dict())?;
            let m = map_from(vec![
                ("output", Value::Str(sim_out)),
                ("returncode", Value::Int(self.simulation_returncode)),
            ]);
            res.vars.deep_merge_into("shell", m);
        }
        Ok(())
    }
}
