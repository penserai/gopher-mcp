use std::collections::HashMap;
use std::sync::Arc;
use crate::gopher::{MenuItem, GopherClient};
use crate::store::{LocalStore, ContentNode};
use crate::adapters::{AdapterError, SourceAdapter};
use crate::gopher::ItemType;
use thiserror::Error;

pub struct DumpResult {
    pub published: u32,
    pub skipped: u32,
}

#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Selector not found: {0} in {1}")]
    SelectorNotFound(String, String),
    #[error("Gopher error: {0}")]
    Gopher(#[from] crate::gopher::GopherError),
    #[error("Not writable: {0}")]
    NotWritable(String),
    #[error("Adapter error: {0}")]
    Adapter(#[from] AdapterError),
}

pub struct Router {
    pub local_store: LocalStore,
    adapters: HashMap<String, Arc<dyn SourceAdapter>>,
}

impl Router {
    pub fn new(local_store: LocalStore) -> Self {
        Router {
            local_store,
            adapters: HashMap::new(),
        }
    }

    pub fn register_adapter(&mut self, adapter: Arc<dyn SourceAdapter>) {
        let namespace = adapter.namespace().to_string();
        self.adapters.insert(namespace, adapter);
    }

    pub async fn browse(&self, path: &str) -> Result<Vec<MenuItem>, RouterError> {
        let (host, selector) = self.parse_path(path);

        if self.is_local(host) {
            match self.local_store.get_content(host, selector) {
                Some(ContentNode::Menu(items)) => Ok(items),
                Some(ContentNode::Document(_)) => Err(RouterError::SelectorNotFound(selector.to_string(), host.to_string())),
                None => Err(RouterError::SelectorNotFound(selector.to_string(), host.to_string())),
            }
        } else {
            Ok(GopherClient::fetch_menu(host, 70, selector).await?)
        }
    }

    pub async fn fetch(&self, path: &str) -> Result<String, RouterError> {
        let (host, selector) = self.parse_path(path);

        if self.is_local(host) {
            match self.local_store.get_content(host, selector) {
                Some(ContentNode::Document(content)) => Ok(content),
                Some(ContentNode::Menu(_)) => Err(RouterError::SelectorNotFound(selector.to_string(), host.to_string())),
                None => Err(RouterError::SelectorNotFound(selector.to_string(), host.to_string())),
            }
        } else {
            Ok(GopherClient::fetch_text(host, 70, selector).await?)
        }
    }

    pub async fn search(&self, path: &str, query: &str) -> Result<Vec<MenuItem>, RouterError> {
        let (host, selector) = self.parse_path(path);

        if self.is_local(host) {
            // Try adapter-native search first
            if let Some(adapter) = self.adapters.get(host) {
                if let Some(results) = adapter.search(selector, query).await {
                    return Ok(results);
                }
            }

            // Fall back to filtering menu items
            match self.local_store.get_content(host, selector) {
                Some(ContentNode::Menu(items)) => {
                    let filtered = items.into_iter()
                        .filter(|i| i.display.to_lowercase().contains(&query.to_lowercase()))
                        .collect();
                    Ok(filtered)
                }
                _ => Err(RouterError::SelectorNotFound(selector.to_string(), host.to_string())),
            }
        } else {
            Ok(GopherClient::search(host, 70, selector, query).await?)
        }
    }

    fn parse_path<'a>(&self, path: &'a str) -> (&'a str, &'a str) {
        if let Some(pos) = path.find('/') {
            let host = &path[..pos];
            let selector = &path[pos..];
            if selector == "/" {
                (host, "")
            } else {
                (host, selector)
            }
        } else {
            (path, "")
        }
    }

    pub async fn publish(&self, path: &str, content: &str) -> Result<(), RouterError> {
        let (host, selector) = self.parse_path(path);

        if !self.is_local(host) {
            return Err(RouterError::NotWritable(host.to_string()));
        }

        let adapter = self.adapters.get(host)
            .ok_or_else(|| RouterError::NotWritable(host.to_string()))?;

        if !adapter.is_writable() {
            return Err(RouterError::NotWritable(host.to_string()));
        }

        adapter.publish(&self.local_store, selector, content).await?;
        Ok(())
    }

    pub async fn delete(&self, path: &str) -> Result<(), RouterError> {
        let (host, selector) = self.parse_path(path);

        if !self.is_local(host) {
            return Err(RouterError::NotWritable(host.to_string()));
        }

        let adapter = self.adapters.get(host)
            .ok_or_else(|| RouterError::NotWritable(host.to_string()))?;

        if !adapter.is_writable() {
            return Err(RouterError::NotWritable(host.to_string()));
        }

        adapter.delete(&self.local_store, selector).await?;
        Ok(())
    }

    pub async fn dump(&self, source: &str, destination: &str, max_depth: u32) -> Result<DumpResult, RouterError> {
        let mut result = DumpResult { published: 0, skipped: 0 };
        self.dump_recursive(source, destination, 0, max_depth, &mut result).await?;
        Ok(result)
    }

    async fn dump_recursive(
        &self,
        source: &str,
        dest: &str,
        depth: u32,
        max_depth: u32,
        result: &mut DumpResult,
    ) -> Result<(), RouterError> {
        let items = match self.browse(source).await {
            Ok(items) => items,
            Err(_) => {
                result.skipped += 1;
                return Ok(());
            }
        };

        for item in items {
            match item.itype {
                ItemType::Menu => {
                    if depth < max_depth {
                        let child_source = format!("{}/{}", item.host, item.selector.trim_start_matches('/'));
                        let child_dest = format!("{}/{}", dest.trim_end_matches('/'), item.display);
                        Box::pin(self.dump_recursive(&child_source, &child_dest, depth + 1, max_depth, result)).await?;
                    } else {
                        result.skipped += 1;
                    }
                }
                ItemType::TextFile | ItemType::Html => {
                    let fetch_path = format!("{}/{}", item.host, item.selector.trim_start_matches('/'));
                    match self.fetch(&fetch_path).await {
                        Ok(content) => {
                            let publish_path = format!("{}/{}", dest.trim_end_matches('/'), item.display);
                            self.publish(&publish_path, &content).await?;
                            result.published += 1;
                        }
                        Err(_) => {
                            result.skipped += 1;
                        }
                    }
                }
                _ => {
                    result.skipped += 1;
                }
            }
        }

        Ok(())
    }

    /// Return all known namespace names (from local store and registered adapters).
    pub fn namespaces(&self) -> Vec<String> {
        let mut ns: Vec<String> = self
            .local_store
            .content
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for key in self.adapters.keys() {
            if !ns.contains(key) {
                ns.push(key.clone());
            }
        }
        ns.sort();
        ns
    }

    fn is_local(&self, host: &str) -> bool {
        self.local_store.has_namespace(host)
    }
}
