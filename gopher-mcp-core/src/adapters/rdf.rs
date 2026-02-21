use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use sophia::api::graph::Graph;
use sophia::api::parser::TripleParser;
use sophia::api::source::TripleSource;
use sophia::api::term::Term;
use sophia::api::triple::Triple;
use sophia::inmem::graph::LightGraph;
use sophia::turtle::parser::nt::NTriplesParser;
use sophia::turtle::parser::turtle::TurtleParser;
use sophia::xml::parser::RdfXmlParser;

use crate::adapters::{AdapterError, SourceAdapter};
use crate::gopher::{ItemType, MenuItem};
use crate::store::{ContentNode, LocalStore};

/// The RDF namespace IRI for `rdf:type`.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// The RDFS namespace IRI for `rdfs:label`.
const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";

/// Supported RDF serialization formats.
#[derive(Clone, Debug)]
pub enum RdfFormat {
    Turtle,
    RdfXml,
    NTriples,
}

/// A source adapter that exposes RDF data through the Gopher protocol hierarchy.
///
/// The adapter can load RDF from a local file or remote URL, parse it using
/// sophia, and build a class-centric navigation structure. Optionally, it can
/// proxy SPARQL queries to a remote endpoint.
pub struct RdfAdapter {
    pub namespace: String,
    pub source: Option<String>,
    pub format: RdfFormat,
    pub sparql_endpoint: Option<String>,
}

impl RdfAdapter {
    /// Create a new `RdfAdapter`.
    ///
    /// # Arguments
    ///
    /// * `namespace` - Unique namespace under which the content will be registered.
    /// * `source` - Optional file path or URL to RDF data.
    /// * `format` - The RDF serialization format of the source.
    /// * `sparql_endpoint` - Optional SPARQL endpoint URL for search queries.
    pub fn new(
        namespace: String,
        source: Option<String>,
        format: RdfFormat,
        sparql_endpoint: Option<String>,
    ) -> Self {
        Self {
            namespace,
            source,
            format,
            sparql_endpoint,
        }
    }

    /// Load RDF content from the configured source (file or URL).
    async fn load_content(&self, source: &str) -> Result<String, AdapterError> {
        if source.starts_with("http://") || source.starts_with("https://") {
            let response = reqwest::get(source)
                .await
                .map_err(|e| AdapterError::Network(format!("Failed to fetch RDF data: {e}")))?;
            response
                .text()
                .await
                .map_err(|e| AdapterError::Network(format!("Failed to read response body: {e}")))
        } else {
            std::fs::read_to_string(source).map_err(AdapterError::Io)
        }
    }

    /// Parse RDF content into a LightGraph using the configured format.
    fn parse_graph(&self, content: &str) -> Result<LightGraph, AdapterError> {
        match self.format {
            RdfFormat::Turtle => {
                let parser = TurtleParser::default();
                parser
                    .parse_str(content)
                    .collect_triples()
                    .map_err(|e| AdapterError::Parse(format!("Failed to parse Turtle: {e}")))
            }
            RdfFormat::RdfXml => {
                let parser = RdfXmlParser::default();
                parser
                    .parse_str(content)
                    .collect_triples()
                    .map_err(|e| AdapterError::Parse(format!("Failed to parse RDF/XML: {e}")))
            }
            RdfFormat::NTriples => {
                let parser = NTriplesParser::default();
                parser
                    .parse_str(content)
                    .collect_triples()
                    .map_err(|e| AdapterError::Parse(format!("Failed to parse N-Triples: {e}")))
            }
        }
    }

    /// Extract a displayable string value from a term.
    ///
    /// Prefers `lexical_form()` for literals, falls back to `iri()`, then
    /// to the debug representation.
    fn term_to_string(term: &impl Term) -> String {
        if let Some(iri) = term.iri() {
            iri.to_string()
        } else if let Some(lit) = term.lexical_form() {
            lit.to_string()
        } else {
            format!("{:?}", term.kind())
        }
    }

    /// Build the class-centric navigation structure from a parsed graph and
    /// populate the store.
    fn populate_from_graph(
        &self,
        graph: &LightGraph,
        store: &LocalStore,
    ) -> Result<Vec<MenuItem>, AdapterError> {
        // Collect all triples into a working structure:
        //   subject_uri -> Vec<(predicate_uri, object_string)>
        // Also track rdf:type relationships: class_uri -> Vec<instance_uri>
        let mut resource_props: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let mut class_instances: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for triple in graph.triples() {
            let triple = triple.map_err(|e| AdapterError::Parse(e.to_string()))?;
            let s = Self::term_to_string(&triple.s());
            let p = Self::term_to_string(&triple.p());
            let o = Self::term_to_string(&triple.o());

            // Track rdf:type relationships
            if p == RDF_TYPE {
                class_instances
                    .entry(o.clone())
                    .or_default()
                    .insert(s.clone());
            }

            // Store all predicate-object pairs per subject
            resource_props
                .entry(s)
                .or_default()
                .push((p, o));
        }

        // Create a Document for each unique resource showing its properties
        for (subject_uri, props) in &resource_props {
            let encoded = encode_uri(subject_uri);
            let mut lines = Vec::new();
            lines.push(format!("Resource: {subject_uri}"));
            lines.push(String::new());

            for (pred, obj) in props {
                let pred_name = local_name(pred);
                lines.push(format!("{pred_name}: {obj}"));
            }

            store.add_content(
                &self.namespace,
                &format!("/resource/{encoded}"),
                ContentNode::Document(lines.join("\n")),
            );
        }

        // Build a submenu for each class listing its instances
        for (class_uri, instances) in &class_instances {
            let class_encoded = encode_uri(class_uri);
            let class_label = local_name(class_uri);

            let mut items = Vec::new();

            // Header
            items.push(MenuItem {
                itype: ItemType::Info,
                display: format!("Class: {class_label}"),
                selector: String::new(),
                host: String::new(),
                port: 0,
            });
            items.push(MenuItem {
                itype: ItemType::Info,
                display: "---".to_string(),
                selector: String::new(),
                host: String::new(),
                port: 0,
            });

            for instance_uri in instances {
                let instance_encoded = encode_uri(instance_uri);
                let instance_label = local_name(instance_uri);
                items.push(MenuItem {
                    itype: ItemType::TextFile,
                    display: instance_label,
                    selector: format!("/resource/{instance_encoded}"),
                    host: self.namespace.clone(),
                    port: 0,
                });
            }

            store.add_content(
                &self.namespace,
                &format!("/class/{class_encoded}"),
                ContentNode::Menu(items),
            );
        }

        // Build root menu items for classes
        let mut root_items: Vec<MenuItem> = Vec::new();

        root_items.push(MenuItem {
            itype: ItemType::Info,
            display: "RDF Classes".to_string(),
            selector: String::new(),
            host: String::new(),
            port: 0,
        });
        root_items.push(MenuItem {
            itype: ItemType::Info,
            display: "---".to_string(),
            selector: String::new(),
            host: String::new(),
            port: 0,
        });

        for class_uri in class_instances.keys() {
            let class_encoded = encode_uri(class_uri);
            let class_label = local_name(class_uri);
            root_items.push(MenuItem {
                itype: ItemType::Menu,
                display: class_label,
                selector: format!("/class/{class_encoded}"),
                host: self.namespace.clone(),
                port: 0,
            });
        }

        Ok(root_items)
    }
}

