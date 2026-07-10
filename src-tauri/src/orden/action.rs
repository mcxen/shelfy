use crate::orden::resource::Resource;

/// An action performs an operation on a resource (move, copy, rename, ...).
pub trait Action: Send + Sync {
    fn name(&self) -> &str;

    /// Whether this action supports standalone mode (no file location).
    fn standalone(&self) -> bool {
        false
    }

    /// Whether this action supports files.
    fn supports_files(&self) -> bool {
        true
    }

    /// Whether this action supports directories.
    fn supports_dirs(&self) -> bool {
        false
    }

    /// Run the action. `simulate` is true in simulation mode.
    fn pipeline(&mut self, res: &mut Resource, simulate: bool) -> Result<(), String>;

    /// Run the action with access to the current output sink. Existing actions can
    /// keep implementing `pipeline`; actions that produce rich logs may override this.
    fn pipeline_with_output(
        &mut self,
        res: &mut Resource,
        simulate: bool,
        output: &dyn Output,
    ) -> Result<(), String> {
        let _ = output;
        self.pipeline(res, simulate)
    }
}

/// A simple output sink that collects messages.
pub trait Output: Send + Sync {
    fn msg(&self, res: &Resource, msg: &str, sender: &str, level: Level);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// Default output that prints to stderr.
pub struct DefaultOutput;

impl Output for DefaultOutput {
    fn msg(&self, res: &Resource, msg: &str, sender: &str, level: Level) {
        let level_str = match level {
            Level::Info => "",
            Level::Warn => "WARN ",
            Level::Error => "ERROR",
        };
        let path = res
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "<standalone>".to_string());
        eprintln!(
            "[{}] {} {} {}: {}",
            level_str, sender, res.rule_nr, path, msg
        );
    }
}

/// A single log line captured by `CollectingOutput`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogEntry {
    pub level: String,
    pub sender: String,
    pub rule_nr: i64,
    pub path: String,
    pub msg: String,
}

/// Thread-safe output sink that collects messages for programmatic consumers
/// (e.g. Tauri commands returning simulation results to the GUI).
pub struct CollectingOutput {
    pub logs: std::sync::Mutex<Vec<LogEntry>>,
}

impl CollectingOutput {
    pub fn new() -> Self {
        Self {
            logs: std::sync::Mutex::new(Vec::new()),
        }
    }
    pub fn take(&self) -> Vec<LogEntry> {
        std::mem::take(&mut *self.logs.lock().unwrap())
    }
}

impl Output for CollectingOutput {
    fn msg(&self, res: &Resource, msg: &str, sender: &str, level: Level) {
        let entry = LogEntry {
            level: match level {
                Level::Info => "info".into(),
                Level::Warn => "warn".into(),
                Level::Error => "error".into(),
            },
            sender: sender.into(),
            rule_nr: res.rule_nr,
            path: res
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "<standalone>".into()),
            msg: msg.into(),
        };
        self.logs.lock().unwrap().push(entry);
    }
}
