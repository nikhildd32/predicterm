use anyhow::Result;
use duckdb::Connection;

use crate::models::{CohortResponse, CohortStats, FilterParams};
use super::{build_filter_summary, build_where_clause};

/// Trade-size cohort analysis (Bürgi et al. trade size distribution, spec §2.6)
pub fn query_cohorts(conn: &Connection, params: &FilterParams) -> Result<CohortResponse> {
    let where_clause = build_where_clause(params);

    let sql = format!(
        r#"
        SELECT
            CASE
                WHEN taker_notional / 100.0 < 10 THEN 'micro'
                WHEN taker_notional / 100.0 < 100 THEN 'small'
                WHEN taker_notional / 100.0 < 1000 THEN 'medium'
                ELSE 'large'
            END AS size_cohort,
            COUNT(*) AS n_trades,
            SUM(count) AS n_contracts,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_taker_return,
            AVG((maker_won * 100.0 - maker_price) / maker_price) AS avg_maker_return,
            AVG((maker_won * 100.0 - maker_price) / maker_price) -
                AVG((taker_won * 100.0 - taker_price) / taker_price) AS gap_pp
        FROM enriched_trades
        WHERE {where_clause}
        GROUP BY size_cohort
        ORDER BY CASE size_cohort
            WHEN 'micro' THEN 1
            WHEN 'small' THEN 2
            WHEN 'medium' THEN 3
            WHEN 'large' THEN 4
        END
        "#
    );

    let mut stmt = conn.prepare(&sql)?;
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

    let cohorts: Vec<CohortStats> = rows.filter_map(|r| r.ok()).collect();

    Ok(CohortResponse {
        cohorts,
        cohort_type: "size".to_string(),
        filters_applied: build_filter_summary(params),
    })
}
