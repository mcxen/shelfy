use crate::db::{
    get_settings, get_watched_folders, is_folder_manual_mode, is_folder_paused_mode,
    log_scheduler_event,
};
use crate::rules::manual_scan_folder;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, Timelike};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration as StdDuration;
use tauri::{AppHandle, Emitter};

const CHECK_INTERVAL_SECONDS: u64 = 60;

/// Background scheduler for running Clean Now at configured local times.
pub struct Scheduler {
    /// Last run date (local) for each schedule slot, to avoid double-triggering.
    last_run_dates: Arc<Mutex<HashMap<usize, NaiveDate>>>,
    last_cron_minute: Arc<Mutex<Option<String>>>,
    last_keepalive: Arc<Mutex<Option<DateTime<Local>>>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            last_run_dates: Arc::new(Mutex::new(HashMap::new())),
            last_cron_minute: Arc::new(Mutex::new(None)),
            last_keepalive: Arc::new(Mutex::new(None)),
        }
    }

    /// Spawn the scheduler thread. It wakes every 60 seconds and checks whether
    /// any configured schedule time has been reached in the last minute.
    pub fn start(&self, app_handle: AppHandle) {
        let last_run_dates = self.last_run_dates.clone();
        let last_cron_minute = self.last_cron_minute.clone();
        let last_keepalive = self.last_keepalive.clone();
        thread::spawn(move || loop {
            thread::sleep(StdDuration::from_secs(CHECK_INTERVAL_SECONDS));
            if let Err(e) = check_and_run(
                &app_handle,
                &last_run_dates,
                &last_cron_minute,
                &last_keepalive,
            ) {
                let _ = log_scheduler_event("error", "scheduler_error", &e, None);
                eprintln!("[scheduler] error: {}", e);
            }
        });
    }
}

fn check_and_run(
    app_handle: &AppHandle,
    last_run_dates: &Arc<Mutex<HashMap<usize, NaiveDate>>>,
    last_cron_minute: &Arc<Mutex<Option<String>>>,
    last_keepalive: &Arc<Mutex<Option<DateTime<Local>>>>,
) -> Result<(), String> {
    let settings = get_settings().map_err(|e| e.to_string())?;
    let now = Local::now();

    if settings.keepalive_enabled {
        maybe_emit_keepalive(
            app_handle,
            last_keepalive,
            now,
            settings.keepalive_interval_minutes,
        );
    }

    if settings.schedule_enabled {
        let times_per_day = settings.schedule_times_per_day.clamp(1, 4) as usize;
        let schedule_times: Vec<Option<String>> = vec![
            settings.schedule_time_1,
            settings.schedule_time_2,
            settings.schedule_time_3,
            settings.schedule_time_4,
        ];
        let today = now.date_naive();

        for slot in 0..times_per_day {
            let Some(time_str) = schedule_times.get(slot).and_then(|t| t.as_ref()) else {
                continue;
            };
            let scheduled_time = parse_time(time_str)?;
            let scheduled_dt = today.and_time(scheduled_time);
            let scheduled_local = scheduled_dt
                .and_local_timezone(Local)
                .single()
                .ok_or_else(|| "Invalid local time".to_string())?;
            let diff = now.signed_duration_since(scheduled_local);

            if diff >= Duration::zero() && diff < Duration::minutes(1) {
                let already_run = {
                    let guard = last_run_dates.lock().unwrap();
                    guard.get(&slot).copied() == Some(today)
                };
                if !already_run {
                    perform_scheduled_clean(app_handle, "fixed_time")?;
                    last_run_dates.lock().unwrap().insert(slot, today);
                }
            }
        }
    }

    if settings.schedule_cron_enabled {
        let expr = settings
            .schedule_cron_expr
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_string();
        let cron = CronSchedule::parse(&expr)?;
        if cron.matches(now) {
            let minute_key = now.format("%Y-%m-%d %H:%M").to_string();
            let mut last = last_cron_minute.lock().unwrap();
            if last.as_deref() != Some(minute_key.as_str()) {
                perform_scheduled_clean(app_handle, "cron")?;
                *last = Some(minute_key);
            }
        }
    }

    let _ = crate::orden_jobs::run_due_jobs("orden-scheduler", now, None)?;

    Ok(())
}

