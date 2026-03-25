use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{Context, Result};
use duckdb::Connection;

pub type DbPool = Arc<Mutex<Connection>>;

pub fn open(data_dir: &Path) -> Result<Connection> {
    let conn = Connection::open_in_memory()
        .context("Failed to open in-memory DuckDB connection")?;

    conn.execute_batch(
        "SET threads TO 8;
         SET memory_limit = '6GB';
         SET enable_progress_bar = false;"
    ).ok();

    let dir = data_dir.display();

    conn.execute_batch(&format!(
        r#"
        CREATE OR REPLACE VIEW kalshi_trades AS
        SELECT * FROM read_parquet('{dir}/kalshi/trades/**/*.parquet');
        CREATE OR REPLACE VIEW kalshi_markets AS
        SELECT * FROM read_parquet('{dir}/kalshi/markets/**/*.parquet');
        "#
    )).context("Failed to create raw views")?;

    // Materialize enriched_trades
    let t0 = Instant::now();
    conn.execute_batch(&format!(
        r#"
        CREATE TABLE enriched_trades AS
        WITH resolved_markets AS (
            SELECT ticker, result, event_ticker
            FROM kalshi_markets
            WHERE status = 'finalized' AND result IN ('yes', 'no')
        )
        SELECT
            t.ticker,
            t.yes_price,
            t.no_price,
            t.taker_side,
            t.count,
            t.created_time AS trade_time,
            DATE_TRUNC('day', t.created_time) AS trade_date,
            DATE_TRUNC('quarter', t.created_time) AS trade_quarter,
            DATE_TRUNC('month', t.created_time) AS trade_month,
            m.result,
            m.event_ticker,
            CASE WHEN m.event_ticker IS NULL OR m.event_ticker = '' THEN 'Other'
                 ELSE regexp_extract(m.event_ticker, '^([A-Z]+)', 1) END AS event_prefix,
            CASE WHEN t.taker_side = 'yes' THEN t.yes_price ELSE t.no_price END AS taker_price,
            CASE WHEN t.taker_side = 'yes' THEN t.no_price ELSE t.yes_price END AS maker_price,
            CASE WHEN t.taker_side = m.result THEN 1 ELSE 0 END AS taker_won,
            CASE WHEN t.taker_side != m.result THEN 1 ELSE 0 END AS maker_won,
            (CASE WHEN t.taker_side = 'yes' THEN t.yes_price ELSE t.no_price END) * t.count AS taker_notional
        FROM kalshi_trades t
        INNER JOIN resolved_markets m ON t.ticker = m.ticker;
        "#
    )).context("Failed to materialize enriched_trades")?;

    tracing::info!("Materialized enriched_trades in {:.1}s", t0.elapsed().as_secs_f64());

    // Pre-aggregate rollup tables for fast API responses
    let t1 = Instant::now();
    conn.execute_batch(r#"
        -- Per-cent calibration rollup (99 rows)
        CREATE TABLE agg_calibration_per_cent AS
        SELECT
            taker_price AS price_cent,
            COUNT(*) AS n_trades,
            SUM(count) AS n_contracts,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            taker_price / 100.0 AS implied_probability,
            AVG(taker_won::DOUBLE) AS realized_win_rate,
            AVG(taker_won::DOUBLE) - taker_price / 100.0 AS mispricing,
            AVG((taker_won * 100.0 - taker_price) / NULLIF(taker_price, 0)) AS avg_taker_excess_return,
            AVG((maker_won * 100.0 - maker_price) / NULLIF(maker_price, 0)) AS avg_maker_excess_return,
            AVG(POW(yes_price / 100.0 - CASE WHEN result='yes' THEN 1.0 ELSE 0.0 END, 2)) AS brier,
            AVG(ABS(yes_price / 100.0 - CASE WHEN result='yes' THEN 1.0 ELSE 0.0 END)) AS mae,
            SUM(CASE WHEN taker_side='yes' THEN count ELSE 0 END) AS taker_yes_contracts,
            SUM(CASE WHEN taker_side='no' THEN count ELSE 0 END) AS taker_no_contracts,
            SUM((taker_won * 100.0 - taker_price) / NULLIF(taker_price, 0) * taker_notional) AS vw_taker_return_num,
            SUM((maker_won * 100.0 - maker_price) / NULLIF(maker_price, 0) * taker_notional) AS vw_maker_return_num,
            SUM(taker_notional) AS total_notional
        FROM enriched_trades
        WHERE taker_price BETWEEN 1 AND 99 AND maker_price BETWEEN 1 AND 99
        GROUP BY taker_price
        ORDER BY taker_price;

        -- Temporal rollup by quarter
        CREATE TABLE agg_temporal_quarter AS
        SELECT
            trade_quarter,
            STRFTIME(trade_quarter, '%Y-%m-%d') AS period_start,
            STRFTIME(trade_quarter, '%Y-Q') || CAST(EXTRACT(QUARTER FROM trade_quarter) AS VARCHAR) AS period,
            COUNT(*) AS n_trades,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            AVG((taker_won * 100.0 - taker_price) / NULLIF(taker_price, 0)) AS avg_taker_return,
            AVG((maker_won * 100.0 - maker_price) / NULLIF(maker_price, 0)) AS avg_maker_return,
            SUM(CASE WHEN taker_price BETWEEN 1 AND 20 THEN taker_notional ELSE 0 END)::DOUBLE
                / NULLIF(SUM(taker_notional), 0) AS longshot_volume_share
        FROM enriched_trades
        WHERE taker_price BETWEEN 1 AND 99 AND maker_price BETWEEN 1 AND 99
        GROUP BY trade_quarter
        ORDER BY trade_quarter;

        -- Temporal rollup by month
        CREATE TABLE agg_temporal_month AS
        SELECT
            trade_month,
            STRFTIME(trade_month, '%Y-%m-%d') AS period_start,
            STRFTIME(trade_month, '%Y-%m') AS period,
            COUNT(*) AS n_trades,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            AVG((taker_won * 100.0 - taker_price) / NULLIF(taker_price, 0)) AS avg_taker_return,
            AVG((maker_won * 100.0 - maker_price) / NULLIF(maker_price, 0)) AS avg_maker_return,
            SUM(CASE WHEN taker_price BETWEEN 1 AND 20 THEN taker_notional ELSE 0 END)::DOUBLE
                / NULLIF(SUM(taker_notional), 0) AS longshot_volume_share
        FROM enriched_trades
        WHERE taker_price BETWEEN 1 AND 99 AND maker_price BETWEEN 1 AND 99
        GROUP BY trade_month
        ORDER BY trade_month;

        -- Category rollup
        CREATE TABLE agg_categories AS
        SELECT
            event_prefix AS category,
            COUNT(*) AS n_trades,
            SUM(count) AS n_contracts,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            AVG((taker_won * 100.0 - taker_price) / NULLIF(taker_price, 0)) AS avg_taker_return,
            AVG((maker_won * 100.0 - maker_price) / NULLIF(maker_price, 0)) AS avg_maker_return,
            AVG(POW(yes_price / 100.0 - CASE WHEN result='yes' THEN 1.0 ELSE 0.0 END, 2)) AS brier_score,
            AVG(CASE WHEN taker_price BETWEEN 1 AND 20
                     THEN taker_won::DOUBLE - taker_price / 100.0 ELSE NULL END) AS longshot_mispricing
        FROM enriched_trades
        WHERE taker_price BETWEEN 1 AND 99 AND maker_price BETWEEN 1 AND 99
          AND event_prefix IS NOT NULL AND event_prefix != ''
        GROUP BY event_prefix
        HAVING COUNT(*) > 1000
        ORDER BY (AVG((maker_won * 100.0 - maker_price) / NULLIF(maker_price, 0))
                - AVG((taker_won * 100.0 - taker_price) / NULLIF(taker_price, 0))) DESC;

        -- Cohort rollup by trade size
        CREATE TABLE agg_cohorts AS
        SELECT
            CASE
                WHEN taker_notional / 100.0 < 10 THEN 'micro'
                WHEN taker_notional / 100.0 < 100 THEN 'small'
                WHEN taker_notional / 100.0 < 1000 THEN 'medium'
                ELSE 'large'
            END AS size_cohort,
            COUNT(*) AS n_trades,
            SUM(count) AS n_contracts,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            AVG((taker_won * 100.0 - taker_price) / NULLIF(taker_price, 0)) AS avg_taker_return,
            AVG((maker_won * 100.0 - maker_price) / NULLIF(maker_price, 0)) AS avg_maker_return
        FROM enriched_trades
        WHERE taker_price BETWEEN 1 AND 99 AND maker_price BETWEEN 1 AND 99
        GROUP BY size_cohort;

        -- YES/NO asymmetry rollup per cost basis cent
        CREATE TABLE agg_yes_no AS
        WITH yes_agg AS (
            SELECT
                yes_price AS cost_basis,
                COUNT(*) AS n_trades,
                AVG(CASE WHEN result='yes' THEN 1.0 ELSE 0.0 END) AS win_rate,
                AVG((CASE WHEN result='yes' THEN 1 ELSE 0 END * 100.0 - yes_price) / NULLIF(yes_price, 0)) AS avg_return
            FROM enriched_trades
            WHERE yes_price BETWEEN 1 AND 99
            GROUP BY yes_price
        ),
        no_agg AS (
            SELECT
                no_price AS cost_basis,
                COUNT(*) AS n_trades,
                AVG(CASE WHEN result='no' THEN 1.0 ELSE 0.0 END) AS win_rate,
                AVG((CASE WHEN result='no' THEN 1 ELSE 0 END * 100.0 - no_price) / NULLIF(no_price, 0)) AS avg_return
            FROM enriched_trades
            WHERE no_price BETWEEN 1 AND 99
            GROUP BY no_price
        ),
        vol_share AS (
            SELECT
                yes_price AS cb,
                SUM(CASE WHEN taker_side='yes' THEN count ELSE 0 END)::DOUBLE / NULLIF(SUM(count), 0) AS taker_yes_share,
                SUM(CASE WHEN taker_side='no' THEN count ELSE 0 END)::DOUBLE / NULLIF(SUM(count), 0) AS taker_no_share
            FROM enriched_trades
            GROUP BY yes_price
        )
        SELECT
            COALESCE(y.cost_basis, n.cost_basis) AS cost_basis,
            COALESCE(y.avg_return, 0) AS yes_return,
            COALESCE(n.avg_return, 0) AS no_return,
            COALESCE(y.n_trades, 0) AS yes_n_trades,
            COALESCE(n.n_trades, 0) AS no_n_trades,
            COALESCE(n.avg_return, 0) - COALESCE(y.avg_return, 0) AS divergence_pp,
            COALESCE(v.taker_yes_share, 0) AS taker_yes_share,
            COALESCE(v.taker_no_share, 0) AS taker_no_share
        FROM yes_agg y
        FULL OUTER JOIN no_agg n ON y.cost_basis = n.cost_basis
        LEFT JOIN vol_share v ON COALESCE(y.cost_basis, n.cost_basis) = v.cb
        WHERE COALESCE(y.cost_basis, n.cost_basis) IS NOT NULL
        ORDER BY 1;

        -- Summary stats (1 row)
        CREATE TABLE agg_summary AS
        SELECT
            COUNT(*) AS total_trades,
            SUM(count) AS total_contracts,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            (SELECT COUNT(*) FROM kalshi_markets) AS total_markets,
            (SELECT COUNT(*) FROM kalshi_markets WHERE result IN ('yes','no')) AS resolved_markets,
            STRFTIME(MIN(trade_time), '%Y-%m-%d') AS date_start,
            STRFTIME(MAX(trade_time), '%Y-%m-%d') AS date_end
        FROM enriched_trades;
    "#).context("Failed to create pre-aggregated rollups")?;

    tracing::info!("Pre-aggregated rollups in {:.1}s", t1.elapsed().as_secs_f64());
    Ok(conn)
}

pub fn create_pool(data_dir: &Path) -> Result<DbPool> {
    let conn = open(data_dir)?;
    Ok(Arc::new(Mutex::new(conn)))
}
