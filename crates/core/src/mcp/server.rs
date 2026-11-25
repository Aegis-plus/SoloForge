//! A minimal MCP server over streamable HTTP (JSON responses), bound to
//! 127.0.0.1 and guarded by a per-launch bearer token. Claude Code connects
//! to it through the `.mcp.json` SoloForge writes into the world folder.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

use super::tools;
use crate::engine::Engine;

pub const SERVER_NAME: &str = "vtt";
const PROTOCOL: &str = "2025-06-18";

#[derive(Clone)]
struct AppState {
    engine: Arc<Engine>,
    token: String,
}

pub struct McpHandle {
    pub port: u16,
    pub token: String,
}

impl McpHandle {
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/mcp", self.port)
    }
    /// The `--mcp-config` JSON Claude Code reads.
    pub fn config_json(&self) -> Value {
        json!({"mcpServers": {SERVER_NAME: {"type": "http", "url": self.url(), "headers": {"Authorization": format!("Bearer {}", self.token)}}}})
    }
}

/// Start the server on a random local port. Must be called inside a Tokio runtime.
pub async fn start(engine: Arc<Engine>) -> anyhow::Result<McpHandle> {
    let token = format!("{}{}", crate::world::new_id(), crate::world::new_id());
    let state = AppState { engine: engine.clone(), token: token.clone() };
    let app = Router::new().route("/mcp", post(handle).get(not_allowed).delete(not_allowed)).with_state(state);
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
    let port = listener.local_addr()?.port();
    engine.mcp_port.store(port, std::sync::atomic::Ordering::SeqCst);
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("soloforge: MCP server stopped: {e}");
        }
    });
    Ok(McpHandle { port, token })
}

async fn not_allowed() -> StatusCode {
    StatusCode::METHOD_NOT_ALLOWED
}

async fn handle(State(st): State<AppState>, headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let authed = headers.get("authorization").and_then(|v| v.to_str().ok()).map(|v| v == format!("Bearer {}", st.token)).unwrap_or(false);
    if !authed {
        return (StatusCode::UNAUTHORIZED, "missing or wrong bearer token").into_response();
    }
    if let Some(batch) = body.as_array() {
        let out: Vec<Value> = batch.iter().filter_map(|m| dispatch(&st.engine, m)).collect();
        return if out.is_empty() { StatusCode::ACCEPTED.into_response() } else { Json(Value::Array(out)).into_response() };
    }
    match dispatch(&st.engine, &body) {
        Some(v) => Json(v).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

/// Handle one JSON-RPC message; `None` for notifications.
pub fn dispatch(engine: &Arc<Engine>, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = id?; // notifications (no id) get no response
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": params.get("protocolVersion").and_then(|v| v.as_str()).unwrap_or(PROTOCOL),
            "capabilities": {"tools": {"listChanged": false}},
            "serverInfo": {"name": "soloforge-table", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "These tools are the SoloForge table. Call get_state first each turn. Never invent dice results."
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools::definitions()})),
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let r = tools::call(engine, name, &args);
            engine.emit("gm://tool", json!({"name": name, "ok": r.is_ok()}));
            Ok(match r {
                Ok(text) => json!({"content": [{"type": "text", "text": text}], "isError": false}),
                Err(err) => json!({"content": [{"type": "text", "text": format!("Error: {err}")}], "isError": true}),
            })
        }
        "resources/list" => Ok(json!({"resources": []})),
        "prompts/list" => Ok(json!({"prompts": []})),
        _ => Err(json!({"code": -32601, "message": format!("method not found: {method}")})),
    };
    Some(match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
    })
}
