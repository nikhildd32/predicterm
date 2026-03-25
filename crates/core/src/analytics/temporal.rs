use anyhow::Result;
use duckdb::Connection;

use crate::models::{FilterParams, StructuralBreak, TemporalPoint, TemporalResponse};
use super::{build_filter_summary, build_where_clause};

/// Temporal evolution of maker-taker returns (Becker temporal analysis, spec §2.4)
pub fn query_temporal(
    conn: &Connection,
    params: &FilterParams,
    granularity: &str,
) -> Result<TemporalResponse> {
    let where_clause = build_where_clause(params);

    let trunc_fn = match granularity {
        "monthly" => "trade_month",
        _ => "trade_quarter",
    };

    let sql = format!(
        r#"
        SELECT
            STRFTIME({trunc_fn}, '%Y-%m-%d') AS period_start,
            STRFTIME({trunc_fn}, '%Y-%m') AS period,
            COUNT(*) AS n_trades,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_taker_return,
            AVG((maker_won * 100.0 - maker_price) / maker_price) AS avg_maker_return,
            AVG((maker_won * 100.0 - maker_price) / maker_price) -
                AVG((taker_won * 100.0 - taker_price) / taker_price) AS gap_pp,
            SUM(CASE WHEN taker_price BETWEEN 1 AND 20 THEN taker_notional ELSE 0 END)::DOUBLE /
                NULLIF(SUM(taker_notional), 0) AS longshot_volume_share
        FROM enriched_trades
        WHERE {where_clause}
        GROUP BY {trunc_fn}
        ORDER BY period_start
        "#
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

    // Pre/post structural break (Oct 2024, spec §2.4.2)
    let break_sql = format!(
        r#"
        SELECT
            CASE WHEN trade_date < '2024-10-01' THEN 'pre' ELSE 'post' END AS era,
            AVG((maker_won * 100.0 - maker_price) / maker_price) -
                AVG((taker_won * 100.0 - taker_price) / taker_price) AS gap_pp
        FROM enriched_trades
        WHERE {where_clause}
        GROUP BY CASE WHEN trade_date < '2024-10-01' THEN 'pre' ELSE 'post' END
        "#
    );

    let mut pre_gap = 0.0_f64;
    let mut post_gap = 0.0_f64;

    let mut break_stmt = conn.prepare(&break_sql)?;
    let mut break_rows = break_stmt.query([])?;
    while let Some(row) = break_rows.next()? {
        let era: String = row.get(0)?;
        let gap: f64 = row.get(1)?;
        if era == "pre" {
            pre_gap = gap;
        } else {
            post_gap = gap;
        }
    }

    let structural_break = Some(StructuralBreak {
        breakpoint: "2024-10-01".to_string(),
        pre_gap_pp: pre_gap,
        post_gap_pp: post_gap,
        swing_pp: post_gap - pre_gap,
    });

    Ok(TemporalResponse {
        series,
        granularity: granularity.to_string(),
        structural_break,
        filters_applied: build_filter_summary(params),
    })
}
