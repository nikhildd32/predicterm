use anyhow::Result;
use duckdb::Connection;

use crate::models::{CohortResponse, CohortStats, FilterParams};
use super::build_filter_summary;

pub fn query_cohorts(conn: &Connection, params: &FilterParams) -> Result<CohortResponse> {
    let sql = r#"
        SELECT
            size_cohort, n_trades, n_contracts, total_volume_usd,
            avg_taker_return, avg_maker_return,
            avg_maker_return - avg_taker_return AS gap_pp
        FROM agg_cohorts
        ORDER BY CASE size_cohort
            WHEN 'micro' THEN 1 WHEN 'small' THEN 2
            WHEN 'medium' THEN 3 WHEN 'large' THEN 4 END
    "#;

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(CohortStats {
            cohort_label: row.get(0)?,
            n_trades: row.get(1)?,
            n_contracts: row.get(2)?,
            total_volume_usd: row.get(3)?,
            avg_taker_return: row.get(4)?,
            avg_maker_return: row.get(5)?,
            gap_pp: row.get(6)?,
        })
    })?;

    Ok(CohortResponse {
        cohorts: rows.filter_map(|r| r.ok()).collect(),
        cohort_type: "size".to_string(),
        filters_applied: build_filter_summary(params),
    })
}
