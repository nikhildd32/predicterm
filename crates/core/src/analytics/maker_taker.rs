use anyhow::Result;
use duckdb::Connection;

use crate::models::{FilterParams, MakerTakerPoint, MakerTakerResponse};
use super::{build_filter_summary, build_where_clause};

/// Maker-taker wealth transfer by price bucket (Becker Table 1/Figure 2, spec §2.2)
pub fn query_maker_taker(conn: &Connection, params: &FilterParams) -> Result<MakerTakerResponse> {
    let bucket_width = params.bucket_width.unwrap_or(10) as i32;
    let where_clause = build_where_clause(params);

    let sql = format!(
        r#"
        WITH role_returns AS (
            SELECT
                taker_price,
                (taker_won * 100.0 - taker_price) / taker_price AS taker_return,
                (maker_won * 100.0 - maker_price) / maker_price AS maker_return,
                count AS n_contracts,
                taker_notional
            FROM enriched_trades
            WHERE {where_clause}
        )
        SELECT
            CAST(FLOOR((taker_price - 1) / {bucket_width}) * {bucket_width} + 1 AS INTEGER) AS bucket_low,
            CAST(FLOOR((taker_price - 1) / {bucket_width}) * {bucket_width} + {bucket_width} AS INTEGER) AS bucket_high,
            COUNT(*) AS n_trades,
            AVG(taker_return) AS avg_taker_return,
            AVG(maker_return) AS avg_maker_return,
            AVG(maker_return) - AVG(taker_return) AS gap_pp,
            SUM(taker_return * taker_notional) / NULLIF(SUM(taker_notional), 0) AS vw_taker_return,
            SUM(maker_return * taker_notional) / NULLIF(SUM(taker_notional), 0) AS vw_maker_return
        FROM role_returns
        GROUP BY FLOOR((taker_price - 1) / {bucket_width})
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

    // Aggregates
    let agg_sql = format!(
        r#"
        SELECT
            COUNT(*) AS total_trades,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            AVG((taker_won * 100.0 - taker_price) / taker_price) AS agg_taker_return,
            AVG((maker_won * 100.0 - maker_price) / maker_price) AS agg_maker_return
        FROM enriched_trades
        WHERE {where_clause}
        "#
    );

    let (total_trades, total_volume, agg_taker, agg_maker) =
        conn.query_row(&agg_sql, [], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, f64>(1).unwrap_or(0.0),
                row.get::<_, f64>(2).unwrap_or(0.0),
                row.get::<_, f64>(3).unwrap_or(0.0),
            ))
        })?;

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
