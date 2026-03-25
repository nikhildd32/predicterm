use axum::extract::{Query, State};
use axum::Json;

use predicterm_core::analytics::calibration::query_calibration;
use predicterm_core::db::DbPool;
use predicterm_core::models::{CalibrationResponse, FilterParams};

use crate::error::AppError;

pub async fn get_calibration(
    State(pool): State<DbPool>,
    Query(params): Query<FilterParams>,
) -> Result<Json<CalibrationResponse>, AppError> {
    let conn = pool.lock().unwrap();
    let result = query_calibration(&conn, &params)?;
    Ok(Json(result))
}
