"use client";

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { TemporalChart } from "@/components/TemporalChart";
import { StatsCard, formatPp } from "@/components/StatsCard";

export default function TemporalPage() {
  const { data, isLoading } = useQuery({
    queryKey: ["temporal"],
    queryFn: () => api.temporal({ granularity: "quarterly" }),
  });

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold mb-1">Maker-Taker Gap Over Time</h1>
        <p className="text-sm opacity-60">
          The gap flipped from takers winning (+2.0%) to makers winning after Kalshi&apos;s
          Oct 2024 legal victory and the election volume surge — a 5.3pp swing.
        </p>
      </div>

      {data && (
        <>
          {data.structural_break && (
            <div className="grid grid-cols-3 gap-4">
              <StatsCard
                label="Pre-Election Gap"
                value={formatPp(data.structural_break.pre_gap_pp)}
                subtext="Before Oct 2024"
              />
              <StatsCard
                label="Post-Election Gap"
                value={formatPp(data.structural_break.post_gap_pp)}
                subtext="After Oct 2024"
                color="amber"
              />
              <StatsCard
                label="Swing"
                value={formatPp(data.structural_break.swing_pp)}
                color="red"
              />
            </div>
          )}
          <TemporalChart
            data={data.series}
            breakpoint={data.structural_break?.breakpoint}
          />
        </>
      )}
      {isLoading && <p className="text-[var(--accent-green)] animate-pulse font-mono">Loading...</p>}
    </div>
  );
}
