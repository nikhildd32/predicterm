# PredicTerm — Rust Kalshi Microstructure Terminal

A production-style analytics terminal that explores prediction market microstructure on Kalshi. Built with Rust, DuckDB, and Next.js.

## What This Does

PredicTerm ingests 72M+ Kalshi trades ($18B+ volume) as Parquet files, runs microstructure analytics via embedded DuckDB, and serves results through a typed REST API consumed by an interactive Next.js frontend.

**Key findings explored:**

- **Longshot Bias**: Low-probability contracts are systematically overpriced; buyers of longshots lose money on average
- **Optimism Tax**: Takers pay a structural premium; makers harvest it without requiring superior information
- **Temporal Evolution**: The maker-taker gap flipped as professional market makers entered after the Oct 2024 volume surge
- **Category Variation**: Finance markets are near-efficient (tiny gap); entertainment and emotional topics show large wealth transfer
- **YES/NO Asymmetry**: YES longshots dramatically underperform NO longshots at the same cost basis

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Backend** | Rust (axum), embedded DuckDB via `duckdb-rs` |
| **Data** | Parquet files, DuckDB SQL analytics with pre-aggregated rollups |
| **Frontend** | Next.js 16, React 19, TypeScript, Tailwind CSS, Recharts |
| **Infra** | Docker Compose, GitHub Actions CI |

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/calibration` | Longshot bias calibration curve |
| `GET` | `/api/v1/maker-taker` | Maker vs taker returns by price bucket |
| `GET` | `/api/v1/categories` | Per-category efficiency stats |
| `GET` | `/api/v1/temporal` | Returns over time with structural break |
| `GET` | `/api/v1/yes-no` | YES/NO asymmetry at each price |
| `GET` | `/api/v1/cohorts` | Trade-size cohort analysis |
| `GET` | `/api/v1/markets` | Paginated markets list |
| `GET` | `/api/v1/stats/summary` | Dataset summary |

All endpoints accept `FilterParams` query parameters: `category`, `market_id`, `bucket_width`, `min_trade_size`, etc.

## Quick Start

### Prerequisites

- Rust 1.85+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Node.js 20+ (via nvm)
- ~40 GB disk for dataset

### 1. Download dataset

```bash
brew install zstd  # macOS
curl -L -o data.tar.zst https://s3.jbecker.dev/data.tar.zst
tar --use-compress-program=unzstd -xf data.tar.zst
rm data.tar.zst
```

### 2. Run the API

```bash
cargo run -p predicterm-api --release
# Serves on http://localhost:3001
```

### 3. Run the frontend

```bash
cd web && npm install && npm run dev
# Serves on http://localhost:3000
```

### Docker Compose (alternative)

```bash
docker compose up --build
```

## Project Structure

```
predicterm/
├── crates/
│   ├── core/          # Shared models, DuckDB pool, analytics SQL
│   │   └── src/analytics/  # calibration, maker_taker, temporal, yes_no, cohorts, categories
│   ├── etl/           # CLI: validate schema, print dataset stats
│   └── api/           # Axum HTTP server with all endpoints
├── web/               # Next.js frontend
│   ├── app/           # App Router pages (calibration, maker-taker, temporal, etc.)
│   ├── components/    # Recharts visualization components
│   └── lib/           # Typed API client
├── data/              # Parquet files (not in git)
├── Dockerfile.api
├── docker-compose.yml
└── .github/workflows/ci.yml
```

## Performance

On a 72M trade dataset (50 GB on disk), after one-time materialization (~45s on startup):

- All analytics endpoints respond in **<300ms** (most under 25ms)
- Pre-aggregated rollup tables eliminate per-request full table scans
- DuckDB embedded engine with 8 threads and 6 GB memory budget

## License

MIT
