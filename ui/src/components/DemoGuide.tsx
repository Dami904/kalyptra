import { useState } from "react";
import { Link } from "react-router-dom";
import { Chip } from "./ui";

const KEY = "kalyptra-demo-guide-open";

function readOpen(): boolean {
  try {
    return window.localStorage.getItem(KEY) !== "closed";
  } catch {
    return true;
  }
}

const steps: { to: string; where: string; what: string }[] = [
  {
    to: "/trade",
    where: "Trade",
    what: "Place a sealed order in the open round: sell 10 ACME at a limit of 4. Two other investors have already sealed theirs. Nobody, including the nodes, can read any of them yet.",
  },
  {
    to: "/rounds/2",
    where: "Rounds",
    what: "Close round 2. Watch each node receipt, vote, release its share and sign. Only after two nodes sign the final set does anything open.",
  },
  {
    to: "/rounds/2",
    where: "Rounds",
    what: "Read the one clearing price, the volume, and the public check: Passed. The check recomputes the price from published data, not from what the nodes say.",
  },
  {
    to: "/trade",
    where: "Trade",
    what: "Settle your round 2 order from My orders. When every filled order has handed in its part, the leader node assembles one transaction: every order moves, or none does.",
  },
  {
    to: "/portfolio",
    where: "Portfolio",
    what: "Your balances moved by exactly your fill (ACME down, vUSD up). In Audit, a viewing key shows an auditor holdings without letting them spend.",
  },
];

/// A short walk through the sample-data build. Shown only when the page runs without a backend.
export function DemoGuide() {
  const [open, setOpen] = useState(readOpen);
  const set = (next: boolean) => {
    setOpen(next);
    try {
      window.localStorage.setItem(KEY, next ? "open" : "closed");
    } catch {
      /* the choice just is not remembered */
    }
  };
  const toggle = () => set(!open);
  return (
    <div className="mb-8 rounded-2xl border border-line bg-card">
      <button
        type="button"
        onClick={toggle}
        aria-expanded={open}
        className="flex w-full cursor-pointer items-center justify-between gap-3 px-5 py-4 text-left"
      >
        <span className="flex flex-wrap items-center gap-2">
          <Chip tone="warn">Guided demo</Chip>
          <span className="text-[15px] font-medium">Try a whole round in about two minutes</span>
        </span>
        <span className="mono text-muted" aria-hidden="true">
          {open ? "−" : "+"}
        </span>
      </button>
      {open && (
        <div className="border-t border-line px-5 py-4">
          <p className="m-0 mb-4 text-sm leading-relaxed text-muted">
            This page runs on <strong className="font-medium text-ink">sample data</strong> in your browser: no nodes, no chain, nothing leaves this tab. It follows the same rules as the real
            thing, so the refusals are real too. The real system (three nodes, a Zcash regtest chain) runs from the repository.
          </p>
          <ol className="m-0 grid list-none gap-3 p-0">
            {steps.map((s, i) => (
              <li key={i} className="grid grid-cols-[28px_minmax(0,1fr)] items-start gap-3">
                <span className="mono grid h-7 w-7 place-items-center rounded-full border border-line text-[12px]">{i + 1}</span>
                <p className="m-0 text-sm leading-relaxed">
                  <Link to={s.to} onClick={() => set(false)} className="mono mr-2 text-[13px]">
                    {s.where}
                  </Link>
                  <span className="text-muted">{s.what}</span>
                </p>
              </li>
            ))}
          </ol>
          <p className="m-0 mt-4 text-sm leading-relaxed text-muted">
            <strong className="font-medium text-ink">Try to break it:</strong> before step 1, sell 500 ACME (more than you hold) and the order is refused before it is counted. After step 1, place
            a second ACME sell in the same round and it is refused too, because both would spend the same note.
          </p>
          <p className="m-0 mt-2 text-xs text-muted">Following a step folds this guide away; the bar above reopens it. Reloading the page starts the sample over.</p>
        </div>
      )}
    </div>
  );
}
