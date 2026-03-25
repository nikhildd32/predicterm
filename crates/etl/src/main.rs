use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let data_path = PathBuf::from(&data_dir);

    match command {
        "validate" => validate(&data_path),
        "stats" => stats(&data_path),
        _ => {
            println!("PredicTerm ETL");
            println!();
            println!("Usage: etl <command>");
            println!();
            println!("Commands:");
            println!("  validate   Check Parquet schema against expected columns");
            println!("  stats      Print row counts, date ranges, volume (with timing)");
            Ok(())
        }
    }
}

fn validate(data_dir: &PathBuf) -> Result<()> {
    let t0 = Instant::now();
    tracing::info!("Validating data directory: {}", data_dir.display());

    let trades_dir = data_dir.join("kalshi/trades");
    let markets_dir = data_dir.join("kalshi/markets");

    anyhow::ensure!(trades_dir.exists(), "Trades directory not found: {}", trades_dir.display());
    anyhow::ensure!(markets_dir.exists(), "Markets directory not found: {}", markets_dir.display());

    let conn = predicterm_core::db::open(data_dir)?;

    let trade_cols: Vec<String> = conn
        .prepare("SELECT column_name FROM (DESCRIBE kalshi_trades)")?
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;

    let expected_trade_cols = ["ticker", "count", "yes_price", "no_price", "taker_side", "created_time"];
    for col in &expected_trade_cols {
        anyhow::ensure!(
            trade_cols.iter().any(|c| c == col),
            "Missing trade column: {col}. Have: {}", trade_cols.join(", ")
        );
    }

    let market_cols: Vec<String> = conn
        .prepare("SELECT column_name FROM (DESCRIBE kalshi_markets)")?
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;

    let expected_market_cols = ["ticker", "event_ticker", "status", "result", "volume"];
    for col in &expected_market_cols {
        anyhow::ensure!(
            market_cols.iter().any(|c| c == col),
            "Missing market column: {col}. Have: {}", market_cols.join(", ")
        );
    }

    println!("Schema validation PASSED in {:.2}s", t0.elapsed().as_secs_f64());
    println!();
    println!("Trade columns ({}):", trade_cols.len());
    println!("  {}", trade_cols.join(", "));
    println!();
    println!("Market columns ({}):", market_cols.len());
    println!("  {}", market_cols.join(", "));

    // Quick row counts
    let t1 = Instant::now();
    let trade_count: i64 = conn.query_row("SELECT COUNT(*) FROM kalshi_trades", [], |r| r.get(0))?;
    let market_count: i64 = conn.query_row("SELECT COUNT(*) FROM kalshi_markets", [], |r| r.get(0))?;
    println!();
    println!("Quick count ({:.2}s):", t1.elapsed().as_secs_f64());
    println!("  Trades:  {trade_count:>15}");
    println!("  Markets: {market_count:>15}");

    Ok(())
}

