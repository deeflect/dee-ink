import InkCharacter from "@/components/InkCharacter";
import ToolList from "@/components/ToolList";
import ScrollProgress from "@/components/ScrollProgress";

export default function Home() {
  return (
    <main className="max-w-[900px] mx-auto px-6 sm:px-10 py-16 sm:py-28">
      <ScrollProgress />
      {/* Header */}
      <header className="mb-16 sm:mb-24">
        <h1 className="text-[2.75rem] sm:text-[3.5rem] font-mono font-medium tracking-tight text-text leading-none">
          dee.ink<span className="terminal-cursor text-muted font-light">▌</span>
        </h1>
        <p className="mt-3 text-[14px] text-muted tracking-wide">
          Small CLI tools for LLMs and humans. Rust, JSON output, pipe-friendly.
        </p>
      </header>

      {/* Main content: tools + character */}
      <div className="flex flex-col lg:flex-row gap-14 lg:gap-16">
        {/* Tool list */}
        <div className="flex-1 min-w-0">
          <ToolList />
        </div>

        {/* Character — bottom on mobile, sticky right on desktop */}
        <div className="lg:w-[260px] shrink-0 order-first lg:order-last">
          <div className="lg:sticky lg:top-28">
            <InkCharacter />
          </div>
        </div>
      </div>

      {/* Footer */}
      <footer className="mt-32 pt-8 border-t border-border/40">
        <div className="flex items-center gap-5 text-[12px] text-muted/60">
          <a
            href="https://github.com/deeflect"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1.5 hover:text-text transition-colors"
          >
            <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
            </svg>
            github
          </a>
          <a
            href="https://x.com/deeflectcom"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1.5 hover:text-text transition-colors"
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
              <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
            </svg>
            x
          </a>
          <a
            href="https://deeflect.com"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1.5 hover:text-text transition-colors"
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
              <circle cx="8" cy="8" r="6.5" />
              <path d="M1.5 8h13M8 1.5c-2 2-3 4-3 6.5s1 4.5 3 6.5c2-2 3-4 3-6.5s-1-4.5-3-6.5z" />
            </svg>
            deeflect.com
          </a>
        </div>
      </footer>
    </main>
  );
}
