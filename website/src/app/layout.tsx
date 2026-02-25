import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "dee.ink — Small CLI tools for LLMs and humans",
  description:
    "Small Rust CLI tools for LLMs and humans. JSON output, pipe-friendly, single-purpose.",
  metadataBase: new URL("https://dee.ink"),
  icons: {
    icon: "/favicon.png",
    apple: "/favicon.png",
  },
  openGraph: {
    title: "dee.ink",
    description: "Small Rust CLI tools for LLMs and humans. JSON output, pipe-friendly.",
    url: "https://dee.ink",
    siteName: "dee.ink",
    type: "website",
    locale: "en_US",
    images: [{ url: "/og.jpg", width: 1200, height: 630, alt: "dee.ink" }],
  },
  twitter: {
    card: "summary_large_image",
    title: "dee.ink",
    description: "Small Rust CLI tools for LLMs and humans. JSON output, pipe-friendly.",
    creator: "@deeflectcom",
    images: ["/og.jpg"],
  },
  robots: {
    index: true,
    follow: true,
  },
  alternates: {
    canonical: "https://dee.ink",
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link
          rel="preconnect"
          href="https://fonts.gstatic.com"
          crossOrigin="anonymous"
        />
        <link
          href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap"
          rel="stylesheet"
        />
      </head>
      <body className="min-h-screen">{children}</body>
    </html>
  );
}
