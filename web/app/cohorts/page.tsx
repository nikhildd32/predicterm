"use client";

import { useQuery } from "@tanstack/react-query";
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from "recharts";
import { api } from "@/lib/api";
import { formatNumber, formatPct, formatPp } from "@/components/StatsCard";

export default function CohortsPage() {
  const { data, isLoading } = useQuery({
    queryKey: ["cohorts"],
    queryFn: () => api.cohorts(),
  });

  const chartData = data?.cohorts.map((c) => ({
    cohort: c.cohort_label,
    taker: +(c.avg_taker_return * 100).toFixed(2),
    maker: +(c.avg_maker_return * 100).toFixed(2),
  }));

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold mb-1">Trade-Size Cohorts</h1>
        <p className="text-sm opacity-60">
          Segmented by notional value: micro (&lt;$10), small ($10-100),
          medium ($100-1K), large ($1K+). The median Kalshi transaction is ~$35 while the mean is ~$100.
        </p>
      </div>

      {data && (
        <>
          <ResponsiveContainer width="100%" height={350}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#333" />
              <XAxis dataKey="cohort" stroke="#666" fontSize={12} />
              <YAxis stroke="#666" fontSize={11} />
              <Tooltip contentStyle={{ background: "#1a1a1a", border: "1px solid #333", borderRadius: 8 }} />
              <Legend />
              <Bar dataKey="taker" fill="#ff4444" name="Taker Return %" />
              <Bar dataKey="maker" fill="#00ff88" name="Maker Return %" />
            </BarChart>
          </ResponsiveContainer>

          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[var(--border)] text-left">
                  <th className="py-3 pr-4">Cohort</th>
                  <th className="py-3 pr-4 text-right font-mono">Trades</th>
                  <th className="py-3 pr-4 text-right font-mono">Volume</th>
                  <th className="py-3 pr-4 text-right font-mono">Taker</th>
                  <th className="py-3 pr-4 text-right font-mono">Maker</th>
                  <th className="py-3 pr-4 text-right font-mono">Gap</th>
                </tr>
              </thead>
              <tbody>
                {data.cohorts.map((c) => (
                  <tr key={c.cohort_label} className="border-b border-[var(--border)]">
                    <td className="py-2 pr-4 font-semibold capitalize">{c.cohort_label}</td>
                    <td className="py-2 pr-4 text-right font-mono">{formatNumber(c.n_trades)}</td>
                    <td className="py-2 pr-4 text-right font-mono">${formatNumber(c.total_volume_usd)}</td>
                    <td className="py-2 pr-4 text-right font-mono text-[var(--accent-red)]">{formatPct(c.avg_taker_return)}</td>
                    <td className="py-2 pr-4 text-right font-mono text-[var(--accent-green)]">{formatPct(c.avg_maker_return)}</td>
                    <td className="py-2 pr-4 text-right font-mono text-[var(--accent-amber)]">{formatPp(c.gap_pp)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
      {isLoading && <p className="text-[var(--accent-green)] animate-pulse font-mono">Loading...</p>}
    </div>
  );
}
