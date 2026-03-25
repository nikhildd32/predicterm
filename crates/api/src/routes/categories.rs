use axum::extract::{Query, State};
use axum::Json;

use predicterm_core::analytics::categories::query_categories;
use predicterm_core::db::DbPool;
use predicterm_core::models::{CategoryResponse, FilterParams};

use crate::error::AppError;

pub async fn get_categories(
    State(pool): State<DbPool>,
    Query(params): Query<FilterParams>,
) -> Result<Json<CategoryResponse>, AppError> {
    let conn = pool.lock().unwrap();
    let result = query_categories(&conn, &params)?;
    Ok(Json(result))
}