fn parse_time(time_str: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(time_str.trim(), "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(time_str.trim(), "%H:%M:%S"))
        .map_err(|e| format!("Invalid schedule time '{}': {}", time_str, e))
}

fn maybe_emit_keepalive(
    app_handle: &AppHandle,
    last_keepalive: &Arc<Mutex<Option<DateTime<Local>>>>,
    now: DateTime<Local>,
    interval_minutes: i64,
) {
    let interval = interval_minutes.clamp(1, 1440);
    let mut last = last_keepalive.lock().unwrap();
    let should_emit = last
        .map(|last_seen| now.signed_duration_since(last_seen) >= Duration::minutes(interval))
        .unwrap_or(true);
    if !should_emit {
        return;
    }
    *last = Some(now);
    let details = json!({
        "platform": std::env::consts::OS,
        "interval_minutes": interval,
        "pid": std::process::id(),
    })
    .to_string();
    let _ = log_scheduler_event(
        "info",
        "keepalive",
        "Scheduler keepalive heartbeat",
        Some(details),
    );
    let _ = app_handle.emit(
        "scheduler-keepalive",
        json!({
            "timestamp": now.to_rfc3339(),
            "interval_minutes": interval,
        }),
    );
}

fn perform_scheduled_clean(app_handle: &AppHandle, trigger: &str) -> Result<(), String> {
    println!("[scheduler] running scheduled clean ({})", trigger);
    let _ = log_scheduler_event(
        "info",
        "clean_started",
        "Scheduled clean started",
        Some(json!({ "trigger": trigger }).to_string()),
    );

    let folders = get_watched_folders().map_err(|e| e.to_string())?;
    let mut total = 0usize;
    let mut errors = 0usize;
    for folder in folders {
        if !folder.enabled
            || is_folder_paused_mode(&folder.mode)
            || is_folder_manual_mode(&folder.mode)
        {
            continue;
        }
        if !std::path::Path::new(&folder.path).exists() {
            continue;
        }
        match manual_scan_folder(&folder.path) {
            Ok(results) => total += results.len(),
            Err(e) => {
                errors += 1;
                let _ = log_scheduler_event(
                    "error",
                    "clean_folder_failed",
                    &format!("Failed to clean {}", folder.path),
                    Some(json!({ "folder": folder.path, "error": e }).to_string()),
                );
                eprintln!("[scheduler] failed to clean {}: {}", folder.path, e);
            }
        }
    }

    let (_orden_dispatch_success, orden_dispatch_errors, orden_configs) =
        crate::orden_jobs::run_due_jobs("scheduled-clean", Local::now(), None)?;

    println!("[scheduler] organized {} files", total);
    let _ = log_scheduler_event(
        if errors > 0 || orden_dispatch_errors > 0 {
            "warn"
        } else {
            "info"
        },
        "clean_finished",
        &format!(
            "Scheduled clean organized {} files; dispatched {} Orden jobs asynchronously",
            total, orden_configs
        ),
        Some(
            json!({
                "trigger": trigger,
                "organized": total,
                "errors": errors,
                "orden_success": 0,
                "orden_errors": orden_dispatch_errors,
                "orden_configs": orden_configs,
                "orden_async": true,
            })
            .to_string(),
        ),
    );

    let _ = app_handle.emit(
        "scheduled-clean-done",
        json!({
            "organized": total,
            "errors": errors + orden_dispatch_errors,
            "trigger": trigger,
            "orden_success": 0,
            "orden_configs": orden_configs,
            "orden_async": true,
        }),
    );

    Ok(())
}

#[derive(Debug, Clone)]
struct CronSchedule {
    minute: CronField,
    hour: CronField,
    day_of_month: CronField,
    month: CronField,
    day_of_week: CronField,
}

impl CronSchedule {
    fn parse(expr: &str) -> Result<Self, String> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err("Cron expression must have 5 fields: minute hour day month weekday".into());
        }
        Ok(Self {
            minute: CronField::parse(parts[0], 0, 59, false)?,
            hour: CronField::parse(parts[1], 0, 23, false)?,
            day_of_month: CronField::parse(parts[2], 1, 31, false)?,
            month: CronField::parse(parts[3], 1, 12, false)?,
            day_of_week: CronField::parse(parts[4], 0, 7, true)?,
        })
    }

    fn matches(&self, now: DateTime<Local>) -> bool {
        let minute_ok = self.minute.matches(now.minute());
        let hour_ok = self.hour.matches(now.hour());
        let month_ok = self.month.matches(now.month());
        let dom_ok = self.day_of_month.matches(now.day());
        let dow_ok = self
            .day_of_week
            .matches(now.weekday().num_days_from_sunday());
        let day_ok = match (self.day_of_month.is_wildcard, self.day_of_week.is_wildcard) {
            (true, true) => true,
            (true, false) => dow_ok,
            (false, true) => dom_ok,
            (false, false) => dom_ok || dow_ok,
        };
        minute_ok && hour_ok && month_ok && day_ok
    }
}

