use axum::extract::{Query, State};
use axum::Json;

use predicterm_core::analytics::summary::query_markets;
use predicterm_core::db::DbPool;
use predicterm_core::models::{MarketsResponse, PaginationParams};

use crate::error::AppError;

pub async fn list_markets(
    State(pool): State<DbPool>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<MarketsResponse>, AppError> {
    let conn = pool.lock().unwrap();
    let limit = params.limit.unwrap_or(50).min(500);
    let offset = params.offset.unwrap_or(0);
    let result = query_markets(
        &conn,
        limit,
        offset,
        params.search.as_deref(),
        params.status.as_deref(),
    )?;
    Ok(Json(result))
}
