# PredicTerm — Rust Kalshi Microstructure Terminal

A production-style analytics terminal that replicates and extends [Jonathan Becker's Kalshi microstructure research](https://jbecker.dev/research/prediction-market-microstructure) in a live, interactive system. Built with Rust, DuckDB, and Next.js.

## What This Does

PredicTerm ingests 72M+ Kalshi trades ($18B+ volume) as Parquet files, runs microstructure analytics via embedded DuckDB, and serves results through a typed REST API consumed by an interactive Next.js frontend.

**Key findings reproduced:**

- **Longshot Bias**: 5¢ contracts win only 4.18% of the time (implied: 5%) — a -16% mispricing
- **Optimism Tax**: Takers lose -1.12% per trade; makers earn +1.12% — structural, not informational
- **Temporal Flip**: After Oct 2024, the maker-taker gap swung 5.3pp as professional MMs entered
- **Category Variation**: Finance markets have a 0.17pp gap; World Events exceeds 7pp
- **YES/NO Asymmetry**: At 1¢, YES EV = -41%, NO EV = +23%. NO outperforms at 69/99 price levels

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Backend** | Rust (axum), embedded DuckDB via `duckdb-rs` |
| **Data** | Parquet files (Becker dataset), DuckDB SQL analytics |
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

## Research Background

Based on:
- Becker, J. (2026). [*The Microstructure of Wealth Transfer in Prediction Markets*](https://jbecker.dev/research/prediction-market-microstructure)
- Becker, J. (2025). [prediction-market-analysis](https://github.com/Jon-Becker/prediction-market-analysis) (72M trade dataset)
- Bürgi, C., Deng, W. & Whelan, K. (2025). [*Makers and Takers: The Economics of the Kalshi Prediction Market*](https://cepr.org/publications/dp20631)

## License

MIT
