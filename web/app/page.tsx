"use client";

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { StatsCard, formatNumber } from "@/components/StatsCard";

export default function HomePage() {
  const { data, isLoading, error } = useQuery({
    queryKey: ["summary"],
    queryFn: api.summary,
  });

  if (isLoading) return <Loading />;
  if (error) return <ErrorState message={(error as Error).message} />;
  if (!data) return null;

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold mb-2">PredicTerm</h1>
        <p className="text-sm opacity-60 max-w-2xl">
          Exploring the microstructure of wealth transfer in prediction markets.
          Analyzing 72M+ Kalshi trades, $18B+ in volume. Built with Rust, DuckDB, and Parquet.
        </p>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <StatsCard label="Total Trades" value={formatNumber(data.total_trades)} color="green" />
        <StatsCard label="Total Contracts" value={formatNumber(data.total_contracts)} />
        <StatsCard label="Total Volume" value={`$${formatNumber(data.total_volume_usd)}`} color="amber" />
        <StatsCard label="Resolved Markets" value={formatNumber(data.resolved_markets)} />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <StatsCard label="Date Range Start" value={data.date_range_start} />
        <StatsCard label="Date Range End" value={data.date_range_end} />
      </div>

      <div className="bg-[var(--card-bg)] border border-[var(--border)] rounded-lg p-6 space-y-3">
        <h2 className="text-lg font-semibold">Key Insights</h2>
        <p className="text-sm opacity-70">
          This terminal exposes prediction market microstructure patterns from the
          Kalshi dataset. Key findings:
        </p>
        <ul className="text-sm opacity-70 list-disc list-inside space-y-1">
          <li><strong>Longshot Bias:</strong> 5¢ contracts win only 4.18% of the time (implied: 5%)</li>
          <li><strong>Optimism Tax:</strong> Takers lose -1.12% per trade; makers earn +1.12%</li>
          <li><strong>Temporal Flip:</strong> After Oct 2024, makers went from losing to winning — a 5.3pp swing</li>
          <li><strong>Category Variation:</strong> Finance gap is 0.17pp; World Events exceeds 7pp</li>
          <li><strong>YES/NO Asymmetry:</strong> At 1¢, YES EV = -41%, NO EV = +23%</li>
        </ul>
      </div>
    </div>
  );
}

function Loading() {
  return (
    <div className="flex items-center justify-center h-64">
      <div className="text-[var(--accent-green)] animate-pulse font-mono">Loading dataset...</div>
    </div>
  );
}

function ErrorState({ message }: { message: string }) {
  return (
    <div className="flex items-center justify-center h-64">
      <div className="text-[var(--accent-red)] font-mono text-sm">
        Error: {message}
        <p className="text-xs opacity-50 mt-2">Make sure the API is running on port 3001</p>
      </div>
    </div>
  );
}
