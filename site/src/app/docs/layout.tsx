"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useState } from "react";
import { DOCS_NAV } from "@/config/docs-nav";
import { cn } from "@/lib/utils";

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const sidebar = (
    <nav className="space-y-6">
      {DOCS_NAV.map((section) => (
        <div key={section.title}>
          <div className="label mb-2 px-2">{section.title}</div>
          <div className="space-y-0.5">
            {section.items.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                onClick={() => setSidebarOpen(false)}
                className={cn(
                  "block px-2 py-1.5 rounded-md text-sm transition-colors",
                  pathname === item.href
                    ? "text-aztb-200 bg-aztb-accent/10 font-medium"
                    : "text-aztb-400 hover:text-aztb-300 hover:bg-aztb-500/5"
                )}
              >
                {item.title}
              </Link>
            ))}
          </div>
        </div>
      ))}
    </nav>
  );

  return (
    <div className="max-w-7xl mx-auto px-4 flex gap-8">
      {/* Desktop sidebar */}
      <aside className="hidden lg:block w-56 shrink-0 py-8 sticky top-14 h-[calc(100vh-3.5rem)] overflow-y-auto">
        {sidebar}
      </aside>

      {/* Mobile sidebar toggle */}
      <button
        className="lg:hidden fixed bottom-4 right-4 z-40 btn-primary w-12 h-12 rounded-full flex items-center justify-center shadow-lg"
        onClick={() => setSidebarOpen(!sidebarOpen)}
      >
        ☰
      </button>
      {sidebarOpen && (
        <div className="lg:hidden fixed inset-0 z-30 bg-aztb-950/90 backdrop-blur-sm" onClick={() => setSidebarOpen(false)}>
          <div
            className="absolute left-0 top-14 bottom-0 w-64 bg-aztb-900 border-r border-aztb-500/10 p-4 overflow-y-auto"
            onClick={(e) => e.stopPropagation()}
          >
            {sidebar}
          </div>
        </div>
      )}

      {/* Content */}
      <div className="flex-1 min-w-0 py-8">
        <div className="prose max-w-3xl">
          {children}
        </div>
      </div>
    </div>
  );
}
