use crate::db::get_settings;

#[derive(serde::Serialize)]
pub struct McpClientConfig {
    enabled: bool,
    transport: String,
    config_json: String,
}

#[tauri::command]
pub fn mcp_client_config_cmd() -> Result<McpClientConfig, String> {
    let settings = get_settings().map_err(|e| e.to_string())?;
    let server_name = clean_mcp_server_name(&settings.mcp_server_name);
    let transport = clean_mcp_transport(&settings.mcp_transport);
    let config = if transport == "http" {
        let mut server = serde_json::json!({
            "url": settings
                .mcp_http_url
                .as_deref()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or("http://127.0.0.1:8765/mcp")
        });
        if let Some(token) = settings
            .mcp_token
            .as_deref()
            .filter(|v| !v.trim().is_empty())
        {
            server["token"] = serde_json::Value::String(token.to_string());
        }
        serde_json::json!({
            "mcpServers": {
                server_name: server
            }
        })
    } else {
        let command = settings
            .mcp_command
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                std::env::current_exe()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "shelfy".to_string())
            });
        let args = split_mcp_args(settings.mcp_args.as_deref().unwrap_or("--mcp"));
        serde_json::json!({
            "mcpServers": {
                server_name: {
                    "command": command,
                    "args": args
                }
            }
        })
    };
    Ok(McpClientConfig {
        enabled: settings.mcp_enabled,
        transport,
        config_json: serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
    })
}

fn clean_mcp_server_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "shelfy".to_string()
    } else {
        trimmed.to_string()
    }
}

fn clean_mcp_transport(value: &str) -> String {
    if value.eq_ignore_ascii_case("http") {
        "http".to_string()
    } else {
        "stdio".to_string()
    }
}

fn split_mcp_args(value: &str) -> Vec<String> {
    let args = value
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if args.is_empty() {
        vec!["--mcp".to_string()]
    } else {
        args
    }
}
