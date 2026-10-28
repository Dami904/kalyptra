import { useEffect, useRef } from "react";
import { Link, NavLink, Outlet, useLocation } from "react-router-dom";
import { REPO } from "../docs/blocks";
import { USE_MOCK, api, usePoll } from "../lib/api";
import { clearActor, getActor, setActor } from "../lib/real";
import { TourNudge } from "./TourNudge";
import { Chip, cx } from "./ui";

/// The square mark (favicon, avatars, small spaces): a dark tile, two sealed blocks, and one
/// opened block in gold. Gold means "opened" everywhere in the page.
export function LogoMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 32 32" className={className} aria-hidden="true">
      <rect width="32" height="32" rx="8" fill="#111" />
      <rect x="4" y="10" width="7" height="12" rx="2" fill="#5b5d66" />
      <rect x="12.5" y="10" width="7" height="12" rx="2" fill="#5b5d66" />
      <rect x="21" y="10" width="7" height="12" rx="2" fill="#f4b728" />
    </svg>
  );
}

/// The wide mark: two sealed blocks and one opened to show the price.
export function LogoWide({ light, className }: { light?: boolean; className?: string }) {
  const sealed = light ? "#faf9f6" : "#111";
  return (
    <svg viewBox="0 0 108 32" className={className} aria-hidden="true">
      <rect x="0" y="0" width="32" height="32" rx="7" fill={sealed} />
      <rect x="38" y="0" width="32" height="32" rx="7" fill={sealed} />
      <rect x="77" y="1.5" width="29" height="29" rx="6" fill="#f4b728" stroke={sealed} strokeWidth="3" />
    </svg>
  );
}

export function Logo({ light }: { light?: boolean }) {
  return (
    <Link to="/" className="flex shrink-0 items-center gap-2 no-underline sm:gap-2.5" aria-label="Kalyptra home">
      <LogoMark className="h-7 w-7 shrink-0 sm:hidden" />
      <LogoWide light={light} className="hidden h-6 w-auto shrink-0 sm:block" />
      <span className="text-[18px] font-medium sm:text-[20px] tracking-tight text-ink">Kalyptra</span>
    </Link>
  );
}

/// The source code: an icon button in headers, with the word beside it from `sm` up when
/// `label` is set. To hide it on phones, wrap it: its own `inline-flex` would beat `hidden`.
export function GitHubLink({ label, className }: { label?: boolean; className?: string }) {
  return (
    <a
      href={REPO}
      target="_blank"
      rel="noreferrer"
      aria-label="Source code on GitHub"
      title="Source code on GitHub"
      className={cx("mono inline-flex shrink-0 items-center gap-2 rounded-lg p-2 text-[13px] text-muted no-underline hover:text-ink", className)}
    >
      <svg viewBox="0 0 16 16" className="h-[18px] w-[18px]" fill="currentColor" aria-hidden="true">
        <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0 0 16 8c0-4.42-3.58-8-8-8z" />
      </svg>
      {label && <span className="hidden sm:inline">GitHub</span>}
    </a>
  );
}

const tabs: { to: string; label: string; short?: string }[] = [
  { to: "/rounds", label: "Rounds" },
  { to: "/trade", label: "Trade" },
  { to: "/list", label: "List a company", short: "List" },
  { to: "/portfolio", label: "Portfolio" },
  { to: "/audit", label: "Audit" },
  { to: "/docs", label: "Docs" },
];

function Health() {
  const { data, error } = usePoll(() => api.status(), 8000);
  if (error && !data) return <Chip tone="down">Gateway offline</Chip>;
  if (!data) return <Chip>Connecting…</Chip>;
  const up = data.nodes.filter((n) => n.up && n.keys_ready).length;
  const ok = up === data.nodes.length;
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span title={data.group_key ? `Group key ${data.group_key}` : undefined}>
        <Chip tone={ok ? "up" : up >= data.threshold ? "warn" : "down"}>
          <span className={cx("h-1.5 w-1.5 rounded-full", ok ? "bg-up" : "bg-current")} />
          <span className="hidden sm:inline">Nodes </span>
          {up}
          <span className="hidden sm:inline"> of </span>
          <span className="sm:hidden">/</span>
          {data.nodes.length}
        </Chip>
      </span>
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
      {/* the words give way when the header is one row (xl), where the tabs need the room */}
      <span className="eyebrow hidden sm:inline xl:hidden">Acting as</span>
      <select
        aria-label="Acting as"
        value={current}
        onChange={(e) => {
          setActor(e.target.value);
          window.location.reload();
        }}
        className="mono h-9 cursor-pointer rounded-md sm:h-7 border border-line bg-card px-1.5 text-[11.5px] sm:px-2 sm:text-[12px] text-ink hover:border-ink focus:border-ink focus:outline-none"
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
  const { pathname } = useLocation();
  const screen = pathname.split("/")[1] ?? "";
  // On a narrow screen the tab row scrolls: keep the current tab in view.
  const navRef = useRef<HTMLElement>(null);
  useEffect(() => {
    navRef.current?.querySelector<HTMLElement>('[aria-current="page"]')?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [pathname]);
  return (
    <div className="min-h-screen">
      <header className="sticky top-0 z-20 border-b border-line bg-paper/90 backdrop-blur">
        {/* One row from 1280 px wide; below that, the tabs get their own full-width row so none is
            hidden, and that row scrolls sideways on a phone with a fade at its edge. */}
        <div className="mx-auto flex max-w-[1180px] flex-wrap items-center gap-x-3 gap-y-2 px-4 py-3 sm:gap-x-6 sm:px-6 xl:flex-nowrap">
          <Logo />
          <nav ref={navRef} className="nav-fade no-scrollbar order-3 -mx-1 flex w-full items-center gap-1 overflow-x-auto xl:order-none xl:mx-0 xl:w-auto xl:flex-1" aria-label="Main">
            {tabs.map((t) => (
              <NavLink
                key={t.to}
                to={t.to}
                className={({ isActive }) =>
                  cx(
                    "mono shrink-0 whitespace-nowrap rounded-lg px-3.5 py-2.5 text-[13px] no-underline transition-colors sm:py-1.5",
                    isActive ? "bg-ink text-paper" : "text-muted hover:text-ink",
                  )
                }
              >
                {t.short ? (
                  <>
                    <span className="sm:hidden">{t.short}</span>
                    <span className="hidden sm:inline">{t.label}</span>
                  </>
                ) : (
                  t.label
                )}
              </NavLink>
            ))}
          </nav>
          <div className="ml-auto flex items-center gap-1.5 sm:gap-2 xl:ml-0">
            <ActingAs />
            {USE_MOCK ? (
              <Chip tone="warn">
                Sample<span className="hidden sm:inline"> data</span>
              </Chip>
            ) : (
              <Chip>
                <span className="hidden sm:inline">Local </span>regtest
              </Chip>
            )}
            <Health />
            <span className="-mr-2 hidden sm:block">
              <GitHubLink />
            </span>
          </div>
        </div>
      </header>
      <main key={screen} className="rise mx-auto max-w-[1180px] px-6 py-10">
        <Outlet />
      </main>
      {USE_MOCK && <TourNudge />}
    </div>
  );
}
