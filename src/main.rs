use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{routing::post, Router as AxumRouter, Json, extract::State};
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod gopher;
mod store;
mod router;
mod mcp;
mod tls;
mod adapters;

use store::LocalStore;
use router::Router;
use mcp::{McpHandler, McpRequest};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1:8443")]
    bind: SocketAddr,

    #[arg(long, default_value = "certs/server.crt")]
    cert: PathBuf,

    #[arg(long, default_value = "certs/server.key")]
    key: PathBuf,

    #[arg(long, default_value = "certs/ca.crt")]
    client_ca: PathBuf,

    #[arg(long, default_value_t = false)]
    no_tls: bool,

    #[arg(long, default_value_t = false)]
    no_seed: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let local_store = LocalStore::new();
    if !args.no_seed {
        local_store.seed_example();
        info!("Seeded example content into 'local' namespace");
    }

    let router = Arc::new(Router::new(local_store));
    let mcp_handler = Arc::new(McpHandler::new(router));

    let app = AxumRouter::new()
        .route("/mcp", post(handle_mcp))
        .with_state(mcp_handler);

    info!("Starting gopher-mcp server on {}", args.bind);

    if args.no_tls {
        info!("TLS disabled (development mode)");
        axum::Server::bind(&args.bind)
            .serve(app.into_make_service())
            .await?;
    } else {
        info!("TLS enabled with mTLS");
        let config = tls::make_server_config(&args.cert, &args.key, &args.client_ca)?;

        use axum_server::tls_rustls::RustlsConfig;
        let config = RustlsConfig::from_config(config);

        axum_server::bind_rustls(args.bind, config)
            .serve(app.into_make_service())
            .await?;
    }

    Ok(())
}

async fn handle_mcp(
    State(handler): State<Arc<McpHandler>>,
    Json(payload): Json<McpRequest>,
) -> axum::response::Response {
    match handler.handle(payload).await {
        Some(response) => Json(response).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}
