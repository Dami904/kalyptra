import { cx } from "./ui";

/// A thin line of dots: done steps are ticked, the current one is filled, the rest are hollow.
export function Stepper({ steps, current, failed }: { steps: string[]; current: number; failed?: boolean }) {
  return (
    <ol className="m-0 flex list-none items-start p-0" aria-label="Progress">
      {steps.map((s, i) => {
        const done = i < current;
        const now = i === current;
        return (
          <li key={s} className="relative flex flex-1 flex-col items-center gap-2 text-center" aria-current={now ? "step" : undefined}>
            {i > 0 && (
              <span aria-hidden="true" className="absolute top-[9px] right-1/2 h-px w-full bg-line">
                <span className={cx("block h-full origin-left bg-ink transition-transform duration-700 ease-out", i <= current ? "scale-x-100" : "scale-x-0")} />
              </span>
            )}
            <span
              className={cx(
                "relative z-10 grid h-[19px] w-[19px] place-items-center rounded-full border text-[10px]",
                done && "border-ink bg-ink text-paper",
                now && !failed && "border-ink bg-card",
                now && failed && "border-down bg-down-bg text-down",
                !done && !now && "border-line bg-card",
              )}
            >
              {now && !failed && <span aria-hidden="true" className="absolute inset-0 rounded-full bg-ink/25 motion-safe:animate-ping" />}
              {done ? (
                <svg viewBox="0 0 12 12" className="h-2.5 w-2.5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                  <path d="M2.5 6.5l2.2 2.2L9.5 3.8" />
                </svg>
              ) : now ? (
                <span className={cx("h-1.5 w-1.5 rounded-full", failed ? "bg-down" : "bg-ink")} />
              ) : null}
            </span>
            <span className={cx("mono text-[11px]", now ? "text-ink" : "text-muted")}>{s}</span>
          </li>
        );
      })}
    </ol>
  );
}

/// The same idea, vertical, with a title and a line of text per step.
export function VStepper({ steps, current }: { steps: { title: string; text: string }[]; current: number }) {
  return (
    <ol className="m-0 list-none p-0">
      {steps.map((s, i) => {
        const done = i < current;
        const now = i === current;
        return (
          <li key={s.title} className="relative flex gap-4 pb-8 last:pb-0">
            {i < steps.length - 1 && (
              <span aria-hidden="true" className={cx("absolute top-8 bottom-0 left-[13px] w-px", done ? "bg-ink" : "bg-line")} />
            )}
            <span
              className={cx(
                "relative z-10 grid h-7 w-7 shrink-0 place-items-center rounded-full border text-xs font-medium",
                done && "border-ink bg-ink text-paper",
                now && "border-ink bg-card text-ink ring-4 ring-ink/10",
                !done && !now && "border-line bg-card text-faint",
              )}
            >
              {done ? "✓" : i + 1}
            </span>
            <div className="pt-0.5">
              <p className={cx("m-0 text-[15px] font-medium", !done && !now && "text-muted")}>{s.title}</p>
              <p className="m-0 mt-0.5 text-sm text-muted">{s.text}</p>
            </div>
          </li>
        );
      })}
    </ol>
  );
}
