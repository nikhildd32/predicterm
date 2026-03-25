"use client";

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { formatNumber, formatPct, formatPp } from "@/components/StatsCard";

export default function CategoriesPage() {
  const { data, isLoading } = useQuery({
    queryKey: ["categories"],
    queryFn: () => api.categories(),
  });

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold mb-1">Category Efficiency</h1>
        <p className="text-sm opacity-60">
          Market efficiency varies dramatically by category. When the topic is dry and
          quantitative, the market is efficient. When the topic allows for tribalism and
          hope, the wealth transfer intensifies.
        </p>
      </div>

      {data && (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-[var(--border)] text-left">
                <th className="py-3 pr-4">Category</th>
                <th className="py-3 pr-4 text-right font-mono">Trades</th>
                <th className="py-3 pr-4 text-right font-mono">Volume</th>
                <th className="py-3 pr-4 text-right font-mono">Taker Return</th>
                <th className="py-3 pr-4 text-right font-mono">Maker Return</th>
                <th className="py-3 pr-4 text-right font-mono">Gap (pp)</th>
                <th className="py-3 pr-4 text-right font-mono">Brier</th>
              </tr>
            </thead>
            <tbody>
              {data.categories.map((cat) => (
                <tr key={cat.category} className="border-b border-[var(--border)] hover:bg-white/5">
                  <td className="py-2 pr-4 font-semibold">{cat.category}</td>
                  <td className="py-2 pr-4 text-right font-mono">{formatNumber(cat.n_trades)}</td>
                  <td className="py-2 pr-4 text-right font-mono">${formatNumber(cat.total_volume_usd)}</td>
                  <td className="py-2 pr-4 text-right font-mono text-[var(--accent-red)]">
                    {formatPct(cat.avg_taker_return)}
                  </td>
                  <td className="py-2 pr-4 text-right font-mono text-[var(--accent-green)]">
                    {formatPct(cat.avg_maker_return)}
                  </td>
                  <td className="py-2 pr-4 text-right font-mono text-[var(--accent-amber)]">
                    {formatPp(cat.gap_pp)}
                  </td>
                  <td className="py-2 pr-4 text-right font-mono">{cat.brier_score.toFixed(4)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {isLoading && <p className="text-[var(--accent-green)] animate-pulse font-mono">Loading...</p>}
    </div>
  );
}
