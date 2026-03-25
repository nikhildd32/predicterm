use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use duckdb::Connection;

pub type DbPool = Arc<Mutex<Connection>>;

/// Open a DuckDB connection with the enriched_trades view from the spec.
/// Uses Becker's schema: trades have `ts` (BIGINT), taker_side, yes_price, no_price, count.
/// Markets have ticker, event_ticker, status, result.
pub fn open(data_dir: &Path) -> Result<Connection> {
    let conn = Connection::open_in_memory()
        .context("Failed to open in-memory DuckDB connection")?;

    let dir = data_dir.display();

    // Create the enriched_trades view matching the spec's §1.3
    conn.execute_batch(&format!(
        r#"
        CREATE OR REPLACE VIEW kalshi_markets AS
        SELECT * FROM read_parquet('{dir}/kalshi/markets/**/*.parquet');

        CREATE OR REPLACE VIEW enriched_trades AS
        WITH resolved_markets AS (
            SELECT ticker, result, event_ticker
            FROM read_parquet('{dir}/kalshi/markets/**/*.parquet')
            WHERE status = 'finalized'
              AND result IN ('yes', 'no')
        )
        SELECT
            t.ticker,
            t.yes_price,
            t.no_price,
            t.taker_side,
            t.count,
            TO_TIMESTAMP(t.ts) AS trade_time,
            DATE_TRUNC('day', TO_TIMESTAMP(t.ts)) AS trade_date,
            DATE_TRUNC('quarter', TO_TIMESTAMP(t.ts)) AS trade_quarter,
            DATE_TRUNC('month', TO_TIMESTAMP(t.ts)) AS trade_month,
            m.result,
            m.event_ticker,
            CASE
                WHEN m.event_ticker IS NULL OR m.event_ticker = '' THEN 'Other'
                ELSE regexp_extract(m.event_ticker, '^([A-Z]+)', 1)
            END AS event_prefix,
            -- Taker perspective
            CASE WHEN t.taker_side = 'yes' THEN t.yes_price ELSE t.no_price END AS taker_price,
            CASE WHEN t.taker_side = 'yes' THEN t.no_price ELSE t.yes_price END AS maker_price,
            CASE WHEN t.taker_side = m.result THEN 1 ELSE 0 END AS taker_won,
            CASE WHEN t.taker_side != m.result THEN 1 ELSE 0 END AS maker_won,
            (CASE WHEN t.taker_side = 'yes' THEN t.yes_price ELSE t.no_price END) * t.count AS taker_notional
        FROM read_parquet('{dir}/kalshi/trades/**/*.parquet') t
        INNER JOIN resolved_markets m ON t.ticker = m.ticker;
        "#
    ))
    .context("Failed to create DuckDB views")?;

    tracing::info!("DuckDB enriched_trades view created over {dir}/kalshi/");
    Ok(conn)
}

pub fn create_pool(data_dir: &Path) -> Result<DbPool> {
    let conn = open(data_dir)?;
    Ok(Arc::new(Mutex::new(conn)))
}
