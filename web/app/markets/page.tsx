"use client";

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { MarketsResponse } from "@/lib/api";

const fetchMarkets = async (params: Record<string, string>) => {
  const base = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3001";
  const url = new URL(`${base}/api/v1/markets`);
  Object.entries(params).forEach(([k, v]) => {
    if (v) url.searchParams.set(k, v);
  });
  const res = await fetch(url.toString());
  return res.json() as Promise<MarketsResponse>;
};

export default function MarketsPage() {
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(0);
  const limit = 50;

  const { data, isLoading } = useQuery({
    queryKey: ["markets", search, page],
    queryFn: () =>
      fetchMarkets({
        limit: String(limit),
        offset: String(page * limit),
        search,
        status: "finalized",
      }),
  });

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold mb-1">Markets</h1>
        <p className="text-sm opacity-60">Browse resolved Kalshi markets by volume.</p>
      </div>

      <input
        type="text"
        placeholder="Search by ticker or title..."
        value={search}
        onChange={(e) => { setSearch(e.target.value); setPage(0); }}
        className="w-full bg-[var(--card-bg)] border border-[var(--border)] rounded-lg px-4 py-2 text-sm focus:outline-none focus:border-[var(--accent-green)]"
      />

      {data && (
        <>
          <p className="text-xs opacity-50">{data.total.toLocaleString()} markets found</p>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[var(--border)] text-left">
                  <th className="py-2 pr-4">Ticker</th>
                  <th className="py-2 pr-4">Title</th>
                  <th className="py-2 pr-4">Status</th>
                  <th className="py-2 pr-4">Result</th>
                  <th className="py-2 pr-4 text-right font-mono">Volume</th>
                </tr>
              </thead>
              <tbody>
                {data.markets.map((m) => (
                  <tr key={m.ticker} className="border-b border-[var(--border)] hover:bg-white/5">
                    <td className="py-2 pr-4 font-mono text-xs">{m.ticker}</td>
                    <td className="py-2 pr-4 text-xs max-w-[300px] truncate">{m.title || "—"}</td>
                    <td className="py-2 pr-4 text-xs">{m.status}</td>
                    <td className="py-2 pr-4">
                      <span className={m.result === "yes" ? "text-[var(--accent-green)]" : m.result === "no" ? "text-[var(--accent-red)]" : ""}>
                        {m.result || "—"}
                      </span>
                    </td>
                    <td className="py-2 pr-4 text-right font-mono">{m.volume.toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="flex gap-2">
            <button
              disabled={page === 0}
              onClick={() => setPage((p) => Math.max(0, p - 1))}
              className="px-3 py-1 text-xs bg-[var(--card-bg)] border border-[var(--border)] rounded disabled:opacity-30"
            >
              Previous
            </button>
            <span className="px-3 py-1 text-xs opacity-50">Page {page + 1}</span>
            <button
              disabled={data.markets.length < limit}
              onClick={() => setPage((p) => p + 1)}
              className="px-3 py-1 text-xs bg-[var(--card-bg)] border border-[var(--border)] rounded disabled:opacity-30"
            >
              Next
            </button>
          </div>
        </>
      )}
      {isLoading && <p className="text-[var(--accent-green)] animate-pulse font-mono">Loading...</p>}
    </div>
  );
}
