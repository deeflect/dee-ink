"use client";

import { useState, useCallback, useEffect, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";
import allTools from "@/data/tools.json";

const tools = allTools.filter((t) => t.status === "built");

type Tool = (typeof allTools)[number];

const CATEGORIES = [
  "Productivity",
  "Marketing",
  "Finance",
  "Shopping",
  "Dev Tools",
  "Data & Research",
] as const;

/* ── Animated count that ticks up when visible ── */
function AnimatedCount({ target }: { target: number }) {
  const [count, setCount] = useState(0);
  const ref = useRef<HTMLSpanElement>(null);
  const triggered = useRef(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const obs = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting && !triggered.current) {
          triggered.current = true;
          let i = 0;
          const iv = setInterval(() => {
            i++;
            setCount(i);
            if (i >= target) clearInterval(iv);
          }, 60);
        }
      },
      { threshold: 0.5 }
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, [target]);

  return <span ref={ref} className="ml-2 text-muted/30">{count}</span>;
}

/* ── Copy row with ink splat ── */
function CopyRow({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  const [splat, setSplat] = useState<{ x: number; y: number; id: number } | null>(null);

  const copy = useCallback((e: React.MouseEvent<HTMLButtonElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    setSplat({ x, y, id: Date.now() });
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }, [text]);

  return (
    <button
      onClick={copy}
      className="relative w-full flex items-center bg-code-bg rounded-lg px-3 py-2.5 font-mono text-[12px] hover:bg-code-bg/80 transition-colors cursor-pointer group overflow-hidden"
    >
      {splat && (
        <span
          key={splat.id}
          className="ink-splat pointer-events-none absolute rounded-full bg-text/20"
          style={{ left: splat.x, top: splat.y, width: 60, height: 60 }}
          onAnimationEnd={() => setSplat(null)}
        />
      )}
      <code className="text-text flex-1 overflow-x-auto whitespace-nowrap text-left">
        {text}
      </code>
      <span className="ml-2 shrink-0 text-[11px] text-muted/50 group-hover:text-ink transition-colors">
        {copied ? "✓ copied" : "copy"}
      </span>
    </button>
  );
}

function GithubIcon({ size = 13 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 16 16" fill="currentColor">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
    </svg>
  );
}

function ToolRow({
  tool,
  isOpen,
  isHighlighted,
  onToggle,
}: {
  tool: Tool;
  isOpen: boolean;
  isHighlighted: boolean;
  onToggle: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const slug = tool.crate.replace("dee-", "");

  useEffect(() => {
    if (isHighlighted) ref.current?.scrollIntoView({ block: "nearest" });
  }, [isHighlighted]);

  return (
    <div ref={ref}>
      <button
        onClick={onToggle}
        className={`w-full flex items-center gap-3 py-2.5 px-3 text-left hover:bg-text/[0.03] rounded-lg transition-colors cursor-pointer group ${isHighlighted ? "bg-ink/[0.04] border-l-2 border-ink" : ""}`}
      >
        <span className="font-mono text-[13.5px] font-medium text-text tracking-tight whitespace-nowrap shrink-0">
          {tool.name.replace("dee-", "")}
        </span>
        <span className="text-muted/70 text-[13px] hidden sm:inline truncate">
          {tool.description}
        </span>
        <svg
          width="14"
          height="14"
          viewBox="0 0 16 16"
          fill="none"
          className={`ml-auto shrink-0 text-muted/30 group-hover:text-muted/50 transition-transform duration-150 ${isOpen ? "rotate-90" : ""}`}
        >
          <path d="M6 4l4 4-4 4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>

      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.15, ease: "easeOut" }}
            className="overflow-hidden"
          >
            <div className="pl-3 pr-3 pb-5 pt-1 space-y-2">
              <p className="text-muted text-[13px] sm:hidden leading-relaxed">
                {tool.description}
              </p>

              <div className="space-y-1.5">
                <CopyRow text={`cargo install ${tool.crate}`} />
                <CopyRow text={`curl -sSL dee.ink/i/${slug} | sh`} />
              </div>

              <a
                href={tool.github}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex items-center gap-1.5 text-[12px] text-muted hover:text-ink transition-colors"
              >
                <GithubIcon size={12} />
                GitHub
              </a>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

export default function ToolList() {
  const allTools = CATEGORIES.flatMap((c) => tools.filter((t) => t.category === c));
  const [openTool, setOpenTool] = useState<string | null>(null);
  const [highlighted, setHighlighted] = useState<number>(-1);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || (e.target as HTMLElement).isContentEditable) return;

      if (e.key === "j") {
        e.preventDefault();
        setHighlighted((h) => Math.min(h + 1, allTools.length - 1));
      } else if (e.key === "k") {
        e.preventDefault();
        setHighlighted((h) => Math.max(h - 1, 0));
      } else if (e.key === "Enter" && highlighted >= 0) {
        e.preventDefault();
        const t = allTools[highlighted];
        setOpenTool((prev) => (prev === t.name ? null : t.name));
      } else if (e.key === "Escape") {
        e.preventDefault();
        setOpenTool(null);
        setHighlighted(-1);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [highlighted, allTools]);

  let globalIdx = 0;

  return (
    <div className="space-y-14">
      {CATEGORIES.map((category) => {
        const categoryTools = tools.filter((t) => t.category === category);
        if (categoryTools.length === 0) return null;

        const section = (
          <section key={category}>
            <h2 className="text-[11px] font-medium uppercase tracking-[0.16em] text-muted/60 mb-4 pl-3">
              {category}
              <AnimatedCount target={categoryTools.length} />
            </h2>
            <div className="space-y-0.5">
              {categoryTools.map((tool) => {
                const idx = globalIdx++;
                return (
                  <ToolRow
                    key={tool.name}
                    tool={tool}
                    isOpen={openTool === tool.name}
                    isHighlighted={highlighted === idx}
                    onToggle={() =>
                      setOpenTool((prev) => (prev === tool.name ? null : tool.name))
                    }
                  />
                );
              })}
            </div>
          </section>
        );

        return section;
      })}
    </div>
  );
}
