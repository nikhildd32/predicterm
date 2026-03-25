use anyhow::Result;
use duckdb::Connection;

use crate::models::{MarketListItem, MarketsResponse, SummaryStats};

/// Overall dataset summary stats
pub fn query_summary(conn: &Connection) -> Result<SummaryStats> {
    let sql = r#"
        SELECT
            COUNT(*) AS total_trades,
            SUM(count) AS total_contracts,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            (SELECT COUNT(*) FROM kalshi_markets) AS total_markets,
            (SELECT COUNT(*) FROM kalshi_markets WHERE result IN ('yes','no')) AS resolved_markets,
            STRFTIME(MIN(trade_time), '%Y-%m-%d') AS date_start,
            STRFTIME(MAX(trade_time), '%Y-%m-%d') AS date_end
        FROM enriched_trades
    "#;

    let stats = conn.query_row(sql, [], |row| {
        Ok(SummaryStats {
            total_trades: row.get(0)?,
            total_contracts: row.get(1)?,
            total_volume_usd: row.get(2)?,
            total_markets: row.get(3)?,
            resolved_markets: row.get(4)?,
            date_range_start: row.get::<_, String>(5).unwrap_or_default(),
            date_range_end: row.get::<_, String>(6).unwrap_or_default(),
        })
    })?;

    Ok(stats)
}

/// Paginated markets list
pub fn query_markets(
    conn: &Connection,
    limit: i64,
    offset: i64,
    search: Option<&str>,
    status: Option<&str>,
) -> Result<MarketsResponse> {
    let mut conditions = vec!["1=1".to_string()];
    if let Some(s) = status {
        conditions.push(format!("status = '{s}'"));
    }
    if let Some(q) = search {
        conditions.push(format!("(title ILIKE '%{q}%' OR ticker ILIKE '%{q}%')"));
    }
    let where_clause = conditions.join(" AND ");

    let sql = format!(
        r#"
        SELECT ticker, event_ticker, title, status, result, volume
        FROM kalshi_markets
        WHERE {where_clause}
        ORDER BY volume DESC
        LIMIT {limit} OFFSET {offset}
        "#
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(MarketListItem {
            ticker: row.get(0)?,
            event_ticker: row.get(1)?,
            title: row.get(2)?,
            status: row.get(3)?,
            result: row.get(4)?,
            volume: row.get(5)?,
        })
    })?;

    let markets: Vec<MarketListItem> = rows.filter_map(|r| r.ok()).collect();

    let count_sql = format!(
        "SELECT COUNT(*) FROM kalshi_markets WHERE {where_clause}"
    );
    let total: i64 = conn.query_row(&count_sql, [], |row| row.get(0))?;

    Ok(MarketsResponse {
        markets,
        total,
        limit,
        offset,
    })
}
