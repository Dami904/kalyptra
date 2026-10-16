import { cx } from "./ui";
import type { NodeProgress } from "../lib/types";

const stages = [
  { key: "receipts", label: "Receipted", tip: "Signed a receipt for each order it accepted" },
  { key: "voted", label: "Voted", tip: "Voted on which orders count. A node that sees two matching votes first commits without voting itself" },
  { key: "shares", label: "Shared", tip: "Released its share of the key for each order, after the commit" },
  { key: "signed", label: "Signed", tip: "Signed the final result" },
] as const;

type State = "done" | "part" | "todo";

function Dot({ state, tip }: { state: State; tip: string }) {
  return (
    <span
      title={tip}
      className={cx(
        "relative z-10 grid h-6 w-6 place-items-center rounded-full border text-[11px] transition-colors duration-500",
        state === "done" && "border-ink bg-ink text-paper",
        state === "part" && "border-ink bg-card",
        state === "todo" && "border-line bg-card",
      )}
    >
      {state === "done" ? (
        <svg viewBox="0 0 12 12" className="h-3 w-3" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M2.5 6.5l2.2 2.2L9.5 3.8" />
        </svg>
      ) : state === "part" ? (
        <span className="h-2 w-2 rounded-full bg-ink motion-safe:animate-pulse" />
      ) : null}
    </span>
  );
}

/// What each node has done in a round, from the data the nodes published. Three rows of four
/// dots joined by a line: a dot fills as that step is finished.
export function NodeTracker({ progress }: { progress: NodeProgress }) {
  const total = progress.orders;
  return (
    <div className="rounded-xl border border-line">
      <div className="grid grid-cols-[72px_repeat(4,minmax(0,1fr))] items-center gap-x-2 border-b border-line px-4 py-2.5">
        <span className="eyebrow">Node</span>
        {stages.map((s) => (
          <span key={s.key} title={s.tip} className="eyebrow text-center">
            {s.label}
          </span>
        ))}
      </div>
      {progress.nodes.map((n) => {
        const state = (key: (typeof stages)[number]["key"]): State => {
          if (key === "receipts") return total > 0 && n.receipts >= total ? "done" : n.receipts > 0 ? "part" : "todo";
          if (key === "voted") return n.committed ? "done" : "todo";
          if (key === "shares") return total > 0 && n.shares >= total ? "done" : n.shares > 0 ? "part" : "todo";
          return n.signed ? "done" : "todo";
        };
        const detail = (key: (typeof stages)[number]["key"]) =>
          key === "receipts" ? `${n.receipts} of ${total}` : key === "shares" ? `${n.shares} of ${total}` : key === "voted" ? (n.committed ? "voted" : "—") : n.signed ? "signed" : "—";
        return (
          <div key={n.id} className="grid grid-cols-[72px_repeat(4,minmax(0,1fr))] items-center gap-x-2 border-b border-line px-4 py-3 last:border-0">
            <span className="mono text-[13px]">Node {n.id}</span>
            {stages.map((s, i) => {
              const st = state(s.key);
              return (
                <span key={s.key} className="relative flex flex-col items-center gap-1">
                  {i > 0 && (
                    <span aria-hidden="true" className="absolute top-3 right-1/2 h-px w-full bg-line">
                      <span className={cx("block h-full origin-left bg-ink transition-transform duration-700", st !== "todo" ? "scale-x-100" : "scale-x-0")} />
                    </span>
                  )}
                  <Dot state={st} tip={`${s.label}: ${detail(s.key)}`} />
                  <span className="num text-[11px] text-muted">{detail(s.key)}</span>
                </span>
              );
            })}
          </div>
        );
      })}
    </div>
  );
}
