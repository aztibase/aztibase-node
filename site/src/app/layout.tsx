import type { Metadata } from "next";
import { Instrument_Sans, Fira_Code, Jura } from "next/font/google";
import "./globals.css";
import Header from "@/components/Header";
import Footer from "@/components/Footer";

const sans = Instrument_Sans({ subsets: ["latin"], variable: "--font-sans" });
const mono = Fira_Code({ subsets: ["latin"], variable: "--font-mono", weight: ["400", "600"] });
const heading = Jura({ subsets: ["latin"], variable: "--font-heading", weight: ["400", "700"] });

export const metadata: Metadata = {
  title: {
    default: "Aztibase Network",
    template: "%s — Aztibase",
  },
  description: "AI-native Layer 1 blockchain. Server-independent. DAG consensus. Dual VM.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className={`${sans.variable} ${mono.variable} ${heading.variable} font-sans antialiased min-h-screen flex flex-col`}>
        <Header />
        <main className="flex-1">{children}</main>
        <Footer />
      </body>
    </html>
  );
}
