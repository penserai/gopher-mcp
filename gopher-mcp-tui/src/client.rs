use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use gopher_mcp_core::{DumpResult, ItemType, MenuItem, Router};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct BrowseItem {
    #[serde(rename = "type")]
    pub item_type: String,
    pub type_name: String,
    pub display: String,
    pub path: String,
    pub mime: String,
}

#[async_trait]
pub trait ContentClient: Send + Sync {
    async fn browse(&self, path: &str) -> Result<Vec<BrowseItem>>;
    async fn fetch(&self, path: &str) -> Result<String>;
    async fn search(&self, path: &str, query: &str) -> Result<Vec<BrowseItem>>;
    async fn publish(&self, path: &str, content: &str) -> Result<()>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn dump(&self, source: &str, destination: &str, max_depth: u32)
        -> Result<DumpResult>;
}

// --- McpClient (remote mode via HTTP JSON-RPC) ---

pub struct McpClient {
    client: reqwest::Client,
    endpoint: String,
    next_id: AtomicU64,
}

impl McpClient {
    pub fn new(base_url: &str) -> Self {
        let endpoint = format!("{}/mcp", base_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        Self {
            client,
            endpoint,
            next_id: AtomicU64::new(1),
        }
    }

    async fn rpc_call(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments,
            }
        });

        let resp = self
            .client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .context("Failed to connect to server")?;

        let json: Value = resp
            .json()
            .await
            .context("Failed to parse server response")?;

        if let Some(err) = json.get("error") {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown RPC error");
            bail!("RPC error: {}", msg);
        }

        json.get("result")
            .cloned()
            .context("Missing result in response")
    }

    fn extract_text(result: &Value) -> Result<String> {
        result
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .context("Missing text in response content")
    }

    fn check_error(result: &Value, text: &str) -> Result<()> {
        if result.get("isError").and_then(|v| v.as_bool()) == Some(true) {
            bail!("{}", text);
        }
        Ok(())
    }
}

#[async_trait]
impl ContentClient for McpClient {
    async fn browse(&self, path: &str) -> Result<Vec<BrowseItem>> {
        let result = self
            .rpc_call("gopher_browse", serde_json::json!({ "path": path }))
            .await?;
        let text = Self::extract_text(&result)?;
        Self::check_error(&result, &text)?;
        let items: Vec<BrowseItem> =
            serde_json::from_str(&text).context("Failed to parse browse items")?;
        Ok(items)
    }

    async fn fetch(&self, path: &str) -> Result<String> {
        let result = self
            .rpc_call("gopher_fetch", serde_json::json!({ "path": path }))
            .await?;
        let text = Self::extract_text(&result)?;
        Self::check_error(&result, &text)?;
        Ok(text)
    }

    async fn search(&self, path: &str, query: &str) -> Result<Vec<BrowseItem>> {
        let result = self
            .rpc_call(
                "gopher_search",
                serde_json::json!({ "path": path, "query": query }),
            )
            .await?;
        let text = Self::extract_text(&result)?;
        Self::check_error(&result, &text)?;
        let items: Vec<BrowseItem> =
            serde_json::from_str(&text).context("Failed to parse search results")?;
        Ok(items)
    }

    async fn publish(&self, path: &str, content: &str) -> Result<()> {
        let result = self
            .rpc_call(
                "gopher_publish",
                serde_json::json!({ "path": path, "content": content }),
            )
            .await?;
        let text = Self::extract_text(&result)?;
        Self::check_error(&result, &text)?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let result = self
            .rpc_call("gopher_delete", serde_json::json!({ "path": path }))
            .await?;
        let text = Self::extract_text(&result)?;
        Self::check_error(&result, &text)?;
        Ok(())
    }

    async fn dump(
        &self,
        source: &str,
        destination: &str,
        max_depth: u32,
    ) -> Result<DumpResult> {
        let result = self
            .rpc_call(
                "gopher_dump",
                serde_json::json!({
                    "source": source,
                    "destination": destination,
                    "max_depth": max_depth,
                }),
            )
            .await?;
        let text = Self::extract_text(&result)?;
        Self::check_error(&result, &text)?;
        // Parse "Dumped X documents (Y skipped) ..." from server response
        let published = text
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let skipped = text
            .find('(')
            .and_then(|i| {
                text[i + 1..]
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
            })
            .unwrap_or(0);
        Ok(DumpResult { published, skipped })
    }
}

// --- EmbeddedClient (in-process via Router) ---

pub struct EmbeddedClient {
    router: Arc<Router>,
}

impl EmbeddedClient {
    pub fn new(router: Arc<Router>) -> Self {
        Self { router }
    }
}

#[async_trait]
impl ContentClient for EmbeddedClient {
    async fn browse(&self, path: &str) -> Result<Vec<BrowseItem>> {
        if path.is_empty() {
            let namespaces = self.router.namespaces();
            return Ok(namespaces
                .into_iter()
                .map(|ns| BrowseItem {
                    item_type: "1".to_string(),
                    type_name: "Menu".to_string(),
                    display: ns.clone(),
                    path: format!("{}/", ns),
                    mime: "application/x-gopher-menu".to_string(),
                })
                .collect());
        }

        let items = self
            .router
            .browse(path)
            .await
            .context("Browse failed")?;
        Ok(items.into_iter().map(menu_item_to_browse_item).collect())
    }

    async fn fetch(&self, path: &str) -> Result<String> {
        self.router
            .fetch(path)
            .await
            .context("Fetch failed")
    }

    async fn search(&self, path: &str, query: &str) -> Result<Vec<BrowseItem>> {
        let items = self
            .router
            .search(path, query)
            .await
            .context("Search failed")?;
        Ok(items.into_iter().map(menu_item_to_browse_item).collect())
    }

    async fn publish(&self, path: &str, content: &str) -> Result<()> {
        self.router
            .publish(path, content)
            .await
            .context("Publish failed")
    }

    async fn delete(&self, path: &str) -> Result<()> {
        self.router
            .delete(path)
            .await
            .context("Delete failed")
    }

    async fn dump(
        &self,
        source: &str,
        destination: &str,
        max_depth: u32,
    ) -> Result<DumpResult> {
        self.router
            .dump(source, destination, max_depth)
            .await
            .context("Dump failed")
    }
}

fn menu_item_to_browse_item(item: MenuItem) -> BrowseItem {
    let path = if item.itype == ItemType::Info {
        String::new()
    } else {
        format!(
            "{}/{}",
            item.host,
            item.selector.trim_start_matches('/')
        )
    };
    BrowseItem {
        item_type: item.itype.to_char().to_string(),
        type_name: item.itype.name().to_string(),
        display: item.display,
        path,
        mime: item.itype.mime().to_string(),
    }
}
