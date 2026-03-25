use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use predicterm_core::analytics::temporal::query_temporal;
use predicterm_core::db::DbPool;
use predicterm_core::models::{FilterParams, TemporalResponse};

use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct TemporalQuery {
    #[serde(flatten)]
    pub filters: FilterParams,
    pub granularity: Option<String>,
}

pub async fn get_temporal(
    State(pool): State<DbPool>,
    Query(params): Query<TemporalQuery>,
) -> Result<Json<TemporalResponse>, AppError> {
    let conn = pool.lock().unwrap();
    let granularity = params.granularity.as_deref().unwrap_or("quarterly");
    let result = query_temporal(&conn, &params.filters, granularity)?;
    Ok(Json(result))
}
