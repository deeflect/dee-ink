import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "dee.ink — Small CLI tools for LLMs and humans",
  description:
    "dee.ink by Dee (Dmitrii Kargaev) — Small Rust CLI tools for LLMs and humans. JSON output, pipe-friendly, single-purpose.",
  verification: {
    google: "e7dZdH3xq5QQ8pwZ7DKNDM00fe_e-6HrFBYl5p8TeAY",
  },
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
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{
            __html: JSON.stringify({
              "@context": "https://schema.org",
              "@graph": [
                {
                  "@type": "Person",
                  "@id": "https://www.wikidata.org/entity/Q138828544",
                  name: "Dee Kargaev",
                  givenName: "Dmitry",
                  familyName: "Kargaev",
                  alternateName: ["Dmitry Kargaev", "Dmitrii Kargaev", "Deeflect"],
                  url: "https://www.deeflect.com",
                  image: "https://commons.wikimedia.org/wiki/Special:FilePath/Dmitry%20Kargaev%20(entrepreneur).jpg",
                  description: "Design engineer in Los Angeles. Designs and builds AI products end to end and ships them: Genvas, Herdtrude, Kadr, Typeset.md and Wattlas in 2026. 10+ years in product design, including five years as sole product designer at Datai Network (VALK), an institutional fintech platform serving 70+ banks, hedge funds and asset managers on $4B+ in deals. Author of Don't Replace Me and two SSRN papers on generative engine optimization.",
                  jobTitle: "Design Engineer",
                  knowsLanguage: ["en", "ru"],
                  worksFor: { "@type": "ProfessionalService", "@id": "https://www.dee.agency/#organization", name: "Dee Agency" },
                  address: { "@type": "PostalAddress", addressLocality: "Los Angeles", addressRegion: "CA", addressCountry: "US" },
                  knowsAbout: ["Artificial Intelligence", "Product Design", "Multi-Agent Systems", "Fintech", "AI Agent Architecture", "Rust Programming", "LLM Infrastructure", "Design Systems"],
                  sameAs: [
                    "https://www.deeflect.com",
                    "https://www.wikidata.org/entity/Q138828544",
                    "https://orcid.org/0009-0001-4788-2675",
                    "https://isni.org/isni/0000000530156185",
                    "https://www.linkedin.com/in/dkargaev",
                    "https://github.com/deeflect",
                    "https://x.com/deeflectcom",
                  ],
                },
                {
                  "@type": "SoftwareApplication",
                  "@id": "https://dee.ink/#app",
                  name: "dee.ink CLI tools",
                  url: "https://dee.ink",
                  creator: { "@id": "https://www.wikidata.org/entity/Q138828544" },
                  publisher: { "@id": "https://www.wikidata.org/entity/Q138828544" },
                  applicationCategory: "DeveloperApplication",
                  description: "Small Rust CLI tools for LLMs and humans. JSON output, pipe-friendly, single-purpose.",
                },
                {
                  "@type": "WebSite",
                  "@id": "https://dee.ink/#website",
                  url: "https://dee.ink",
                  name: "dee.ink",
                  description: "Small Rust CLI tools for LLMs and humans. JSON output, pipe-friendly, single-purpose.",
                  publisher: { "@id": "https://www.wikidata.org/entity/Q138828544" },
                  inLanguage: "en-US",
                },
                {
                  "@type": "WebPage",
                  "@id": "https://dee.ink/#webpage",
                  url: "https://dee.ink",
                  name: "dee.ink — Small CLI tools for LLMs and humans",
                  isPartOf: { "@id": "https://dee.ink/#website" },
                },
              ],
            }),
          }}
        />
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
