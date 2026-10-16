import { useEffect, useState } from "react";
import { usePrefersReducedMotion } from "./motion";
import { Chip, cx } from "./ui";

// The same four orders as the real demo: they clear at 5 with a volume of 14.
const orders = [
  { side: "sell", size: 10, limit: 4 },
  { side: "sell", size: 4, limit: 5 },
  { side: "buy", size: 8, limit: 6 },
  { side: "buy", size: 6, limit: 5 },
] as const;

const stages = [
  { label: "Orders arrive, sealed", note: "Each one is encrypted to a key no single node holds. The board shows only ciphertext." },
  { label: "The round closes", note: "The nodes agree which orders count, and sign receipts so none can be dropped." },
  { label: "Two nodes open them together", note: "Only now can the orders be read. Too few orders and nothing is ever opened." },
  { label: "One price for everyone", note: "A uniform auction finds the price that trades the most. Anyone can recompute it." },
] as const;

const seal = (n: number) => "▓".repeat(n);

/// A small looping walk through one round. With reduced motion it shows the final state and
/// lets the visitor step through by hand.
export function DemoStrip() {
  const reduced = usePrefersReducedMotion();
  const [stage, setStage] = useState(0);
  const [arrived, setArrived] = useState(0);

  useEffect(() => {
    if (reduced) return;
    const t = setInterval(() => setStage((s) => (s + 1) % stages.length), 3200);
    return () => clearInterval(t);
  }, [reduced]);

  useEffect(() => {
    if (stage !== 0) return;
    setArrived(0);
    if (reduced) {
      setArrived(orders.length);
      return;
    }
    const t = setInterval(() => setArrived((a) => Math.min(a + 1, orders.length)), 600);
    return () => clearInterval(t);
  }, [stage, reduced]);

  const open = stage >= 2;
  const shown = stage === 0 ? arrived : orders.length;

  return (
    <div className="rounded-2xl border border-line bg-card p-5 md:p-7" role="group" aria-label="A round, step by step">
      <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap gap-2">
          {stages.map((s, i) => (
            <button
              key={s.label}
              onClick={() => setStage(i)}
              aria-pressed={i === stage}
              className={cx(
                "mono cursor-pointer rounded-lg px-3 py-1.5 text-[12px] transition-colors",
                i === stage ? "bg-ink text-paper" : "bg-wash text-muted hover:text-ink",
              )}
            >
              {i + 1}. {s.label.split(",")[0]}
            </button>
          ))}
        </div>
        <Chip tone={stage === 3 ? "up" : stage === 0 ? "ink" : "warn"}>{["open", "closed", "committed", "cleared"][stage]}</Chip>
      </div>

      <div className="grid gap-5 md:grid-cols-[minmax(0,1fr)_220px]">
        <table className="w-full border-collapse text-sm" aria-live="polite">
          <thead>
            <tr>
              {["Order", "Side", "Size", "Limit", "Fill"].map((h, i) => (
                <th key={h} className={cx("eyebrow border-b border-line px-3 py-2 font-normal", i === 0 ? "text-left" : "text-right")}>
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {orders.map((o, i) => {
              const here = i < shown;
              const filled = stage === 3 ? o.size : 0;
              return (
                <tr key={i} className={cx("transition-opacity duration-500", here ? "opacity-100" : "opacity-20")}>
                  <td className="num border-b border-line px-3 py-2.5 text-left text-muted">{i + 1}</td>
                  <td className={cx("num border-b border-line px-3 py-2.5 text-right", open && (o.side === "buy" ? "text-up" : "text-down"))}>
                    {open ? o.side : seal(4)}
                  </td>
                  <td className="num border-b border-line px-3 py-2.5 text-right">{open ? o.size : seal(2)}</td>
                  <td className="num border-b border-line px-3 py-2.5 text-right">{open ? o.limit : seal(2)}</td>
                  <td className="num border-b border-line px-3 py-2.5 text-right">{stage === 3 ? filled : "·"}</td>
                </tr>
              );
            })}
          </tbody>
        </table>

        <div className="grid content-start gap-3">
          <div className="rounded-xl bg-wash px-4 py-3">
            <p className="eyebrow m-0">Clearing price</p>
            <p className={cx("num m-0 mt-1 text-3xl transition-opacity duration-500", stage === 3 ? "opacity-100" : "opacity-20")}>{stage === 3 ? "5" : "?"}</p>
          </div>
          <div className="rounded-xl bg-wash px-4 py-3">
            <p className="eyebrow m-0">Volume</p>
            <p className={cx("num m-0 mt-1 text-3xl transition-opacity duration-500", stage === 3 ? "opacity-100" : "opacity-20")}>{stage === 3 ? "14" : "?"}</p>
          </div>
        </div>
      </div>

      <p className="m-0 mt-5 border-t border-line pt-4 text-sm text-muted">
        <span className="font-medium text-ink">{stages[stage].label}.</span> {stages[stage].note}
      </p>
    </div>
  );
}
