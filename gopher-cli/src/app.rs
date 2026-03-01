use crate::client::{BrowseItem, ContentClient};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Menu,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    GoTo,
}

struct HistoryEntry {
    path: String,
    items: Vec<BrowseItem>,
    selected: usize,
}

#[derive(Clone)]
pub struct GotoItem {
    pub path: String,
    pub display: String,
    pub depth: u16,
    pub is_dir: bool,
    pub expanded: bool,
}

pub struct App {
    client: Box<dyn ContentClient>,
    pub current_path: String,
    pub items: Vec<BrowseItem>,
    pub selected: usize,
    history: Vec<HistoryEntry>,
    pub content: String,
    pub content_scroll: u16,
    pub active_pane: Pane,
    pub mode: Mode,
    pub search_input: String,
    pub status_message: String,
    pub loading: bool,
    pub should_quit: bool,
    pub known_paths: Vec<String>,
    config_sources: Vec<String>,
    pub goto_items: Vec<GotoItem>,
    pub goto_filtered: Vec<usize>,
    pub goto_selected: usize,
}

impl App {
    pub fn new(client: Box<dyn ContentClient>, initial_path: &str, sources: Vec<String>) -> Self {
        let config_sources: Vec<String> = sources
            .into_iter()
            .map(|s| {
                if !s.contains('.') && !s.ends_with('/') {
                    format!("{}/", s)
                } else {
                    s
                }
            })
            .collect();
        let mut known_paths = config_sources.clone();
        known_paths.sort();
        known_paths.dedup();

        Self {
            client,
            current_path: initial_path.to_string(),
            items: Vec::new(),
            selected: 0,
            history: Vec::new(),
            content: String::new(),
            content_scroll: 0,
            active_pane: Pane::Menu,
            mode: Mode::Normal,
            search_input: String::new(),
            status_message: String::new(),
            loading: false,
            should_quit: false,
            known_paths,
            config_sources,
            goto_items: Vec::new(),
            goto_filtered: Vec::new(),
            goto_selected: 0,
        }
    }

    pub async fn load_current(&mut self) {
        self.loading = true;
        match self.client.browse(&self.current_path).await {
            Ok(items) => {
                self.learn_paths(&items);
                self.items = items;
                self.selected = 0;
                self.status_message.clear();
            }
            Err(e) => {
                self.items.clear();
                self.selected = 0;
                self.content = format!("Error: {}", e);
                self.status_message = format!("Error: {}", e);
            }
        }
        self.loading = false;
    }

    fn learn_paths(&mut self, items: &[BrowseItem]) {
        for item in items {
            if !matches!(item.item_type.as_str(), "1" | "0" | "7" | "h") {
                continue;
            }
            let mut p = item.path.clone();
            if item.item_type == "1" && !p.ends_with('/') {
                p.push('/');
            }
            if !p.is_empty() && !self.known_paths.contains(&p) {
                self.known_paths.push(p);
            }
        }
        self.known_paths.sort();
    }

    pub async fn open_selected(&mut self) {
        let item = match self.items.get(self.selected) {
            Some(item) => item.clone(),
            None => return,
        };

        match item.item_type.as_str() {
            "1" => {
                self.history.push(HistoryEntry {
                    path: self.current_path.clone(),
                    items: self.items.clone(),
                    selected: self.selected,
                });
                self.current_path = item.path.clone();
                self.load_current().await;
            }
            "0" | "h" => {
                self.loading = true;
                match self.client.fetch(&item.path).await {
                    Ok(text) => {
                        self.content = text;
                        self.content_scroll = 0;
                        self.active_pane = Pane::Content;
                        self.status_message.clear();
                    }
                    Err(e) => {
                        self.content = format!("Error fetching {}: {}", item.path, e);
                        self.status_message = format!("Error: {}", e);
                    }
                }
                self.loading = false;
            }
            "7" => {
                self.mode = Mode::Search;
                self.search_input.clear();
                self.status_message = format!("Search in: {}", item.path);
            }
            _ => {}
        }
    }

    pub async fn submit_search(&mut self) {
        let query = self.search_input.clone();
        self.mode = Mode::Normal;

        if query.is_empty() {
            return;
        }

        let search_path = self
            .items
            .get(self.selected)
            .filter(|item| item.item_type == "7")
            .map(|item| item.path.clone())
            .unwrap_or_else(|| self.current_path.clone());

        self.loading = true;
        match self.client.search(&search_path, &query).await {
            Ok(results) => {
                self.history.push(HistoryEntry {
                    path: self.current_path.clone(),
                    items: self.items.clone(),
                    selected: self.selected,
                });
                self.items = results;
                self.selected = 0;
                self.status_message = format!("Search: \"{}\"", query);
            }
            Err(e) => {
                self.content = format!("Search error: {}", e);
                self.status_message = format!("Error: {}", e);
            }
        }
        self.loading = false;
    }

