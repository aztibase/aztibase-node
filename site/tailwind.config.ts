import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{js,ts,jsx,tsx,mdx}"],
  theme: {
    extend: {
      colors: {
        aztb: {
          950: "#0a0a14",
          900: "#0f0f1e",
          800: "#161628",
          700: "#1e1e36",
          600: "#2a2a48",
          500: "#8878a6",
          400: "#a498c0",
          300: "#c0b8d4",
          200: "#dcd6e8",
          accent: "#7c6aad",
          green: "#5db88a",
          red: "#c9927e",
          cyan: "#6ac4c8",
        },
      },
      fontFamily: {
        sans: ["var(--font-sans)", "system-ui", "sans-serif"],
        mono: ["var(--font-mono)", "monospace"],
        heading: ["var(--font-heading)", "system-ui", "sans-serif"],
      },
      backgroundImage: {
        "gradient-radial": "radial-gradient(var(--tw-gradient-stops))",
        "gradient-conic": "conic-gradient(from 180deg at 50% 50%, var(--tw-gradient-stops))",
      },
    },
  },
  plugins: [],
};
export default config;
