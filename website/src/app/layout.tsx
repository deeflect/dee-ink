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
                  "@id": "https://deeflect.com/#person",
                  name: "Dmitry Kargaev",
                  givenName: "Dmitry",
                  familyName: "Kargaev",
                  alternateName: ["Dee Kargaev", "Dmitrii Kargaev", "Deeflect"],
                  url: "https://www.deeflect.com",
                  image: "https://www.deeflect.com/dmitry-kargaev.jpg",
                  description: "Author, AI Engineer, and Product Designer based in Los Angeles. Author of Don't Replace Me: A Survival Guide to the AI Apocalypse. Former Lead Product Designer at VALK, a $4B+ fintech platform used by 70+ banks across 15 countries. Now builds multi-agent AI systems and ships products for founders.",
                  jobTitle: "Author, AI Engineer & Product Designer",
                  knowsLanguage: ["en", "ru"],
                  worksFor: { "@type": "ProfessionalService", "@id": "https://dee.agency/#organization", name: "Dee Agency" },
                  address: { "@type": "PostalAddress", addressLocality: "Los Angeles", addressRegion: "CA", addressCountry: "US" },
                  knowsAbout: ["Artificial Intelligence", "Product Design", "Multi-Agent Systems", "Fintech", "AI Agent Architecture", "Rust Programming", "LLM Infrastructure", "Design Systems"],
                  sameAs: [
                    "https://www.deeflect.com",
                    "https://dontreplace.me",
                    "https://blog.deeflect.com",
                    "https://dee.agency",
                    "https://dee.rest",
                    "https://dee.house",
                    "https://x.com/deeflectcom",
                    "https://github.com/deeflect",
                    "https://www.linkedin.com/in/dkargaev/",
                    "https://www.behance.net/dkargaev",
                    "https://www.crunchbase.com/person/dmitry-kargaev",
                    "https://wellfound.com/u/deeflect-2",
                    "https://deeflect.hashnode.dev",
                    "https://dev.to/deeflect",
                    "https://www.producthunt.com/@dkargaev",
                    "https://g.dev/deeflect",
                    "https://dribbble.com/dkargaev",
                    "https://hackernoon.com/u/deeflect",
                    "https://substack.com/@deeflect",
                    "https://bsky.app/profile/deeflect.bsky.social",
                    "https://www.threads.net/@deeflect",
                    "https://www.pinterest.com/dkargaev/",
                    "https://www.amazon.com/author/deeflect",
                    "https://openlibrary.org/authors/OL16246203A",
                    "https://orcid.org/0009-0001-4788-2675",
                    "https://isni.org/isni/0000000530156185",
                    "https://www.wikidata.org/wiki/Q138828544",
        "https://www.youtube.com/@deeflectcom",
        "https://www.goodreads.com/author/show/69455077.Dmitry_Kargaev",
                  ],
                },
                {
                  "@type": "WebSite",
                  "@id": "https://dee.ink/#website",
                  url: "https://dee.ink",
                  name: "dee.ink",
                  description: "Small Rust CLI tools for LLMs and humans. JSON output, pipe-friendly, single-purpose.",
                  publisher: { "@id": "https://deeflect.com/#person" },
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