    pub fn go_back(&mut self) {
        if let Some(entry) = self.history.pop() {
            self.current_path = entry.path;
            self.items = entry.items;
            self.selected = entry.selected;
            self.status_message.clear();
        }
    }

    pub fn move_up(&mut self) {
        match self.active_pane {
            Pane::Menu => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            Pane::Content => {
                self.content_scroll = self.content_scroll.saturating_sub(1);
            }
        }
    }

    pub fn move_down(&mut self) {
        match self.active_pane {
            Pane::Menu => {
                if !self.items.is_empty() && self.selected < self.items.len() - 1 {
                    self.selected += 1;
                }
            }
            Pane::Content => {
                self.content_scroll = self.content_scroll.saturating_add(1);
            }
        }
    }

    pub fn page_up(&mut self) {
        self.content_scroll = self.content_scroll.saturating_sub(20);
    }

    pub fn page_down(&mut self) {
        self.content_scroll = self.content_scroll.saturating_add(20);
    }

    pub fn toggle_pane(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::Menu => Pane::Content,
            Pane::Content => Pane::Menu,
        };
    }

    pub fn go_home(&mut self) {
        self.history.clear();
        self.current_path = String::new();
    }

    // --- GoTo popup ---

    pub fn enter_goto(&mut self) {
        self.mode = Mode::GoTo;
        self.search_input.clear();
        self.goto_selected = 0;

        // Build initial tree from config sources only
        let mut sources = self.config_sources.clone();
        sources.sort();
        sources.dedup();
        self.goto_items = sources
            .into_iter()
            .map(|s| {
                let is_dir = s.ends_with('/');
                GotoItem {
                    display: s.clone(),
                    path: s,
                    depth: 0,
                    is_dir,
                    expanded: false,
                }
            })
            .collect();
        self.update_goto_filter();
    }

    pub fn update_goto_filter(&mut self) {
        let query = self.search_input.to_lowercase();
        self.goto_filtered = self
            .goto_items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if query.is_empty() {
                    true
                } else {
                    item.display.to_lowercase().contains(&query)
                        || item.path.to_lowercase().contains(&query)
                }
            })
            .map(|(i, _)| i)
            .collect();

        if self.goto_selected >= self.goto_filtered.len() {
            self.goto_selected = self.goto_filtered.len().saturating_sub(1);
        }
    }

    pub fn goto_up(&mut self) {
        if self.goto_selected > 0 {
            self.goto_selected -= 1;
        }
    }

    pub fn goto_down(&mut self) {
        if !self.goto_filtered.is_empty() && self.goto_selected < self.goto_filtered.len() - 1 {
            self.goto_selected += 1;
        }
    }

    pub async fn toggle_goto_expand(&mut self) {
        let item_idx = match self.goto_filtered.get(self.goto_selected) {
            Some(&idx) => idx,
            None => return,
        };

        let item = &self.goto_items[item_idx];
        if !item.is_dir {
            return;
        }

        if item.expanded {
            // Collapse: remove consecutive children deeper than this item
            let depth = item.depth;
            let mut end = item_idx + 1;
            while end < self.goto_items.len() && self.goto_items[end].depth > depth {
                end += 1;
            }
            self.goto_items.drain((item_idx + 1)..end);
            self.goto_items[item_idx].expanded = false;
        } else {
            // Expand: browse and insert children
            let path = item.path.clone();
            let child_depth = item.depth + 1;
            if let Ok(browse_items) = self.client.browse(&path).await {
                self.learn_paths(&browse_items);
                let children: Vec<GotoItem> = browse_items
                    .iter()
                    .filter(|i| matches!(i.item_type.as_str(), "1" | "0" | "7" | "h"))
                    .map(|i| {
                        let is_dir = i.item_type == "1";
                        let mut p = i.path.clone();
                        if is_dir && !p.ends_with('/') {
                            p.push('/');
                        }
                        GotoItem {
                            display: i.display.clone(),
                            path: p,
                            depth: child_depth,
                            is_dir,
                            expanded: false,
                        }
                    })
                    .collect();
                let insert_pos = item_idx + 1;
                for (i, child) in children.into_iter().enumerate() {
                    self.goto_items.insert(insert_pos + i, child);
                }
                self.goto_items[item_idx].expanded = true;
            }
        }

        self.update_goto_filter();
    }

    pub async fn submit_goto(&mut self) {
        let path = self
            .goto_filtered
            .get(self.goto_selected)
            .map(|&idx| self.goto_items[idx].path.clone())
            .unwrap_or_else(|| self.search_input.clone());
        self.mode = Mode::Normal;

        if path.is_empty() {
            return;
        }

        self.history.push(HistoryEntry {
            path: self.current_path.clone(),
            items: self.items.clone(),
            selected: self.selected,
        });
        self.current_path = path;
        self.load_current().await;
    }

    pub fn cancel_goto(&mut self) {
        self.mode = Mode::Normal;
        self.search_input.clear();
    }
}
