import { useEffect } from "react";
import { Link, NavLink, Outlet, useLocation } from "react-router-dom";
import { USE_MOCK, api, usePoll } from "../lib/api";
import { clearActor, getActor, setActor } from "../lib/real";
import { Chip, cx, short } from "./ui";

/// Sealed, then opened: three blocks, two still sealed and one opened to show the price. It is
/// the same picture as the sealed-glyph animation on the landing page.
export function LogoMark({ light, className }: { light?: boolean; className?: string }) {
  const bg = light ? "#faf9f6" : "#111";
  const fg = light ? "#111" : "#faf9f6";
  return (
    <svg viewBox="0 0 32 32" className={className} aria-hidden="true">
      <rect width="32" height="32" rx="8" fill={bg} />
      <rect x="5" y="9.5" width="6" height="13" rx="1.6" fill={fg} opacity="0.5" />
      <rect x="13" y="9.5" width="6" height="13" rx="1.6" fill={fg} opacity="0.5" />
      <rect x="21.9" y="10.4" width="4.2" height="11.2" rx="1.2" fill="none" stroke={fg} strokeWidth="1.8" />
      <circle cx="24" cy="16" r="1.4" fill={fg} />
    </svg>
  );
}

export function Logo({ light }: { light?: boolean }) {
  return (
    <Link to="/" className="flex items-center gap-2.5 no-underline" aria-label="Kalyptra home">
      <LogoMark light={light} className="h-7 w-7" />
      <span className="text-[20px] font-medium tracking-tight text-ink">Kalyptra</span>
      <span className="eyebrow hidden sm:inline">SEALED</span>
    </Link>
  );
}

const tabs = [
  { to: "/rounds", label: "Rounds" },
  { to: "/trade", label: "Trade" },
  { to: "/list", label: "List a company" },
  { to: "/portfolio", label: "Portfolio" },
  { to: "/audit", label: "Audit" },
];

function Health() {
  const { data, error } = usePoll(() => api.status(), 8000);
  if (error && !data) return <Chip tone="down">Gateway offline</Chip>;
  if (!data) return <Chip>Connecting…</Chip>;
  const up = data.nodes.filter((n) => n.up && n.keys_ready).length;
  const ok = up === data.nodes.length;
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Chip tone={ok ? "up" : up >= data.threshold ? "warn" : "down"}>
        <span className={cx("h-1.5 w-1.5 rounded-full", ok ? "bg-up" : "bg-current")} />
        Nodes {up} of {data.nodes.length}
      </Chip>
      {data.group_key && <Chip>Group key {short(data.group_key, 5)}</Chip>}
    </div>
  );
}

/// "Acting as": which investor's wallet this page uses. Shown only when the gateway holds more
/// than one. Switching reloads the page, so nothing from the previous investor stays on screen.
function ActingAs() {
  const { data } = usePoll(() => api.status(), 0);
  const names = data?.wallets ?? [];
  // A name remembered from another gateway would be sent with every request: forget it.
  useEffect(() => {
    const stored = getActor();
    if (data && stored && !names.includes(stored)) {
      clearActor();
      window.location.reload();
    }
  }, [data, names]);
  if (names.length < 2) return null;
  const current = getActor() && names.includes(getActor()!) ? getActor()! : (data?.default ?? names[0]!);
  return (
    <label className="flex items-center gap-2">
      <span className="eyebrow hidden sm:inline">Acting as</span>
      <select
        value={current}
        onChange={(e) => {
          setActor(e.target.value);
          window.location.reload();
        }}
        className="mono h-7 cursor-pointer rounded-md border border-line bg-card px-2 text-[12px] text-ink hover:border-ink focus:border-ink focus:outline-none"
      >
        {names.map((n) => (
          <option key={n} value={n}>
            {n}
          </option>
        ))}
      </select>
    </label>
  );
}

export function AppShell() {
  // Fade between screens, but keep a screen mounted while moving between its own sub-pages.
  const screen = useLocation().pathname.split("/")[1] ?? "";
  return (
    <div className="min-h-screen">
      <header className="sticky top-0 z-20 border-b border-line bg-paper/90 backdrop-blur">
        <div className="mx-auto flex max-w-[1180px] flex-wrap items-center justify-between gap-x-8 gap-y-2 px-6 py-3 lg:flex-nowrap">
          <Logo />
          <nav className="no-scrollbar order-3 -mx-1 flex w-full items-center gap-1 overflow-x-auto lg:order-none lg:mx-0 lg:w-auto lg:flex-1" aria-label="Main">
            {tabs.map((t) => (
              <NavLink
                key={t.to}
                to={t.to}
                className={({ isActive }) =>
                  cx(
                    "mono shrink-0 whitespace-nowrap rounded-lg px-3.5 py-1.5 text-[13px] no-underline transition-colors",
                    isActive ? "bg-ink text-paper" : "text-muted hover:text-ink",
                  )
                }
              >
                {t.label}
              </NavLink>
            ))}
          </nav>
          <div className="flex items-center gap-2">
            <ActingAs />
            {USE_MOCK ? <Chip tone="warn">Sample data</Chip> : <Chip>Local regtest</Chip>}
            <Health />
          </div>
        </div>
      </header>
      <main key={screen} className="rise mx-auto max-w-[1180px] px-6 py-10">
        <Outlet />
      </main>
    </div>
  );
}
