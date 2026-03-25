use anyhow::Result;
use duckdb::Connection;

use crate::models::{FilterParams, YesNoPoint, YesNoResponse};
use super::build_filter_summary;

pub fn query_yes_no(conn: &Connection, params: &FilterParams) -> Result<YesNoResponse> {
    let sql = "SELECT cost_basis, yes_return, no_return, yes_n_trades, no_n_trades,
                      divergence_pp, taker_yes_share, taker_no_share
               FROM agg_yes_no
               ORDER BY cost_basis";

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

    let n_no_outperforms = points.iter().filter(|p| p.no_return > p.yes_return).count() as i32;
    let len = points.len().max(1) as f64;
    let agg_yes = points.iter().map(|p| p.yes_return).sum::<f64>() / len;
    let agg_no = points.iter().map(|p| p.no_return).sum::<f64>() / len;

    Ok(YesNoResponse {
        points,
        aggregate_yes_return: agg_yes,
        aggregate_no_return: agg_no,
        n_levels_no_outperforms: n_no_outperforms,
        filters_applied: build_filter_summary(params),
    })
}
