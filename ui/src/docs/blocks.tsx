import { useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import { cx } from "../components/ui";

/* Small building blocks for the docs pages. Plain JSX, no markdown dependency. */

export function H3({ children }: { children: ReactNode }) {
  return <h3 className="m-0 mt-7 text-[17px] font-medium text-ink">{children}</h3>;
}
export function P({ children }: { children: ReactNode }) {
  return <p className="m-0 mt-4 text-[15.5px] leading-relaxed text-muted first:mt-0">{children}</p>;
}
export function C({ children }: { children: ReactNode }) {
  return <code className="mono rounded bg-wash px-1.5 py-0.5 text-[13px] text-ink">{children}</code>;
}
export function B({ children }: { children: ReactNode }) {
  return <strong className="font-medium text-ink">{children}</strong>;
}
export function Ul({ children }: { children: ReactNode }) {
  return <ul className="m-0 mt-4 grid gap-2 pl-5 text-[15.5px] leading-relaxed text-muted marker:text-faint">{children}</ul>;
}

/// Numbered steps with a ring marker, like a short how-to.
export function Steps({ items }: { items: ReactNode[] }) {
  return (
    <ol className="m-0 mt-5 grid list-none gap-3 p-0">
      {items.map((it, i) => (
        <li key={i} className="grid grid-cols-[28px_minmax(0,1fr)] gap-3 text-[15px] leading-relaxed text-muted">
          <span className="mono grid h-7 w-7 place-items-center rounded-full border border-line bg-card text-[12px] text-ink">{i + 1}</span>
          <span className="pt-0.5">{it}</span>
        </li>
      ))}
    </ol>
  );
}

export function Code({ children }: { children: string }) {
  const [copied, setCopied] = useState(false);
  const text = children.trim();
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* no clipboard access: the text can still be selected */
    }
  };
  return (
    <div className="relative mt-4">
      <pre className="mono m-0 overflow-x-auto rounded-xl border border-ink bg-[#111] p-4 pr-16 text-[13px] leading-relaxed text-[#e9e7e1]">{text}</pre>
      <button
        type="button"
        onClick={copy}
        className="mono absolute top-2.5 right-2.5 cursor-pointer rounded-md border border-[#333] bg-[#1b1b1b] px-2 py-1 text-[11px] text-[#bbb] hover:text-white"
      >
        {copied ? "Copied" : "Copy"}
      </button>
    </div>
  );
}

export function Table({ head, rows }: { head: string[]; rows: ReactNode[][] }) {
  return (
    <div className="mt-5 overflow-x-auto rounded-xl border border-line bg-card">
      <table className="w-full min-w-[540px] border-collapse text-[14px]">
        <thead>
          <tr className="bg-wash">
            {head.map((h, i) => (
              <th key={i} className="eyebrow px-4 py-2.5 text-left font-normal">
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i}>
              {r.map((c, j) => (
                <td key={j} className={cx("border-t border-line px-4 py-2.5 align-top leading-relaxed", j === 0 ? "text-ink" : "text-muted")}>
                  {c}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function Note({ title, children }: { title?: string; children: ReactNode }) {
  return (
    <div className="mt-5 rounded-xl border border-ink bg-card px-4 py-3 text-[14.5px] leading-relaxed text-muted">
      {title && <p className="mono m-0 mb-1 text-[11.5px] tracking-wide text-ink uppercase">{title}</p>}
      {children}
    </div>
  );
}

/// "Choose your path" cards linking to other pages.
export function Cards({ items }: { items: { to: string; title: string; text: string }[] }) {
  return (
    <div className="mt-5 grid gap-3 sm:grid-cols-2">
      {items.map((c) => (
        <Link key={c.to} to={c.to} className="group rounded-xl border border-ink bg-card px-4 py-3.5 no-underline transition-colors hover:bg-wash">
          <span className="mono block text-[12px] tracking-wide text-ink uppercase">{c.title}</span>
          <span className="mt-1.5 block text-[14px] leading-relaxed text-muted">{c.text}</span>
        </Link>
      ))}
    </div>
  );
}

export const REPO = "https://github.com/Dami904/kalyptra";
export const VIDEO = "https://youtu.be/tFmisu23AHM";
