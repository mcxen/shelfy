use crate::db::{
    get_orden_config, get_orden_run_logs, get_recent_logs, get_recent_orden_run_logs, get_rules,
    get_settings, get_watched_folders, list_orden_configs, list_orden_jobs, log_orden_run,
    upsert_orden_config, OrdenRunLog,
};
use crate::rules::manual_scan_folder;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

#[derive(Debug, Deserialize)]
struct JsonRpcMessage {
    id: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
}

pub fn run_stdio() -> Result<(), String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }

        let message: JsonRpcMessage = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(error) => {
                write_response(
                    &mut stdout,
                    None,
                    None,
                    Some(json!({
                        "code": -32700,
                        "message": error.to_string(),
                    })),
                )?;
                continue;
            }
        };

        let Some(id) = message.id.clone() else {
            continue;
        };
        let Some(method) = message.method.as_deref() else {
            write_response(
                &mut stdout,
                Some(id),
                None,
                Some(json!({"code": -32600, "message": "Missing method"})),
            )?;
            continue;
        };

        match handle_method(method, message.params) {
            Ok(result) => write_response(&mut stdout, Some(id), Some(result), None)?,
            Err(error) => write_response(
                &mut stdout,
                Some(id),
                None,
                Some(json!({"code": -32000, "message": error})),
            )?,
        }
    }

    Ok(())
}

fn handle_method(method: &str, params: Option<Value>) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {
                    "subscribe": false,
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": "shelfy",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "tools/list" => Ok(json!({ "tools": tools()? })),
        "tools/call" => call_tool(params.unwrap_or(Value::Null)),
        "resources/list" => Ok(json!({ "resources": resources()? })),
        "resources/read" => read_resource(params.unwrap_or(Value::Null)),
        _ => Err(format!("Unsupported MCP method: {}", method)),
    }
}

fn tools() -> Result<Value, String> {
    let settings = get_settings().map_err(|e| e.to_string())?;
    if !settings.mcp_enabled {
        return Ok(Value::Array(vec![]));
    }

    let mut tools = vec![
        json!({
            "name": "shelfy_list_folders",
            "description": "List Shelfy watched folders and modes.",
            "inputSchema": object_schema(vec![]),
        }),
        json!({
            "name": "shelfy_list_rules",
            "description": "List Shelfy simple organizer rules.",
            "inputSchema": object_schema(vec![]),
        }),
        json!({
            "name": "shelfy_recent_logs",
            "description": "Read recent Shelfy action history.",
            "inputSchema": object_schema(vec![("limit", json!({"type": "number"}))]),
        }),
        json!({
            "name": "shelfy_list_orden_configs",
            "description": "List saved Orden configurations with timestamps and MCP resource URIs.",
            "inputSchema": object_schema(vec![]),
        }),
        json!({
            "name": "shelfy_get_orden_config",
            "description": "Read a saved Orden configuration by name, including its YAML.",
            "inputSchema": object_schema(vec![("name", json!({"type": "string"}))]),
        }),
        json!({
            "name": "shelfy_list_orden_jobs",
            "description": "List configured Orden automation jobs and their current status.",
            "inputSchema": object_schema(vec![]),
        }),
        json!({
            "name": "shelfy_orden_history",
            "description": "Read detailed Orden run history. Omit config_name to read all configurations.",
            "inputSchema": object_schema(vec![
                ("config_name", json!({"type": "string"})),
                ("limit", json!({"type": "number"})),
            ]),
        }),
        json!({
            "name": "shelfy_orden_simulate",
            "description": "Simulate a saved Orden config by config_name or ad-hoc YAML and return structured logs.",
            "inputSchema": object_schema(vec![
                ("config_name", json!({"type": "string"})),
                ("yaml", json!({"type": "string"})),
                ("tags", json!({"type": "array", "items": {"type": "string"}})),
                ("skip_tags", json!({"type": "array", "items": {"type": "string"}})),
            ]),
        }),
    ];

    if settings.mcp_allow_write {
        tools.push(json!({
            "name": "shelfy_scan_folder",
            "description": "Run Shelfy's organizer on a watched folder. This may move files.",
            "inputSchema": object_schema(vec![("path", json!({"type": "string"}))]),
        }));
        tools.push(json!({
            "name": "shelfy_orden_run",
            "description": "Run a saved Orden config by config_name or ad-hoc YAML. This may modify files.",
            "inputSchema": object_schema(vec![
                ("config_name", json!({"type": "string"})),
                ("yaml", json!({"type": "string"})),
                ("tags", json!({"type": "array", "items": {"type": "string"}})),
                ("skip_tags", json!({"type": "array", "items": {"type": "string"}})),
            ]),
        }));
    }

    Ok(Value::Array(tools))
}

