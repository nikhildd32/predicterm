use anyhow::Result;
use duckdb::Connection;

use crate::models::{CategoryResponse, CategoryStats, FilterParams};
use super::build_filter_summary;

/// Category-level microstructure stats (Becker Table 2, spec §2.5)
pub fn query_categories(conn: &Connection, params: &FilterParams) -> Result<CategoryResponse> {
    let sql = r#"
        SELECT
            event_prefix AS category,
            COUNT(*) AS n_trades,
            SUM(count) AS n_contracts,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_taker_return,
            AVG((maker_won * 100.0 - maker_price) / maker_price) AS avg_maker_return,
            AVG((maker_won * 100.0 - maker_price) / maker_price) -
                AVG((taker_won * 100.0 - taker_price) / taker_price) AS gap_pp,
            AVG(POW(yes_price / 100.0 - CASE WHEN result = 'yes' THEN 1.0 ELSE 0.0 END, 2)) AS brier_score,
            -- Longshot mispricing: mispricing in 1-20 cent bucket
            AVG(CASE WHEN taker_price BETWEEN 1 AND 20
                     THEN taker_won::DOUBLE - taker_price / 100.0
                     ELSE NULL END) AS longshot_mispricing
        FROM enriched_trades
        WHERE taker_price BETWEEN 1 AND 99
          AND event_prefix IS NOT NULL
          AND event_prefix != ''
        GROUP BY event_prefix
        HAVING COUNT(*) > 1000
        ORDER BY gap_pp DESC
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

    let categories: Vec<CategoryStats> = rows.filter_map(|r| r.ok()).collect();

    Ok(CategoryResponse {
        categories,
        filters_applied: build_filter_summary(params),
    })
}
