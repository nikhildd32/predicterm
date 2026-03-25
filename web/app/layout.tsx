import type { Metadata } from "next";
import Link from "next/link";
import { Providers } from "@/lib/providers";
import "./globals.css";

export const metadata: Metadata = {
  title: "PredicTerm — Kalshi Microstructure Terminal",
  description:
    "Explore prediction market microstructure: longshot bias, maker-taker wealth transfer, and the Optimism Tax on Kalshi.",
};

const NAV_ITEMS = [
  { href: "/", label: "Overview" },
  { href: "/calibration", label: "Calibration" },
  { href: "/maker-taker", label: "Maker/Taker" },
  { href: "/temporal", label: "Temporal" },
  { href: "/categories", label: "Categories" },
  { href: "/yes-no", label: "YES/NO" },
  { href: "/cohorts", label: "Cohorts" },
  { href: "/markets", label: "Markets" },
];

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className="min-h-screen">
        <Providers>
          <header className="border-b border-[var(--border)] px-6 py-3 flex items-center gap-8">
            <Link href="/" className="text-lg font-bold tracking-tight text-[var(--accent-green)]">
              PredicTerm
            </Link>
            <nav className="flex gap-4 text-sm">
              {NAV_ITEMS.map((item) => (
                <Link
                  key={item.href}
                  href={item.href}
                  className="text-[var(--foreground)] opacity-70 hover:opacity-100 transition-opacity"
                >
                  {item.label}
                </Link>
              ))}
            </nav>
          </header>
          <main className="max-w-[1400px] mx-auto px-6 py-8">{children}</main>
        </Providers>
      </body>
    </html>
  );
}
