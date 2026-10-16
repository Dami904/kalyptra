import { useEffect, useState } from "react";
import { usePrefersReducedMotion, useInView } from "./motion";
import { cx } from "./ui";

export type Line = { kind: "cmd" | "out" | "ok" | "err" | "gap"; text: string };

const color = (k: Line["kind"]) => (k === "ok" ? "text-[#7ddba3]" : k === "err" ? "text-[#f0a39b]" : k === "cmd" ? "text-paper" : "text-[#c9c7bf]");

/// A terminal that types its commands the first time it is seen, then prints their output.
/// Reduced motion shows everything at once.
export function Terminal({ lines, className }: { lines: Line[]; className?: string }) {
  const reduced = usePrefersReducedMotion();
  const { ref, seen } = useInView<HTMLPreElement>();
  const [done, setDone] = useState<Line[]>([]);
  const [typing, setTyping] = useState<string | null>(null);
  const [run, setRun] = useState(0);

  useEffect(() => {
    if (!seen) return;
    if (reduced) {
      setDone(lines);
      setTyping(null);
      return;
    }
    let cancelled = false;
    const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));
    (async () => {
      setDone([]);
      setTyping(null);
      for (const l of lines) {
        if (cancelled) return;
        if (l.kind === "cmd") {
          for (let i = 1; i <= l.text.length; i++) {
            if (cancelled) return;
            setTyping(l.text.slice(0, i));
            await sleep(18 + Math.random() * 26);
          }
          await sleep(320);
          setTyping(null);
          setDone((d) => [...d, l]);
        } else {
          await sleep(l.kind === "gap" ? 250 : 130);
          if (cancelled) return;
          setDone((d) => [...d, l]);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [seen, reduced, lines, run]);

  const finished = done.length === lines.length;

  return (
    <div className={cx("relative", className)}>
      <pre
        ref={ref}
        className="mono m-0 min-h-[286px] rounded-2xl border border-line bg-ink p-5 text-[13px] leading-relaxed break-words whitespace-pre-wrap"
        aria-label={lines.map((l) => l.text).join("\n")}
      >
        <code aria-hidden="true">
          {done.map((l, i) => (
            <div key={i} className={cx(color(l.kind), l.kind === "gap" && "h-3")}>
              {l.kind === "cmd" && <span className="text-faint">$ </span>}
              {l.text}
            </div>
          ))}
          {typing !== null && (
            <div className="text-paper">
              <span className="text-faint">$ </span>
              {typing}
              <span className="caret" />
            </div>
          )}
          {finished && !reduced && <span className="caret text-paper" />}
        </code>
      </pre>
      {finished && !reduced && (
        <button
          onClick={() => setRun((n) => n + 1)}
          className="mono absolute top-3 right-3 cursor-pointer rounded-md bg-white/10 px-2.5 py-1 text-[11px] text-paper/80 transition-colors hover:bg-white/20"
        >
          Replay
        </button>
      )}
    </div>
  );
}
