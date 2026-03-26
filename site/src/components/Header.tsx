"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useState, useEffect } from "react";
import { NAV_LINKS } from "@/config/chain";
import { cn } from "@/lib/utils";
import WalletConnect from "./WalletConnect";

export default function Header() {
  const pathname = usePathname();
  const [menuOpen, setMenuOpen] = useState(false);
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 8);
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <header
      className={cn(
        "sticky top-0 z-50 transition-all duration-300",
        scrolled
          ? "bg-aztb-950/90 backdrop-blur-xl border-b border-aztb-500/10 shadow-lg shadow-black/10"
          : "bg-transparent border-b border-transparent"
      )}
    >
      <div className="max-w-7xl mx-auto px-4 h-14 flex items-center gap-6">
        <Link href="/" className="flex items-center gap-2.5 shrink-0 group">
          <div className="relative w-6 h-6 flex items-center justify-center">
            <div className="absolute inset-0 rounded-md bg-aztb-accent/20 group-hover:bg-aztb-accent/30 transition-colors" />
            <svg viewBox="0 0 20 20" fill="none" className="w-3.5 h-3.5 text-aztb-accent relative">
              <path d="M10 2L2 6v8l8 4 8-4V6l-8-4z" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" />
            </svg>
          </div>
          <span className="font-heading text-sm font-bold tracking-wider text-aztb-200 uppercase">
            Aztibase
          </span>
        </Link>

        <nav className="hidden md:flex items-center gap-0.5 flex-1">
          {NAV_LINKS.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              className={cn(
                "px-3 py-1.5 rounded-md text-sm transition-all duration-200",
                pathname === link.href || pathname.startsWith(link.href + "/")
                  ? "text-aztb-200 bg-aztb-accent/10 font-medium"
                  : "text-aztb-400 hover:text-aztb-200 hover:bg-aztb-500/8"
              )}
            >
              {link.label}
            </Link>
          ))}
        </nav>

        <div className="hidden md:block">
          <WalletConnect />
        </div>

        <button
          className="md:hidden ml-auto text-aztb-400 hover:text-aztb-200 p-2 rounded-lg hover:bg-aztb-500/8 transition-colors"
          onClick={() => setMenuOpen(!menuOpen)}
          aria-label="Toggle menu"
        >
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            {menuOpen ? (
              <path d="M5 5l10 10M15 5L5 15" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            ) : (
              <path d="M3 5h14M3 10h14M3 15h14" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            )}
          </svg>
        </button>
      </div>

      {/* Mobile menu */}
      <div
        className={cn(
          "md:hidden overflow-hidden transition-all duration-300 border-t",
          menuOpen
            ? "max-h-96 opacity-100 border-aztb-500/10 bg-aztb-950/95 backdrop-blur-xl"
            : "max-h-0 opacity-0 border-transparent"
        )}
      >
        <div className="px-4 py-3 space-y-1">
          {NAV_LINKS.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              onClick={() => setMenuOpen(false)}
              className={cn(
                "block px-3 py-2.5 rounded-lg text-sm transition-colors",
                pathname === link.href
                  ? "text-aztb-200 bg-aztb-accent/10 font-medium"
                  : "text-aztb-400 hover:text-aztb-200 hover:bg-aztb-500/8"
              )}
            >
              {link.label}
            </Link>
          ))}
          <div className="pt-3 border-t border-aztb-500/8">
            <WalletConnect />
          </div>
        </div>
      </div>
    </header>
  );
}