#[derive(Debug, Clone)]
struct CronField {
    is_wildcard: bool,
    allowed: Vec<u32>,
}

impl CronField {
    fn parse(field: &str, min: u32, max: u32, sunday_alias: bool) -> Result<Self, String> {
        if field.trim().is_empty() {
            return Err("Cron field cannot be empty".into());
        }
        let mut allowed = Vec::new();
        let mut is_wildcard = false;
        for part in field.split(',') {
            let part = part.trim();
            if part == "*" {
                is_wildcard = true;
                allowed.extend(min..=max);
                continue;
            }

            let (range_part, step) = if let Some((range, step)) = part.split_once('/') {
                let step = step
                    .parse::<u32>()
                    .map_err(|_| format!("Invalid cron step: {}", step))?;
                if step == 0 {
                    return Err("Cron step cannot be 0".into());
                }
                (range, step)
            } else {
                (part, 1)
            };

            let (start, end) = if range_part == "*" {
                is_wildcard = true;
                (min, max)
            } else if let Some((a, b)) = range_part.split_once('-') {
                (
                    parse_cron_number(a, min, max, sunday_alias)?,
                    parse_cron_number(b, min, max, sunday_alias)?,
                )
            } else {
                let value = parse_cron_number(range_part, min, max, sunday_alias)?;
                (value, value)
            };

            if start > end {
                return Err(format!("Invalid cron range: {}", range_part));
            }
            for value in (start..=end).step_by(step as usize) {
                let normalized = if sunday_alias && value == 7 { 0 } else { value };
                allowed.push(normalized);
            }
        }
        allowed.sort_unstable();
        allowed.dedup();
        Ok(Self {
            is_wildcard,
            allowed,
        })
    }

    fn matches(&self, value: u32) -> bool {
        self.allowed.binary_search(&value).is_ok()
    }
}

fn parse_cron_number(value: &str, min: u32, max: u32, sunday_alias: bool) -> Result<u32, String> {
    let n = value
        .parse::<u32>()
        .map_err(|_| format!("Invalid cron value: {}", value))?;
    if sunday_alias && n == 7 {
        return Ok(7);
    }
    if n < min || n > max {
        return Err(format!("Cron value {} out of range {}-{}", n, min, max));
    }
    Ok(n)
}

pub fn validate_cron_expression(expr: &str) -> Result<(), String> {
    CronSchedule::parse(expr).map(|_| ())
}

pub fn cron_matches(expr: &str, now: DateTime<Local>) -> bool {
    CronSchedule::parse(expr)
        .map(|cron| cron.matches(now))
        .unwrap_or(false)
}

