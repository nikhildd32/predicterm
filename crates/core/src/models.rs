use serde::{Deserialize, Serialize};

// === Request types ===

#[derive(Debug, Clone, Deserialize, Default)]
pub struct FilterParams {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub category: Option<String>,
    pub market_id: Option<String>,
    pub min_trade_size: Option<f64>,
    pub min_market_volume: Option<f64>,
    pub role_filter: Option<String>,
    pub include_fees: Option<bool>,
    pub bucket_width: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CohortParams {
    #[serde(flatten)]
    pub filters: FilterParams,
    pub cohort_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemporalParams {
    #[serde(flatten)]
    pub filters: FilterParams,
    pub granularity: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>,
}

// === Response types ===

#[derive(Debug, Clone, Serialize)]
pub struct FilterSummary {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub category: Option<String>,
    pub market_id: Option<String>,
    pub min_trade_size: Option<f64>,
    pub min_market_volume: Option<f64>,
    pub include_fees: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalibrationPoint {
    pub price_bucket_low: i32,
    pub price_bucket_high: i32,
    pub n_trades: i64,
    pub n_contracts: i64,
    pub total_volume_usd: f64,
    pub implied_probability: f64,
    pub realized_win_rate: f64,
    pub mispricing: f64,
    pub avg_excess_return: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalibrationResponse {
    pub points: Vec<CalibrationPoint>,
    pub overall_brier_score: f64,
    pub overall_mae: f64,
    pub total_trades: i64,
    pub filters_applied: FilterSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct MakerTakerPoint {
    pub price_bucket_low: i32,
    pub price_bucket_high: i32,
    pub n_trades: i64,
    pub avg_taker_return: f64,
    pub avg_maker_return: f64,
    pub gap_pp: f64,
    pub vw_taker_return: f64,
    pub vw_maker_return: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MakerTakerResponse {
    pub points: Vec<MakerTakerPoint>,
    pub aggregate_taker_return: f64,
    pub aggregate_maker_return: f64,
    pub aggregate_gap_pp: f64,
    pub total_trades: i64,
    pub total_volume_usd: f64,
    pub filters_applied: FilterSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryStats {
    pub category: String,
    pub n_trades: i64,
    pub n_contracts: i64,
    pub total_volume_usd: f64,
    pub avg_taker_return: f64,
    pub avg_maker_return: f64,
    pub gap_pp: f64,
    pub brier_score: f64,
    pub longshot_mispricing: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryResponse {
    pub categories: Vec<CategoryStats>,
    pub filters_applied: FilterSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemporalPoint {
    pub period: String,
    pub period_start: String,
    pub n_trades: i64,
    pub total_volume_usd: f64,
    pub avg_taker_return: f64,
    pub avg_maker_return: f64,
    pub gap_pp: f64,
    pub longshot_volume_share: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructuralBreak {
    pub breakpoint: String,
    pub pre_gap_pp: f64,
    pub post_gap_pp: f64,
    pub swing_pp: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemporalResponse {
    pub series: Vec<TemporalPoint>,
    pub granularity: String,
    pub structural_break: Option<StructuralBreak>,
    pub filters_applied: FilterSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct YesNoPoint {
    pub cost_basis: i32,
    pub yes_return: f64,
    pub no_return: f64,
    pub yes_n_trades: i64,
    pub no_n_trades: i64,
    pub divergence_pp: f64,
    pub taker_yes_share: f64,
    pub taker_no_share: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct YesNoResponse {
    pub points: Vec<YesNoPoint>,
    pub aggregate_yes_return: f64,
    pub aggregate_no_return: f64,
    pub n_levels_no_outperforms: i32,
    pub filters_applied: FilterSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct CohortStats {
    pub cohort_label: String,
    pub n_trades: i64,
    pub n_contracts: i64,
    pub total_volume_usd: f64,
    pub avg_taker_return: f64,
    pub avg_maker_return: f64,
    pub gap_pp: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CohortResponse {
    pub cohorts: Vec<CohortStats>,
    pub cohort_type: String,
    pub filters_applied: FilterSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketListItem {
    pub ticker: String,
    pub event_ticker: Option<String>,
    pub title: Option<String>,
    pub status: String,
    pub result: Option<String>,
    pub volume: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketsResponse {
    pub markets: Vec<MarketListItem>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SummaryStats {
    pub total_trades: i64,
    pub total_contracts: i64,
    pub total_volume_usd: f64,
    pub total_markets: i64,
    pub resolved_markets: i64,
    pub date_range_start: String,
    pub date_range_end: String,
}
