use async_trait::async_trait;
use crate::store::LocalStore;
use crate::gopher::MenuItem;
use thiserror::Error;

#[cfg(feature = "adapter-fs")]
pub mod fs;
#[cfg(feature = "adapter-rss")]
pub mod rss;
#[cfg(feature = "adapter-rdf")]
pub mod rdf;

#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Config error: {0}")]
    Config(String),
    #[error("Not writable: {0}")]
    NotWritable(String),
    #[error("Path traversal rejected: {0}")]
    PathTraversal(String),
}

#[async_trait]
pub trait SourceAdapter: Send + Sync {
    /// Unique namespace this adapter registers (e.g., "rdf.mydata", "feed.hackernews")
    fn namespace(&self) -> &str;

    /// Populate or refresh content in the local store
    async fn sync(&self, store: &LocalStore) -> Result<(), AdapterError>;

    /// Optional: handle search queries natively instead of filtering menu entries
    async fn search(&self, selector: &str, query: &str) -> Option<Vec<MenuItem>>;

    /// Whether this adapter supports write operations (publish/delete).
    fn is_writable(&self) -> bool { false }

    /// Write or update a document at `selector` with the given `content`.
    async fn publish(&self, _store: &LocalStore, _selector: &str, _content: &str) -> Result<(), AdapterError> {
        Err(AdapterError::NotWritable(self.namespace().to_string()))
    }

    /// Delete the document or directory at `selector`.
    async fn delete(&self, _store: &LocalStore, _selector: &str) -> Result<(), AdapterError> {
        Err(AdapterError::NotWritable(self.namespace().to_string()))
    }
}
