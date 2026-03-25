use axum::extract::{Query, State};
use axum::Json;

use predicterm_core::analytics::maker_taker::query_maker_taker;
use predicterm_core::db::DbPool;
use predicterm_core::models::{FilterParams, MakerTakerResponse};

use crate::error::AppError;

pub async fn get_maker_taker(
    State(pool): State<DbPool>,
    Query(params): Query<FilterParams>,
) -> Result<Json<MakerTakerResponse>, AppError> {
    let conn = pool.lock().unwrap();
    let result = query_maker_taker(&conn, &params)?;
    Ok(Json(result))
}
