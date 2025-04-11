mod routes;
mod storage;

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Request},
    handler::Handler,
    http::{HeaderValue, header},
    middleware::Next,
    routing::{get, head, post},
};
use bytesize::ByteSize;
use clap::Parser;
use clap_duration::duration_range_value_parse;
use dotenvy::dotenv;
use duration_human::{DurationHuman, DurationHumanValidator};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use storage::StorageProvider;
use tokio::{net::TcpListener, signal};
use tower_http::{
    catch_panic::CatchPanicLayer,
    normalize_path::NormalizePathLayer,
    trace::{self, TraceLayer},
};
use tracing::{Level, debug, info};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[clap(author, about, version)]
struct Arguments {
    /// Internet socket address that the server should be ran on.
    #[arg(
        long = "address",
        env = "XFER_SERVER_ADDRESS",
        default_value = "127.0.0.1:8255"
    )]
    address: SocketAddr,

    /// The directory where data should be stored.
    ///
    /// CAUTION: This directory should not be used for anything else as it and all subdirectories will be automatically managed.
    #[clap(
        long = "data-path", 
        env = "XFER_SERVER_DATA_DIRECTORY",
        default_value = dirs::data_local_dir().unwrap().join(env!("CARGO_PKG_NAME")).into_os_string()
    )]
    data_directory: PathBuf,

    /// Amount of time after-upload before a transfer is automatically deleted from storage.
    ///
    /// Upload expiry time will be sent to clients upon upload with the X-Xfer-ExpiresAt header.
    #[clap(long = "transfer-expire-after", env = "XFER_SERVER_TRANSFER_EXPIRE_AFTER", default_value="1h", value_parser = duration_range_value_parse!(min: 1min, max: 31days))]
    transfer_expire_after: DurationHuman,

    /// The maximum transfer size that is permitted.
    #[clap(
        long = "transfer-max-size",
        env = "XFER_SERVER_TRANSFER_MAX_SIZE",
        default_value = "50MB"
    )]
    transfer_max_size: ByteSize,
}

#[derive(Clone)]
struct AppState {
    storage_provider: Arc<StorageProvider>,
    transfer_expire_after: Duration,
    transfer_max_size: ByteSize,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info")))
        .init();
    let args = Arguments::parse();

    let storage = Arc::new(StorageProvider::new(
        args.data_directory.join("transfers"),
        Duration::from(&args.transfer_expire_after),
    )?);

    let router = Router::new()
        .route("/", get(routes::index_handler))
        .route("/configuration", get(routes::configuration_handler))
        .route("/transfer/{id}", post(routes::upload_handler))
        .route(
            "/transfer/{id}",
            get(routes::download_get_handler.layer(DefaultBodyLimit::max(
                args.transfer_max_size
                    .0
                    .try_into()
                    .context("transfer limit does not fit into usize")?,
            ))),
        )
        .route("/transfer/{id}", head(routes::download_head_handler))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(CatchPanicLayer::new())
        .layer(axum::middleware::from_fn(
            async |req: Request, next: Next| {
                let mut res = next.run(req).await;
                let res_headers = res.headers_mut();
                res_headers.insert(
                    header::SERVER,
                    HeaderValue::from_static(env!("CARGO_PKG_NAME")),
                );
                res_headers.insert("X-Robots-Tag", HeaderValue::from_static("none"));
                res
            },
        ))
        .with_state(AppState {
            storage_provider: Arc::clone(&storage),
            transfer_expire_after: Duration::from(&args.transfer_expire_after),
            transfer_max_size: args.transfer_max_size,
        });

    let storage_clone = Arc::clone(&storage);
    tokio::spawn(async move {
        loop {
            debug!("Running check to find expired transfers");
            storage_clone.remove_expired_transfers().unwrap();
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });

    let tcp_listener = TcpListener::bind(args.address).await?;
    info!(
        "\nInternal server started\n* Listening on: http://{}",
        args.address,
    );
    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// https://github.com/tokio-rs/axum/blob/15917c6dbcb4a48707a20e9cfd021992a279a662/examples/graceful-shutdown/src/main.rs#L55
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
