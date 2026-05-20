use std::sync::Arc;

use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth;
use crate::substack::SubstackClient;
use crate::tools;

pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "unofficial-substack-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

#[derive(Deserialize)]
pub struct Request {
    #[serde(default)]
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

pub struct ServerState {
    pub client: SubstackClient,
}

impl ServerState {
    pub fn new() -> Result<Self> {
        let creds = auth::load();
        let http = reqwest::Client::builder()
            .cookie_store(true)
            .user_agent(format!("{SERVER_NAME}/{SERVER_VERSION}"))
            .build()?;
        let client = SubstackClient::new(http, creds);
        Ok(Self { client })
    }
}

pub fn ok(id: Value, result: Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string()
}

pub fn err(id: Value, code: i32, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message},
    })
    .to_string()
}

pub async fn handle_line(state: Arc<ServerState>, line: &str) -> Option<String> {
    let req: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(_) => return Some(err(Value::Null, PARSE_ERROR, "invalid JSON")),
    };
    if req.jsonrpc != "2.0" {
        return Some(err(
            req.id.unwrap_or(Value::Null),
            INVALID_REQUEST,
            "jsonrpc must be \"2.0\"",
        ));
    }

    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => Some(ok(id.unwrap_or(Value::Null), initialize_result())),
        "initialized" | "notifications/initialized" => None,
        "ping" => Some(ok(id.unwrap_or(Value::Null), json!({}))),
        "tools/list" => Some(ok(id.unwrap_or(Value::Null), tools::tools_list())),
        "tools/call" => match handle_tools_call(state, req.params).await {
            Ok(v) => Some(ok(id.unwrap_or(Value::Null), v)),
            Err(JsonRpcError { code, message }) => {
                Some(err(id.unwrap_or(Value::Null), code, &message))
            }
        },
        other => {
            if id.is_none() {
                None
            } else {
                Some(err(
                    id.unwrap_or(Value::Null),
                    METHOD_NOT_FOUND,
                    &format!("unknown method `{other}`"),
                ))
            }
        }
    }
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
        "capabilities": {
            "tools": {"listChanged": false},
        },
    })
}

pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcError {
    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: INVALID_PARAMS,
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: INTERNAL_ERROR,
            message: msg.into(),
        }
    }
}

fn map_tool_error(e: anyhow::Error) -> JsonRpcError {
    let msg = e.to_string();
    if msg.contains("not configured")
        || msg.contains("missing required parameter")
        || msg.contains("must be")
        || msg.contains("invalid")
        || msg.contains("at least one")
    {
        JsonRpcError::invalid_params(msg)
    } else {
        JsonRpcError::internal(msg)
    }
}

async fn handle_tools_call(
    state: Arc<ServerState>,
    params: Option<Value>,
) -> std::result::Result<Value, JsonRpcError> {
    let params = params.ok_or_else(|| JsonRpcError::invalid_params("missing params"))?;
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| JsonRpcError::invalid_params("missing tool name"))?
        .to_string();
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = tools::call(state, &name, arguments)
        .await
        .map_err(map_tool_error)?;

    let text = match &result {
        Value::String(s) => s.clone(),
        v => serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()),
    };
    Ok(json!({
        "content": [{"type": "text", "text": text}],
        "isError": false,
    }))
}