fn object_schema(properties: Vec<(&str, Value)>) -> Value {
    let mut map = serde_json::Map::new();
    for (key, value) in properties {
        map.insert(key.to_string(), value);
    }
    json!({
        "type": "object",
        "properties": map
    })
}

fn call_tool(params: Value) -> Result<Value, String> {
    let settings = get_settings().map_err(|e| e.to_string())?;
    if !settings.mcp_enabled {
        return tool_text(json!({"error": "Shelfy MCP is disabled"}), true);
    }

    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing tool name".to_string())?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match name {
        "shelfy_list_folders" => tool_text(
            json!(get_watched_folders().map_err(|e| e.to_string())?),
            false,
        ),
        "shelfy_list_rules" => tool_text(json!(get_rules().map_err(|e| e.to_string())?), false),
        "shelfy_recent_logs" => {
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(20);
            tool_text(
                json!(get_recent_logs(limit.clamp(1, 100)).map_err(|e| e.to_string())?),
                false,
            )
        }
        "shelfy_list_orden_configs" => {
            sync_orden_configs_from_disk();
            let configs = list_orden_configs().map_err(|e| e.to_string())?;
            let values = configs
                .into_iter()
                .map(|config| {
                    let resource_uri = config_resource_uri(&config.name);
                    json!({
                        "id": config.id,
                        "name": config.name,
                        "created_at": config.created_at,
                        "updated_at": config.updated_at,
                        "resource_uri": resource_uri,
                    })
                })
                .collect::<Vec<_>>();
            tool_text(Value::Array(values), false)
        }
        "shelfy_get_orden_config" => {
            sync_orden_configs_from_disk();
            let name = required_string(&args, "name")?;
            let config = get_orden_config(name)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Orden config '{}' not found", name))?;
            tool_text(json!(config), false)
        }
        "shelfy_list_orden_jobs" => {
            tool_text(json!(list_orden_jobs().map_err(|e| e.to_string())?), false)
        }
        "shelfy_orden_history" => {
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(50);
            let logs = if let Some(name) = args.get("config_name").and_then(|v| v.as_str()) {
                get_orden_run_logs(name, limit.clamp(1, 200)).map_err(|e| e.to_string())?
            } else {
                get_recent_orden_run_logs(limit.clamp(1, 200)).map_err(|e| e.to_string())?
            };
            tool_text(
                Value::Array(logs.into_iter().map(run_log_value).collect()),
                false,
            )
        }
        "shelfy_orden_simulate" => run_orden_tool(args, true),
        "shelfy_scan_folder" => {
            ensure_write_allowed(settings.mcp_allow_write)?;
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing path".to_string())?;
            tool_text(json!(manual_scan_folder(path)?), false)
        }
        "shelfy_orden_run" => {
            ensure_write_allowed(settings.mcp_allow_write)?;
            run_orden_tool(args, false)
        }
        _ => tool_text(json!({"error": format!("Unknown tool: {}", name)}), true),
    }
}

fn ensure_write_allowed(allow_write: bool) -> Result<(), String> {
    if allow_write {
        Ok(())
    } else {
        Err("MCP write tools are disabled in Shelfy settings".to_string())
    }
}

