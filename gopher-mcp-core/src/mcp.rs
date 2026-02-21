use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use crate::gopher::{ItemType, MenuItem};
use crate::router::Router;

#[derive(Debug, Deserialize)]
pub struct McpRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(Debug, Serialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
}

pub struct McpHandler {
    pub router: Arc<Router>,
}

impl McpHandler {
    pub fn new(router: Arc<Router>) -> Self {
        McpHandler { router }
    }

    pub async fn handle(&self, req: McpRequest) -> Option<McpResponse> {
        match req.method.as_str() {
            "initialize" => Some(self.initialize(req.id)),
            "tools/list" => Some(self.list_tools(req.id)),
            "tools/call" => Some(self.call_tool(req.id, req.params).await),
            "ping" => Some(self.ping(req.id)),
            m if m.starts_with("notifications/") => None,
            _ => Some(McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(McpError {
                    code: -32601,
                    message: format!("Method not found: {}", req.method),
                }),
            }),
        }
    }

    fn initialize(&self, id: Option<Value>) -> McpResponse {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "gopher-mcp",
                    "version": "0.1.0"
                }
            })),
            error: None,
        }
    }

    fn list_tools(&self, id: Option<Value>) -> McpResponse {
        let tools = serde_json::json!([
            {
                "name": "gopher_browse",
                "description": "Navigate a Gopher menu. Returns structured items with type, display text, and navigable path.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "host/selector" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "gopher_fetch",
                "description": "Retrieve a Gopher document's text content.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "host/selector" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "gopher_search",
                "description": "Execute a search query against a Gopher search endpoint (type 7) or filter local menu entries.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "host/selector for search endpoint" },
                        "query": { "type": "string" }
                    },
                    "required": ["path", "query"]
                }
            }
        ]);

        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({ "tools": tools })),
            error: None,
        }
    }

    async fn call_tool(&self, id: Option<Value>, params: Option<Value>) -> McpResponse {
        let params = match params {
            Some(Value::Object(o)) => o,
            _ => return McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError { code: -32602, message: "Invalid parameters".to_string() }),
            },
        };

        let name = match params.get("name").and_then(|n| n.as_str()) {
            Some(n) => n,
            None => return McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError { code: -32602, message: "Missing tool name".to_string() }),
            },
        };

        let arguments = match params.get("arguments").and_then(|a| a.as_object()) {
            Some(a) => a,
            None => return McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError { code: -32602, message: "Missing tool arguments".to_string() }),
            },
        };

        let tool_result = match name {
            "gopher_browse" => {
                let path = arguments.get("path").and_then(|p| p.as_str()).unwrap_or("");
                match self.router.browse(path).await {
                    Ok(items) => Self::menu_items_response(items),
                    Err(e) => Self::tool_error(e),
                }
            },
            "gopher_fetch" => {
                let path = arguments.get("path").and_then(|p| p.as_str()).unwrap_or("");
                match self.router.fetch(path).await {
                    Ok(content) => serde_json::json!({ "content": [{ "type": "text", "text": content }] }),
                    Err(e) => Self::tool_error(e),
                }
            },
            "gopher_search" => {
                let path = arguments.get("path").and_then(|p| p.as_str()).unwrap_or("");
                let query = arguments.get("query").and_then(|q| q.as_str()).unwrap_or("");
                match self.router.search(path, query).await {
                    Ok(items) => Self::menu_items_response(items),
                    Err(e) => Self::tool_error(e),
                }
            },
            _ => Self::tool_error(format!("Tool not found: {}", name)),
        };

        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(tool_result),
            error: None,
        }
    }

    fn menu_items_response(items: Vec<MenuItem>) -> Value {
        let mcp_items: Vec<Value> = items.into_iter().map(|item| {
            let path = if item.itype == ItemType::Info {
                String::new()
            } else {
                format!("{}/{}", item.host, item.selector.trim_start_matches('/'))
            };
            serde_json::json!({
                "type": item.itype.to_char().to_string(),
                "type_name": item.itype.name(),
                "display": item.display,
                "path": path,
                "mime": item.itype.mime(),
            })
        }).collect();
        serde_json::json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&mcp_items).unwrap() }] })
    }

    fn tool_error(err: impl std::fmt::Display) -> Value {
        serde_json::json!({ "content": [{ "type": "text", "text": format!("Error: {}", err) }], "isError": true })
    }

    fn ping(&self, id: Option<Value>) -> McpResponse {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::json!({})),
            error: None,
        }
    }
}
