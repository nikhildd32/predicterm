use anyhow::Result;
use duckdb::Connection;

use crate::models::{FilterParams, MakerTakerPoint, MakerTakerResponse};
use super::build_filter_summary;

pub fn query_maker_taker(conn: &Connection, params: &FilterParams) -> Result<MakerTakerResponse> {
    let bucket_width = params.bucket_width.unwrap_or(10) as i32;

    let sql = format!(
        r#"
        SELECT
            CAST(FLOOR((price_cent - 1) / {bucket_width}) * {bucket_width} + 1 AS INTEGER) AS bucket_low,
            CAST(FLOOR((price_cent - 1) / {bucket_width}) * {bucket_width} + {bucket_width} AS INTEGER) AS bucket_high,
            SUM(n_trades) AS n_trades,
            SUM(avg_taker_excess_return * n_trades) / SUM(n_trades) AS avg_taker_return,
            SUM(avg_maker_excess_return * n_trades) / SUM(n_trades) AS avg_maker_return,
            SUM(avg_maker_excess_return * n_trades) / SUM(n_trades)
              - SUM(avg_taker_excess_return * n_trades) / SUM(n_trades) AS gap_pp,
            SUM(vw_taker_return_num) / NULLIF(SUM(total_notional), 0) AS vw_taker_return,
            SUM(vw_maker_return_num) / NULLIF(SUM(total_notional), 0) AS vw_maker_return
        FROM agg_calibration_per_cent
        GROUP BY FLOOR((price_cent - 1) / {bucket_width})
        ORDER BY bucket_low
        "#
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(MakerTakerPoint {
            price_bucket_low: row.get(0)?,
            price_bucket_high: row.get(1)?,
            n_trades: row.get(2)?,
            avg_taker_return: row.get(3)?,
            avg_maker_return: row.get(4)?,
            gap_pp: row.get(5)?,
            vw_taker_return: row.get::<_, f64>(6).unwrap_or(0.0),
            vw_maker_return: row.get::<_, f64>(7).unwrap_or(0.0),
        })
    })?;

    let points: Vec<MakerTakerPoint> = rows.filter_map(|r| r.ok()).collect();

    let (total_trades, total_volume, agg_taker, agg_maker) = conn.query_row(
        r#"SELECT
            SUM(n_trades),
            SUM(total_volume_usd),
            SUM(avg_taker_excess_return * n_trades) / SUM(n_trades),
            SUM(avg_maker_excess_return * n_trades) / SUM(n_trades)
        FROM agg_calibration_per_cent"#,
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1).unwrap_or(0.0), row.get::<_, f64>(2).unwrap_or(0.0), row.get::<_, f64>(3).unwrap_or(0.0))),
    )?;

    Ok(MakerTakerResponse {
        points,
        aggregate_taker_return: agg_taker,
        aggregate_maker_return: agg_maker,
        aggregate_gap_pp: agg_maker - agg_taker,
        total_trades,
        total_volume_usd: total_volume,
        filters_applied: build_filter_summary(params),
    })
}