fn run_orden_tool(args: Value, simulate: bool) -> Result<Value, String> {
    sync_orden_configs_from_disk();
    let (config_name, yaml) = resolve_orden_input(&args)?;
    let tags = string_array(args.get("tags"));
    let skip_tags = string_array(args.get("skip_tags"));
    let opts = crate::orden::ExecuteOptions {
        simulate,
        tags: tags.into_iter().collect(),
        skip_tags: skip_tags.into_iter().collect(),
        working_dir: std::env::current_dir().unwrap_or_default(),
    };
    let execution = std::thread::Builder::new()
        .name("orden-mcp".to_string())
        .spawn(move || crate::orden::run_yaml(&yaml, &opts))
        .map_err(|error| format!("Failed to start Orden MCP worker: {error}"))?
        .join()
        .map_err(|_| "Orden MCP worker thread panicked".to_string())?;
    match execution {
        Ok(result) => {
            let logs_json = serde_json::to_string(&result.logs).unwrap_or_else(|_| "[]".into());
            log_orden_run(
                &config_name,
                simulate,
                result.success as i64,
                result.errors as i64,
                if simulate { "mcp-simulate" } else { "mcp-run" },
                &logs_json,
            )
            .map_err(|e| e.to_string())?;
            let mut value = serde_json::to_value(&result).map_err(|e| e.to_string())?;
            if let Some(object) = value.as_object_mut() {
                object.insert("config_name".into(), Value::String(config_name));
            }
            tool_text(value, false)
        }
        Err(error) => {
            let logs = json!([{
                "level": "error",
                "sender": "orden",
                "rule_nr": -1,
                "path": "<config>",
                "msg": error.clone(),
            }]);
            let _ = log_orden_run(
                &config_name,
                simulate,
                0,
                1,
                if simulate { "mcp-simulate" } else { "mcp-run" },
                &logs.to_string(),
            );
            tool_text(
                json!({ "config_name": config_name, "error": error, "logs": logs }),
                true,
            )
        }
    }
}

fn resolve_orden_input(args: &Value) -> Result<(String, String), String> {
    if let Some(name) = args
        .get("config_name")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let config = get_orden_config(name)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Orden config '{}' not found", name))?;
        return Ok((config.name, config.yaml));
    }

    let yaml = required_string(args, "yaml")?.to_string();
    let config_name = list_orden_configs()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|config| config.yaml == yaml)
        .map(|config| config.name)
        .unwrap_or_else(|| "<mcp-ad-hoc>".to_string());
    Ok((config_name, yaml))
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("Missing {}", key))
}

fn sync_orden_configs_from_disk() {
    let Some(project_dirs) = directories::ProjectDirs::from("cc", "shelfy", "shelfy") else {
        return;
    };
    let data_dir = project_dirs.data_dir();
    for name in crate::orden::list_config_names(data_dir) {
        if get_orden_config(&name).ok().flatten().is_some() {
            continue;
        }
        if let Ok(yaml) = crate::orden::load_config_text(data_dir, &name) {
            let _ = upsert_orden_config(&name, &yaml);
        }
    }
}

fn resources() -> Result<Value, String> {
    let settings = get_settings().map_err(|e| e.to_string())?;
    if !settings.mcp_enabled {
        return Ok(Value::Array(vec![]));
    }
    sync_orden_configs_from_disk();
    let mut resources = vec![
        json!({
            "uri": "shelfy://orden/configs",
            "name": "Orden configurations",
            "description": "Saved Orden configuration index",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "shelfy://orden/jobs",
            "name": "Orden automation jobs",
            "description": "Configured schedules and monitor jobs",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "shelfy://orden/history",
            "name": "Orden run history",
            "description": "Recent Orden execution logs and process details",
            "mimeType": "application/json"
        }),
    ];
    for config in list_orden_configs().map_err(|e| e.to_string())? {
        resources.push(json!({
            "uri": config_resource_uri(&config.name),
            "name": format!("Orden: {}", config.name),
            "description": format!("Saved Orden YAML configuration updated {}", config.updated_at),
            "mimeType": "application/yaml"
        }));
        resources.push(json!({
            "uri": config_history_resource_uri(&config.name),
            "name": format!("Orden history: {}", config.name),
            "description": "Detailed execution history for this Orden configuration",
            "mimeType": "application/json"
        }));
    }
    Ok(Value::Array(resources))
}

