use crate::gopher::{MenuItem, GopherClient};
use crate::store::{LocalStore, ContentNode};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Selector not found: {0} in {1}")]
    SelectorNotFound(String, String),
    #[error("Gopher error: {0}")]
    Gopher(#[from] crate::gopher::GopherError),
}

pub struct Router {
    pub local_store: LocalStore,
}

impl Router {
    pub fn new(local_store: LocalStore) -> Self {
        Router { local_store }
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
            // Local search is just filtering the menu items for now
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

    fn is_local(&self, host: &str) -> bool {
        self.local_store.has_namespace(host)
    }
}
