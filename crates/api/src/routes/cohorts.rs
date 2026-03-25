use axum::extract::{Query, State};
use axum::Json;

use predicterm_core::analytics::cohorts::query_cohorts;
use predicterm_core::db::DbPool;
use predicterm_core::models::{CohortResponse, FilterParams};

use crate::error::AppError;

pub async fn get_cohorts(
    State(pool): State<DbPool>,
    Query(params): Query<FilterParams>,
) -> Result<Json<CohortResponse>, AppError> {
    let conn = pool.lock().unwrap();
    let result = query_cohorts(&conn, &params)?;
    Ok(Json(result))
}
