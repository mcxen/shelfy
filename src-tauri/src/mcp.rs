use crate::db::{get_recent_logs, get_rules, get_settings, get_watched_folders};
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
                "tools": {}
            },
            "serverInfo": {
                "name": "shelfy",
                "version": env!("CARGO_PKG_VERSION")
            }
        })),
        "tools/list" => Ok(json!({ "tools": tools()? })),
        "tools/call" => call_tool(params.unwrap_or(Value::Null)),
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
            "name": "shelfy_orden_simulate",
            "description": "Simulate an Orden YAML config and return structured logs.",
            "inputSchema": object_schema(vec![
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
            "description": "Run an Orden YAML config. This may move, copy, rename, or delete files depending on the YAML.",
            "inputSchema": object_schema(vec![
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
    let yaml = args
        .get("yaml")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing yaml".to_string())?;
    let tags = string_array(args.get("tags"));
    let skip_tags = string_array(args.get("skip_tags"));
    let opts = crate::orden::ExecuteOptions {
        simulate,
        tags: tags.into_iter().collect(),
        skip_tags: skip_tags.into_iter().collect(),
        working_dir: std::env::current_dir().unwrap_or_default(),
    };
    let result = crate::orden::run_yaml(yaml, &opts)?;
    tool_text(json!(result), false)
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
