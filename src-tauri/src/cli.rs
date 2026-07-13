use crate::db::{
    add_rule_record, add_watched_folder, delete_all_rules, delete_rule, get_config_snapshot,
    get_rules, get_watched_folders, import_config_snapshot, is_valid_folder_mode,
    remove_watched_folder, update_folder_mode, ConfigSnapshot, Rule, FOLDER_MODE_SILENT,
};
use crate::rules::manual_scan_folder;
use directories::ProjectDirs;
use serde::Serialize;
use std::env;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
const CLI_LAUNCHER_MARKER: &str = "# Managed by Shelfy CLI launcher";
#[cfg(target_os = "macos")]
const CLI_PATH_LINE: &str = "export PATH=\"$HOME/.local/bin:$PATH\"";

#[derive(Serialize)]
struct ConfigPaths {
    data_dir: String,
    database: String,
}

pub fn try_run_from_env() -> bool {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if let Some(mcp_index) = args.iter().position(|arg| arg == "--mcp") {
        if args[mcp_index + 1..].iter().any(|arg| is_help_arg(arg)) {
            println!("{}", crate::mcp::help_text_from_env());
            return true;
        }
        if let Err(error) = init_cli_storage().and_then(|_| crate::mcp::run_stdio()) {
            eprintln!("{error}");
            std::process::exit(2);
        }
        return true;
    }

    let Some(cli_index) = args.iter().position(|arg| arg == "--cli") else {
        return false;
    };
    args.drain(0..=cli_index);

    if let Err(error) = run(args) {
        eprintln!("{error}");
        std::process::exit(2);
    }

    true
}

