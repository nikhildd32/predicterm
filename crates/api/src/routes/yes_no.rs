use axum::extract::{Query, State};
use axum::Json;

use predicterm_core::analytics::yes_no::query_yes_no;
use predicterm_core::db::DbPool;
use predicterm_core::models::{FilterParams, YesNoResponse};

use crate::error::AppError;

pub async fn get_yes_no(
    State(pool): State<DbPool>,
    Query(params): Query<FilterParams>,
) -> Result<Json<YesNoResponse>, AppError> {
    let conn = pool.lock().unwrap();
    let result = query_yes_no(&conn, &params)?;
    Ok(Json(result))
}