#[async_trait]
impl SourceAdapter for RdfAdapter {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    async fn sync(&self, store: &LocalStore) -> Result<(), AdapterError> {
        store.register_namespace(&self.namespace);

        let mut root_items: Vec<MenuItem> = Vec::new();

        if let Some(ref source) = self.source {
            let content = self.load_content(source).await?;
            let graph = self.parse_graph(&content)?;
            root_items = self.populate_from_graph(&graph, store)?;
        } else if self.sparql_endpoint.is_some() {
            // No static source, but a SPARQL endpoint is available.
            // Create a minimal root menu with just the search item.
            root_items.push(MenuItem {
                itype: ItemType::Info,
                display: "SPARQL Endpoint".to_string(),
                selector: String::new(),
                host: String::new(),
                port: 0,
            });
            root_items.push(MenuItem {
                itype: ItemType::Info,
                display: "---".to_string(),
                selector: String::new(),
                host: String::new(),
                port: 0,
            });
        }

        // If a SPARQL endpoint is configured, add a Search item to the root menu
        if let Some(ref endpoint) = self.sparql_endpoint {
            root_items.push(MenuItem {
                itype: ItemType::Search,
                display: format!("SPARQL Search ({})", local_name(endpoint)),
                selector: "/sparql".to_string(),
                host: self.namespace.clone(),
                port: 0,
            });
        }

        store.add_content(&self.namespace, "", ContentNode::Menu(root_items));
        Ok(())
    }

    async fn search(&self, _selector: &str, query: &str) -> Option<Vec<MenuItem>> {
        let endpoint = self.sparql_endpoint.as_ref()?;

        let sparql_query = format!(
            r#"SELECT ?s ?label WHERE {{
  ?s ?p ?o .
  FILTER(CONTAINS(LCASE(STR(?o)), LCASE("{query}")))
  OPTIONAL {{ ?s <{RDFS_LABEL}> ?label }}
}} LIMIT 20"#,
        );

        let client = reqwest::Client::new();
        let response = client
            .post(endpoint)
            .header("Content-Type", "application/sparql-query")
            .header("Accept", "application/sparql-results+json")
            .body(sparql_query)
            .send()
            .await
            .ok()?;

        let body = response.text().await.ok()?;
        let json: serde_json::Value = serde_json::from_str(&body).ok()?;

        let bindings = json
            .get("results")?
            .get("bindings")?
            .as_array()?;

        let mut items = Vec::new();
        let mut seen = BTreeSet::new();

        for binding in bindings {
            let subject_uri = binding
                .get("s")
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            // Avoid duplicates
            if !seen.insert(subject_uri.to_string()) {
                continue;
            }

            let label = binding
                .get("label")
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| local_name_str(subject_uri));

            let encoded = encode_uri(subject_uri);

            items.push(MenuItem {
                itype: ItemType::TextFile,
                display: label.to_string(),
                selector: format!("/resource/{encoded}"),
                host: self.namespace.clone(),
                port: 0,
            });
        }

        Some(items)
    }
}

/// Encode a URI for use as a safe Gopher selector component.
///
/// Replaces `://`, `/`, and `#` with underscores.
fn encode_uri(uri: &str) -> String {
    uri.replace("://", "_").replace('/', "_").replace('#', "_")
}

/// Extract the local name from a URI (the part after the last `/` or `#`).
fn local_name(uri: &str) -> String {
    if let Some(pos) = uri.rfind('#') {
        uri[pos + 1..].to_string()
    } else if let Some(pos) = uri.rfind('/') {
        uri[pos + 1..].to_string()
    } else {
        uri.to_string()
    }
}

/// Same as `local_name` but returns a `&str` when possible.
/// Falls back to the full input if no separator is found.
fn local_name_str(uri: &str) -> &str {
    if let Some(pos) = uri.rfind('#') {
        &uri[pos + 1..]
    } else if let Some(pos) = uri.rfind('/') {
        &uri[pos + 1..]
    } else {
        uri
    }
}
