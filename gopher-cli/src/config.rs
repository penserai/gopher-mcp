use std::path::PathBuf;
use std::sync::Arc;

use serde::Deserialize;
use tracing::{info, warn};

use gopher_cli_core::{AdapterError, SourceAdapter};

#[derive(Debug, Deserialize, Default)]
pub struct TuiConfig {
    pub url: Option<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub adapter: Vec<AdapterConfig>,
}

#[derive(Debug, Deserialize)]
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
        #[serde(default)]
        writable: bool,
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

impl TuiConfig {
    pub fn load() -> Self {
        for path in Self::candidate_paths() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                match toml::from_str::<TuiConfig>(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        warn!(path = %path.display(), error = %e, "Failed to parse config");
                    }
                }
            }
        }
        Self::default()
    }

    fn candidate_paths() -> Vec<PathBuf> {
        let home = match std::env::var("HOME") {
            Ok(h) => PathBuf::from(h),
            Err(_) => return Vec::new(),
        };

        vec![home.join(".gopher-cli.toml")]
    }

    pub fn adapter_namespaces(&self) -> Vec<String> {
        self.adapter
            .iter()
            .filter_map(|a| {
                let ns = match a {
                    #[cfg(feature = "adapter-rss")]
                    AdapterConfig::Rss { namespace, .. } => namespace,
                    #[cfg(feature = "adapter-fs")]
                    AdapterConfig::Fs { namespace, .. } => namespace,
                    #[cfg(feature = "adapter-rdf")]
                    AdapterConfig::Rdf { namespace, .. } => namespace,
                };
                if ns.is_empty() {
                    None
                } else {
                    Some(format!("{}/", ns))
                }
            })
            .collect()
    }
}

pub fn create_adapters(
    config: &TuiConfig,
) -> Result<Vec<Arc<dyn SourceAdapter>>, AdapterError> {
    let mut adapters: Vec<Arc<dyn SourceAdapter>> = Vec::new();

    for adapter_config in &config.adapter {
        match adapter_config {
            #[cfg(feature = "adapter-rss")]
            AdapterConfig::Rss { namespace, url } => {
                info!(namespace = %namespace, url = %url, "Creating RSS adapter");
                let adapter =
                    gopher_cli_core::RssAdapter::new(namespace.clone(), url.clone());
                adapters.push(Arc::new(adapter));
            }

            #[cfg(feature = "adapter-fs")]
            AdapterConfig::Fs {
                namespace,
                root,
                extensions,
                writable,
            } => {
                info!(namespace = %namespace, root = %root, writable = %writable, "Creating FS adapter");
                let adapter = gopher_cli_core::FsAdapter::new(
                    namespace.clone(),
                    PathBuf::from(root),
                    extensions.clone(),
                    *writable,
                )?;
                adapters.push(Arc::new(adapter));
            }

            #[cfg(feature = "adapter-rdf")]
            AdapterConfig::Rdf {
                namespace,
                source,
                format,
                sparql_endpoint,
            } => {
                info!(namespace = %namespace, "Creating RDF adapter");
                let rdf_format = match format.as_str() {
                    "turtle" | "ttl" => gopher_cli_core::adapters::rdf::RdfFormat::Turtle,
                    "rdfxml" | "rdf/xml" | "xml" => {
                        gopher_cli_core::adapters::rdf::RdfFormat::RdfXml
                    }
                    "ntriples" | "nt" => gopher_cli_core::adapters::rdf::RdfFormat::NTriples,
                    _ => {
                        return Err(AdapterError::Config(format!(
                            "Unknown RDF format: {}. Use turtle, rdfxml, or ntriples",
                            format
                        )));
                    }
                };
                let adapter = gopher_cli_core::adapters::rdf::RdfAdapter::new(
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
