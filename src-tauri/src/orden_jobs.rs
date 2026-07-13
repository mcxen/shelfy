use crate::db::{
    get_orden_config, list_orden_jobs, log_scheduler_event, mark_orden_job_run, OrdenJob,
};
use chrono::{DateTime, Duration, Local, NaiveTime, Utc};
use once_cell::sync::Lazy;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Mutex;

static RUNNING_JOBS: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

fn job_key(job: &OrdenJob) -> String {
    job.id
        .map(|id| format!("id:{id}"))
        .unwrap_or_else(|| format!("name:{}", job.name))
}

fn try_mark_running(job: &OrdenJob) -> bool {
    let key = job_key(job);
    let mut running = RUNNING_JOBS.lock().unwrap();
    if running.contains(&key) {
        return false;
    }
    running.insert(key);
    true
}

fn mark_finished(job: &OrdenJob) {
    RUNNING_JOBS.lock().unwrap().remove(&job_key(job));
}

pub fn run_due_jobs(
    trigger: &str,
    now: DateTime<Local>,
    event_path: Option<&std::path::Path>,
) -> Result<(usize, usize, usize), String> {
    // The tuple reports dispatch-time information only. Worker results are
    // persisted to orden_run_logs when each background job finishes.
    let jobs = list_orden_jobs().map_err(|e| e.to_string())?;
    let total_success = 0usize;
    let mut total_errors = 0usize;
    let mut ran_jobs = 0usize;

    for job in jobs {
        if !job.enabled || !job_due(&job, now, event_path) {
            continue;
        }
        if !try_mark_running(&job) {
            continue;
        }
        let worker_job = job.clone();
        let worker_trigger = trigger.to_string();
        match std::thread::Builder::new()
            .name(format!("orden-job-{}", worker_job.name))
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_job_summary(&worker_job, &worker_trigger)
                }));
                if let Ok(Ok(_)) = &result {
                    if let Some(id) = worker_job.id {
                        let _ = mark_orden_job_run(id);
                    }
                } else {
                    let error = match result {
                        Ok(Err(error)) => error,
                        Err(_) => "Orden job worker thread panicked".to_string(),
                        Ok(Ok(_)) => unreachable!(),
                    };
                    let _ = log_scheduler_event(
                        "error",
                        "orden_job_failed",
                        &format!("Orden job '{}' failed", worker_job.name),
                        Some(
                            json!({
                                "trigger": worker_trigger,
                                "job": worker_job.name,
                                "error": error,
                            })
                            .to_string(),
                        ),
                    );
                }
                mark_finished(&worker_job);
            }) {
            Ok(_) => ran_jobs += 1,
            Err(error) => {
                mark_finished(&job);
                total_errors += 1;
                let _ = log_scheduler_event(
                    "error",
                    "orden_job_dispatch_failed",
                    &format!("Orden job '{}' could not be dispatched", job.name),
                    Some(
                        json!({ "trigger": trigger, "job": job.name, "error": error.to_string() })
                            .to_string(),
                    ),
                );
            }
        }
    }

    Ok((total_success, total_errors, ran_jobs))
}

pub fn run_monitor_jobs(event_path: &std::path::Path) -> Result<(usize, usize, usize), String> {
    run_due_jobs("orden-monitor", Local::now(), Some(event_path))
}

pub fn run_job_result(job: &OrdenJob, _trigger: &str) -> Result<crate::orden::RunResult, String> {
    let yaml = get_orden_config(&job.config_name)
        .map_err(|e| e.to_string())?
        .map(|record| record.yaml)
        .ok_or_else(|| format!("Orden config '{}' not found", job.config_name))?;
    if job.min_file_count > 0 && count_existing_watch_files(job) < job.min_file_count {
        return Ok(crate::orden::RunResult {
            success: 0,
            errors: 0,
            simulate: job.simulate,
            logs: Vec::new(),
        });
    }
    let opts = crate::orden::ExecuteOptions {
        simulate: job.simulate,
        tags: split_csv(&job.tags).into_iter().collect(),
        skip_tags: split_csv(&job.skip_tags).into_iter().collect(),
        working_dir: std::env::current_dir().unwrap_or_default(),
        preview: None,
    };
    crate::orden::run_yaml(&yaml, &opts)
}

