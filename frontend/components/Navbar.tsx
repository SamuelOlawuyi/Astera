"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import WalletConnect from "./WalletConnect";

const links = [
  { href: "/dashboard", label: "Dashboard" },
  { href: "/invest", label: "Invest" },
  { href: "/history", label: "History" },
  { href: "/invoice/new", label: "New Invoice" },
];

export default function Navbar() {
  const path = usePathname();

  return (
    <header className="fixed top-0 left-0 right-0 z-50 border-b border-brand-border bg-brand-dark/80 backdrop-blur-md">
      <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between gap-8">
        <Link href="/" className="font-bold text-xl tracking-tight">
          <span className="gradient-text">Astera</span>
        </Link>

        <nav className="hidden md:flex items-center gap-1">
          {links.map((l) => (
            <Link
              key={l.href}
              href={l.href}
              className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                path === l.href
                  ? "bg-brand-gold/10 text-brand-gold"
                  : "text-brand-muted hover:text-white hover:bg-brand-card"
              }`}
            >
              {l.label}
            </Link>
          ))}
        </nav>

        <WalletConnect />
      </div>
    </header>
  );
}
