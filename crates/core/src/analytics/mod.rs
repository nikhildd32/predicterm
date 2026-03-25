pub mod calibration;
pub mod categories;
pub mod cohorts;
pub mod maker_taker;
pub mod summary;
pub mod temporal;
pub mod yes_no;

use crate::models::{FilterParams, FilterSummary};

pub fn build_filter_summary(f: &FilterParams) -> FilterSummary {
    FilterSummary {
        start_time: f.start_time.clone(),
        end_time: f.end_time.clone(),
        category: f.category.clone(),
        market_id: f.market_id.clone(),
        min_trade_size: f.min_trade_size,
        min_market_volume: f.min_market_volume,
        include_fees: f.include_fees.unwrap_or(false),
    }
}

/// Build a WHERE clause fragment from filter params.
pub fn build_where_clause(f: &FilterParams) -> String {
    let mut conditions = vec!["taker_price BETWEEN 1 AND 99".to_string()];

    if let Some(ref cat) = f.category {
        conditions.push(format!("event_prefix = '{cat}'"));
    }
    if let Some(ref market_id) = f.market_id {
        conditions.push(format!("ticker = '{market_id}'"));
    }
    if let Some(min_size) = f.min_trade_size {
        conditions.push(format!("taker_notional / 100.0 >= {min_size}"));
    }

    conditions.join(" AND ")
}
