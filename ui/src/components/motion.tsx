import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { cx } from "./ui";

export function usePrefersReducedMotion() {
  const [r, setR] = useState(() => (typeof window !== "undefined" ? window.matchMedia("(prefers-reduced-motion: reduce)").matches : false));
  useEffect(() => {
    const m = window.matchMedia("(prefers-reduced-motion: reduce)");
    const f = () => setR(m.matches);
    m.addEventListener("change", f);
    return () => m.removeEventListener("change", f);
  }, []);
  return r;
}

/// True once the element has scrolled into view (and stays true).
export function useInView<T extends Element>(margin = "0px 0px -8% 0px") {
  const ref = useRef<T>(null);
  const [seen, setSeen] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (typeof IntersectionObserver === "undefined") {
      setSeen(true);
      return;
    }
    const io = new IntersectionObserver(
      ([e]) => {
        if (e?.isIntersecting) {
          setSeen(true);
          io.disconnect();
        }
      },
      { rootMargin: margin, threshold: 0.08 },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [margin]);
  return { ref, seen };
}

/// Fades and lifts its children into place as they scroll into view.
export function Reveal({ children, delay = 0, className }: { children: ReactNode; delay?: number; className?: string }) {
  const { ref, seen } = useInView<HTMLDivElement>();
  return (
    <div ref={ref} className={cx("reveal", seen && "in", className)} style={{ transitionDelay: seen ? `${delay}ms` : "0ms" }}>
      {children}
    </div>
  );
}