#[cfg(target_os = "macos")]
pub fn ensure_cli_launcher() -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let home = directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .ok_or_else(|| "Unable to resolve home directory".to_string())?;
    let bin_dir = home.join(".local/bin");
    std::fs::create_dir_all(&bin_dir).map_err(|error| error.to_string())?;

    let launcher = bin_dir.join("shelfy");
    if launcher.exists() {
        let existing = std::fs::read_to_string(&launcher).unwrap_or_default();
        if !existing
            .lines()
            .take(3)
            .any(|line| line == CLI_LAUNCHER_MARKER)
        {
            return Err(format!(
                "{} already exists and is not managed by Shelfy",
                launcher.display()
            ));
        }
    }

    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let script = format!(
        "#!/bin/sh\n{CLI_LAUNCHER_MARKER}\nexec {} --cli \"$@\"\n",
        shell_single_quote(&executable.to_string_lossy())
    );
    std::fs::write(&launcher, script).map_err(|error| error.to_string())?;
    let mut permissions = std::fs::metadata(&launcher)
        .map_err(|error| error.to_string())?
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&launcher, permissions).map_err(|error| error.to_string())?;

    ensure_shell_path(&home.join(".zprofile"))?;
    ensure_shell_path(&home.join(".zshrc"))?;
    ensure_shell_path(&home.join(".bash_profile"))?;
    ensure_shell_path(&home.join(".bashrc"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn ensure_shell_path(profile: &std::path::Path) -> Result<(), String> {
    let existing = std::fs::read_to_string(profile).unwrap_or_default();
    if existing.contains("$HOME/.local/bin") || existing.contains("${HOME}/.local/bin") {
        return Ok(());
    }
    let separator = if existing.is_empty() || existing.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    let addition = format!("{separator}\n{CLI_LAUNCHER_MARKER}\n{CLI_PATH_LINE}\n");
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(profile)
        .map_err(|error| error.to_string())?;
    file.write_all(addition.as_bytes())
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn run(args: Vec<String>) -> Result<(), String> {
    let data_dir = init_cli_storage()?;

    match args.first().map(String::as_str) {
        Some("scan") => {
            let folder = arg(&args, 1, "scan <folder>")?;
            print_json(&manual_scan_folder(folder)?)?;
        }
        Some("rules") => run_rules(&args[1..])?,
        Some("folders") => run_folders(&args[1..])?,
        Some("config") => run_config(&args[1..], data_dir)?,
        Some("orden") | Some("organize") => run_orden(&args[1..])?,
        Some("mcp") => {
            if args.get(1).is_some_and(|arg| is_help_arg(arg)) {
                println!("{}", crate::mcp::help_text_from_env());
            } else {
                crate::mcp::run_stdio()?;
            }
        }
        Some("help") | Some("--help") | Some("-h") | None => print_usage(),
        Some(command) => return Err(format!("Unknown CLI command: {command}\n\n{}", usage())),
    }

    Ok(())
}

fn run_rules(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("list") => print_json(&get_rules().map_err(|e| e.to_string())?)?,
        Some("export") => {
            let path = arg(args, 1, "rules export <path>")?;
            let rules = get_rules().map_err(|e| e.to_string())?;
            write_json(path, &rules)?;
            println!("{{\"ok\":true,\"path\":\"{}\"}}", escape_json(path));
        }
        Some("import") => {
            let path = arg(args, 1, "rules import <path> [--replace]")?;
            let replace = args.iter().any(|arg| arg == "--replace");
            let count = import_rules(path, replace)?;
            println!("{{\"ok\":true,\"imported\":{count}}}");
        }
        Some("delete") => {
            let id = parse_i64(arg(args, 1, "rules delete <id>")?)?;
            delete_rule(id).map_err(|e| e.to_string())?;
            println!("{{\"ok\":true}}");
        }
        _ => return Err(format!("Invalid rules command\n\n{}", usage())),
    }

    Ok(())
}

fn import_rules(path: &str, replace: bool) -> Result<usize, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut rules: Vec<Rule> = serde_json::from_str(&data).map_err(|e| e.to_string())?;

    if replace {
        delete_all_rules().map_err(|e| e.to_string())?;
    }

    let count = rules.len();
    for rule in &mut rules {
        rule.id = None;
        add_rule_record(rule, false).map_err(|e| e.to_string())?;
    }
    Ok(count)
}

fn run_folders(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("list") => print_json(&get_watched_folders().map_err(|e| e.to_string())?)?,
        Some("add") => {
            let path = arg(args, 1, "folders add <path> [mode]")?;
            let mode = args
                .get(2)
                .map(String::as_str)
                .unwrap_or(FOLDER_MODE_SILENT);
            if !is_valid_folder_mode(mode) {
                return Err(format!("Invalid folder mode: {mode}"));
            }
            let id = add_watched_folder(path, mode).map_err(|e| e.to_string())?;
            println!("{{\"ok\":true,\"id\":{id}}}");
        }
        Some("remove") => {
            let id = parse_i64(arg(args, 1, "folders remove <id>")?)?;
            remove_watched_folder(id).map_err(|e| e.to_string())?;
            println!("{{\"ok\":true}}");
        }
        Some("mode") => {
            let id = parse_i64(arg(args, 1, "folders mode <id> <mode>")?)?;
            let mode = arg(args, 2, "folders mode <id> <mode>")?;
            if !is_valid_folder_mode(mode) {
                return Err(format!("Invalid folder mode: {mode}"));
            }
            update_folder_mode(id, mode).map_err(|e| e.to_string())?;
            println!("{{\"ok\":true}}");
        }
        _ => return Err(format!("Invalid folders command\n\n{}", usage())),
    }

    Ok(())
}

fn run_orden(args: &[String]) -> Result<(), String> {
    let cmd = args.first().map(String::as_str);
    let simulate = match cmd {
        Some("sim") => true,
        Some("run") => false,
        Some("check") => {
            let path = arg(args, 1, "orden check <config>")?;
            let yaml = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let _cfg = crate::orden::Config::from_string(&yaml)?;
            println!("{{\"ok\":true}}");
            return Ok(());
        }
        _ => return Err(format!("Invalid orden command\n\n{}", usage())),
    };

    let path = arg(args, 1, "orden sim|run <config>")?;
    let yaml = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let cfg = crate::orden::Config::from_string(&yaml)?;

    let mut tags = std::collections::HashSet::new();
    let mut skip_tags = std::collections::HashSet::new();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--tags" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    for t in v.split(',') {
                        tags.insert(t.to_string());
                    }
                }
            }
            "--skip-tags" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    for t in v.split(',') {
                        skip_tags.insert(t.to_string());
                    }
                }
            }
            "--working-dir" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    let _ = std::env::set_current_dir(v);
                }
            }
            _ => {}
        }
        i += 1;
    }

    let opts = crate::orden::ExecuteOptions {
        simulate,
        tags,
        skip_tags,
        working_dir: std::env::current_dir().unwrap_or_default(),
        preview: None,
    };
    let summary = std::thread::Builder::new()
        .name("orden-cli".to_string())
        .spawn(move || cfg.execute(&opts, &crate::orden::action::DefaultOutput))
        .map_err(|error| format!("Failed to start Orden worker: {error}"))?
        .join()
        .map_err(|_| "Orden worker thread panicked".to_string())?;
    println!(
        "{{\"success\":{},\"errors\":{},\"simulate\":{}}}",
        summary.success, summary.errors, simulate
    );
    Ok(())
}

