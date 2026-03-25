# PredicTerm: Kalshi Microstructure Analytics Engine — Implementation Spec

## Executive Summary

This document translates the Kalshi prediction market microstructure literature — primarily Jonathan Becker's "The Microstructure of Wealth Transfer in Prediction Markets" and Bürgi, Deng & Whelan's "Makers and Takers: The Economics of the Kalshi Prediction Market" — into an implementation-ready specification for a Rust + DuckDB analytics backend and Next.js frontend. Every metric includes a plain-English definition, a mathematical formula, a DuckDB SQL sketch, and implementation caveats. The document concludes with prioritized MVP views tied to specific paper findings.

---

## 1. Data Model

### 1.1 Source Schema (Becker's Parquet Dataset)

The dataset from [Becker's `prediction-market-analysis` repo](https://github.com/Jon-Becker/prediction-market-analysis) contains two table families stored as partitioned Parquet files under `data/kalshi/`.

**`trades` table** — one row per executed trade:

| Column | Type | Description |
|--------|------|-------------|
| `ticker` | `VARCHAR` | Market ticker identifier (e.g., `HIGHNY-22DEC23-B53.5`) |
| `yes_price` | `INT` (1–99) | YES contract price in cents |
| `no_price` | `INT` (1–99) | NO contract price in cents; `no_price = 100 - yes_price` |
| `taker_side` | `VARCHAR` (`'yes'` / `'no'`) | Which side the liquidity taker bought |
| `count` | `INT` | Number of contracts in this trade |
| `ts` | `BIGINT` | Unix timestamp of execution |

Per the [Kalshi public trades API](https://docs.kalshi.com/websockets/public-trades), each trade also exposes `trade_id`, `yes_price_dollars`, `no_price_dollars`, and `count_fp`, though Becker's Parquet files use integer-cent pricing.

**`markets` table** — one row per contract:

| Column | Type | Description |
|--------|------|-------------|
| `ticker` | `VARCHAR` | Market ticker (join key to trades) |
| `event_ticker` | `VARCHAR` | Parent event ticker (used for category extraction) |
| `status` | `VARCHAR` | `'finalized'`, `'voided'`, `'open'`, etc. |
| `result` | `VARCHAR` | `'yes'`, `'no'`, or `NULL` for unresolved |
| `category` | `VARCHAR` | High-level category (may be null in older data) |

**Category extraction** — In Becker's codebase, categories are derived from `event_ticker` prefix via a mapping utility ([GitHub: categories.py](https://github.com/Jon-Becker/prediction-market-analysis/blob/main/docs/ANALYSIS.md)):

```sql
CASE
  WHEN event_ticker IS NULL OR event_ticker = '' THEN 'Independent'
  ELSE regexp_extract(event_ticker, '^([A-Z0-9]+)', 1)
END AS event_prefix
```

Then `event_prefix` maps to a group via a lookup table (e.g., `NFLGAME → Sports`, `OSCARS → Entertainment`).

### 1.2 Derived Columns for Analytics

For your Rust backend, precompute these on ingest or as a DuckDB view:

| Derived Column | Formula | Notes |
|----------------|---------|-------|
| `taker_price` | `CASE WHEN taker_side='yes' THEN yes_price ELSE no_price END` | Cost basis for the taker in cents |
| `maker_price` | `100 - taker_price` | Cost basis for the maker (counterparty) |
| `taker_won` | `CASE WHEN taker_side = result THEN 1 ELSE 0 END` | 1 if taker's side matched the resolution |
| `maker_won` | `1 - taker_won` | Zero-sum: if taker won, maker lost |
| `cost_basis` | (see §2.1) | Normalized price from taker or maker perspective |
| `notional` | `taker_price * count` | Dollar volume of this trade (taker side) |
| `trade_date` | `DATE_TRUNC('day', TO_TIMESTAMP(ts))` | For temporal bucketing |

### 1.3 Recommended DuckDB View

```sql
CREATE OR REPLACE VIEW enriched_trades AS
WITH resolved_markets AS (
    SELECT ticker, result, event_ticker, category
    FROM read_parquet('data/kalshi/markets/*.parquet')
    WHERE status = 'finalized'
      AND result IN ('yes', 'no')
)
SELECT
    t.ticker,
    t.yes_price,
    t.no_price,
    t.taker_side,
    t.count,
    t.ts,
    TO_TIMESTAMP(t.ts) AS trade_time,
    DATE_TRUNC('day', TO_TIMESTAMP(t.ts)) AS trade_date,
    DATE_TRUNC('quarter', TO_TIMESTAMP(t.ts)) AS trade_quarter,
    m.result,
    m.event_ticker,
    m.category,
    -- Taker perspective
    CASE WHEN t.taker_side = 'yes' THEN t.yes_price ELSE t.no_price END AS taker_price,
    CASE WHEN t.taker_side = 'yes' THEN t.no_price ELSE t.yes_price END AS maker_price,
    CASE WHEN t.taker_side = m.result THEN 1 ELSE 0 END AS taker_won,
    CASE WHEN t.taker_side != m.result THEN 1 ELSE 0 END AS maker_won,
    -- Notional
    (CASE WHEN t.taker_side = 'yes' THEN t.yes_price ELSE t.no_price END) * t.count AS taker_notional,
    -- Fee (pre-2025 Kalshi structure per Bürgi et al.)
    -- $0.07 * P * (1-P) per contract, P in dollars
    0.07 * (CASE WHEN t.taker_side='yes' THEN t.yes_price ELSE t.no_price END / 100.0)
         * (1.0 - (CASE WHEN t.taker_side='yes' THEN t.yes_price ELSE t.no_price END / 100.0))
         AS taker_fee_per_contract
FROM read_parquet('data/kalshi/trades/*.parquet') t
INNER JOIN resolved_markets m ON t.ticker = m.ticker;
```

### 1.4 Filtering & Exclusions

Following [Becker's methodology](https://jbecker.dev/research/prediction-market-microstructure):

- **Exclude** unresolved, voided, and delisted markets.
- **Exclude** markets with < $100 total notional volume (Becker) or < $1,000 (Bürgi et al., who also require bid-ask spread ≤ 20¢ and market duration ≥ 24h).
- **Include** only `yes_price BETWEEN 1 AND 99` (contracts at 0¢ or 100¢ are trivial).

For your API, expose `min_notional_volume` as a configurable filter parameter so users can replicate either threshold.

---

## 2. Metric Definitions, Formulas, and SQL

### 2.1 Longshot Bias / Favorite-Longshot Bias

**Definition**: The tendency for low-probability contracts to be overpriced (win less often than implied) and high-probability contracts to be underpriced (win more often than implied). First documented by [Griffith (1949)](https://www.jstor.org/stable/1418469) in horse racing, confirmed on Kalshi by both [Becker (2026)](https://jbecker.dev/research/prediction-market-microstructure) and [Bürgi et al. (2025)](https://cepr.org/publications/dp20631).

**Key finding**: Contracts at 5¢ win only 4.18% of the time (implied: 5%), a mispricing of -16.36%. Contracts at 95¢ win 95.83% ([Becker](https://jbecker.dev/research/prediction-market-microstructure)). Buyers of contracts ≤10¢ lose over 60% of their money ([Bürgi et al.](https://cepr.org/voxeu/columns/economics-kalshi-prediction-market)).

#### 2.1.1 Cost Basis Normalization

Becker normalizes all trades to "cost basis" — the capital risked by the participant regardless of YES/NO side:

\[
\text{CostBasis}_{\text{YES}} = \text{yes\_price} \quad \text{(cents)}
\]

\[
\text{CostBasis}_{\text{NO}} = \text{no\_price} = 100 - \text{yes\_price} \quad \text{(cents)}
\]

A 5¢ YES trade risks 5¢. A 5¢ NO trade (i.e., `no_price = 5`, meaning `yes_price = 95`) risks 5¢. Both map to cost basis = 5.

#### 2.1.2 Mispricing

For a set of trades \(S\) at a given price bucket:

\[
\text{Mispricing}(S) = \frac{\sum_{i \in S} \mathbb{1}[\text{won}_i]}{\lvert S \rvert} - \frac{\text{price}}{100}
\]

where \(\text{price}\) is the bucket center (or exact cent value) and \(\text{won}_i\) indicates whether the position resolved profitably.

Equivalently, Mispricing = Realized Win Rate - Implied Probability.

#### 2.1.3 Gross Excess Return

Per [Becker](https://jbecker.dev/research/prediction-market-microstructure), the return relative to cost for a single trade:

\[
R_i = \frac{\text{payout}_i - \text{price}_i}{\text{price}_i} = \frac{\mathbb{1}[\text{won}_i] \times 100 - \text{price}_i}{\text{price}_i}
\]

where \(\text{price}_i\) is in cents, and payout is 100¢ if won, 0 if lost.

#### 2.1.4 Aggregation: Calibration Curve

**Granularity choice**: Per-cent (1–99) or per-bucket (deciles: 1–10, 11–20, ..., 91–99). Becker uses per-cent for calibration curves and 10¢ buckets for summary tables. Bürgi et al. use 10¢ buckets weighted by volume.

**DuckDB SQL — Calibration by price cent**:

```sql
SELECT
    taker_price AS price_cent,
    COUNT(*) AS n_trades,
    SUM(count) AS n_contracts,
    AVG(taker_won::DOUBLE) AS realized_win_rate,
    taker_price / 100.0 AS implied_probability,
    AVG(taker_won::DOUBLE) - taker_price / 100.0 AS mispricing,
    AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_excess_return
FROM enriched_trades
WHERE taker_price BETWEEN 1 AND 99
GROUP BY taker_price
ORDER BY taker_price;
```

**DuckDB SQL — Calibration by 10¢ bucket**:

```sql
SELECT
    FLOOR((taker_price - 1) / 10) * 10 + 1 AS bucket_low,
    FLOOR((taker_price - 1) / 10) * 10 + 10 AS bucket_high,
    COUNT(*) AS n_trades,
    SUM(count) AS n_contracts,
    SUM(taker_notional) / 100.0 AS total_volume_usd,
    AVG(taker_won::DOUBLE) AS realized_win_rate,
    AVG(taker_price / 100.0) AS avg_implied_prob,
    AVG(taker_won::DOUBLE) - AVG(taker_price / 100.0) AS mispricing,
    AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_excess_return
FROM enriched_trades
WHERE taker_price BETWEEN 1 AND 99
GROUP BY FLOOR((taker_price - 1) / 10)
ORDER BY bucket_low;
```

**Caveats**:
- Volume-weighted vs. equal-weighted: Bürgi et al. weight by trading volume share (68% of weight on 1–10¢ and 91–99¢ tails). For your API, support both and let the frontend toggle.
- Partial fills: Each row in Becker's dataset is a completed fill. No partial fill concern at the trade level.
- Survivorship bias: Excluding unresolved markets may bias toward shorter-duration markets. Document this in your UI.

---

### 2.2 Maker-Taker Wealth Transfer (Optimism Tax)

**Definition**: The systematic transfer of capital from liquidity takers (who cross the spread) to liquidity makers (who provide resting orders). On Kalshi, takers lose an average of -1.12% excess return per trade while makers earn +1.12% — a gap of 2.24 percentage points ([Becker](https://jbecker.dev/research/prediction-market-microstructure)). Bürgi et al. find an even larger gap: takers at -31.46% average return, makers at -9.64% (their methodology includes fees and uses contract-level rather than trade-level aggregation).

**Key mechanism**: Makers do not win by out-forecasting takers. Decomposing maker returns by direction: makers buying YES earn +0.77pp, makers buying NO earn +1.25pp — a negligible gap (Cohen's d ≈ 0.02). They profit structurally, not informationally ([Becker](https://jbecker.dev/research/prediction-market-microstructure)).

#### 2.2.1 Per-Trade Excess Return by Role

**Taker**:

\[
R^{\text{taker}}_i = \frac{\mathbb{1}[\text{taker\_won}_i] \times 100 - \text{taker\_price}_i}{\text{taker\_price}_i}
\]

**Maker**:

\[
R^{\text{maker}}_i = \frac{\mathbb{1}[\text{maker\_won}_i] \times 100 - \text{maker\_price}_i}{\text{maker\_price}_i}
\]

where `maker_price = 100 - taker_price`.

Note: These are **gross** of fees. For **net** of fees (per Bürgi et al.):

\[
R^{\text{taker,net}}_i = \frac{\mathbb{1}[\text{taker\_won}_i] \times 100 - \text{taker\_price}_i - \text{fee}_i}{\text{taker\_price}_i + \text{fee}_i}
\]

where \(\text{fee}_i = 7 \times \frac{P_i}{100} \times (1 - \frac{P_i}{100})\) cents per contract, \(P_i\) = taker_price ([Bürgi et al.](https://www2.gwu.edu/~forcpgm/2026-001.pdf)). Makers paid no fees pre-2025.

#### 2.2.2 Aggregate Returns by Role

**DuckDB SQL — Role-level returns by price bucket**:

```sql
WITH role_returns AS (
    SELECT
        taker_price,
        -- Taker return
        (taker_won * 100.0 - taker_price) / taker_price AS taker_return,
        -- Maker return
        (maker_won * 100.0 - maker_price) / maker_price AS maker_return,
        count AS n_contracts,
        taker_notional
    FROM enriched_trades
    WHERE taker_price BETWEEN 1 AND 99
)
SELECT
    FLOOR((taker_price - 1) / 10) * 10 + 1 AS bucket_low,
    FLOOR((taker_price - 1) / 10) * 10 + 10 AS bucket_high,
    COUNT(*) AS n_trades,
    SUM(n_contracts) AS n_contracts,
    -- Equal-weighted
    AVG(taker_return) AS avg_taker_return,
    AVG(maker_return) AS avg_maker_return,
    AVG(maker_return) - AVG(taker_return) AS gap_pp,
    -- Volume-weighted
    SUM(taker_return * taker_notional) / NULLIF(SUM(taker_notional), 0) AS vw_taker_return,
    SUM(maker_return * taker_notional) / NULLIF(SUM(taker_notional), 0) AS vw_maker_return
FROM role_returns
GROUP BY FLOOR((taker_price - 1) / 10)
ORDER BY bucket_low;
```

**DuckDB SQL — Maker return decomposed by direction** (testing spread capture vs. informational edge):

```sql
SELECT
    taker_side AS maker_bought,  -- If taker bought YES, maker bought NO
    CASE WHEN taker_side = 'yes' THEN 'no' ELSE 'yes' END AS maker_side_label,
    COUNT(*) AS n_trades,
    AVG((maker_won * 100.0 - maker_price) / maker_price) AS avg_maker_return,
    STDDEV((maker_won * 100.0 - maker_price) / maker_price) AS std_maker_return
FROM enriched_trades
GROUP BY taker_side;
```

**Caveats**:
- Becker computes returns gross of fees; Bürgi et al. include fees. Your API should support a `include_fees` toggle.
- The maker/taker label is per-trade, not per-account. The same account can be a maker on one trade and a taker on another.
- [HN commenter MajroMax](https://news.ycombinator.com/item?id=46680515) notes that after fees, maker returns in most categories may actually be net negative — only Entertainment, Media, and World Events have gaps large enough to survive the fee. Important caveat for your UI.
- The absence of bid-ask spread data in historical trades means you cannot fully disentangle spread capture from directional edge ([Becker, Limitations](https://jbecker.dev/research/prediction-market-microstructure)).

---

### 2.3 YES/NO Asymmetry (Affirmative Bias)

**Definition**: Takers disproportionately buy YES contracts at longshot prices, and YES longshots underperform NO longshots at the same cost basis. At 1¢, YES has an expected value of -41% while NO has +23% — a 64pp divergence ([Becker](https://jbecker.dev/research/prediction-market-microstructure)). NO outperforms YES at 69 of 99 price levels.

**Mechanism**: Takers account for 41–47% of YES volume in the 1–10¢ range but only 20–24% of NO volume at equivalent prices. The imbalance inverts at 90–99¢. Makers don't need directional skill — they just absorb the optimistic flow.

#### 2.3.1 YES vs NO Return by Price

For this metric, reframe every trade from the perspective of the contract type (YES or NO) rather than the role:

\[
R^{\text{YES}}_i = \frac{\mathbb{1}[\text{result}=\text{'yes'}] \times 100 - \text{yes\_price}_i}{\text{yes\_price}_i}
\]

\[
R^{\text{NO}}_i = \frac{\mathbb{1}[\text{result}=\text{'no'}] \times 100 - \text{no\_price}_i}{\text{no\_price}_i}
\]

**DuckDB SQL — YES vs NO excess return by cost basis**:

```sql
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
)
SELECT
    cost_basis,
    side,
    COUNT(*) AS n_trades,
    SUM(n_contracts) AS n_contracts,
    AVG(won::DOUBLE) AS realized_win_rate,
    cost_basis / 100.0 AS implied_prob,
    AVG(won::DOUBLE) - cost_basis / 100.0 AS mispricing,
    AVG((won * 100.0 - cost_basis) / cost_basis) AS avg_excess_return
FROM all_sides
WHERE cost_basis BETWEEN 1 AND 99
GROUP BY cost_basis, side
ORDER BY cost_basis, side;
```

#### 2.3.2 Role × Side Volume Share

Quantifies the imbalance: what fraction of YES longshot volume comes from takers vs. makers?

```sql
SELECT
    FLOOR((yes_price - 1) / 10) * 10 + 1 AS price_bucket_low,
    -- YES side: taker bought yes, maker bought no
    SUM(CASE WHEN taker_side = 'yes' THEN count ELSE 0 END) AS taker_yes_contracts,
    SUM(CASE WHEN taker_side = 'no' THEN count ELSE 0 END) AS maker_yes_contracts,
    -- Share
    SUM(CASE WHEN taker_side = 'yes' THEN count ELSE 0 END)::DOUBLE /
        NULLIF(SUM(count), 0) AS taker_yes_share,
    SUM(CASE WHEN taker_side = 'no' THEN count ELSE 0 END)::DOUBLE /
        NULLIF(SUM(count), 0) AS taker_no_share
FROM enriched_trades
GROUP BY FLOOR((yes_price - 1) / 10)
ORDER BY price_bucket_low;
```

**Caveats**:
- Becker's cost basis normalization is critical here. Without it, a "5¢ YES" and a "5¢ NO" represent completely different market states (the former is a longshot YES, the latter is a near-certainty YES).
- [HN commenter hbarka](https://news.ycombinator.com/item?id=46680515) asks whether rewording contracts in the negative would change the bias. This is an open question — your UI could flag this as a known limitation.

---

### 2.4 Temporal Evolution of Returns

**Definition**: The maker-taker gap is not static. From 2021–2023, takers earned +2.0% and makers lost -2.0%. After Kalshi's legal victory (Oct 2024) and the election volume surge, this flipped: the gap went from -2.9pp (takers winning) to +2.5pp (makers winning), a 5.3pp swing ([Becker](https://jbecker.dev/research/prediction-market-microstructure)).

**Key insight**: The flip was driven by professional market makers entering after volume grew (from $30M in Q3 2024 to $820M in Q4 2024), not by changes in taker behavior. Taker longshot share was flat at ~4.7% pre- and post-election.

#### 2.4.1 Time-Series of Returns by Role

**DuckDB SQL — Monthly/quarterly maker-taker returns**:

```sql
SELECT
    trade_quarter AS period,
    COUNT(*) AS n_trades,
    SUM(count) AS n_contracts,
    SUM(taker_notional) / 100.0 AS total_volume_usd,
    AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_taker_return,
    AVG((maker_won * 100.0 - maker_price) / maker_price) AS avg_maker_return,
    AVG((maker_won * 100.0 - maker_price) / maker_price) -
        AVG((taker_won * 100.0 - taker_price) / taker_price) AS gap_pp
FROM enriched_trades
GROUP BY trade_quarter
ORDER BY trade_quarter;
```

#### 2.4.2 Pre/Post Structural Break Analysis

Define the breakpoint as `2024-10-01` (Kalshi's legal victory):

```sql
SELECT
    CASE WHEN trade_date < '2024-10-01' THEN 'pre_election' ELSE 'post_election' END AS era,
    COUNT(*) AS n_trades,
    SUM(count) AS n_contracts,
    SUM(taker_notional) / 100.0 AS total_volume_usd,
    AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_taker_return,
    AVG((maker_won * 100.0 - maker_price) / maker_price) AS avg_maker_return,
    AVG((maker_won * 100.0 - maker_price) / maker_price) -
        AVG((taker_won * 100.0 - taker_price) / taker_price) AS gap_pp,
    -- Longshot share stability check
    SUM(CASE WHEN taker_price BETWEEN 1 AND 20 THEN taker_notional ELSE 0 END)::DOUBLE /
        NULLIF(SUM(taker_notional), 0) AS longshot_volume_share
FROM enriched_trades
GROUP BY CASE WHEN trade_date < '2024-10-01' THEN 'pre_election' ELSE 'post_election' END;
```

**Caveats**:
- Monthly granularity may be noisy for early periods (pre-2024 had very low volume). Quarterly is safer.
- The breakpoint is a judgment call. Your API should accept arbitrary `start_time` / `end_time` rather than hardcoding.
- Becker notes taker flow composition was stable — but this should be verified per-category.

---

### 2.5 Category-Level Microstructure

**Definition**: Market efficiency varies dramatically by category. Finance has a maker-taker gap of just 0.17pp; World Events and Media exceed 7pp. This reflects participant selection: dry/quantitative topics attract calibrated traders, while emotional/entertainment topics attract biased flow ([Becker](https://jbecker.dev/research/prediction-market-microstructure)).

| Category | Taker Return | Maker Return | Gap (pp) | N Trades |
|----------|-------------|-------------|----------|----------|
| Sports | -1.11% | +1.12% | 2.23 | 43.6M |
| Politics | -0.51% | +0.51% | 1.02 | 4.9M |
| Crypto | -1.34% | +1.34% | 2.69 | 6.7M |
| Finance | -0.08% | +0.08% | 0.17 | 4.4M |
| Weather | -1.29% | +1.29% | 2.57 | 4.4M |
| Entertainment | -2.40% | +2.40% | 4.79 | 1.5M |
| Media | -3.64% | +3.64% | 7.28 | 0.6M |
| World Events | -3.66% | +3.66% | 7.32 | 0.2M |

Source: [Becker (2026)](https://jbecker.dev/research/prediction-market-microstructure)

**DuckDB SQL — Category returns**:

```sql
SELECT
    category,
    COUNT(*) AS n_trades,
    SUM(count) AS n_contracts,
    SUM(taker_notional) / 100.0 AS total_volume_usd,
    AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_taker_return,
    AVG((maker_won * 100.0 - maker_price) / maker_price) AS avg_maker_return,
    AVG((maker_won * 100.0 - maker_price) / maker_price) -
        AVG((taker_won * 100.0 - taker_price) / taker_price) AS gap_pp
FROM enriched_trades
WHERE category IS NOT NULL
GROUP BY category
ORDER BY gap_pp DESC;
```

**Category + longshot bias intersection**:

```sql
SELECT
    category,
    FLOOR((taker_price - 1) / 10) * 10 + 1 AS bucket_low,
    COUNT(*) AS n_trades,
    AVG(taker_won::DOUBLE) - AVG(taker_price / 100.0) AS mispricing,
    AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_taker_return,
    AVG((maker_won * 100.0 - maker_price) / maker_price) AS avg_maker_return
FROM enriched_trades
WHERE category IS NOT NULL
GROUP BY category, FLOOR((taker_price - 1) / 10)
ORDER BY category, bucket_low;
```

**Caveats**:
- Category mapping depends on Becker's `categories.py` utility. If you're ingesting raw Kalshi API data, you'll need to maintain a similar prefix→category mapping.
- [HN commenter MajroMax](https://news.ycombinator.com/item?id=46680515) observes that Finance markets closely replicate binary options, allowing hedging against traditional markets — this structural difference (not just participant quality) likely explains Finance's efficiency. Note this in UI tooltips.
- Small categories (World Events: 0.2M trades) have wide confidence intervals.

---

### 2.6 Trader Cohorts

**Definition**: Without unique account IDs in Becker's public dataset, trader "cohorts" must be approximated using observable trade characteristics. Neither [Becker](https://jbecker.dev/research/prediction-market-microstructure) nor [Bürgi et al.](https://cepr.org/publications/dp20631) define explicit cohorts, but their work and the [Kalshi API trade schema](https://docs.kalshi.com/api-reference/market/get-trades) suggest practical segmentation approaches.

#### 2.6.1 Trade-Size Cohorts

The [Bürgi et al.](https://www2.gwu.edu/~forcpgm/2026-001.pdf) dataset shows mean trade size of $100 and median of $35. Segment by order size:

| Cohort | Definition | Proxy For |
|--------|-----------|-----------|
| Micro | `count * taker_price / 100 < 10` USD | Casual/retail |
| Small | `10 ≤ notional < 100` USD | Active retail |
| Medium | `100 ≤ notional < 1000` USD | Semi-pro |
| Large | `notional ≥ 1000` USD | Professional/algorithmic |

```sql
SELECT
    CASE
        WHEN taker_notional / 100.0 < 10 THEN 'micro'
        WHEN taker_notional / 100.0 < 100 THEN 'small'
        WHEN taker_notional / 100.0 < 1000 THEN 'medium'
        ELSE 'large'
    END AS size_cohort,
    COUNT(*) AS n_trades,
    AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_taker_return,
    AVG((maker_won * 100.0 - maker_price) / maker_price) AS avg_maker_return,
    AVG((maker_won * 100.0 - maker_price) / maker_price) -
        AVG((taker_won * 100.0 - taker_price) / taker_price) AS gap_pp
FROM enriched_trades
GROUP BY size_cohort
ORDER BY size_cohort;
```

#### 2.6.2 Role-Mix Cohort (Per-Market Aggregation)

Without account IDs, you can still characterize individual markets by their taker/maker composition:

```sql
-- Markets where takers dominate YES longshots
SELECT
    ticker,
    SUM(CASE WHEN taker_side = 'yes' AND taker_price < 20 THEN count ELSE 0 END)::DOUBLE /
        NULLIF(SUM(CASE WHEN taker_price < 20 THEN count ELSE 0 END), 0) AS taker_yes_longshot_share,
    AVG((taker_won * 100.0 - taker_price) / taker_price) AS avg_taker_return
FROM enriched_trades
GROUP BY ticker
HAVING COUNT(*) > 100
ORDER BY taker_yes_longshot_share DESC;
```

#### 2.6.3 Kalshi API Account-Level Data (Private)

If you have private API access with fills data (`/portfolio/fills`), each fill includes `is_taker`, `order_id`, and can be grouped by account. This enables true cohort analysis:

- **Volume bucket**: Total $ traded → quintiles
- **Market diversity**: Number of distinct tickers traded
- **Role consistency**: % of trades where user was maker vs. taker
- **Longshot preference**: Average price of contracts purchased

**Caveat**: Becker's public dataset does not include account IDs. True cohort analysis requires Kalshi's private fills data or a separate data partnership. For v1, trade-size cohorts are the best proxy.

---

### 2.7 Calibration and Scoring Metrics

These metrics assess overall market accuracy rather than wealth transfer dynamics.

#### 2.7.1 Brier Score

The standard metric for probabilistic calibration. Per [Marginal Revolution](https://marginalrevolution.com/?p=91721), Polymarket achieves a Brier Score of 0.058 at 12 hours before resolution — "on par with the best prediction models in existence."

\[
\text{Brier}(S) = \frac{1}{|S|} \sum_{i \in S} (p_i - o_i)^2
\]

where \(p_i\) = implied probability (yes_price / 100) and \(o_i \in \{0, 1\}\) = outcome.

Lower is better. Benchmarks: < 0.125 is good, < 0.1 is great, < 0.05 is excellent.

```sql
SELECT
    category,
    COUNT(*) AS n_trades,
    AVG(POW(yes_price / 100.0 - CASE WHEN result = 'yes' THEN 1.0 ELSE 0.0 END, 2))
        AS brier_score
FROM enriched_trades
GROUP BY category
ORDER BY brier_score;
```

#### 2.7.2 Log Score

More sensitive to extreme miscalibration than Brier:

\[
\text{LogScore}(S) = -\frac{1}{|S|} \sum_{i \in S} \left[ o_i \log(p_i) + (1 - o_i) \log(1 - p_i) \right]
\]

Lower is better. Undefined when \(p_i = 0\) or \(p_i = 1\); clip to [0.01, 0.99].

```sql
SELECT
    category,
    -AVG(
        CASE WHEN result = 'yes'
            THEN LN(GREATEST(yes_price / 100.0, 0.01))
            ELSE LN(GREATEST(1.0 - yes_price / 100.0, 0.01))
        END
    ) AS log_score
FROM enriched_trades
GROUP BY category
ORDER BY log_score;
```

#### 2.7.3 Mean Absolute Error (MAE)

Per [Bürgi et al.](https://cepr.org/voxeu/columns/economics-kalshi-prediction-market), MAE declines as markets approach closing:

\[
\text{MAE} = \frac{1}{|S|} \sum_{i \in S} |p_i - o_i|
\]

This is useful for the "accuracy over time" chart — compute MAE at different horizons (e.g., 10d, 7d, 3d, 1d, 12h before resolution).

**Caveats**:
- Brier/Log scores should be computed on the **last traded price** per market, not every trade. Otherwise high-volume markets dominate.
- For time-to-resolution analysis, you need to compute `market_close_time - trade_time` per trade, which requires the market resolution timestamp.

---

### 2.8 Mincer-Zarnowitz Regression (FLB Statistical Test)

[Bürgi et al.](https://www2.gwu.edu/~forcpgm/2026-001.pdf) use the Mincer-Zarnowitz regression to formally test for favorite-longshot bias:

\[
Y_{ij} - P_{ij} = \alpha + \psi P_{ij} + \varepsilon_{ij}
\]

where \(Y_{ij} \in \{0,1\}\) is the outcome and \(P_{ij}\) is the contract price. Under the null of unbiased pricing, \(\alpha = 0\) and \(\psi = 0\). Rejection (with \(\psi > 0, \alpha < 0\)) confirms FLB.

Their result: \(\psi = 0.034\), \(\alpha = -1.736\), both highly significant, across 156,986 contracts.

This is a regression, not a pure SQL computation. For your Rust backend, either:
1. Precompute in Python/R and cache the result, or
2. Use DuckDB's `regr_slope` and `regr_intercept` aggregate functions:

```sql
SELECT
    REGR_SLOPE(
        (CASE WHEN result = 'yes' THEN 1.0 ELSE 0.0 END) - yes_price / 100.0,
        yes_price / 100.0
    ) AS psi,
    REGR_INTERCEPT(
        (CASE WHEN result = 'yes' THEN 1.0 ELSE 0.0 END) - yes_price / 100.0,
        yes_price / 100.0
    ) AS alpha,
    REGR_R2(
        (CASE WHEN result = 'yes' THEN 1.0 ELSE 0.0 END) - yes_price / 100.0,
        yes_price / 100.0
    ) AS r_squared
FROM enriched_trades;
```

---

## 3. API Specifications

### 3.1 Rust Types

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDate};

// === Request Types ===

#[derive(Deserialize)]
pub struct TimeRange {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct FilterParams {
    #[serde(flatten)]
    pub time_range: TimeRange,
    pub category: Option<String>,
    pub market_id: Option<String>,       // ticker
    pub min_trade_size: Option<f64>,      // USD
    pub min_market_volume: Option<f64>,   // USD, default 100
    pub role_filter: Option<String>,      // "maker", "taker", or None for both
    pub include_fees: Option<bool>,       // default false (gross)
    pub bucket_width: Option<u32>,        // 1 (per-cent), 10 (decile), 20, etc.
}

#[derive(Deserialize)]
pub struct CohortParams {
    #[serde(flatten)]
    pub filters: FilterParams,
    pub cohort_type: Option<String>,      // "size", "role_mix"
}

// === Response Types ===

#[derive(Serialize)]
pub struct CalibrationPoint {
    pub price_bucket_low: u32,
    pub price_bucket_high: u32,
    pub n_trades: u64,
    pub n_contracts: u64,
    pub total_volume_usd: f64,
    pub implied_probability: f64,
    pub realized_win_rate: f64,
    pub mispricing: f64,               // realized - implied
    pub avg_excess_return: f64,
}

#[derive(Serialize)]
pub struct CalibrationResponse {
    pub points: Vec<CalibrationPoint>,
    pub overall_brier_score: f64,
    pub overall_mae: f64,
    pub total_trades: u64,
    pub filters_applied: FilterSummary,
}

#[derive(Serialize)]
pub struct MakerTakerPoint {
    pub price_bucket_low: u32,
    pub price_bucket_high: u32,
    pub n_trades: u64,
    pub avg_taker_return: f64,
    pub avg_maker_return: f64,
    pub gap_pp: f64,
    pub vw_taker_return: f64,          // volume-weighted
    pub vw_maker_return: f64,
}

#[derive(Serialize)]
pub struct MakerTakerResponse {
    pub points: Vec<MakerTakerPoint>,
    pub aggregate_taker_return: f64,
    pub aggregate_maker_return: f64,
    pub aggregate_gap_pp: f64,
    pub total_trades: u64,
    pub total_volume_usd: f64,
    pub filters_applied: FilterSummary,
}

#[derive(Serialize)]
pub struct CategoryStats {
    pub category: String,
    pub n_trades: u64,
    pub n_contracts: u64,
    pub total_volume_usd: f64,
    pub avg_taker_return: f64,
    pub avg_maker_return: f64,
    pub gap_pp: f64,
    pub brier_score: f64,
    pub longshot_mispricing: f64,      // mispricing in 1-20¢ bucket
}

#[derive(Serialize)]
pub struct CategoryResponse {
    pub categories: Vec<CategoryStats>,
    pub filters_applied: FilterSummary,
}

#[derive(Serialize)]
pub struct TemporalPoint {
    pub period: String,               // "2024-Q3", "2024-10", etc.
    pub period_start: NaiveDate,
    pub n_trades: u64,
    pub total_volume_usd: f64,
    pub avg_taker_return: f64,
    pub avg_maker_return: f64,
    pub gap_pp: f64,
    pub longshot_volume_share: f64,   // % of volume in 1-20¢
}

#[derive(Serialize)]
pub struct TemporalResponse {
    pub series: Vec<TemporalPoint>,
    pub granularity: String,           // "monthly", "quarterly"
    pub structural_break: Option<StructuralBreak>,
    pub filters_applied: FilterSummary,
}

#[derive(Serialize)]
pub struct StructuralBreak {
    pub breakpoint: NaiveDate,
    pub pre_gap_pp: f64,
    pub post_gap_pp: f64,
    pub swing_pp: f64,
}

#[derive(Serialize)]
pub struct YesNoPoint {
    pub cost_basis: u32,
    pub yes_return: f64,
    pub no_return: f64,
    pub yes_n_trades: u64,
    pub no_n_trades: u64,
    pub divergence_pp: f64,           // no_return - yes_return
    pub taker_yes_share: f64,         // taker share of YES volume
    pub taker_no_share: f64,
}

#[derive(Serialize)]
pub struct YesNoResponse {
    pub points: Vec<YesNoPoint>,
    pub aggregate_yes_return: f64,
    pub aggregate_no_return: f64,
    pub n_levels_no_outperforms: u32, // out of 99
    pub filters_applied: FilterSummary,
}

#[derive(Serialize)]
pub struct CohortStats {
    pub cohort_label: String,
    pub n_trades: u64,
    pub n_contracts: u64,
    pub total_volume_usd: f64,
    pub avg_taker_return: f64,
    pub avg_maker_return: f64,
    pub gap_pp: f64,
}

#[derive(Serialize)]
pub struct CohortResponse {
    pub cohorts: Vec<CohortStats>,
    pub cohort_type: String,
    pub filters_applied: FilterSummary,
}

#[derive(Serialize)]
pub struct FilterSummary {
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub category: Option<String>,
    pub market_id: Option<String>,
    pub min_trade_size: Option<f64>,
    pub min_market_volume: Option<f64>,
    pub include_fees: bool,
}
```

### 3.2 REST API Endpoints

| Method | Path | Request Params | Response Type | Description |
|--------|------|---------------|---------------|-------------|
| `GET` | `/api/v1/calibration` | `FilterParams` | `CalibrationResponse` | Longshot bias calibration curve |
| `GET` | `/api/v1/maker-taker` | `FilterParams` | `MakerTakerResponse` | Maker vs taker returns by price bucket |
| `GET` | `/api/v1/categories` | `FilterParams` | `CategoryResponse` | Per-category efficiency stats |
| `GET` | `/api/v1/temporal` | `FilterParams` + `granularity` | `TemporalResponse` | Returns over time |
| `GET` | `/api/v1/yes-no` | `FilterParams` | `YesNoResponse` | YES/NO asymmetry at each price |
| `GET` | `/api/v1/cohorts` | `CohortParams` | `CohortResponse` | Trade-size cohort analysis |
| `GET` | `/api/v1/market/{ticker}` | — | `MarketDetail` | Single-market deep dive |
| `GET` | `/api/v1/stats/summary` | `FilterParams` | `SummaryStats` | Overall dataset summary |

### 3.3 TypeScript Types (Next.js Frontend)

```typescript
// === Shared ===
interface FilterParams {
  start_time?: string;        // ISO 8601
  end_time?: string;
  category?: string;
  market_id?: string;
  min_trade_size?: number;
  min_market_volume?: number;
  role_filter?: 'maker' | 'taker';
  include_fees?: boolean;
  bucket_width?: number;
}

// === Calibration ===
interface CalibrationPoint {
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

interface CalibrationResponse {
  points: CalibrationPoint[];
  overall_brier_score: number;
  overall_mae: number;
  total_trades: number;
  filters_applied: FilterSummary;
}

// === Maker-Taker ===
interface MakerTakerPoint {
  price_bucket_low: number;
  price_bucket_high: number;
  n_trades: number;
  avg_taker_return: number;
  avg_maker_return: number;
  gap_pp: number;
  vw_taker_return: number;
  vw_maker_return: number;
}

interface MakerTakerResponse {
  points: MakerTakerPoint[];
  aggregate_taker_return: number;
  aggregate_maker_return: number;
  aggregate_gap_pp: number;
  total_trades: number;
  total_volume_usd: number;
  filters_applied: FilterSummary;
}

// === Categories ===
interface CategoryStats {
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

// === Temporal ===
interface TemporalPoint {
  period: string;
  period_start: string;
  n_trades: number;
  total_volume_usd: number;
  avg_taker_return: number;
  avg_maker_return: number;
  gap_pp: number;
  longshot_volume_share: number;
}

interface StructuralBreak {
  breakpoint: string;
  pre_gap_pp: number;
  post_gap_pp: number;
  swing_pp: number;
}

// === YES/NO ===
interface YesNoPoint {
  cost_basis: number;
  yes_return: number;
  no_return: number;
  yes_n_trades: number;
  no_n_trades: number;
  divergence_pp: number;
  taker_yes_share: number;
  taker_no_share: number;
}
```

### 3.4 DuckDB Performance Considerations

With ~72M trades across Parquet files:

1. **Partition by date**: The `data/kalshi/trades/` directory is already partitioned. Leverage DuckDB's Hive-style partition pruning if you repartition by `trade_date` or `year/month`.

2. **Materialized views**: Precompute `enriched_trades` as a materialized Parquet file rather than a virtual view — the JOIN to resolved markets is the most expensive operation. Estimated one-time cost: ~30s on the full dataset.

3. **Pre-aggregate summary tables**: For the category and temporal endpoints, store daily/weekly pre-aggregated rollups:

```sql
CREATE TABLE daily_category_rollup AS
SELECT
    trade_date,
    category,
    COUNT(*) AS n_trades,
    SUM(count) AS n_contracts,
    SUM(taker_notional) AS total_taker_notional,
    SUM(taker_won * count) AS taker_wins,
    SUM(maker_won * count) AS maker_wins,
    SUM(taker_price * count) AS sum_taker_price_x_contracts,
    SUM(maker_price * count) AS sum_maker_price_x_contracts
FROM enriched_trades
GROUP BY trade_date, category;
```

4. **Column pruning**: DuckDB on Parquet natively pushes down column selection. Your queries should `SELECT` only needed columns.

5. **Approximate distinct**: If you add account-level analysis later, use `APPROX_COUNT_DISTINCT` for trader counts on large datasets.

6. **Concurrent reads**: DuckDB supports concurrent read queries from multiple threads. For your axum server, use a connection pool or a single reader connection with `access_mode = 'READ_ONLY'`.

---

## 4. MVP Prioritization

### 4.1 Headline Views (v1 — Must Ship)

#### View 1: Calibration Curve (Longshot Bias)

**Paper tie-in**: Becker's Figure 1 — the calibration curve showing implied vs. realized probability. The "5¢ contracts win 4.18% of the time" finding is the paper's opening hook and the single most striking statistic.

**What to show**: Scatter/line chart: X = implied probability (1–99¢), Y = realized win rate. A 45° reference line shows perfect calibration. Points below the line (low prices) represent overpricing; points above (high prices) represent underpricing. Overlay mispricing magnitude as a bar chart below.

**Filters**: Category, time range, bucket width (1¢ or 10¢ decile).

**API**: `GET /api/v1/calibration`

---

#### View 2: Maker-Taker PnL by Price Level

**Paper tie-in**: Becker's Table 1 and Figure 2 — the core wealth transfer finding. Takers at -1.12%, makers at +1.12%. The divergence is sharpest at tails: -57% mispricing for takers at 1¢.

**What to show**: Dual-line chart: X = price bucket, Y = average excess return. One line for takers (red/negative), one for makers (green/positive). Shaded area between them is the "Optimism Tax." KPI cards above: aggregate taker return, maker return, gap, total volume.

**Filters**: Category, time range, include fees toggle, volume-weighted toggle.

**API**: `GET /api/v1/maker-taker`

---

#### View 3: Maker-Taker Gap Over Time

**Paper tie-in**: Becker's temporal evolution analysis showing the flip from takers winning (+2.0%) to makers winning (-2.0%) coinciding with the Oct 2024 legal victory and election volume surge. "Pre-election gap: -2.9pp, post-election: +2.5pp, swing of 5.3pp."

**What to show**: Time-series area chart: X = quarter/month, Y = maker-taker gap. Secondary Y-axis or overlay: total volume. Vertical annotation line at Oct 2024. KPI cards: pre/post gap, swing.

**Filters**: Category, granularity (monthly/quarterly).

**API**: `GET /api/v1/temporal`

---

#### View 4: Category Efficiency Heatmap

**Paper tie-in**: Becker's Table 2 — the category variation from Finance (0.17pp) to World Events (7.32pp). "When the topic is dry and quantitative, the market is efficient. When the topic allows for tribalism and hope, the market transforms into a mechanism for transferring wealth."

**What to show**: Horizontal bar chart or heatmap: categories ranked by maker-taker gap. Color intensity = gap magnitude. Volume shown as bar width or secondary axis. Click-through to per-category calibration curve.

**Filters**: Time range.

**API**: `GET /api/v1/categories`

---

### 4.2 Secondary Views (v1.1 — High Value)

#### View 5: YES/NO Asymmetry Explorer

**Paper tie-in**: Becker's YES/NO section — "At 1¢, YES has EV of -41%, NO has +23%, a 64pp divergence." NO outperforms YES at 69 of 99 price levels.

**What to show**: Diverging bar chart: X = cost basis (1–99¢), Y = return for YES (left) vs. NO (right). Separate panel: taker share of YES vs. NO volume by price bucket.

**API**: `GET /api/v1/yes-no`

---

#### View 6: Trade-Size Cohort Returns

**Paper tie-in**: Bürgi et al.'s observation that median transaction is $35 while mean is $100, implying heavy tail of large trades. Implicit in Becker's discussion of "professionalization of liquidity."

**What to show**: Grouped bar chart: X = cohort (micro/small/medium/large), Y = average return by role. Table below with detailed stats.

**API**: `GET /api/v1/cohorts`

---

#### View 7: Market-Level Deep Dive

**What to show**: For a selected market ticker: trade timeline, final price vs. outcome, per-trade returns colored by role, volume bars. Useful for case studies (e.g., a specific election market or sports event).

**API**: `GET /api/v1/market/{ticker}`

---

### 4.3 Skip for Now (Low ROI for v1)

| Metric | Reason to Defer |
|--------|----------------|
| Mincer-Zarnowitz regression (§2.8) | Statistical test, not a visual insight. Useful for a "methodology" page, not a dashboard. |
| Log Score | Brier is more widely understood. Add later as an alternative scoring toggle. |
| Bürgi et al.'s matching model calibration (β, σ, θ) | Theoretical model parameters. Interesting for researchers but not actionable for traders. |
| Time-to-resolution accuracy decay | Requires market close timestamps not always present in the trade-level data. Add when you have reliable market metadata. |
| Cross-market arbitrage detection | Out of scope for Kalshi-only v1; relevant when Polymarket support is added. |
| Bid-ask spread analysis | Historical trade data doesn't include order book state ([Becker, Limitations](https://jbecker.dev/research/prediction-market-microstructure)). Cannot compute without orderbook snapshots. |

---

## 5. Implementation Notes

### 5.1 Known Critiques to Address in Design

1. **Fee impact** ([HN: MajroMax](https://news.ycombinator.com/item?id=46680515)): After Kalshi's fee (7% × P × (1-P) per contract), maker returns are net negative in most categories. Your UI should default to gross returns (matching Becker) but offer a "net of fees" toggle with a clear disclaimer.

2. **Time value of money** ([HN: MajroMax](https://news.ycombinator.com/item?id=46680515)): Contracts tie up capital until resolution. A "sure thing" at 97.5¢ for 6 months may lose to risk-free rate. This is not modeled in either paper. Add a note in your "About" section.

3. **Spread compensation confound** ([Becker, Limitations](https://jbecker.dev/research/prediction-market-microstructure)): Without order book data, maker returns cannot be cleanly decomposed into spread capture vs. alpha. Becker's direction-decomposition test (makers buying YES vs. NO) partially addresses this but with tiny effect sizes (Cohen's d ≈ 0.02).

4. **Maker ≠ Sophisticated** ([Becker](https://jbecker.dev/research/prediction-market-microstructure)): In the early period (2021–2023), makers lost money. The maker/taker label is a per-trade role, not an identity. Your UI should explain this clearly.

5. **Finance efficiency alternative explanation** ([HN: MajroMax](https://news.ycombinator.com/item?id=46680515)): Finance markets may be efficient because they are hedgeable against traditional instruments (binary options on S&P), not just because participants are more sophisticated. Surface this in tooltips.

6. **Post-April 2025 fee change** ([Bürgi et al.](https://www2.gwu.edu/~forcpgm/2026-001.pdf)): Kalshi began charging maker fees after April 2025. If your data extends past this date, update the fee formula to include maker fees.

### 5.2 Parquet Ingestion Pipeline (Rust)

For your Rust backend using `duckdb-rs`:

```rust
// Pseudocode for DuckDB connection setup
use duckdb::{Connection, params};

fn create_connection(data_dir: &str) -> Connection {
    let conn = Connection::open_in_memory().unwrap();

    // Create the enriched view
    conn.execute_batch(&format!(r#"
        CREATE VIEW enriched_trades AS
        WITH resolved_markets AS (
            SELECT ticker, result, event_ticker, category
            FROM read_parquet('{data_dir}/kalshi/markets/*.parquet')
            WHERE status = 'finalized' AND result IN ('yes', 'no')
        )
        SELECT
            t.*, m.result, m.event_ticker, m.category,
            CASE WHEN t.taker_side='yes' THEN t.yes_price
                 ELSE t.no_price END AS taker_price,
            CASE WHEN t.taker_side='yes' THEN t.no_price
                 ELSE t.yes_price END AS maker_price,
            CASE WHEN t.taker_side = m.result THEN 1 ELSE 0 END AS taker_won,
            CASE WHEN t.taker_side != m.result THEN 1 ELSE 0 END AS maker_won,
            (CASE WHEN t.taker_side='yes' THEN t.yes_price
                  ELSE t.no_price END) * t.count AS taker_notional,
            TO_TIMESTAMP(t.ts) AS trade_time,
            DATE_TRUNC('day', TO_TIMESTAMP(t.ts)) AS trade_date,
            DATE_TRUNC('quarter', TO_TIMESTAMP(t.ts)) AS trade_quarter
        FROM read_parquet('{data_dir}/kalshi/trades/*.parquet') t
        INNER JOIN resolved_markets m ON t.ticker = m.ticker
    "#)).unwrap();

    conn
}
```

### 5.3 Category Mapping

Maintain a `HashMap<String, String>` in your Rust code that maps event ticker prefixes to high-level categories. Becker's repo includes a Python utility for this. Port it to Rust:

```rust
use std::collections::HashMap;

fn build_category_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Sports
    for prefix in ["NFLGAME", "NBAGAME", "MLB", "NHL", "MLS", "SOCCER", "UFC", "GOLF"] {
        m.insert(prefix, "Sports");
    }
    // Politics
    for prefix in ["PRES", "SENATE", "HOUSE", "GOV", "SCOTUS", "CONGRESS"] {
        m.insert(prefix, "Politics");
    }
    // Crypto
    for prefix in ["BTC", "ETH", "SOL", "CRYPTO", "DOGE"] {
        m.insert(prefix, "Crypto");
    }
    // Finance
    for prefix in ["FED", "CPI", "GDP", "JOBS", "SPX", "NASDAQ", "RATES"] {
        m.insert(prefix, "Finance");
    }
    // ... extend as needed from Becker's categories.py
    m
}
```

---

## 6. References

All findings and formulas in this spec are derived from:

- Becker, J. (2026). "The Microstructure of Wealth Transfer in Prediction Markets." [jbecker.dev](https://jbecker.dev/research/prediction-market-microstructure)
- Bürgi, C., Deng, W. & Whelan, K. (2025). "Makers and Takers: The Economics of the Kalshi Prediction Market." CEPR Discussion Paper No. 20631. [CEPR](https://cepr.org/publications/dp20631) / [GWU PDF](https://www2.gwu.edu/~forcpgm/2026-001.pdf)
- Bürgi, C., Deng, W. & Whelan, K. (2026). "The economics of the Kalshi prediction market." [VoxEU](https://cepr.org/voxeu/columns/economics-kalshi-prediction-market)
- Becker, J. (2025). prediction-market-analysis GitHub repository. [GitHub](https://github.com/Jon-Becker/prediction-market-analysis)
- Kalshi API documentation. [docs.kalshi.com](https://docs.kalshi.com/api-reference/market/get-trades)
- HN Discussion thread on Becker's paper. [Hacker News](https://news.ycombinator.com/item?id=46680515)
- Tabarrok, A. (2025). "Prediction Markets Are Very Accurate." [Marginal Revolution](https://marginalrevolution.com/?p=91721)
