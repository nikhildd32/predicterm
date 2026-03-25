use anyhow::Result;
use duckdb::Connection;

use crate::models::{CategoryResponse, CategoryStats, FilterParams};
use super::build_filter_summary;

pub fn query_categories(conn: &Connection, params: &FilterParams) -> Result<CategoryResponse> {
    let sql = r#"
        SELECT
            category,
            n_trades,
            n_contracts,
            total_volume_usd,
            avg_taker_return,
            avg_maker_return,
            avg_maker_return - avg_taker_return AS gap_pp,
            brier_score,
            COALESCE(longshot_mispricing, 0) AS longshot_mispricing
        FROM agg_categories
        ORDER BY (avg_maker_return - avg_taker_return) DESC
    "#;

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(CategoryStats {
            category: row.get(0)?,
            n_trades: row.get(1)?,
            n_contracts: row.get(2)?,
            total_volume_usd: row.get(3)?,
            avg_taker_return: row.get(4)?,
            avg_maker_return: row.get(5)?,
            gap_pp: row.get(6)?,
            brier_score: row.get::<_, f64>(7).unwrap_or(0.0),
            longshot_mispricing: row.get::<_, f64>(8).unwrap_or(0.0),
        })
    })?;

    Ok(CategoryResponse {
        categories: rows.filter_map(|r| r.ok()).collect(),
        filters_applied: build_filter_summary(params),
    })
}