fn read_resource(params: Value) -> Result<Value, String> {
    let settings = get_settings().map_err(|e| e.to_string())?;
    if !settings.mcp_enabled {
        return Err("Shelfy MCP is disabled".into());
    }
    sync_orden_configs_from_disk();
    let uri = required_string(&params, "uri")?;
    let (mime_type, text) = if uri == "shelfy://orden/configs" {
        let configs = list_orden_configs().map_err(|e| e.to_string())?;
        let index = configs
            .into_iter()
            .map(|config| {
                let resource_uri = config_resource_uri(&config.name);
                json!({
                    "name": config.name,
                    "created_at": config.created_at,
                    "updated_at": config.updated_at,
                    "resource_uri": resource_uri,
                })
            })
            .collect::<Vec<_>>();
        ("application/json", pretty_json(Value::Array(index))?)
    } else if uri == "shelfy://orden/jobs" {
        (
            "application/json",
            pretty_json(json!(list_orden_jobs().map_err(|e| e.to_string())?))?,
        )
    } else if uri == "shelfy://orden/history" {
        let logs = get_recent_orden_run_logs(100).map_err(|e| e.to_string())?;
        (
            "application/json",
            pretty_json(Value::Array(logs.into_iter().map(run_log_value).collect()))?,
        )
    } else if let Some(encoded_name) = uri.strip_prefix("shelfy://orden/config/") {
        let name = decode_uri_component(encoded_name)?;
        let config = get_orden_config(&name)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Orden config '{}' not found", name))?;
        ("application/yaml", config.yaml)
    } else if let Some(encoded_name) = uri.strip_prefix("shelfy://orden/history/") {
        let name = decode_uri_component(encoded_name)?;
        let logs = get_orden_run_logs(&name, 100).map_err(|e| e.to_string())?;
        (
            "application/json",
            pretty_json(Value::Array(logs.into_iter().map(run_log_value).collect()))?,
        )
    } else {
        return Err(format!("Unknown Shelfy resource: {}", uri));
    };

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": mime_type,
            "text": text
        }]
    }))
}

fn run_log_value(log: OrdenRunLog) -> Value {
    let logs = serde_json::from_str::<Value>(&log.logs_json).unwrap_or_else(|_| json!([]));
    json!({
        "id": log.id,
        "config_name": log.config_name,
        "timestamp": log.timestamp,
        "simulate": log.simulate,
        "success": log.success,
        "errors": log.errors,
        "trigger": log.trigger,
        "logs": logs,
    })
}

fn pretty_json(value: Value) -> Result<String, String> {
    serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
}

fn config_resource_uri(name: &str) -> String {
    format!("shelfy://orden/config/{}", encode_uri_component(name))
}

fn config_history_resource_uri(name: &str) -> String {
    format!("shelfy://orden/history/{}", encode_uri_component(name))
}

fn encode_uri_component(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{:02X}", byte),
        })
        .collect()
}

fn decode_uri_component(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("Invalid percent-encoded resource URI".into());
            }
            let hex =
                std::str::from_utf8(&bytes[index + 1..index + 3]).map_err(|e| e.to_string())?;
            decoded.push(u8::from_str_radix(hex, 16).map_err(|e| e.to_string())?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|e| e.to_string())
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn tool_text(value: Value, is_error: bool) -> Result<Value, String> {
    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
        }],
        "isError": is_error
    }))
}

fn write_response(
    stdout: &mut io::Stdout,
    id: Option<Value>,
    result: Option<Value>,
    error: Option<Value>,
) -> Result<(), String> {
    let response = if let Some(error) = error {
        json!({
            "jsonrpc": "2.0",
            "id": id.unwrap_or(Value::Null),
            "error": error,
        })
    } else {
        json!({
            "jsonrpc": "2.0",
            "id": id.unwrap_or(Value::Null),
            "result": result.unwrap_or(Value::Null),
        })
    };
    writeln!(stdout, "{}", response).map_err(|e| e.to_string())?;
    stdout.flush().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_uri_component_round_trips_unicode_and_spaces() {
        let name = "工作 config #1";
        let encoded = encode_uri_component(name);
        assert!(!encoded.contains(' '));
        assert_eq!(decode_uri_component(&encoded).unwrap(), name);
    }

    #[test]
    fn initialize_advertises_orden_resources() {
        let response = handle_method("initialize", None).unwrap();
        assert!(response["capabilities"]["tools"].is_object());
        assert!(response["capabilities"]["resources"].is_object());
    }
}
