import Link from "next/link";

const FOOTER_LINKS = [
  {
    title: "Protocol",
    items: [
      { label: "Swap", href: "/swap" },
      { label: "Faucet", href: "/faucet" },
      { label: "Explorer", href: "/explorer" },
    ],
  },
  {
    title: "Developers",
    items: [
      { label: "Documentation", href: "/docs" },
      { label: "RPC API", href: "/docs/rpc" },
      { label: "TypeScript SDK", href: "/docs/sdk" },
      { label: "Litepaper", href: "/litepaper" },
    ],
  },
  {
    title: "Network",
    items: [
      { label: "Run a Validator", href: "/docs/run-validator" },
      { label: "Join Testnet", href: "/docs/testnet" },
    ],
  },
];

const SOCIALS = [
  {
    label: "GitHub",
    href: "https://github.com/aztibase/aztibase-node",
    icon: (
      <svg viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
        <path fillRule="evenodd" d="M10 0C4.477 0 0 4.477 0 10c0 4.42 2.865 8.166 6.839 9.489.5.092.682-.217.682-.482 0-.237-.009-.866-.013-1.7-2.782.604-3.369-1.34-3.369-1.34-.454-1.156-1.11-1.463-1.11-1.463-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.831.092-.646.35-1.086.636-1.336-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0110 4.836c.85.004 1.705.115 2.504.337 1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.203 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.743 0 .267.18.578.688.48C17.138 18.163 20 14.418 20 10c0-5.523-4.477-10-10-10z" clipRule="evenodd" />
      </svg>
    ),
  },
  {
    label: "X / Twitter",
    href: "https://x.com/aztibase",
    icon: (
      <svg viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
        <path d="M15.27 1.61h2.97l-6.49 7.42 7.63 10.09h-5.98l-4.68-6.12-5.36 6.12H.39l6.94-7.93L0 1.61h6.13l4.23 5.6 4.91-5.6zm-1.04 15.7h1.64L5.88 3.29H4.12l10.11 14.02z" />
      </svg>
    ),
  },
];

export default function Footer() {
  return (
    <footer className="border-t border-aztb-500/8 bg-aztb-950 mt-auto">
      <div className="max-w-7xl mx-auto px-4 py-12">
        <div className="grid grid-cols-2 md:grid-cols-5 gap-8">
          {/* Brand */}
          <div className="col-span-2">
            <div className="flex items-center gap-2.5 mb-4">
              <div className="w-6 h-6 rounded-md bg-aztb-accent/15 flex items-center justify-center">
                <svg viewBox="0 0 20 20" fill="none" className="w-3.5 h-3.5 text-aztb-accent">
                  <path d="M10 2L2 6v8l8 4 8-4V6l-8-4z" stroke="currentColor" strokeWidth="2" strokeLinejoin="round" />
                </svg>
              </div>
              <span className="font-heading text-sm font-bold tracking-wider text-aztb-200 uppercase">
                Aztibase
              </span>
            </div>
            <p className="text-sm text-aztb-500 leading-relaxed mb-5 max-w-xs">
              AI-native Layer 1 blockchain. Server-independent architecture, DAG consensus, dual VM execution.
            </p>
            <div className="flex items-center gap-2">
              {SOCIALS.map((s) => (
                <a
                  key={s.label}
                  href={s.href}
                  target="_blank"
                  rel="noopener"
                  className="w-8 h-8 rounded-lg bg-aztb-500/8 border border-aztb-500/10 flex items-center justify-center
                             text-aztb-500 hover:text-aztb-200 hover:border-aztb-500/25 hover:bg-aztb-500/12 transition-all duration-200"
                  aria-label={s.label}
                >
                  {s.icon}
                </a>
              ))}
            </div>
          </div>

          {/* Link columns */}
          {FOOTER_LINKS.map((section) => (
            <div key={section.title}>
              <div className="label mb-3">{section.title}</div>
              <div className="space-y-2.5">
                {section.items.map((item) => (
                  <Link
                    key={item.href}
                    href={item.href}
                    className="block text-sm text-aztb-400 hover:text-aztb-200 transition-colors"
                  >
                    {item.label}
                  </Link>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="border-t border-aztb-500/5 py-5 px-4">
        <div className="max-w-7xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-2 text-xs text-aztb-500">
          <span>Aztibase (Pty) Ltd. Dual licensed MIT / Apache-2.0.</span>
          <span className="font-mono text-aztb-600">aztibase.com</span>
        </div>
      </div>
    </footer>
  );
}
