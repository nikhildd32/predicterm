const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3001";

async function fetchApi<T>(path: string, params?: Record<string, string>): Promise<T> {
  const url = new URL(`${API_BASE}${path}`);
  if (params) {
    Object.entries(params).forEach(([k, v]) => {
      if (v !== undefined && v !== "") url.searchParams.set(k, v);
    });
  }
  const res = await fetch(url.toString());
  if (!res.ok) throw new Error(`API error: ${res.status} ${res.statusText}`);
  return res.json();
}

// Types matching the Rust API responses
export interface FilterParams {
  start_time?: string;
  end_time?: string;
  category?: string;
  market_id?: string;
  min_trade_size?: string;
  min_market_volume?: string;
  bucket_width?: string;
}

export interface CalibrationPoint {
  price_bucket_low: number;
  price_bucket_high: number;
  n_trades: number;
  n_contracts: number;
  total_volume_usd: number;
  implied_probability: number;
  realized_win_rate: number;
  mispricing: number;
  avg_excess_return: number;
}

export interface CalibrationResponse {
  points: CalibrationPoint[];
  overall_brier_score: number;
  overall_mae: number;
  total_trades: number;
  filters_applied: FilterSummary;
}

export interface MakerTakerPoint {
  price_bucket_low: number;
  price_bucket_high: number;
  n_trades: number;
  avg_taker_return: number;
  avg_maker_return: number;
  gap_pp: number;
  vw_taker_return: number;
  vw_maker_return: number;
}

export interface MakerTakerResponse {
  points: MakerTakerPoint[];
  aggregate_taker_return: number;
  aggregate_maker_return: number;
  aggregate_gap_pp: number;
  total_trades: number;
  total_volume_usd: number;
  filters_applied: FilterSummary;
}

export interface CategoryStats {
  category: string;
  n_trades: number;
  n_contracts: number;
  total_volume_usd: number;
  avg_taker_return: number;
  avg_maker_return: number;
  gap_pp: number;
  brier_score: number;
  longshot_mispricing: number;
}

export interface TemporalPoint {
  period: string;
  period_start: string;
  n_trades: number;
  total_volume_usd: number;
  avg_taker_return: number;
  avg_maker_return: number;
  gap_pp: number;
  longshot_volume_share: number;
}

export interface StructuralBreak {
  breakpoint: string;
  pre_gap_pp: number;
  post_gap_pp: number;
  swing_pp: number;
}

export interface TemporalResponse {
  series: TemporalPoint[];
  granularity: string;
  structural_break: StructuralBreak | null;
  filters_applied: FilterSummary;
}

export interface YesNoPoint {
  cost_basis: number;
  yes_return: number;
  no_return: number;
  yes_n_trades: number;
  no_n_trades: number;
  divergence_pp: number;
  taker_yes_share: number;
  taker_no_share: number;
}

export interface YesNoResponse {
  points: YesNoPoint[];
  aggregate_yes_return: number;
  aggregate_no_return: number;
  n_levels_no_outperforms: number;
  filters_applied: FilterSummary;
}

export interface CohortStats {
  cohort_label: string;
  n_trades: number;
  n_contracts: number;
  total_volume_usd: number;
  avg_taker_return: number;
  avg_maker_return: number;
  gap_pp: number;
}

export interface MarketListItem {
  ticker: string;
  event_ticker: string | null;
  title: string | null;
  status: string;
  result: string | null;
  volume: number;
}

export interface MarketsResponse {
  markets: MarketListItem[];
  total: number;
  limit: number;
  offset: number;
}

export interface SummaryStats {
  total_trades: number;
  total_contracts: number;
  total_volume_usd: number;
  total_markets: number;
  resolved_markets: number;
  date_range_start: string;
  date_range_end: string;
}

export interface FilterSummary {
  start_time: string | null;
  end_time: string | null;
  category: string | null;
  market_id: string | null;
  include_fees: boolean;
}

type Params = Record<string, string>;

export const api = {
  calibration: (p?: Params) => fetchApi<CalibrationResponse>("/api/v1/calibration", p),
  makerTaker: (p?: Params) => fetchApi<MakerTakerResponse>("/api/v1/maker-taker", p),
  categories: (p?: Params) => fetchApi<{ categories: CategoryStats[]; filters_applied: FilterSummary }>("/api/v1/categories", p),
  temporal: (p?: Params) => fetchApi<TemporalResponse>("/api/v1/temporal", p),
  yesNo: (p?: Params) => fetchApi<YesNoResponse>("/api/v1/yes-no", p),
  cohorts: (p?: Params) => fetchApi<{ cohorts: CohortStats[]; cohort_type: string; filters_applied: FilterSummary }>("/api/v1/cohorts", p),
  summary: () => fetchApi<SummaryStats>("/api/v1/stats/summary"),
};