/// Counts up from zero to `to` the first time it is seen.
export function CountUp({ to, duration = 1300, className }: { to: number; duration?: number; className?: string }) {
  const { ref, seen } = useInView<HTMLSpanElement>();
  const reduced = usePrefersReducedMotion();
  const [n, setN] = useState(0);
  useEffect(() => {
    if (!seen) return;
    if (reduced) {
      setN(to);
      return;
    }
    let raf = 0;
    const start = performance.now();
    const tick = (t: number) => {
      const p = Math.min(1, (t - start) / duration);
      setN(Math.round(to * (1 - Math.pow(1 - p, 3))));
      if (p < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [seen, to, duration, reduced]);
  return (
    <span ref={ref} className={className}>
      {seen ? n : 0}
    </span>
  );
}

/// Cycles through words in place: the old one blurs out as the new one blurs in.
export function Rotator({ words, interval = 2600, className }: { words: string[]; interval?: number; className?: string }) {
  const reduced = usePrefersReducedMotion();
  const [i, setI] = useState(0);
  const [widths, setWidths] = useState<number[]>([]);
  const inner = useRef<(HTMLSpanElement | null)[]>([]);
  useEffect(() => {
    if (reduced || words.length < 2) return;
    const t = setInterval(() => setI((x) => (x + 1) % words.length), interval);
    return () => clearInterval(t);
  }, [reduced, words.length, interval]);
  // Measure each word once, so the slot can slide to the current word's own width and the
  // text after it never sits behind a gap.
  // Measure again once the web fonts have loaded (they are wider than the fallback) and when
  // the window is resized, or the word overruns its slot and touches the next one.
  useLayoutEffect(() => {
    const measure = () => setWidths(inner.current.map((el) => el?.offsetWidth ?? 0));
    measure();
    let alive = true;
    void document.fonts?.ready.then(() => alive && measure());
    window.addEventListener("resize", measure);
    return () => {
      alive = false;
      window.removeEventListener("resize", measure);
    };
  }, [words]);
  return (
    <span
      className={cx("relative inline-grid align-baseline transition-[width] duration-700 ease-out", className)}
      style={widths.length ? { width: widths[i] } : undefined}
    >
      {words.map((w, k) => (
        <span
          key={w}
          aria-hidden={k !== i}
          style={{ gridArea: "1 / 1" }}
          className={cx("whitespace-nowrap transition-all duration-700 ease-out", k === i ? "translate-y-0 opacity-100 blur-0" : "translate-y-2 opacity-0 blur-sm")}
        >
          <span
            ref={(el) => {
              inner.current[k] = el;
            }}
            className="inline-block"
          >
            {w}
          </span>
        </span>
      ))}
    </span>
  );
}

const CIPHER = "▓▒░█#%&*+=";

/// Shows `text` as scrambled cipher characters that resolve, left to right, into the real text.
/// This is the "sealed, then opened" moment, used where an order's contents appear.
export function Decrypt({ text, delay = 0, duration = 750, className }: { text: string; delay?: number; duration?: number; className?: string }) {
  const reduced = usePrefersReducedMotion();
  const [shown, setShown] = useState(reduced ? text : "");
  useEffect(() => {
    if (reduced) {
      setShown(text);
      return;
    }
    let raf = 0;
    let timer = 0;
    const begin = () => {
      const start = performance.now();
      const tick = (t: number) => {
        const p = Math.min(1, (t - start) / duration);
        const resolved = Math.floor(p * text.length);
        let out = "";
        for (let k = 0; k < text.length; k++) {
          out += k < resolved || text[k] === " " ? text[k] : CIPHER[Math.floor(Math.random() * CIPHER.length)];
        }
        setShown(p >= 1 ? text : out);
        if (p < 1) raf = requestAnimationFrame(tick);
      };
      raf = requestAnimationFrame(tick);
    };
    setShown(CIPHER[0]!.repeat(text.length));
    timer = window.setTimeout(begin, delay);
    return () => {
      window.clearTimeout(timer);
      cancelAnimationFrame(raf);
    };
  }, [text, delay, duration, reduced]);
  return (
    <span className={className} aria-label={text}>
      <span aria-hidden="true">{shown}</span>
    </span>
  );
}

/// A slow, endless strip of words.
export function Marquee({ items, className }: { items: string[]; className?: string }) {
  const row = (k: number) => (
    <div key={k} className="flex shrink-0 items-center" aria-hidden={k === 1}>
      {items.map((t) => (
        <span key={t} className="mono flex items-center gap-8 pr-8 text-[12px] tracking-wide whitespace-nowrap text-muted">
          {t}
          <span className="text-faint">✦</span>
        </span>
      ))}
    </div>
  );
  return (
    <div className={cx("marquee overflow-hidden", className)}>
      <div className="marquee-track">
        {row(0)}
        {row(1)}
      </div>
    </div>
  );
}

/// A field of sealed glyphs drifting around a ring. Now and then one opens into a digit and
/// seals again: the idea of the product, as a backdrop. Drawn on a canvas, paused when the tab
/// is hidden, and still when reduced motion is on.
export function GlyphField({ className }: { className?: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const reduced = usePrefersReducedMotion();
  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const ctx = el.getContext("2d");
    if (!ctx) return;
    type P = { a: number; r: number; w: number; ch: string; alpha: number; open: number };
    const SEALED = ["▓", "▒", "░", "▌", "▐", "│"];
    const parts: P[] = Array.from({ length: 96 }, () => ({
      a: Math.random() * Math.PI * 2,
      r: 0.28 + Math.random() * 0.7,
      w: (Math.random() - 0.5) * 0.0005,
      ch: SEALED[Math.floor(Math.random() * SEALED.length)]!,
      alpha: 0.1 + Math.random() * 0.22,
      open: 0,
    }));
    let w = 0;
    let h = 0;
    const size = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      w = el.clientWidth;
      h = el.clientHeight;
      el.width = Math.round(w * dpr);
      el.height = Math.round(h * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    const draw = () => {
      ctx.clearRect(0, 0, w, h);
      const cx = w * (w > 760 ? 0.74 : 0.5);
      const cy = h * 0.5;
      const R = Math.min(w, h) * 0.62;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      for (const p of parts) {
        const x = cx + Math.cos(p.a) * R * p.r * 1.25;
        const y = cy + Math.sin(p.a) * R * p.r * 0.8;
        const opened = p.open > 0;
        ctx.font = `${opened ? 17 : 15}px "IBM Plex Mono", monospace`;
        // an opened glyph turns the logo's gold (a deeper shade, to read on the light page)
        ctx.fillStyle = opened ? `rgba(201,138,0,${0.4 + 0.55 * p.open})` : `rgba(17,17,17,${p.alpha})`;
        ctx.fillText(opened ? p.ch : p.ch, x, y);
      }
    };
    size();
    draw();
    if (reduced) return;
    const ro = new ResizeObserver(() => {
      size();
      draw();
    });
    ro.observe(el);
    let raf = 0;
    let last = 0;
    let nextOpen = performance.now() + 1200;
    const frame = (t: number) => {
      raf = requestAnimationFrame(frame);
      if (document.hidden || t - last < 33) return;
      const dt = t - last;
      last = t;
      for (const p of parts) {
        p.a += p.w * dt;
        if (p.open > 0) {
          p.open -= dt / 1600;
          if (p.open <= 0) {
            p.open = 0;
            p.ch = SEALED[Math.floor(Math.random() * SEALED.length)]!;
          }
        }
      }
      if (t > nextOpen) {
        const p = parts[Math.floor(Math.random() * parts.length)]!;
        if (p.open === 0) {
          p.open = 1;
          p.ch = String(Math.floor(Math.random() * 10));
        }
        nextOpen = t + 500 + Math.random() * 900;
      }
      draw();
    };
    raf = requestAnimationFrame(frame);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
    };
  }, [reduced]);
  return <canvas ref={canvas} className={cx("pointer-events-none absolute inset-0 h-full w-full", className)} aria-hidden="true" />;
}