pub fn install_system_keepalive(interval_minutes: i64) -> Result<(), String> {
    let interval = interval_minutes.clamp(1, 1440);
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        let task_name = "ShelfyKeepAlive";
        let task = format!("\"{}\" --autostart", exe.display());
        let output = std::process::Command::new("schtasks")
            .args([
                "/Create",
                "/SC",
                "MINUTE",
                "/MO",
                &interval.to_string(),
                "/TN",
                task_name,
                "/TR",
                &task,
                "/F",
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        let _ = log_scheduler_event(
            "info",
            "keepalive_installed",
            "Windows Task Scheduler keepalive installed",
            Some(json!({ "interval_minutes": interval, "task": task_name }).to_string()),
        );
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
        let launch_agents = std::path::Path::new(&home).join("Library/LaunchAgents");
        std::fs::create_dir_all(&launch_agents).map_err(|e| e.to_string())?;
        let plist_path = launch_agents.join("cc.shelfy.keepalive.plist");
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>cc.shelfy.keepalive</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
    <string>--autostart</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>StartInterval</key>
  <integer>{}</integer>
  <key>StandardOutPath</key>
  <string>{}/Library/Logs/shelfy-keepalive.log</string>
  <key>StandardErrorPath</key>
  <string>{}/Library/Logs/shelfy-keepalive.err.log</string>
</dict>
</plist>
"#,
            xml_escape(&exe.to_string_lossy()),
            interval * 60,
            xml_escape(&home),
            xml_escape(&home)
        );
        std::fs::write(&plist_path, plist).map_err(|e| e.to_string())?;
        let uid = String::from_utf8_lossy(
            &std::process::Command::new("id")
                .arg("-u")
                .output()
                .map_err(|e| e.to_string())?
                .stdout,
        )
        .trim()
        .to_string();
        let _ = std::process::Command::new("launchctl")
            .args([
                "bootout",
                &format!("gui/{}", uid),
                plist_path.to_string_lossy().as_ref(),
            ])
            .output();
        let output = std::process::Command::new("launchctl")
            .args([
                "bootstrap",
                &format!("gui/{}", uid),
                plist_path.to_string_lossy().as_ref(),
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        let _ = log_scheduler_event(
            "info",
            "keepalive_installed",
            "macOS LaunchAgent keepalive installed",
            Some(json!({ "interval_minutes": interval, "plist": plist_path }).to_string()),
        );
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = exe;
        let _ = interval;
        Err("System keepalive install is only supported on Windows and macOS".into())
    }
}

pub fn uninstall_system_keepalive() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("schtasks")
            .args(["/Delete", "/TN", "ShelfyKeepAlive", "/F"])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        let _ = log_scheduler_event(
            "info",
            "keepalive_uninstalled",
            "Windows Task Scheduler keepalive removed",
            None,
        );
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
        let plist_path = std::path::Path::new(&home)
            .join("Library/LaunchAgents")
            .join("cc.shelfy.keepalive.plist");
        if plist_path.exists() {
            let uid = String::from_utf8_lossy(
                &std::process::Command::new("id")
                    .arg("-u")
                    .output()
                    .map_err(|e| e.to_string())?
                    .stdout,
            )
            .trim()
            .to_string();
            let _ = std::process::Command::new("launchctl")
                .args([
                    "bootout",
                    &format!("gui/{}", uid),
                    plist_path.to_string_lossy().as_ref(),
                ])
                .output();
            std::fs::remove_file(&plist_path).map_err(|e| e.to_string())?;
        }
        let _ = log_scheduler_event(
            "info",
            "keepalive_uninstalled",
            "macOS LaunchAgent keepalive removed",
            None,
        );
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err("System keepalive uninstall is only supported on Windows and macOS".into())
    }
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn cron_accepts_common_forms() {
        assert!(validate_cron_expression("* * * * *").is_ok());
        assert!(validate_cron_expression("*/15 8-18/2 * 1,6,12 1-5").is_ok());
        assert!(validate_cron_expression("0 0 * * 7").is_ok());
        assert!(validate_cron_expression("60 * * * *").is_err());
        assert!(validate_cron_expression("* * * *").is_err());
    }

    #[test]
    fn cron_matches_sunday_aliases() {
        let sunday = Local
            .with_ymd_and_hms(2026, 7, 5, 10, 30, 0)
            .single()
            .unwrap();

        assert!(CronSchedule::parse("30 10 * * 0").unwrap().matches(sunday));
        assert!(CronSchedule::parse("30 10 * * 7").unwrap().matches(sunday));
    }

    #[test]
    fn cron_uses_dom_dow_or_semantics() {
        let july_first = Local
            .with_ymd_and_hms(2026, 7, 1, 9, 0, 0)
            .single()
            .unwrap();
        let july_seventh = Local
            .with_ymd_and_hms(2026, 7, 7, 9, 0, 0)
            .single()
            .unwrap();

        assert!(CronSchedule::parse("0 9 1 * 2")
            .unwrap()
            .matches(july_first));
        assert!(CronSchedule::parse("0 9 1 * 2")
            .unwrap()
            .matches(july_seventh));
        assert!(!CronSchedule::parse("0 9 1 * 3")
            .unwrap()
            .matches(july_seventh));
    }
}
