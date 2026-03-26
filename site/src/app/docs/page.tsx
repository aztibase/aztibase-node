import Link from "next/link";
import { DOCS_NAV } from "@/config/docs-nav";

const SECTION_ICONS: Record<string, React.ReactNode> = {
  "Getting Started": (
    <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4">
      <path d="M10 2v16M2 10h16" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  ),
  "Architecture": (
    <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4">
      <path d="M10 2L2 6v8l8 4 8-4V6l-8-4z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
    </svg>
  ),
  "Run a Node": (
    <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4">
      <rect x="3" y="5" width="14" height="10" rx="2" stroke="currentColor" strokeWidth="1.5" />
      <circle cx="7" cy="10" r="1" fill="currentColor" />
      <path d="M10 8h4M10 12h4" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  ),
  "Network": (
    <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4">
      <circle cx="10" cy="10" r="7" stroke="currentColor" strokeWidth="1.5" />
      <circle cx="10" cy="10" r="2" stroke="currentColor" strokeWidth="1.2" />
      <path d="M10 3v4M10 13v4M3 10h4M13 10h4" stroke="currentColor" strokeWidth="1" strokeLinecap="round" />
    </svg>
  ),
  "API Reference": (
    <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4">
      <path d="M7 5l-4 5 4 5M13 5l4 5-4 5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  ),
  "Reference": (
    <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4">
      <path d="M4 4h12v12H4z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
      <path d="M7 8h6M7 12h4" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  ),
};

export default function DocsIndex() {
  return (
    <div>
      <h1>Documentation</h1>
      <p>
        Everything you need to build on, run, and understand the Aztibase Network.
      </p>

      <div className="grid md:grid-cols-2 gap-4 mt-8 not-prose">
        {DOCS_NAV.map((section) => (
          <div key={section.title} className="card-glow p-5 group">
            <div className="flex items-center gap-2.5 mb-3">
              <div className="w-7 h-7 rounded-lg bg-aztb-accent/10 border border-aztb-accent/15 flex items-center justify-center text-aztb-accent
                              group-hover:bg-aztb-accent/15 group-hover:border-aztb-accent/25 transition-all duration-300">
                {SECTION_ICONS[section.title] || SECTION_ICONS["Reference"]}
              </div>
              <h3 className="text-sm font-heading font-bold text-aztb-200 uppercase tracking-wider">
                {section.title}
              </h3>
            </div>
            <div className="space-y-1">
              {section.items.map((item) => (
                <Link
                  key={item.href}
                  href={item.href}
                  className="flex items-center gap-2 text-sm text-aztb-400 hover:text-aztb-accent transition-colors py-0.5 group/link"
                >
                  <span className="w-1 h-1 rounded-full bg-aztb-500/40 group-hover/link:bg-aztb-accent transition-colors shrink-0" />
                  {item.title}
                </Link>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
