use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::adapters::{AdapterError, SourceAdapter};
use crate::gopher::{GopherClient, ItemType, MenuItem};
use crate::store::{ContentNode, LocalStore};

/// Binary file extensions — files with these extensions are served as type 9 (Binary)
/// rather than type 0 (TextFile).
const BINARY_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "zip", "tar", "gz", "exe", "bin", "pdf",
];

/// A source adapter that serves content from a local filesystem directory tree.
///
/// The adapter recursively walks a root directory and maps it into the Gopher
/// namespace. Directories become menus (type 1), text files become documents
/// (type 0), and binary files are listed as type 9.
///
/// If a `.gophermap` file exists in a directory it is parsed as a gophermap and
/// used as the menu for that directory instead of auto-generating one.
///
/// When `writable` is true, the adapter supports `publish` and `delete`
/// operations, turning the directory into an agent-writable vault.
pub struct FsAdapter {
    namespace: String,
    root: PathBuf,
    extensions: Option<Vec<String>>,
    writable: bool,
}

impl FsAdapter {
    /// Create a new `FsAdapter`.
    ///
    /// # Arguments
    ///
    /// * `namespace` — Unique namespace under which the content will be registered.
    /// * `root` — Path to the root directory to serve. Must exist and be a directory
    ///   (unless `writable` is true, in which case it will be created).
    /// * `extensions` — Optional list of file extensions to include (e.g. `["txt", "md"]`).
    ///   When `None`, all files are included.
    /// * `writable` — Whether publish/delete operations are allowed.
    ///
    /// # Errors
    ///
    /// Returns `AdapterError::Config` if `root` does not exist (and is not writable)
    /// or is not a directory.
    pub fn new(
        namespace: String,
        root: PathBuf,
        extensions: Option<Vec<String>>,
        writable: bool,
    ) -> Result<Self, AdapterError> {
        if !root.exists() {
            if writable {
                std::fs::create_dir_all(&root)?;
            } else {
                return Err(AdapterError::Config(format!(
                    "Root path does not exist: {}",
                    root.display()
                )));
            }
        }
        if !root.is_dir() {
            return Err(AdapterError::Config(format!(
                "Root path is not a directory: {}",
                root.display()
            )));
        }

        Ok(Self {
            namespace,
            root,
            extensions,
            writable,
        })
    }