fn run_job_summary(job: &OrdenJob, trigger: &str) -> Result<(usize, usize), String> {
    let _ = log_scheduler_event(
        "info",
        "orden_job_started",
        &format!("Orden job '{}' started", job.name),
        Some(json!({ "trigger": trigger, "job": job.name, "config": job.config_name, "mode": job.mode }).to_string()),
    );
    let result = match run_job_result(job, trigger) {
        Ok(result) => result,
        Err(error) => {
            let logs = json!([{
                "level": "error",
                "sender": "orden",
                "rule_nr": -1,
                "path": "<config>",
                "msg": error.clone(),
            }]);
            let _ = crate::db::log_orden_run(
                &job.config_name,
                job.simulate,
                0,
                1,
                trigger,
                &logs.to_string(),
            );
            return Err(error);
        }
    };
    let _ = crate::db::log_orden_run(
        &job.config_name,
        job.simulate,
        result.success as i64,
        result.errors as i64,
        trigger,
        &serde_json::to_string(&result.logs).unwrap_or_else(|_| "[]".to_string()),
    );
    let _ = log_scheduler_event(
        if result.errors > 0 { "warn" } else { "info" },
        "orden_job_finished",
        &format!(
            "Orden job '{}' matched {} items with {} errors",
            job.name, result.success, result.errors
        ),
        Some(
            json!({
                "trigger": trigger,
                "job": job.name,
                "config": job.config_name,
                "success": result.success,
                "errors": result.errors,
            })
            .to_string(),
        ),
    );
    Ok((result.success as usize, result.errors as usize))
}

fn job_due(job: &OrdenJob, now: DateTime<Local>, event_path: Option<&std::path::Path>) -> bool {
    if !time_window_matches(job, now) || !path_condition_matches(job) {
        return false;
    }
    match job.mode.as_str() {
        "manual" => false,
        "cron" => job
            .cron_expr
            .as_deref()
            .map(|expr| crate::scheduler::cron_matches(expr, now) && !ran_this_minute(job, now))
            .unwrap_or(false),
        "fixed" => job
            .fixed_time
            .as_deref()
            .and_then(|time| parse_time(time).ok())
            .map(|time| {
                let scheduled = now
                    .date_naive()
                    .and_time(time)
                    .and_local_timezone(Local)
                    .single();
                scheduled
                    .map(|scheduled| {
                        let diff = now.signed_duration_since(scheduled);
                        diff >= Duration::zero()
                            && diff < Duration::minutes(1)
                            && !ran_today(job, now)
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false),
        "interval" => job
            .last_run_at
            .map(|last| {
                Utc::now().signed_duration_since(last)
                    >= Duration::minutes(job.interval_minutes.clamp(1, 10080))
            })
            .unwrap_or(true),
        "monitor" => event_path
            .map(|path| monitor_matches(job, path))
            .unwrap_or(false),
        _ => false,
    }
}

fn ran_this_minute(job: &OrdenJob, now: DateTime<Local>) -> bool {
    job.last_run_at
        .map(|last| {
            last.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
                == now.format("%Y-%m-%d %H:%M").to_string()
        })
        .unwrap_or(false)
}

fn ran_today(job: &OrdenJob, now: DateTime<Local>) -> bool {
    job.last_run_at
        .map(|last| last.with_timezone(&Local).date_naive() == now.date_naive())
        .unwrap_or(false)
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn time_window_matches(job: &OrdenJob, now: DateTime<Local>) -> bool {
    let start = job
        .time_window_start
        .as_deref()
        .and_then(|s| parse_time(s).ok());
    let end = job
        .time_window_end
        .as_deref()
        .and_then(|s| parse_time(s).ok());
    match (start, end) {
        (Some(start), Some(end)) if start <= end => now.time() >= start && now.time() <= end,
        (Some(start), Some(end)) => now.time() >= start || now.time() <= end,
        (Some(start), None) => now.time() >= start,
        (None, Some(end)) => now.time() <= end,
        (None, None) => true,
    }
}

fn path_condition_matches(job: &OrdenJob) -> bool {
    job.path_exists
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|path| std::path::Path::new(&expand_home(path)).exists())
        .unwrap_or(true)
}

fn monitor_matches(job: &OrdenJob, event_path: &std::path::Path) -> bool {
    split_lines(&job.watch_paths)
        .iter()
        .any(|watch| event_path.starts_with(expand_home(watch)))
}

fn count_existing_watch_files(job: &OrdenJob) -> i64 {
    split_lines(&job.watch_paths)
        .iter()
        .filter_map(|path| std::fs::read_dir(expand_home(path)).ok())
        .flat_map(|entries| entries.filter_map(Result::ok))
        .filter(|entry| entry.path().is_file())
        .count() as i64
}

fn split_lines(value: &str) -> Vec<String> {
    value
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn expand_home(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf()) {
            return home.join(rest).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

fn parse_time(time_str: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(time_str.trim(), "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(time_str.trim(), "%H:%M:%S"))
        .map_err(|e| format!("Invalid schedule time '{}': {}", time_str, e))
}
