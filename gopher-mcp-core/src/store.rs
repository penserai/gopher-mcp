use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::gopher::{MenuItem, ItemType};

#[derive(Debug, Clone)]
pub enum ContentNode {
    Menu(Vec<MenuItem>),
    Document(String),
}

pub struct LocalStore {
    // namespace -> selector -> ContentNode
    pub content: Arc<RwLock<HashMap<String, HashMap<String, ContentNode>>>>,
}

impl LocalStore {
    pub fn new() -> Self {
        LocalStore {
            content: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_namespace(&self, namespace: &str) {
        let mut content = self.content.write().unwrap();
        content.entry(namespace.to_string()).or_insert_with(HashMap::new);
    }

    pub fn add_content(&self, namespace: &str, selector: &str, node: ContentNode) {
        let mut content = self.content.write().unwrap();
        if let Some(ns_map) = content.get_mut(namespace) {
            ns_map.insert(selector.to_string(), node);
        }
    }

    pub fn get_content(&self, namespace: &str, selector: &str) -> Option<ContentNode> {
        let content = self.content.read().unwrap();
        content.get(namespace)?.get(selector).cloned()
    }

    pub fn has_namespace(&self, name: &str) -> bool {
        let content = self.content.read().unwrap();
        content.contains_key(name)
    }

    pub fn seed_example(&self) {
        self.register_namespace("local");
        
        let root_menu = vec![
            MenuItem {
                itype: ItemType::TextFile,
                display: "Welcome to gopher-mcp".to_string(),
                selector: "/welcome".to_string(),
                host: "local".to_string(),
                port: 0,
            },
            MenuItem {
                itype: ItemType::Info,
                display: "-----------------------".to_string(),
                selector: String::new(),
                host: String::new(),
                port: 0,
            },
             MenuItem {
                itype: ItemType::Menu,
                display: "Submenu Example".to_string(),
                selector: "/sub".to_string(),
                host: "local".to_string(),
                port: 0,
            },
        ];

        self.add_content("local", "", ContentNode::Menu(root_menu));
        self.add_content("local", "/welcome", ContentNode::Document("This is a local document served by gopher-mcp.\nContent here is served directly from the local store.".to_string()));
        
        let sub_menu = vec![
            MenuItem {
                itype: ItemType::TextFile,
                display: "Back to root".to_string(),
                selector: "".to_string(),
                host: "local".to_string(),
                port: 0,
            },
             MenuItem {
                itype: ItemType::TextFile,
                display: "Deep document".to_string(),
                selector: "/sub/deep".to_string(),
                host: "local".to_string(),
                port: 0,
            },
        ];
        
        self.add_content("local", "/sub", ContentNode::Menu(sub_menu));
        self.add_content("local", "/sub/deep", ContentNode::Document("This is a document deep in the local hierarchy.".to_string()));
    }
}
