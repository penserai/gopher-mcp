use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;

use crate::adapters::{AdapterError, SourceAdapter};
use crate::gopher::{ItemType, MenuItem};
use crate::store::{ContentNode, LocalStore};

/// A source adapter that fetches and exposes RSS/Atom feeds through the Gopher
/// protocol hierarchy.
///
/// Feed entries become text documents, and categories become sub-menus that
/// group related entries together.
pub struct RssAdapter {
    pub namespace: String,
    pub url: String,
}

impl RssAdapter {
    pub fn new(namespace: String, url: String) -> Self {
        Self { namespace, url }
    }
}

#[async_trait]
impl SourceAdapter for RssAdapter {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    async fn sync(&self, store: &LocalStore) -> Result<(), AdapterError> {
        // Fetch the feed
        let response = reqwest::get(&self.url)
            .await
            .map_err(|e| AdapterError::Network(format!("Failed to fetch feed: {e}")))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AdapterError::Network(format!("Failed to read response body: {e}")))?;

        // Parse the feed
        let feed = feed_rs::parser::parse(bytes.as_ref())
            .map_err(|e| AdapterError::Parse(format!("Failed to parse feed: {e}")))?;

        // Register namespace in the store
        store.register_namespace(&self.namespace);

        // Collect categories across all entries.
        // BTreeMap keeps categories sorted alphabetically.
        // Maps slug -> (label, Vec<(index, entry title)>)
        let mut categories: BTreeMap<String, (String, Vec<(usize, String)>)> = BTreeMap::new();

        // Build per-entry documents and collect category mappings
        for (index, entry) in feed.entries.iter().enumerate() {
            let title = entry
                .title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_else(|| "Untitled".to_string());

            // Build document text for this entry
            let mut doc_lines = Vec::new();
            doc_lines.push(title.clone());

            if let Some(published) = &entry.published {
                doc_lines.push(format!("Published: {published}"));
            } else if let Some(updated) = &entry.updated {
                doc_lines.push(format!("Published: {updated}"));
            }

            doc_lines.push(String::new());

            // Prefer content body, fall back to summary, then default message
            let body = entry
                .content
                .as_ref()
                .and_then(|c| c.body.clone())
                .or_else(|| {
                    entry
                        .summary
                        .as_ref()
                        .map(|s| s.content.clone())
                })
                .unwrap_or_else(|| "No content available".to_string());
            doc_lines.push(body);

            // Append links
            if !entry.links.is_empty() {
                doc_lines.push(String::new());
                for link in &entry.links {
                    doc_lines.push(format!("Link: {}", link.href));
                }
            }

            store.add_content(
                &self.namespace,
                &format!("/entry/{index}"),
                ContentNode::Document(doc_lines.join("\n")),
            );

            // Collect categories for this entry
            for cat in &entry.categories {
                let label = cat.label.as_deref().unwrap_or(&cat.term);
                let slug = label.to_lowercase().replace(' ', "-");
                categories
                    .entry(slug)
                    .or_insert_with(|| (label.to_string(), Vec::new()))
                    .1
                    .push((index, title.clone()));
            }
        }

        // Build category menus
        for (slug, (label, entries)) in &categories {
            let mut items = Vec::new();

            // Header
            items.push(MenuItem {
                itype: ItemType::Info,
                display: format!("Category: {label}"),
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

            for (index, title) in entries {
                items.push(MenuItem {
                    itype: ItemType::TextFile,
                    display: title.clone(),
                    selector: format!("/entry/{index}"),
                    host: self.namespace.clone(),
                    port: 0,
                });
            }

            store.add_content(
                &self.namespace,
                &format!("/category/{slug}"),
                ContentNode::Menu(items),
            );
        }

        // Build root menu
        let mut root_items = Vec::new();

        // Feed title info line
        let feed_title = feed
            .title
            .as_ref()
            .map(|t| t.content.clone())
            .unwrap_or_else(|| "RSS Feed".to_string());
        root_items.push(MenuItem {
            itype: ItemType::Info,
            display: feed_title,
            selector: String::new(),
            host: String::new(),
            port: 0,
        });

        // Separator
        root_items.push(MenuItem {
            itype: ItemType::Info,
            display: "---".to_string(),
            selector: String::new(),
            host: String::new(),
            port: 0,
        });

        // Entry items
        for (index, entry) in feed.entries.iter().enumerate() {
            let title = entry
                .title
                .as_ref()
                .map(|t| t.content.clone())
                .unwrap_or_else(|| "Untitled".to_string());

            root_items.push(MenuItem {
                itype: ItemType::TextFile,
                display: title,
                selector: format!("/entry/{index}"),
                host: self.namespace.clone(),
                port: 0,
            });
        }

        // Category items (deduplicated slugs already handled by BTreeMap)
        // Use BTreeSet for deterministic ordering of category slugs
        let unique_slugs: BTreeSet<&String> = categories.keys().collect();
        for slug in unique_slugs {
            if let Some((label, _)) = categories.get(slug) {
                root_items.push(MenuItem {
                    itype: ItemType::Menu,
                    display: label.clone(),
                    selector: format!("/category/{slug}"),
                    host: self.namespace.clone(),
                    port: 0,
                });
            }
        }

        store.add_content(&self.namespace, "", ContentNode::Menu(root_items));

        Ok(())
    }

    async fn search(&self, _selector: &str, _query: &str) -> Option<Vec<MenuItem>> {
        None
    }
}
