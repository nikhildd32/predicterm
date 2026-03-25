use anyhow::Result;
use duckdb::Connection;

use crate::models::{FilterParams, StructuralBreak, TemporalPoint, TemporalResponse};
use super::build_filter_summary;

pub fn query_temporal(
    conn: &Connection,
    params: &FilterParams,
    granularity: &str,
) -> Result<TemporalResponse> {
    let table = match granularity {
        "monthly" => "agg_temporal_month",
        _ => "agg_temporal_quarter",
    };

    let sql = format!(
        "SELECT period_start, period, n_trades, total_volume_usd,
                avg_taker_return, avg_maker_return,
                avg_maker_return - avg_taker_return AS gap_pp,
                longshot_volume_share
         FROM {table}
         ORDER BY period_start"
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(TemporalPoint {
            period_start: row.get(0)?,
            period: row.get(1)?,
            n_trades: row.get(2)?,
            total_volume_usd: row.get(3)?,
            avg_taker_return: row.get(4)?,
            avg_maker_return: row.get(5)?,
            gap_pp: row.get(6)?,
            longshot_volume_share: row.get::<_, f64>(7).unwrap_or(0.0),
        })
    })?;

    let series: Vec<TemporalPoint> = rows.filter_map(|r| r.ok()).collect();

    // Structural break: pre/post Oct 2024
    let break_sql = format!(
        "SELECT
            SUM(CASE WHEN period_start < '2024-10-01' THEN (avg_maker_return - avg_taker_return) * n_trades ELSE 0 END)
                / NULLIF(SUM(CASE WHEN period_start < '2024-10-01' THEN n_trades ELSE 0 END), 0) AS pre_gap,
            SUM(CASE WHEN period_start >= '2024-10-01' THEN (avg_maker_return - avg_taker_return) * n_trades ELSE 0 END)
                / NULLIF(SUM(CASE WHEN period_start >= '2024-10-01' THEN n_trades ELSE 0 END), 0) AS post_gap
         FROM {table}"
    );

    let (pre_gap, post_gap): (f64, f64) = conn.query_row(&break_sql, [], |row| {
        Ok((
            row.get::<_, f64>(0).unwrap_or(0.0),
            row.get::<_, f64>(1).unwrap_or(0.0),
        ))
    })?;

    Ok(TemporalResponse {
        series,
        granularity: granularity.to_string(),
        structural_break: Some(StructuralBreak {
            breakpoint: "2024-10-01".to_string(),
            pre_gap_pp: pre_gap,
            post_gap_pp: post_gap,
            swing_pp: post_gap - pre_gap,
        }),
        filters_applied: build_filter_summary(params),
    })
}
