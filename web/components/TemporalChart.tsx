"use client";

import {
  ComposedChart,
  Area,
  Line,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ReferenceLine,
  ResponsiveContainer,
} from "recharts";
import type { TemporalPoint } from "@/lib/api";

interface Props {
  data: TemporalPoint[];
  breakpoint?: string;
}

export function TemporalChart({ data, breakpoint }: Props) {
  const chartData = data.map((d) => ({
    period: d.period,
    taker: +(d.avg_taker_return * 100).toFixed(2),
    maker: +(d.avg_maker_return * 100).toFixed(2),
    gap: +(d.gap_pp * 100).toFixed(2),
    volume: +(d.total_volume_usd / 1e6).toFixed(1),
  }));

  return (
    <div className="w-full">
      <h3 className="text-sm font-semibold mb-4 opacity-80">
        Maker-Taker Gap Over Time
      </h3>
      <ResponsiveContainer width="100%" height={400}>
        <ComposedChart data={chartData}>
          <CartesianGrid strokeDasharray="3 3" stroke="#333" />
          <XAxis dataKey="period" stroke="#666" fontSize={10} />
          <YAxis yAxisId="return" stroke="#666" fontSize={11} />
          <YAxis yAxisId="vol" orientation="right" stroke="#666" fontSize={11} />
          <Tooltip
            contentStyle={{ background: "#1a1a1a", border: "1px solid #333", borderRadius: 8 }}
          />
          <Legend />
          {breakpoint && (
            <ReferenceLine
              yAxisId="return"
              x={breakpoint}
              stroke="#ffaa00"
              strokeDasharray="3 3"
              label={{ value: "Legal Victory", fill: "#ffaa00", fontSize: 10 }}
            />
          )}
          <Bar yAxisId="vol" dataKey="volume" fill="#ffffff11" name="Volume ($M)" />
          <Area yAxisId="return" type="monotone" dataKey="gap" fill="#ffaa0022" stroke="transparent" name="Gap (pp)" />
          <Line yAxisId="return" type="monotone" dataKey="taker" stroke="#ff4444" strokeWidth={2} name="Taker %" dot={false} />
          <Line yAxisId="return" type="monotone" dataKey="maker" stroke="#00ff88" strokeWidth={2} name="Maker %" dot={false} />
        </ComposedChart>
      </ResponsiveContainer>
    </div>
  );
}
