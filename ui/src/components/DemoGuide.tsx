import { useEffect, useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { api, usePoll } from "../lib/api";
import { cx } from "./ui";

/// The sample round the guide walks through (see lib/mock.ts).
const ROUND = 2;
const KEY = "kalyptra-demo-guide";

type Progress = { ordered: boolean; closed: boolean; cleared: boolean; settled: boolean };

const steps: { to: string; title: string; text: string; done: (p: Progress, seen: Set<string>) => boolean }[] = [
  {
    to: "/trade",
    title: "Seal an order",
    text: "Sell 10 ACME at a limit of 4. Two other investors have sealed theirs; nobody, not even the nodes, can read any of them.",
    done: (p) => p.ordered,
  },
  {
    to: `/rounds/${ROUND}`,
    title: "Close the round",
    text: "Watch each node receipt, vote, release its share and sign. Nothing opens until two nodes sign the same final set.",
    done: (p) => p.closed,
  },
  {
    to: `/rounds/${ROUND}`,
    title: "Read the result",
    text: "One clearing price for everyone, and the public check: Passed. It is recomputed from published data, not taken from the nodes.",
    done: (p, seen) => p.cleared && seen.has("result"),
  },
  {
    to: "/trade",
    title: "Settle",
    text: "Settle your round 2 order from My orders. Every filled order moves in one transaction, or none does.",
    done: (p) => p.settled,
  },
  {
    to: "/portfolio",
    title: "Check your balances",
    text: "ACME went down and vUSD went up by exactly your fill. In Audit, a viewing key shows holdings without the power to spend.",
    done: (_, seen) => seen.has("portfolio"),
  },
];

function readMode(): "open" | "min" {
  try {
    return window.localStorage.getItem(KEY) === "min" ? "min" : "open";
  } catch {
    return "open";
  }
}

/// A small floating guide for the sample-data build. It follows what you do and ticks steps off
/// on its own, so it never sits on top of the page as a wall of instructions.
export function DemoGuide() {
  const path = useLocation().pathname;
  const [mode, setMode] = useState<"open" | "min">(readMode);
  const [seen, setSeen] = useState<Set<string>>(new Set());
  const { data } = usePoll(async (): Promise<Progress> => {
    const [tickets, round] = await Promise.all([api.tickets(), api.round(ROUND)]);
    const phase = round.round.phase;
    return {
      ordered: tickets.some((t) => t.round_id === ROUND),
      closed: phase !== "open",
      cleared: phase === "cleared",
      settled: round.settlement?.status === "settled",
    };
  }, 1500);
  const p = data ?? { ordered: false, closed: false, cleared: false, settled: false };

  // "Seen" steps: the result is read on the round's page once it has cleared, the balances on
  // Portfolio once the swap has settled.
  useEffect(() => {
    setSeen((s) => {
      const next = new Set(s);
      if (p.cleared && path.startsWith(`/rounds`)) next.add("result");
      if (p.settled && path.startsWith("/portfolio")) next.add("portfolio");
      return next.size === s.size ? s : next;
    });
  }, [path, p.cleared, p.settled]);

  const set = (m: "open" | "min") => {
    setMode(m);
    try {
      window.localStorage.setItem(KEY, m);
    } catch {
      /* not remembered, which is fine */
    }
  };

  const doneCount = steps.filter((s) => s.done(p, seen)).length;
  const current = steps.findIndex((s) => !s.done(p, seen));
  const finished = current === -1;

  const step = finished ? null : steps[current]!;
  const onStepPage = step ? path.startsWith(step.to.split("/").slice(0, 2).join("/")) : true;

  // A slim bar along the bottom of the screen: the app reserves room for it, so it never
  // covers a button. Minimised, it shrinks to a small pill.
  // On the docs the bar would sit on top of reading: show the small pill there instead.
  if (mode === "min" || path.startsWith("/docs")) {
    return (
      <button
        type="button"
        onClick={() => set("open")}
        className="fixed right-4 bottom-4 z-30 flex cursor-pointer items-center gap-2 rounded-full border border-line bg-card px-4 py-2 text-[13px] shadow-lg hover:border-ink"
      >
        <span className="mono text-muted">Guided demo</span>
        <span className="num font-medium">
          {doneCount}/{steps.length}
        </span>
      </button>
    );
  }

  return (
    <aside aria-label="Guided demo" className="fixed inset-x-0 bottom-0 z-30 border-t border-line bg-card/95 backdrop-blur">
      <div className="mx-auto flex max-w-[1180px] flex-wrap items-center gap-x-5 gap-y-2 px-6 py-3">
        <div className="flex shrink-0 items-center gap-3">
          <span className="mono text-[12px] text-muted">GUIDED DEMO</span>
          <span className="flex gap-1" aria-hidden="true">
            {steps.map((s, i) => (
              <span key={i} className={cx("h-1.5 w-5 rounded-full", s.done(p, seen) ? "bg-up" : i === current ? "bg-ink" : "bg-line")} />
            ))}
          </span>
          <span className="num text-[12px] text-muted">
            {doneCount}/{steps.length}
          </span>
        </div>
        <p className="m-0 min-w-[240px] flex-1 text-sm leading-snug">
          {step ? (
            <>
              <span className="font-medium">{step.title}.</span> <span className="text-muted">{step.text}</span>
              {current === 0 && <span className="text-muted"> Try selling 500 ACME first: it is refused, you hold 90.</span>}
            </>
          ) : (
            <>
              <span className="font-medium">You ran a whole round.</span>{" "}
              <span className="text-muted">
                Sealed, opened together, one checked price, one transaction. Reload to start again, or read the <Link to="/docs">docs</Link>.
              </span>
            </>
          )}
        </p>
        <div className="flex shrink-0 items-center gap-4">
          {step && !onStepPage && (
            <Link to={step.to} className="mono text-[13px]">
              Go to {step.to.split("/")[1]} →
            </Link>
          )}
          <button type="button" onClick={() => set("min")} aria-label="Minimise the guide" className="mono cursor-pointer text-[13px] text-muted hover:text-ink">
            Hide
          </button>
        </div>
      </div>
    </aside>
  );
}