fn run_config(args: &[String], data_dir: PathBuf) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("path") => {
            let paths = ConfigPaths {
                data_dir: data_dir.to_string_lossy().to_string(),
                database: data_dir.join("shelfy.db").to_string_lossy().to_string(),
            };
            print_json(&paths)?;
        }
        Some("export") => {
            let path = arg(args, 1, "config export <path>")?;
            write_json(path, &get_config_snapshot().map_err(|e| e.to_string())?)?;
            println!("{{\"ok\":true,\"path\":\"{}\"}}", escape_json(path));
        }
        Some("import") => {
            let path = arg(args, 1, "config import <path> [--replace]")?;
            let replace = args.iter().any(|arg| arg == "--replace");
            let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let snapshot: ConfigSnapshot =
                serde_json::from_str(&data).map_err(|e| e.to_string())?;
            import_config_snapshot(&snapshot, replace).map_err(|e| e.to_string())?;
            println!("{{\"ok\":true}}");
        }
        Some("reset-rules") => {
            delete_all_rules().map_err(|e| e.to_string())?;
            println!("{{\"ok\":true}}");
        }
        _ => return Err(format!("Invalid config command\n\n{}", usage())),
    }

    Ok(())
}

fn init_cli_storage() -> Result<PathBuf, String> {
    let proj_dirs = ProjectDirs::from("cc", "shelfy", "shelfy")
        .ok_or_else(|| "Unable to resolve Shelfy data directory".to_string())?;
    let data_dir = proj_dirs.data_dir().to_path_buf();
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    crate::db::init_db(data_dir.clone()).map_err(|e| e.to_string())?;

    if let Ok(settings) = crate::db::get_settings() {
        if settings.first_run {
            let downloads = crate::commands::get_downloads_folder();
            let _ = crate::db::add_watched_folder(&downloads, FOLDER_MODE_SILENT);
            let _ = crate::db::insert_default_rules(&downloads);
            let mut initialized = settings;
            initialized.first_run = false;
            let _ = crate::db::update_settings(&initialized);
        }
    }

    Ok(data_dir)
}

fn arg<'a>(args: &'a [String], index: usize, hint: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("Missing argument: {hint}"))
}

fn parse_i64(value: &str) -> Result<i64, String> {
    value
        .parse::<i64>()
        .map_err(|_| format!("Invalid number: {value}"))
}

fn is_help_arg(value: &str) -> bool {
    value == "--help" || value == "-h" || value == "help"
}

fn print_json<T: Serialize>(value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    println!("{json}");
    Ok(())
}

fn write_json<T: Serialize>(path: &str, value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn print_usage() {
    println!("{}", usage());
}

fn usage() -> &'static str {
    "Shelfy local CLI

Usage:
  shelfy scan <folder>
  shelfy rules list
  shelfy rules export <path>
  shelfy rules import <path> [--replace]
  shelfy rules delete <id>
  shelfy folders list
  shelfy folders add <path> [silent|manual|paused]
  shelfy folders remove <id>
  shelfy folders mode <id> <silent|manual|paused>
  shelfy config path
  shelfy config export <path>
  shelfy config import <path> [--replace]
  shelfy config reset-rules
  shelfy orden sim <config> [--tags t1,t2] [--skip-tags t3] [--working-dir <dir>]
  shelfy orden run <config> [--tags t1,t2] [--skip-tags t3] [--working-dir <dir>]
  shelfy orden check <config>
  shelfy --mcp
  shelfy --mcp --help
  shelfy mcp [--help]

When invoking the Shelfy.app executable directly instead of the installed
terminal launcher, insert --cli before the command."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_mcp_help_aliases() {
        assert!(is_help_arg("--help"));
        assert!(is_help_arg("-h"));
        assert!(is_help_arg("help"));
        assert!(!is_help_arg("--mcp"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn quotes_cli_executable_paths_for_shell() {
        assert_eq!(
            shell_single_quote("/Applications/Shelfy.app/bin"),
            "'/Applications/Shelfy.app/bin'"
        );
        assert_eq!(
            shell_single_quote("/tmp/user's app"),
            "'/tmp/user'\\''s app'"
        );
    }
}
