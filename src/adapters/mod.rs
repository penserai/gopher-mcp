use async_trait::async_trait;
use crate::store::LocalStore;
use crate::gopher::MenuItem;
use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(String),
}

#[allow(dead_code)]
#[async_trait]
pub trait SourceAdapter: Send + Sync {
    /// Unique namespace this adapter registers (e.g., "rdf.mydata", "feed.hackernews")
    fn namespace(&self) -> &str;

    /// Populate or refresh content in the local store
    async fn sync(&self, store: &LocalStore) -> Result<(), AdapterError>;

    /// Optional: handle search queries natively instead of filtering menu entries
    async fn search(&self, selector: &str, query: &str) -> Option<Vec<MenuItem>>;
}
