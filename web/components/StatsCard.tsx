"use client";

interface StatsCardProps {
  label: string;
  value: string | number;
  subtext?: string;
  color?: "green" | "red" | "amber" | "default";
}

const colorMap = {
  green: "text-[var(--accent-green)]",
  red: "text-[var(--accent-red)]",
  amber: "text-[var(--accent-amber)]",
  default: "text-[var(--foreground)]",
};

export function StatsCard({ label, value, subtext, color = "default" }: StatsCardProps) {
  return (
    <div className="bg-[var(--card-bg)] border border-[var(--border)] rounded-lg p-4">
      <p className="text-xs uppercase tracking-wider opacity-60 mb-1">{label}</p>
      <p className={`text-2xl font-mono font-bold ${colorMap[color]}`}>{value}</p>
      {subtext && <p className="text-xs opacity-50 mt-1">{subtext}</p>}
    </div>
  );
}

export function formatNumber(n: number): string {
  if (Math.abs(n) >= 1e9) return `${(n / 1e9).toFixed(2)}B`;
  if (Math.abs(n) >= 1e6) return `${(n / 1e6).toFixed(2)}M`;
  if (Math.abs(n) >= 1e3) return `${(n / 1e3).toFixed(1)}K`;
  return n.toFixed(2);
}

export function formatPct(n: number): string {
  return `${(n * 100).toFixed(2)}%`;
}

export function formatPp(n: number): string {
  return `${n.toFixed(2)}pp`;
}
