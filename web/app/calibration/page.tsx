"use client";

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { CalibrationChart } from "@/components/CalibrationChart";
import { StatsCard, formatPct } from "@/components/StatsCard";

export default function CalibrationPage() {
  const { data, isLoading } = useQuery({
    queryKey: ["calibration"],
    queryFn: () => api.calibration({ bucket_width: "10" }),
  });

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold mb-1">Longshot Bias / Calibration Curve</h1>
        <p className="text-sm opacity-60">
          Becker Figure 1 — implied vs realized probability. Points below the 45° line
          indicate overpricing (longshot bias). Contracts at 5¢ win only 4.18% of the time.
        </p>
      </div>

      {data && (
        <>
          <div className="grid grid-cols-3 gap-4">
            <StatsCard
              label="Brier Score"
              value={data.overall_brier_score.toFixed(4)}
              subtext="Lower = better calibrated"
            />
            <StatsCard label="MAE" value={data.overall_mae.toFixed(4)} />
            <StatsCard label="Total Trades" value={data.total_trades.toLocaleString()} />
          </div>
          <CalibrationChart data={data.points} />
        </>
      )}
      {isLoading && <p className="text-[var(--accent-green)] animate-pulse font-mono">Loading...</p>}
    </div>
  );
}
