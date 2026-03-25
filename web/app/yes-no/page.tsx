"use client";

import { useQuery } from "@tanstack/react-query";
import {
  ComposedChart,
  Bar,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  ReferenceLine,
} from "recharts";
import { api } from "@/lib/api";
import { StatsCard, formatPct } from "@/components/StatsCard";

export default function YesNoPage() {
  const { data, isLoading } = useQuery({
    queryKey: ["yes-no"],
    queryFn: () => api.yesNo(),
  });

  const chartData = data?.points
    .filter((p) => p.cost_basis >= 1 && p.cost_basis <= 50)
    .map((p) => ({
      basis: p.cost_basis,
      yes: +(p.yes_return * 100).toFixed(1),
      no: +(p.no_return * 100).toFixed(1),
      divergence: +(p.divergence_pp * 100).toFixed(1),
    }));

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold mb-1">YES/NO Asymmetry</h1>
        <p className="text-sm opacity-60">
          At 1¢, YES has an EV of -41% while NO has +23% — a 64pp divergence.
          NO outperforms YES at 69 of 99 price levels. Takers disproportionately buy YES longshots.
        </p>
      </div>

      {data && (
        <>
          <div className="grid grid-cols-3 gap-4">
            <StatsCard label="Agg YES Return" value={formatPct(data.aggregate_yes_return)} color="red" />
            <StatsCard label="Agg NO Return" value={formatPct(data.aggregate_no_return)} color="green" />
            <StatsCard
              label="NO Outperforms at"
              value={`${data.n_levels_no_outperforms}/99 levels`}
              color="amber"
            />
          </div>
          <div className="w-full">
            <h3 className="text-sm font-semibold mb-4 opacity-80">
              Excess Return: YES vs NO (Longshot Half, 1-50¢)
            </h3>
            <ResponsiveContainer width="100%" height={400}>
              <ComposedChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis dataKey="basis" stroke="#666" fontSize={11} label={{ value: "Cost Basis (¢)", position: "bottom", fill: "#666" }} />
                <YAxis stroke="#666" fontSize={11} />
                <Tooltip contentStyle={{ background: "#1a1a1a", border: "1px solid #333", borderRadius: 8 }} />
                <Legend />
                <ReferenceLine y={0} stroke="#555" />
                <Line type="monotone" dataKey="yes" stroke="#ff4444" strokeWidth={2} name="YES Return %" dot={false} />
                <Line type="monotone" dataKey="no" stroke="#00ff88" strokeWidth={2} name="NO Return %" dot={false} />
                <Bar dataKey="divergence" fill="#ffaa0044" name="Divergence (pp)" />
              </ComposedChart>
            </ResponsiveContainer>
          </div>
        </>
      )}
      {isLoading && <p className="text-[var(--accent-green)] animate-pulse font-mono">Loading...</p>}
    </div>
  );
}
