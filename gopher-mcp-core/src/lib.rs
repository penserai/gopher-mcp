pub mod gopher;
pub mod store;
pub mod router;
pub mod mcp;
pub mod adapters;

pub use gopher::{GopherClient, GopherError, ItemType, MenuItem};
pub use store::{LocalStore, ContentNode};
pub use router::{DumpResult, Router, RouterError};
pub use mcp::{McpHandler, McpRequest, McpResponse, McpError};
pub use adapters::{SourceAdapter, AdapterError};

#[cfg(feature = "adapter-fs")]
pub use adapters::fs::FsAdapter;
#[cfg(feature = "adapter-rss")]
pub use adapters::rss::RssAdapter;
#[cfg(feature = "adapter-rdf")]
pub use adapters::rdf::RdfAdapter;
