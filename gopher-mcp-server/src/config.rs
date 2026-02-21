use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;
use tracing::info;

use gopher_mcp_core::{AdapterError, SourceAdapter};

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub adapter: Vec<AdapterConfig>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum AdapterConfig {
    #[cfg(feature = "adapter-rss")]
    #[serde(rename = "rss")]
    Rss {
        namespace: String,
        url: String,
    },

    #[cfg(feature = "adapter-fs")]
    #[serde(rename = "fs")]
    Fs {
        namespace: String,
        root: String,
        extensions: Option<Vec<String>>,
    },

    #[cfg(feature = "adapter-rdf")]
    #[serde(rename = "rdf")]
    Rdf {
        namespace: String,
        source: Option<String>,
        #[serde(default = "default_rdf_format")]
        format: String,
        sparql_endpoint: Option<String>,
    },
}

#[cfg(feature = "adapter-rdf")]
fn default_rdf_format() -> String {
    "turtle".to_string()
}

pub fn create_adapters(
    config: &Config,
) -> Result<Vec<Arc<dyn SourceAdapter>>, AdapterError> {
    let mut adapters: Vec<Arc<dyn SourceAdapter>> = Vec::new();

    for adapter_config in &config.adapter {
        match adapter_config {
            #[cfg(feature = "adapter-rss")]
            AdapterConfig::Rss { namespace, url } => {
                info!(namespace = %namespace, url = %url, "Creating RSS adapter");
                let adapter = gopher_mcp_core::RssAdapter::new(
                    namespace.clone(),
                    url.clone(),
                );
                adapters.push(Arc::new(adapter));
            }

            #[cfg(feature = "adapter-fs")]
            AdapterConfig::Fs { namespace, root, extensions } => {
                info!(namespace = %namespace, root = %root, "Creating FS adapter");
                let adapter = gopher_mcp_core::FsAdapter::new(
                    namespace.clone(),
                    PathBuf::from(root),
                    extensions.clone(),
                )?;
                adapters.push(Arc::new(adapter));
            }

            #[cfg(feature = "adapter-rdf")]
            AdapterConfig::Rdf { namespace, source, format, sparql_endpoint } => {
                info!(namespace = %namespace, "Creating RDF adapter");
                let rdf_format = match format.as_str() {
                    "turtle" | "ttl" => gopher_mcp_core::adapters::rdf::RdfFormat::Turtle,
                    "rdfxml" | "rdf/xml" | "xml" => gopher_mcp_core::adapters::rdf::RdfFormat::RdfXml,
                    "ntriples" | "nt" => gopher_mcp_core::adapters::rdf::RdfFormat::NTriples,
                    _ => return Err(AdapterError::Config(
                        format!("Unknown RDF format: {}. Use turtle, rdfxml, or ntriples", format),
                    )),
                };
                let adapter = gopher_mcp_core::adapters::rdf::RdfAdapter::new(
                    namespace.clone(),
                    source.clone(),
                    rdf_format,
                    sparql_endpoint.clone(),
                );
                adapters.push(Arc::new(adapter));
            }
        }
    }

    Ok(adapters)
}

pub fn load_config(path: &PathBuf) -> Result<Config, anyhow::Error> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}
