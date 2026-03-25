use anyhow::Result;
use duckdb::Connection;

use crate::models::{CalibrationPoint, CalibrationResponse, FilterParams};
use super::build_filter_summary;

pub fn query_calibration(conn: &Connection, params: &FilterParams) -> Result<CalibrationResponse> {
    let bucket_width = params.bucket_width.unwrap_or(10) as i32;

    let sql = format!(
        r#"
        SELECT
            CAST(FLOOR((price_cent - 1) / {bucket_width}) * {bucket_width} + 1 AS INTEGER) AS bucket_low,
            CAST(FLOOR((price_cent - 1) / {bucket_width}) * {bucket_width} + {bucket_width} AS INTEGER) AS bucket_high,
            SUM(n_trades) AS n_trades,
            SUM(n_contracts) AS n_contracts,
            SUM(total_volume_usd) AS total_volume_usd,
            SUM(implied_probability * n_trades) / SUM(n_trades) AS implied_probability,
            SUM(realized_win_rate * n_trades) / SUM(n_trades) AS realized_win_rate,
            SUM(mispricing * n_trades) / SUM(n_trades) AS mispricing,
            SUM(avg_taker_excess_return * n_trades) / SUM(n_trades) AS avg_excess_return
        FROM agg_calibration_per_cent
        GROUP BY FLOOR((price_cent - 1) / {bucket_width})
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

    let (total_trades, brier, mae) = conn.query_row(
        "SELECT SUM(n_trades), SUM(brier * n_trades) / SUM(n_trades), SUM(mae * n_trades) / SUM(n_trades) FROM agg_calibration_per_cent",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1).unwrap_or(0.0), row.get::<_, f64>(2).unwrap_or(0.0))),
    )?;

    Ok(CalibrationResponse {
        points,
        overall_brier_score: brier,
        overall_mae: mae,
        total_trades,
        filters_applied: build_filter_summary(params),
    })
}
