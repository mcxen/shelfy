use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct OrdenTaskHandle {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrdenTaskStatus {
    pub task_id: String,
    pub state: String,
    pub result: Option<Value>,
    pub error: Option<String>,
}

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);
static TASKS: Lazy<Mutex<HashMap<String, OrdenTaskStatus>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn next_task_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let sequence = NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed);
    format!("orden-{millis}-{sequence}")
}

fn update_task(task_id: &str, update: impl FnOnce(&mut OrdenTaskStatus)) {
    if let Some(task) = TASKS.lock().unwrap().get_mut(task_id) {
        update(task);
    }
}

fn prune_tasks(tasks: &mut HashMap<String, OrdenTaskStatus>) {
    if tasks.len() <= 256 {
        return;
    }
    let completed: Vec<String> = tasks
        .iter()
        .filter(|(_, task)| task.state == "completed" || task.state == "failed")
        .map(|(id, _)| id.clone())
        .take(tasks.len().saturating_sub(256))
        .collect();
    for id in completed {
        tasks.remove(&id);
    }
}

/// Dispatch a blocking Orden execution to a dedicated worker thread.
/// The returned handle can be queried through `orden_task_status_cmd`.
pub fn spawn<F>(work: F) -> OrdenTaskHandle
where
    F: FnOnce() -> Result<Value, String> + Send + 'static,
{
    let task_id = next_task_id();
    {
        let mut tasks = TASKS.lock().unwrap();
        tasks.insert(
            task_id.clone(),
            OrdenTaskStatus {
                task_id: task_id.clone(),
                state: "queued".to_string(),
                result: None,
                error: None,
            },
        );
        prune_tasks(&mut tasks);
    }

    let worker_id = task_id.clone();
    let worker_result = thread::Builder::new()
        .name(format!("orden-{worker_id}"))
        .spawn(move || {
            update_task(&worker_id, |task| task.state = "running".to_string());
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work));
            match outcome {
                Ok(Ok(result)) => update_task(&worker_id, |task| {
                    task.state = "completed".to_string();
                    task.result = Some(result);
                }),
                Ok(Err(error)) => update_task(&worker_id, |task| {
                    task.state = "failed".to_string();
                    task.error = Some(error);
                }),
                Err(_) => update_task(&worker_id, |task| {
                    task.state = "failed".to_string();
                    task.error = Some("Orden worker thread panicked".to_string());
                }),
            }
        });
    if let Err(error) = worker_result {
        update_task(&task_id, |task| {
            task.state = "failed".to_string();
            task.error = Some(format!("Failed to spawn Orden worker: {error}"));
        });
    }

    OrdenTaskHandle { task_id }
}

pub fn status(task_id: &str) -> Result<OrdenTaskStatus, String> {
    TASKS
        .lock()
        .unwrap()
        .get(task_id)
        .cloned()
        .ok_or_else(|| format!("Orden task '{}' was not found", task_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn spawn_returns_before_worker_finishes() {
        let handle = spawn(|| {
            thread::sleep(Duration::from_millis(40));
            Ok(serde_json::json!({ "success": 1 }))
        });

        assert_ne!(status(&handle.task_id).unwrap().state, "completed");
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let task = status(&handle.task_id).unwrap();
            if task.state == "completed" {
                assert_eq!(task.result.unwrap()["success"], 1);
                break;
            }
            assert!(Instant::now() < deadline, "worker did not finish in time");
            thread::sleep(Duration::from_millis(5));
        }
    }
}
