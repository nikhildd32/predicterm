"use client";

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { MakerTakerChart } from "@/components/MakerTakerChart";
import { StatsCard, formatNumber, formatPct, formatPp } from "@/components/StatsCard";

export default function MakerTakerPage() {
  const { data, isLoading } = useQuery({
    queryKey: ["maker-taker"],
    queryFn: () => api.makerTaker({ bucket_width: "10" }),
  });

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold mb-1">Maker-Taker Wealth Transfer</h1>
        <p className="text-sm opacity-60">
          The core &ldquo;Optimism Tax&rdquo; finding: takers pay a structural premium
          and makers harvest it without requiring superior forecasting ability.
        </p>
      </div>

      {data && (
        <>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <StatsCard
              label="Taker Return"
              value={formatPct(data.aggregate_taker_return)}
              color="red"
            />
            <StatsCard
              label="Maker Return"
              value={formatPct(data.aggregate_maker_return)}
              color="green"
            />
            <StatsCard
              label="Gap"
              value={formatPp(data.aggregate_gap_pp)}
              color="amber"
            />
            <StatsCard label="Volume" value={`$${formatNumber(data.total_volume_usd)}`} />
          </div>
          <MakerTakerChart data={data.points} />
        </>
      )}
      {isLoading && <p className="text-[var(--accent-green)] animate-pulse font-mono">Loading...</p>}
    </div>
  );
}
