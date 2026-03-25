use axum::extract::State;
use axum::Json;

use predicterm_core::analytics::summary::query_summary;
use predicterm_core::db::DbPool;
use predicterm_core::models::SummaryStats;

use crate::error::AppError;

pub async fn get_summary(
    State(pool): State<DbPool>,
) -> Result<Json<SummaryStats>, AppError> {
    let conn = pool.lock().unwrap();
    let result = query_summary(&conn)?;
    Ok(Json(result))
}
