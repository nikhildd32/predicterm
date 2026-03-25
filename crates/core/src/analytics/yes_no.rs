use anyhow::Result;
use duckdb::Connection;

use crate::models::{FilterParams, YesNoPoint, YesNoResponse};
use super::build_filter_summary;

/// YES/NO asymmetry analysis (Becker YES/NO section, spec §2.3)
pub fn query_yes_no(conn: &Connection, params: &FilterParams) -> Result<YesNoResponse> {
    let sql = r#"
        WITH yes_trades AS (
            SELECT
                yes_price AS cost_basis,
                CASE WHEN result = 'yes' THEN 1 ELSE 0 END AS won,
                count AS n_contracts,
                'YES' AS side
            FROM enriched_trades
        ),
        no_trades AS (
            SELECT
                no_price AS cost_basis,
                CASE WHEN result = 'no' THEN 1 ELSE 0 END AS won,
                count AS n_contracts,
                'NO' AS side
            FROM enriched_trades
        ),
        all_sides AS (
            SELECT * FROM yes_trades
            UNION ALL
            SELECT * FROM no_trades
        ),
        by_side AS (
            SELECT
                cost_basis,
                side,
                COUNT(*) AS n_trades,
                AVG((won * 100.0 - cost_basis) / cost_basis) AS avg_return
            FROM all_sides
            WHERE cost_basis BETWEEN 1 AND 99
            GROUP BY cost_basis, side
        ),
        pivoted AS (
            SELECT
                y.cost_basis,
                COALESCE(y.avg_return, 0) AS yes_return,
                COALESCE(n.avg_return, 0) AS no_return,
                COALESCE(y.n_trades, 0) AS yes_n_trades,
                COALESCE(n.n_trades, 0) AS no_n_trades
            FROM (SELECT * FROM by_side WHERE side = 'YES') y
            FULL OUTER JOIN (SELECT * FROM by_side WHERE side = 'NO') n
                ON y.cost_basis = n.cost_basis
        ),
        volume_share AS (
            SELECT
                yes_price AS cost_basis_ref,
                SUM(CASE WHEN taker_side = 'yes' THEN count ELSE 0 END)::DOUBLE /
                    NULLIF(SUM(count), 0) AS taker_yes_share,
                SUM(CASE WHEN taker_side = 'no' THEN count ELSE 0 END)::DOUBLE /
                    NULLIF(SUM(count), 0) AS taker_no_share
            FROM enriched_trades
            GROUP BY yes_price
        )
        SELECT
            p.cost_basis,
            p.yes_return,
            p.no_return,
            p.yes_n_trades,
            p.no_n_trades,
            p.no_return - p.yes_return AS divergence_pp,
            COALESCE(v.taker_yes_share, 0) AS taker_yes_share,
            COALESCE(v.taker_no_share, 0) AS taker_no_share
        FROM pivoted p
        LEFT JOIN volume_share v ON p.cost_basis = v.cost_basis_ref
        WHERE p.cost_basis IS NOT NULL
        ORDER BY p.cost_basis
    "#;

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(YesNoPoint {
            cost_basis: row.get(0)?,
            yes_return: row.get(1)?,
            no_return: row.get(2)?,
            yes_n_trades: row.get(3)?,
            no_n_trades: row.get(4)?,
            divergence_pp: row.get(5)?,
            taker_yes_share: row.get::<_, f64>(6).unwrap_or(0.0),
            taker_no_share: row.get::<_, f64>(7).unwrap_or(0.0),
        })
    })?;

    let points: Vec<YesNoPoint> = rows.filter_map(|r| r.ok()).collect();

    let n_no_outperforms = points
        .iter()
        .filter(|p| p.no_return > p.yes_return)
        .count() as i32;

    let agg_yes = if points.is_empty() {
        0.0
    } else {
        points.iter().map(|p| p.yes_return).sum::<f64>() / points.len() as f64
    };
    let agg_no = if points.is_empty() {
        0.0
    } else {
        points.iter().map(|p| p.no_return).sum::<f64>() / points.len() as f64
    };

    Ok(YesNoResponse {
        points,
        aggregate_yes_return: agg_yes,
        aggregate_no_return: agg_no,
        n_levels_no_outperforms: n_no_outperforms,
        filters_applied: build_filter_summary(params),
    })
}