fn stats(data_dir: &PathBuf) -> Result<()> {
    println!("PredicTerm Dataset Statistics");
    println!("=============================");
    println!();

    let t0 = Instant::now();
    let conn = predicterm_core::db::open(data_dir)?;
    println!("DuckDB init: {:.2}s", t0.elapsed().as_secs_f64());

    // --- Raw trades ---
    let t1 = Instant::now();
    let mut stmt = conn.prepare(
        "SELECT
            COUNT(*) AS trade_count,
            STRFTIME(MIN(created_time), '%Y-%m-%d %H:%M:%S') AS earliest,
            STRFTIME(MAX(created_time), '%Y-%m-%d %H:%M:%S') AS latest,
            SUM(count) AS total_contracts
         FROM kalshi_trades"
    )?;

    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let trade_count: i64 = row.get(0)?;
        let earliest: String = row.get(1)?;
        let latest: String = row.get(2)?;
        let total_contracts: i64 = row.get(3)?;

        println!();
        println!("=== Raw Trades (full table scan: {:.2}s) ===", t1.elapsed().as_secs_f64());
        println!("  Total trades:     {:>15}", trade_count);
        println!("  Total contracts:  {:>15}", total_contracts);
        println!("  Earliest:         {earliest}");
        println!("  Latest:           {latest}");
    }

    // --- Markets ---
    let t2 = Instant::now();
    let mut stmt = conn.prepare(
        "SELECT
            COUNT(*) AS market_count,
            COUNT(CASE WHEN result IN ('yes','no') THEN 1 END) AS resolved,
            COUNT(CASE WHEN result IS NULL OR result = '' THEN 1 END) AS unresolved,
            SUM(volume) AS total_volume
         FROM kalshi_markets"
    )?;

    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let market_count: i64 = row.get(0)?;
        let resolved: i64 = row.get(1)?;
        let unresolved: i64 = row.get(2)?;
        let total_volume: i64 = row.get(3)?;

        println!();
        println!("=== Markets (scan: {:.2}s) ===", t2.elapsed().as_secs_f64());
        println!("  Total markets:    {:>15}", market_count);
        println!("  Resolved:         {:>15}", resolved);
        println!("  Unresolved:       {:>15}", unresolved);
        println!("  Total volume:     {:>15}", total_volume);
    }

    // --- Enriched trades (JOIN) ---
    let t3 = Instant::now();
    let mut stmt = conn.prepare(
        "SELECT
            COUNT(*) AS enriched_count,
            SUM(count) AS enriched_contracts,
            SUM(taker_notional) / 100.0 AS total_volume_usd,
            COUNT(DISTINCT ticker) AS unique_markets
         FROM enriched_trades"
    )?;

    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let enriched_count: i64 = row.get(0)?;
        let enriched_contracts: i64 = row.get(1)?;
        let total_volume_usd: f64 = row.get(2)?;
        let unique_markets: i64 = row.get(3)?;

        println!();
        println!("=== Enriched Trades [resolved only] (JOIN scan: {:.2}s) ===", t3.elapsed().as_secs_f64());
        println!("  Enriched trades:  {:>15}", enriched_count);
        println!("  Enriched contr.:  {:>15}", enriched_contracts);
        println!("  Volume (USD):     {:>15.2}", total_volume_usd);
        println!("  Unique markets:   {:>15}", unique_markets);
    }

    // --- Taker side breakdown ---
    let t4 = Instant::now();
    let mut stmt = conn.prepare(
        "SELECT taker_side, COUNT(*), SUM(count)
         FROM kalshi_trades
         GROUP BY taker_side"
    ).context("Failed taker_side breakdown")?;

    let mut rows = stmt.query([])?;
    println!();
    println!("=== Taker Side Breakdown (scan: {:.2}s) ===", t4.elapsed().as_secs_f64());
    while let Some(row) = rows.next()? {
        let side: String = row.get(0)?;
        let cnt: i64 = row.get(1)?;
        let contracts: i64 = row.get(2)?;
        println!("  {side:>5}: {cnt:>12} trades, {contracts:>12} contracts");
    }

    // --- Quick analytics smoke test ---
    let t5 = Instant::now();
    let (avg_taker_ret, avg_maker_ret): (f64, f64) = conn.query_row(
        "SELECT
            AVG((taker_won * 100.0 - taker_price) / NULLIF(taker_price, 0)),
            AVG((maker_won * 100.0 - maker_price) / NULLIF(maker_price, 0))
         FROM enriched_trades
         WHERE taker_price BETWEEN 1 AND 99
           AND maker_price BETWEEN 1 AND 99",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    println!();
    println!("=== Maker-Taker Smoke Test (scan: {:.2}s) ===", t5.elapsed().as_secs_f64());
    println!("  Avg taker excess return: {:.4}%", avg_taker_ret * 100.0);
    println!("  Avg maker excess return: {:.4}%", avg_maker_ret * 100.0);
    println!("  Gap:                     {:.4}pp", (avg_maker_ret - avg_taker_ret) * 100.0);

    println!();
    println!("Total wall time: {:.2}s", t0.elapsed().as_secs_f64());

    Ok(())
}
