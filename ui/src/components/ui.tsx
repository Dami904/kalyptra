import type { ButtonHTMLAttributes, InputHTMLAttributes, ReactNode, SelectHTMLAttributes } from "react";

export const cx = (...c: (string | false | null | undefined)[]) => c.filter(Boolean).join(" ");

export const short = (h: string, n = 8) => (h.length > n * 2 + 1 ? `${h.slice(0, n)}…${h.slice(-4)}` : h);

export function Spinner({ className }: { className?: string }) {
  return (
    <svg className={cx("spin h-4 w-4", className)} viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="2.5" opacity="0.2" />
      <path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" />
    </svg>
  );
}

type BtnProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "outline" | "ghost";
  busy?: boolean;
  size?: "md" | "sm";
};

export function Button({ variant = "primary", busy, size = "md", className, children, disabled, ...p }: BtnProps) {
  return (
    <button
      {...p}
      disabled={disabled || busy}
      className={cx(
        "inline-flex cursor-pointer items-center justify-center gap-2 rounded-full font-medium transition-[background-color,border-color,transform] duration-200 active:scale-[0.97] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ink disabled:cursor-not-allowed disabled:opacity-50",
        size === "md" ? "h-11 px-6 text-[15px]" : "h-8 px-4 text-[13px]",
        variant === "primary" && "bg-ink text-paper hover:bg-[#2b2b2b]",
        variant === "outline" && "border border-line bg-card text-ink hover:border-ink",
        variant === "ghost" && "text-muted hover:text-ink",
        className,
      )}
    >
      {busy && <Spinner />}
      {children}
    </button>
  );
}

export function Skeleton({ className }: { className?: string }) {
  return <div className={cx("skeleton rounded-lg", className)} aria-hidden="true" />;
}

export function Card({ children, className, tour }: { children: ReactNode; className?: string; tour?: string }) {
  return (
    <div data-tour={tour} className={cx("rounded-2xl border border-line bg-card", className)}>
      {children}
    </div>
  );
}

export function Metric({ label, value, tone, hint, opened }: { label: string; value: ReactNode; tone?: "up" | "down"; hint?: string; opened?: boolean }) {
  return (
    <div className={cx("rounded-xl bg-wash px-4 py-3", opened && "border-l-4 border-gold")}>
      <p className="eyebrow m-0">{label}</p>
      <p className={cx("num m-0 mt-1 text-2xl", tone === "up" && "text-up", tone === "down" && "text-down")}>{value}</p>
      {hint && <p className="m-0 mt-0.5 text-xs text-muted">{hint}</p>}
    </div>
  );
}

export function Chip({ children, tone = "neutral" }: { children: ReactNode; tone?: "neutral" | "up" | "down" | "warn" | "ink" }) {
  return (
    <span
      className={cx(
        "mono inline-flex items-center gap-1.5 rounded-md px-2 py-0.5 text-[12px] whitespace-nowrap",
        tone === "neutral" && "bg-wash text-muted",
        tone === "up" && "bg-up-bg text-up",
        tone === "down" && "bg-down-bg text-down",
        tone === "warn" && "bg-warn-bg text-warn",
        tone === "ink" && "bg-ink text-paper",
      )}
    >
      {children}
    </span>
  );
}

export function Notice({ tone = "neutral", title, children }: { tone?: "neutral" | "down" | "warn" | "up"; title?: string; children?: ReactNode }) {
  return (
    <div
      role={tone === "down" ? "alert" : undefined}
      className={cx(
        "rounded-xl border px-4 py-3 text-sm leading-relaxed",
        tone === "neutral" && "border-line bg-wash text-muted",
        tone === "down" && "border-down/30 bg-down-bg text-down",
        tone === "warn" && "border-warn/30 bg-warn-bg text-warn",
        tone === "up" && "border-up/30 bg-up-bg text-up",
      )}
    >
      {title && <p className="m-0 font-medium">{title}</p>}
      {children && <div className={title ? "mt-0.5" : ""}>{children}</div>}
    </div>
  );
}

export function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className="block">
      <span className="eyebrow mb-1.5 block">{label}</span>
      {children}
      {hint && <span className="mt-1 block text-xs text-muted">{hint}</span>}
    </label>
  );
}

const fieldCls =
  "h-11 w-full rounded-xl border border-line bg-card px-3.5 text-[15px] text-ink placeholder:text-faint hover:border-faint focus:border-ink focus:outline-none";

export function Input(p: InputHTMLAttributes<HTMLInputElement>) {
  return <input {...p} className={cx(fieldCls, p.className)} />;
}

export function Select(p: SelectHTMLAttributes<HTMLSelectElement>) {
  return <select {...p} className={cx(fieldCls, "cursor-pointer", p.className)} />;
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  tone,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
  tone?: (v: T) => "up" | "down" | undefined;
}) {
  return (
    <div role="radiogroup" className="grid h-11 grid-flow-col auto-cols-fr overflow-hidden rounded-xl border border-line bg-card">
      {options.map((o) => {
        const on = o.value === value;
        const t = tone?.(o.value);
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={on}
            onClick={() => onChange(o.value)}
            className={cx(
              "cursor-pointer text-[15px] font-medium transition-colors",
              on && !t && "bg-ink text-paper",
              on && t === "up" && "bg-up text-white",
              on && t === "down" && "bg-down text-white",
              !on && "text-muted hover:text-ink",
            )}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

export function Table({ head, children, empty }: { head: string[]; children: ReactNode; empty?: ReactNode }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr>
            {head.map((h, i) => (
              <th key={h} className={cx("eyebrow border-b border-line px-3 py-2 font-normal", i === 0 ? "text-left" : "text-right")}>
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>{children}</tbody>
      </table>
      {empty}
    </div>
  );
}

export function Td({ children, left, className }: { children?: ReactNode; left?: boolean; className?: string }) {
  return <td className={cx("num border-b border-line px-3 py-2.5", left ? "text-left" : "text-right", className)}>{children}</td>;
}

export function Empty({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <div className="px-4 py-10 text-center">
      <p className="m-0 font-medium">{title}</p>
      {children && <p className="m-0 mt-1 text-sm text-muted">{children}</p>}
    </div>
  );
}

export function PageHead({ eyebrow, title, children }: { eyebrow: string; title: string; children?: ReactNode }) {
  return (
    <div className="mb-8 flex flex-wrap items-end justify-between gap-4">
      <div>
        <p className="eyebrow m-0">{eyebrow}</p>
        <h1 className="m-0 mt-1 text-[34px] leading-tight font-medium tracking-tight">{title}</h1>
      </div>
      {children}
    </div>
  );
}
