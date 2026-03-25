use std::path::PathBuf;

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
            println!("  validate   Check that Parquet files exist and schema is correct");
            println!("  stats      Print trade/market counts and date ranges");
            Ok(())
        }
    }
}

fn validate(data_dir: &PathBuf) -> Result<()> {
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

    let expected_trade_cols = ["trade_id", "ticker", "count", "yes_price", "no_price", "taker_side", "created_time"];
    for col in &expected_trade_cols {
        anyhow::ensure!(
            trade_cols.iter().any(|c| c == col),
            "Missing trade column: {col}"
        );
    }

    let market_cols: Vec<String> = conn
        .prepare("SELECT column_name FROM (DESCRIBE kalshi_markets)")?
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;

    let expected_market_cols = ["ticker", "event_ticker", "title", "status", "result", "volume"];
    for col in &expected_market_cols {
        anyhow::ensure!(
            market_cols.iter().any(|c| c == col),
            "Missing market column: {col}"
        );
    }

    tracing::info!("Schema validation passed");
    tracing::info!("Trade columns: {}", trade_cols.join(", "));
    tracing::info!("Market columns: {}", market_cols.join(", "));

    Ok(())
}

fn stats(data_dir: &PathBuf) -> Result<()> {
    tracing::info!("Computing dataset statistics...");

    let conn = predicterm_core::db::open(data_dir)?;

    let mut stmt = conn.prepare(
        "SELECT
            COUNT(*) as trade_count,
            MIN(created_time) as earliest,
            MAX(created_time) as latest,
            SUM(count) as total_contracts
         FROM kalshi_trades"
    )?;

    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let trade_count: i64 = row.get(0)?;
        let earliest: String = row.get(1)?;
        let latest: String = row.get(2)?;
        let total_contracts: i64 = row.get(3)?;

        println!("=== Kalshi Trades ===");
        println!("  Total trades:     {trade_count:>15}");
        println!("  Total contracts:  {total_contracts:>15}");
        println!("  Earliest:         {earliest}");
        println!("  Latest:           {latest}");
    }

    let mut stmt = conn.prepare(
        "SELECT
            COUNT(*) as market_count,
            COUNT(CASE WHEN result IN ('yes','no') THEN 1 END) as resolved,
            COUNT(CASE WHEN result IS NULL OR result = '' THEN 1 END) as unresolved,
            SUM(volume) as total_volume
         FROM kalshi_markets"
    )?;

    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        let market_count: i64 = row.get(0)?;
        let resolved: i64 = row.get(1)?;
        let unresolved: i64 = row.get(2)?;
        let total_volume: i64 = row.get(3)?;

        println!();
        println!("=== Kalshi Markets ===");
        println!("  Total markets:    {market_count:>15}");
        println!("  Resolved:         {resolved:>15}");
        println!("  Unresolved:       {unresolved:>15}");
        println!("  Total volume:     {total_volume:>15}");
    }

    let mut stmt = conn.prepare(
        "SELECT taker_side, COUNT(*), SUM(count)
         FROM kalshi_trades
         GROUP BY taker_side"
    ).context("Failed taker_side breakdown")?;

    let mut rows = stmt.query([])?;
    println!();
    println!("=== Taker Side Breakdown ===");
    while let Some(row) = rows.next()? {
        let side: String = row.get(0)?;
        let cnt: i64 = row.get(1)?;
        let contracts: i64 = row.get(2)?;
        println!("  {side:>5}: {cnt:>12} trades, {contracts:>12} contracts");
    }

    Ok(())
}
