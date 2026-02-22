mod app;
mod cli;
mod client;
mod config;
mod ui;

use std::io;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use tracing::{info, warn};

use app::{App, Mode, Pane};
use client::{ContentClient, EmbeddedClient, McpClient};
use config::TuiConfig;
use gopher_mcp_core::{LocalStore, Router};

/// Terminal browser and CLI for gopher-mcp.
///
/// Without a subcommand, launches the interactive TUI. Use subcommands
/// (browse, fetch, search, publish, delete, dump) for scripting and
/// agent workflows.
///
/// Paths use the format: namespace/selector
///   e.g.  local/welcome, feed.hackernews/, vault/notes/idea.md
///
/// Output is auto-JSON when stdout is piped (for agents). Force with --json.
#[derive(Parser, Debug)]
#[command(name = "gopher-mcp-tui", version)]
struct Args {
    /// Connect to a remote gopher-mcp server instead of the embedded engine
    #[arg(long, global = true, env = "GOPHER_MCP_URL")]
    url: Option<String>,

    /// Skip seeding example content into the 'local' namespace
    #[arg(long, global = true)]
    no_seed: bool,

    /// Force JSON output (auto-enabled when stdout is piped)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Launch the interactive terminal browser
    Tui {
        /// Initial path to browse
        #[arg(default_value = "")]
        path: String,
    },

    /// List items at a path (default: root listing of all namespaces)
    Browse {
        /// Path to browse (e.g., local/, feed.hackernews/)
        #[arg(default_value = "")]
        path: String,
    },

    /// Retrieve a document's text content
    Fetch {
        /// Document path (e.g., local/welcome, feed.hackernews/entry/0)
        path: String,
    },

    /// Search within a namespace or path
    Search {
        /// Path scope for search (e.g., local/, feed.hackernews/)
        path: String,

        /// Search query
        query: String,
    },

    /// Write or update a document (reads stdin if no --content)
    Publish {
        /// Target path (e.g., vault/notes/idea.md)
        path: String,

        /// Content to publish; reads stdin if omitted
        #[arg(long)]
        content: Option<String>,
    },

    /// Delete a document or directory
    Delete {
        /// Path to delete (e.g., vault/notes/idea.md)
        path: String,
    },

    /// Recursively copy documents from a source into a writable namespace
    Dump {
        /// Source path to walk (e.g., feed.hackernews/, rdf.demo/)
        source: String,

        /// Writable destination prefix (e.g., vault/mirrors/hn)
        destination: String,

        /// Maximum menu depth to recurse
        #[arg(long, default_value_t = 3)]
        max_depth: u32,
    },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Tracing to stderr — never pollutes stdout
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_ansi(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let config = TuiConfig::load();
    let json = cli::use_json(args.json);

    let result = match args.command {
        None | Some(Command::Tui { .. }) => {
            let path = match &args.command {
                Some(Command::Tui { path }) => path.clone(),
                _ => String::new(),
            };
            let client = create_client(&args, &config).await;

            let mut sources = config.sources.clone();
            sources.extend(config.adapter_namespaces());
            sources.sort();
            sources.dedup();

            run_tui(client, &path, sources).await
        }
        Some(ref cmd) => {
            let client = create_client(&args, &config).await;
            run_cli(cmd, client.as_ref(), json).await
        }
    };

    if let Err(e) = result {
        cli::handle_error(e, json);
    }
}

async fn run_cli(cmd: &Command, client: &dyn ContentClient, json: bool) -> Result<()> {
    match cmd {
        Command::Browse { path } => cli::browse(client, path, json).await,
        Command::Fetch { path } => cli::fetch(client, path, json).await,
        Command::Search { path, query } => cli::search(client, path, query, json).await,
        Command::Publish { path, content } => {
            cli::publish(client, path, content.clone(), json).await
        }
        Command::Delete { path } => cli::delete(client, path, json).await,
        Command::Dump {
            source,
            destination,
            max_depth,
        } => cli::dump(client, source, destination, *max_depth, json).await,
        Command::Tui { .. } => unreachable!(),
    }
}

async fn create_client(args: &Args, config: &TuiConfig) -> Box<dyn ContentClient> {
    let url = args.url.clone().or(config.url.clone());
    if let Some(url) = url {
        info!(url = %url, "Connecting to remote server");
        Box::new(McpClient::new(&url))
    } else {
        info!("Starting embedded engine");
        let local_store = LocalStore::new();
        if !args.no_seed {
            local_store.seed_example();
            info!("Seeded example content into 'local' namespace");
        }

        let mut router = Router::new(local_store);

        match config::create_adapters(config) {
            Ok(adapters) => {
                for adapter in adapters {
                    info!(namespace = %adapter.namespace(), "Syncing adapter");
                    if let Err(e) = adapter.sync(&router.local_store).await {
                        warn!(
                            namespace = %adapter.namespace(),
                            error = %e,
                            "Failed to sync adapter, skipping"
                        );
                        continue;
                    }
                    info!(namespace = %adapter.namespace(), "Adapter synced");
                    router.register_adapter(adapter);
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to create adapters");
            }
        }

        Box::new(EmbeddedClient::new(Arc::new(router)))
    }
}

async fn run_tui(
    client: Box<dyn ContentClient>,
    path: &str,
    sources: Vec<String>,
) -> Result<()> {
    let mut app = App::new(client, path, sources);

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|frame| ui::draw(frame, &app))?;
    app.load_current().await;

    let result = run_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.mode {
                    Mode::Normal => handle_normal_key(app, key).await,
                    Mode::Search => handle_search_key(app, key).await,
                    Mode::GoTo => handle_goto_key(app, key).await,
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

async fn handle_normal_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Enter => app.open_selected().await,
        KeyCode::Backspace | KeyCode::Char('b') => app.go_back(),
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.search_input.clear();
        }
        KeyCode::Char(':') => app.enter_goto(),
        KeyCode::Tab => app.toggle_pane(),
        KeyCode::Home => {
            app.go_home();
            app.load_current().await;
        }
        KeyCode::Char(' ') if app.active_pane == Pane::Content => app.page_down(),
        KeyCode::PageUp => app.page_up(),
        KeyCode::PageDown => app.page_down(),
        _ => {}
    }
}

async fn handle_goto_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => app.submit_goto().await,
        KeyCode::Esc => app.cancel_goto(),
        KeyCode::Up => app.goto_up(),
        KeyCode::Down => app.goto_down(),
        KeyCode::Tab => app.toggle_goto_expand().await,
        KeyCode::Backspace => {
            app.search_input.pop();
            app.update_goto_filter();
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
            app.update_goto_filter();
        }
        _ => {}
    }
}

async fn handle_search_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => app.submit_search().await,
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.search_input.clear();
        }
        KeyCode::Backspace => {
            app.search_input.pop();
        }
        KeyCode::Char(c) => {
            app.search_input.push(c);
        }
        _ => {}
    }
}