    /// Determine whether a file extension marks the file as binary.
    fn is_binary_extension(ext: &str) -> bool {
        BINARY_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str())
    }

    /// Check whether a file should be included based on the extensions filter.
    fn should_include_file(&self, path: &Path) -> bool {
        match &self.extensions {
            None => true,
            Some(exts) => {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    exts.iter().any(|e| {
                        let e = e.strip_prefix('.').unwrap_or(e);
                        e.eq_ignore_ascii_case(ext)
                    })
                } else {
                    // Files without an extension are excluded when a filter is set
                    false
                }
            }
        }
    }

    /// Convert a filesystem path to a Gopher selector relative to the root.
    /// The root directory itself maps to `""`, subdirectories to `/subdir`, etc.
    fn path_to_selector(&self, path: &Path) -> String {
        let rel = path.strip_prefix(&self.root).unwrap_or(path);
        if rel.as_os_str().is_empty() {
            String::new()
        } else {
            format!("/{}", rel.display())
        }
    }

    /// Convert a selector back to a filesystem path, with path traversal protection.
    fn selector_to_path(&self, selector: &str) -> Result<PathBuf, AdapterError> {
        // Reject any selector containing `..` components
        if selector.split('/').any(|seg| seg == "..") {
            return Err(AdapterError::PathTraversal(selector.to_string()));
        }

        let relative = selector.strip_prefix('/').unwrap_or(selector);
        let candidate = self.root.join(relative);

        // Canonicalize what exists of the path to catch symlink escapes.
        // For new files, canonicalize the parent directory instead.
        let canonical = if candidate.exists() {
            candidate.canonicalize()?
        } else if let Some(parent) = candidate.parent() {
            if parent.exists() {
                parent.canonicalize()?.join(candidate.file_name().unwrap_or_default())
            } else {
                // Parent doesn't exist yet — that's OK for publish (we'll create it).
                // Walk up until we find an existing ancestor.
                let mut ancestor = parent.to_path_buf();
                let mut suffix = vec![candidate.file_name().unwrap_or_default().to_os_string()];
                while !ancestor.exists() {
                    if let Some(name) = ancestor.file_name() {
                        suffix.push(name.to_os_string());
                    }
                    if !ancestor.pop() {
                        break;
                    }
                }
                let mut result = ancestor.canonicalize()?;
                for component in suffix.into_iter().rev() {
                    result.push(component);
                }
                result
            }
        } else {
            return Err(AdapterError::PathTraversal(selector.to_string()));
        };

        let canon_root = self.root.canonicalize()?;
        if !canonical.starts_with(&canon_root) {
            return Err(AdapterError::PathTraversal(selector.to_string()));
        }

        Ok(candidate)
    }

    /// Walk the directory tree and populate the store.
    fn walk_and_populate(&self, store: &LocalStore) -> Result<(), AdapterError> {
        let mut queue: VecDeque<PathBuf> = VecDeque::new();
        queue.push_back(self.root.clone());

        while let Some(dir) = queue.pop_front() {
            self.process_directory(&dir, store, &mut queue)?;
        }

        Ok(())
    }

    /// Process a single directory: read its entries, build a menu, and recurse.
    fn process_directory(
        &self,
        dir: &Path,
        store: &LocalStore,
        queue: &mut VecDeque<PathBuf>,
    ) -> Result<(), AdapterError> {
        let selector = self.path_to_selector(dir);

        // Read directory entries
        let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());

        // Check for .gophermap
        let gophermap_path = dir.join(".gophermap");
        if gophermap_path.is_file() {
            let content = std::fs::read_to_string(&gophermap_path)?;
            let items = GopherClient::parse_menu_lines(&content);
            store.add_content(&self.namespace, &selector, ContentNode::Menu(items));
        } else {
            // Auto-generate menu from directory entries
            let menu_items = self.build_menu_items(&entries);
            store.add_content(&self.namespace, &selector, ContentNode::Menu(menu_items));
        }

        // Enqueue subdirectories and process files for content
        for entry in &entries {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                queue.push_back(path);
            } else if path.is_file() && self.should_include_file(&path) {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");

                // Only store content for text files; binary files are listed but not stored as Documents
                if !Self::is_binary_extension(ext) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let file_selector = self.path_to_selector(&path);
                        store.add_content(
                            &self.namespace,
                            &file_selector,
                            ContentNode::Document(content),
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Build menu items from directory entries (shared by process_directory and refresh).
    fn build_menu_items(&self, entries: &[std::fs::DirEntry]) -> Vec<MenuItem> {
        let mut menu_items: Vec<MenuItem> = Vec::new();

        for entry in entries {
            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if name.starts_with('.') {
                continue;
            }

            let entry_selector = self.path_to_selector(&path);

            if path.is_dir() {
                menu_items.push(MenuItem {
                    itype: ItemType::Menu,
                    display: name.to_string(),
                    selector: entry_selector,
                    host: self.namespace.clone(),
                    port: 0,
                });
            } else if path.is_file() && self.should_include_file(&path) {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");

                let itype = if Self::is_binary_extension(ext) {
                    ItemType::Binary
                } else {
                    ItemType::TextFile
                };

                menu_items.push(MenuItem {
                    itype,
                    display: name.to_string(),
                    selector: entry_selector,
                    host: self.namespace.clone(),
                    port: 0,
                });
            }
        }

        menu_items
    }

    /// Re-scan a single directory and update its menu in the store.
    fn refresh_directory_menu(&self, dir: &Path, store: &LocalStore) -> Result<(), AdapterError> {
        let selector = self.path_to_selector(dir);

        // If a .gophermap exists, honour it
        let gophermap_path = dir.join(".gophermap");
        if gophermap_path.is_file() {
            let content = std::fs::read_to_string(&gophermap_path)?;
            let items = GopherClient::parse_menu_lines(&content);
            store.add_content(&self.namespace, &selector, ContentNode::Menu(items));
            return Ok(());
        }

        let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());

        let menu_items = self.build_menu_items(&entries);
        store.add_content(&self.namespace, &selector, ContentNode::Menu(menu_items));

        Ok(())
    }

    /// Walk from `from` up to the root, refreshing the menu for each directory.
    fn refresh_ancestor_menus(&self, from: &Path, store: &LocalStore) -> Result<(), AdapterError> {
        let canon_root = self.root.canonicalize()?;
        let mut dir = from.to_path_buf();

        loop {
            if dir.is_dir() {
                self.refresh_directory_menu(&dir, store)?;
            }
            if dir.canonicalize().map_or(false, |c| c == canon_root) {
                break;
            }
            if !dir.pop() {
                break;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl SourceAdapter for FsAdapter {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    async fn sync(&self, store: &LocalStore) -> Result<(), AdapterError> {
        store.register_namespace(&self.namespace);
        self.walk_and_populate(store)?;
        Ok(())
    }

    async fn search(&self, _selector: &str, _query: &str) -> Option<Vec<MenuItem>> {
        None
    }

    fn is_writable(&self) -> bool {
        self.writable
    }

    async fn publish(&self, store: &LocalStore, selector: &str, content: &str) -> Result<(), AdapterError> {
        if !self.writable {
            return Err(AdapterError::NotWritable(self.namespace.clone()));
        }

        let path = self.selector_to_path(selector)?;

        // Create parent directories as needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write the file
        std::fs::write(&path, content)?;

        // Update the store: add the document
        store.add_content(&self.namespace, selector, ContentNode::Document(content.to_string()));

        // Refresh menus from the file's parent up to the root
        if let Some(parent) = path.parent() {
            self.refresh_ancestor_menus(parent, store)?;
        }

        Ok(())
    }

    async fn delete(&self, store: &LocalStore, selector: &str) -> Result<(), AdapterError> {
        if !self.writable {
            return Err(AdapterError::NotWritable(self.namespace.clone()));
        }

        let path = self.selector_to_path(selector)?;

        if !path.exists() {
            return Err(AdapterError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path not found: {}", path.display()),
            )));
        }

        if path.is_dir() {
            // Remove all store entries under this prefix
            let prefix = if selector.ends_with('/') {
                selector.to_string()
            } else {
                format!("{}/", selector)
            };
            for key in store.selectors_with_prefix(&self.namespace, &prefix) {
                store.remove_content(&self.namespace, &key);
            }
            // Also remove the directory's own menu entry
            store.remove_content(&self.namespace, selector);

            std::fs::remove_dir_all(&path)?;
        } else {
            store.remove_content(&self.namespace, selector);
            std::fs::remove_file(&path)?;
        }

        // Refresh ancestor menus
        if let Some(parent) = path.parent() {
            self.refresh_ancestor_menus(parent, store)?;
        }

        Ok(())
    }
}
