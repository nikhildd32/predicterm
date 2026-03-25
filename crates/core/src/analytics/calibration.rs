use anyhow::Result;
use duckdb::Connection;

use crate::models::{CalibrationPoint, CalibrationResponse, FilterParams};
use super::{build_filter_summary, build_where_clause};

/// Longshot bias calibration curve (Becker Figure 1, spec §2.1)
pub fn query_calibration(conn: &Connection, params: &FilterParams) -> Result<CalibrationResponse> {
    let bucket_width = params.bucket_width.unwrap_or(10) as i32;
    let where_clause = build_where_clause(params);

    let sql = format!(
        r#"
        SELECT
            CAST(FLOOR((taker_price - 1) / {bucket_width}) * {bucket_width} + 1 AS INTEGER) AS bucket_low,
            CAST(FLOOR((taker_price - 1) / {bucket_width}) * {bucket_width} + {bucket_width} AS INTEGER) AS bucket_high,
            COUNT(*) AS n_trades,
            SUM(count) AS n_contracts,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            AVG(taker_price / 100.0) AS implied_probability,
            AVG(taker_won::DOUBLE) AS realized_win_rate,
            AVG(taker_won::DOUBLE) - AVG(taker_price / 100.0) AS mispricing,
            AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_excess_return
        FROM enriched_trades
        WHERE {where_clause}
        GROUP BY FLOOR((taker_price - 1) / {bucket_width})
        ORDER BY bucket_low
        "#
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(CalibrationPoint {
            price_bucket_low: row.get(0)?,
            price_bucket_high: row.get(1)?,
            n_trades: row.get(2)?,
            n_contracts: row.get(3)?,
            total_volume_usd: row.get(4)?,
            implied_probability: row.get(5)?,
            realized_win_rate: row.get(6)?,
            mispricing: row.get(7)?,
            avg_excess_return: row.get(8)?,
        })
    })?;

    let points: Vec<CalibrationPoint> = rows.filter_map(|r| r.ok()).collect();

    // Brier score and MAE (spec §2.7)
    let scoring_sql = format!(
        r#"
        SELECT
            COUNT(*) AS total_trades,
            AVG(POW(yes_price / 100.0 - CASE WHEN result = 'yes' THEN 1.0 ELSE 0.0 END, 2)) AS brier_score,
            AVG(ABS(yes_price / 100.0 - CASE WHEN result = 'yes' THEN 1.0 ELSE 0.0 END)) AS mae
        FROM enriched_trades
        WHERE {where_clause}
        "#
    );

    let (total_trades, brier, mae) = conn.query_row(&scoring_sql, [], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, f64>(1).unwrap_or(0.0),
            row.get::<_, f64>(2).unwrap_or(0.0),
        ))
    })?;

    Ok(CalibrationResponse {
        points,
        overall_brier_score: brier,
        overall_mae: mae,
        total_trades,
        filters_applied: build_filter_summary(params),
    })
}
