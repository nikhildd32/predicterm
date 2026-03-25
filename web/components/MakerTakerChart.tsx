"use client";

import {
  AreaChart,
  Area,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  ComposedChart,
} from "recharts";
import type { MakerTakerPoint } from "@/lib/api";

interface Props {
  data: MakerTakerPoint[];
}

export function MakerTakerChart({ data }: Props) {
  const chartData = data.map((d) => ({
    bucket: `${d.price_bucket_low}-${d.price_bucket_high}`,
    taker: +(d.avg_taker_return * 100).toFixed(2),
    maker: +(d.avg_maker_return * 100).toFixed(2),
    gap: +(d.gap_pp * 100).toFixed(2),
  }));

  return (
    <div className="w-full">
      <h3 className="text-sm font-semibold mb-4 opacity-80">
        Maker vs Taker Excess Return by Price Bucket
      </h3>
      <ResponsiveContainer width="100%" height={400}>
        <ComposedChart data={chartData}>
          <CartesianGrid strokeDasharray="3 3" stroke="#333" />
          <XAxis dataKey="bucket" stroke="#666" fontSize={11} />
          <YAxis stroke="#666" fontSize={11} />
          <Tooltip
            contentStyle={{ background: "#1a1a1a", border: "1px solid #333", borderRadius: 8 }}
          />
          <Legend />
          <Area
            type="monotone"
            dataKey="gap"
            fill="#ffaa0022"
            stroke="transparent"
            name="Gap (pp)"
          />
          <Line type="monotone" dataKey="taker" stroke="#ff4444" strokeWidth={2} name="Taker %" dot={{ r: 3 }} />
          <Line type="monotone" dataKey="maker" stroke="#00ff88" strokeWidth={2} name="Maker %" dot={{ r: 3 }} />
        </ComposedChart>
      </ResponsiveContainer>
    </div>
  );
}
