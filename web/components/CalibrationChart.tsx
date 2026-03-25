"use client";

import {
  ComposedChart,
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
import type { CalibrationPoint } from "@/lib/api";

interface Props {
  data: CalibrationPoint[];
}

export function CalibrationChart({ data }: Props) {
  const chartData = data.map((d) => ({
    bucket: `${d.price_bucket_low}-${d.price_bucket_high}`,
    implied: +(d.implied_probability * 100).toFixed(1),
    realized: +(d.realized_win_rate * 100).toFixed(1),
    mispricing: +(d.mispricing * 100).toFixed(2),
  }));

  return (
    <div className="w-full">
      <h3 className="text-sm font-semibold mb-4 opacity-80">
        Calibration: Implied vs Realized Win Rate
      </h3>
      <ResponsiveContainer width="100%" height={400}>
        <ComposedChart data={chartData}>
          <CartesianGrid strokeDasharray="3 3" stroke="#333" />
          <XAxis dataKey="bucket" stroke="#666" fontSize={11} />
          <YAxis yAxisId="pct" stroke="#666" fontSize={11} domain={[0, 100]} />
          <YAxis yAxisId="mis" orientation="right" stroke="#666" fontSize={11} />
          <Tooltip
            contentStyle={{ background: "#1a1a1a", border: "1px solid #333", borderRadius: 8 }}
          />
          <Legend />
          <ReferenceLine yAxisId="mis" y={0} stroke="#555" strokeDasharray="3 3" />
          <Line
            yAxisId="pct"
            type="monotone"
            dataKey="implied"
            stroke="#666"
            strokeDasharray="5 5"
            name="Implied %"
            dot={false}
          />
          <Line
            yAxisId="pct"
            type="monotone"
            dataKey="realized"
            stroke="#00ff88"
            strokeWidth={2}
            name="Realized %"
            dot={{ r: 3 }}
          />
          <Bar yAxisId="mis" dataKey="mispricing" fill="#ffaa0066" name="Mispricing (pp)" />
        </ComposedChart>
      </ResponsiveContainer>
    </div>
  );
}
