use std::net::SocketAddr;
use std::path::PathBuf;

use axum::routing::get;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use predicterm_core::db;

mod error;
mod routes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let data_path = PathBuf::from(&data_dir);

    tracing::info!("Loading data from: {}", data_path.display());
    let pool = db::create_pool(&data_path)?;
    tracing::info!("DuckDB pool initialized with enriched_trades view");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Health
        .route("/health", get(routes::health::health))
        // Spec §3.2 endpoints
        .route("/api/v1/calibration", get(routes::calibration::get_calibration))
        .route("/api/v1/maker-taker", get(routes::maker_taker::get_maker_taker))
        .route("/api/v1/categories", get(routes::categories::get_categories))
        .route("/api/v1/temporal", get(routes::temporal::get_temporal))
        .route("/api/v1/yes-no", get(routes::yes_no::get_yes_no))
        .route("/api/v1/cohorts", get(routes::cohorts::get_cohorts))
        .route("/api/v1/markets", get(routes::markets::list_markets))
        .route("/api/v1/stats/summary", get(routes::summary::get_summary))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(pool);

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("PredicTerm API listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
