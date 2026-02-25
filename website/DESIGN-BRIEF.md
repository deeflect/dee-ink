# dee.ink Website Design Brief

## Vibe
Minimalist, clean, notion-like + art/character element. Not a typical dev tools page. More like a personal creative portfolio that happens to be CLI tools. Think: dee.rest energy but with an animated character.

## Layout (Desktop)
```
┌──────────────────────────────────────────────────────┐
│                    dee.ink                            │
│              max-width: ~900px, centered             │
│              big padding left/right                  │
│                                                      │
│  ┌─────────────┐    ┌──────────────────────────┐    │
│  │             │    │  TOOLS                    │    │
│  │  Animated   │    │                           │    │
│  │  Character  │    │  ▸ Productivity           │    │
│  │  (left)     │    │    dee-contacts            │    │
│  │             │    │    dee-habit                │    │
│  │  Subtle     │    │    dee-todo                 │    │
│  │  idle anim  │    │    dee-stash                │    │
│  │  or float   │    │    dee-timer                │    │
│  │             │    │                            │    │
│  │             │    │  ▸ Marketing               │    │
│  │             │    │    dee-crosspost            │    │
│  │             │    │    dee-mentions             │    │
│  │             │    │                            │    │
│  │             │    │  ▸ Finance                  │    │
│  │             │    │    dee-invoice              │    │
│  │             │    │    dee-receipt              │    │
│  │             │    │    dee-rates                │    │
│  │             │    │  ...etc                     │    │
│  └─────────────┘    └──────────────────────────┘    │
│                                                      │
│              footer: github · x · dee.ink            │
└──────────────────────────────────────────────────────┘
```

## Character
- Left side, sticky/fixed as you scroll (or just top area on mobile)
- Animated: subtle idle animation — floating, breathing, blinking, something chill
- Style TBD — could be the borb character, could be something new for dee brand
- CSS/SVG animation or Lottie, NOT heavy JS framework

## Tool List (Right Side)
- Grouped by category (Productivity, Marketing, Finance, Shopping, Dev Tools, Data/Research)
- Each tool = one line: `dee-toolname` with a tiny status indicator (✅ released / 🔜 coming)
- On click/tap: expands accordion-style showing:
  ```
  dee-habit — Track daily habits and streaks
  
  cargo install dee-habit          [copy button]
  
  curl -sSL dee.ink/i/habit | sh   [copy button]  ← agent-friendly install
  
  [GitHub →]
  ```
- Collapsed by default, clean
- No search, no filters — just scroll and browse

## Mobile
- Character moves to top (small, centered)
- Tool list below, full width
- Same accordion behavior

## Style
- Background: off-white or very light gray (#fafafa)
- Text: near-black (#1a1a1a)
- Accent: one color (ink blue? #2563eb or similar)
- Font: Inter or similar clean sans-serif
- Monospace for CLI commands: JetBrains Mono or Fira Code
- Spacing: generous, airy, notion-like
- No gradients, no shadows, no cards — flat and clean
- Category headers: uppercase, small, muted color, letter-spaced

## Tech Stack
- Next.js (static export, no server needed)
- Tailwind CSS
- Framer Motion for character animation + accordion
- Deploy on Vercel
- Domain: dee.ink

## Pages
- Just ONE page. Everything on index.
- Maybe a `/tool/dee-habit` dynamic route later for SEO but not v1

## Data Source
- Tool list from a single `tools.json` file:
```json
[
  {
    "name": "dee-habit",
    "description": "Track daily habits and streaks",
    "category": "Productivity",
    "status": "coming",
    "github": "https://github.com/deeflect/dee-habit",
    "crate": "dee-habit"
  }
]
```

## Copy/Tone
- Header: "dee.ink" (big, clean)
- Subheader: "CLI tools built for AI agents." (one line, done)
- No paragraphs, no "about" section, no "why" section
- The tools ARE the content
- Footer: minimal links

## Inspiration
- dee.rest (Dee's existing site)
- notion.so (spacing, typography)
- charm.sh (CLI tool brand)
- fig.io (was clean before shutdown)
