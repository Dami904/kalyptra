import { useEffect, useMemo, useRef, useState } from "react";
import { Link, useLocation, useNavigate, useParams } from "react-router-dom";
import { GitHubLink, LogoMark, LogoWide } from "../components/Shell";
import { Button, cx } from "../components/ui";
import { DOC_PAGES, docGroups, docHref, findDocPage, type DocPage } from "../docs/content";
import { REPO, VIDEO } from "../docs/blocks";

/* /docs: a three-column docs layout. Grouped pages on the left, the article in the middle,
   "On this page" on the right, previous/next at the bottom, and Ctrl/Cmd+K search. */

function useScrollSpy(ids: string[]) {
  const [active, setActive] = useState(ids[0] ?? "");
  useEffect(() => {
    setActive(ids[0] ?? "");
    const els = ids.map((id) => document.getElementById(id)).filter((e): e is HTMLElement => !!e);
    if (els.length === 0) return;
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (e.isIntersecting) {
            setActive(e.target.id);
            break;
          }
        }
      },
      { rootMargin: "0px 0px -70% 0px" },
    );
    els.forEach((e) => io.observe(e));
    return () => io.disconnect();
  }, [ids]);
  return active;
}

function SearchModal({ onClose }: { onClose: () => void }) {
  const [q, setQ] = useState("");
  const navigate = useNavigate();
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => input.current?.focus(), []);
  const results = useMemo(() => {
    const s = q.trim().toLowerCase();
    const out: { page: DocPage; section?: string; anchor?: string }[] = [];
    for (const page of DOC_PAGES) {
      if (!s || page.title.toLowerCase().includes(s) || page.description.toLowerCase().includes(s)) out.push({ page });
      if (s) for (const sec of page.sections) if (sec.title.toLowerCase().includes(s)) out.push({ page, section: sec.title, anchor: sec.id });
    }
    return out;
  }, [q]);
  const go = (r: (typeof results)[number]) => {
    onClose();
    navigate(docHref(r.page) + (r.anchor ? `#${r.anchor}` : ""));
  };
  return (
    <div
      className="fixed inset-0 z-50 bg-ink/40 px-4 pt-[12vh] backdrop-blur-sm"
      onClick={onClose}
      onKeyDown={(e) => {
        if (e.key === "Escape") onClose();
        if (e.key === "Enter" && results[0]) go(results[0]);
      }}
    >
      <div role="dialog" aria-label="Search the docs" className="mx-auto w-full max-w-[560px] overflow-hidden rounded-2xl border border-ink bg-card shadow-2xl" onClick={(e) => e.stopPropagation()}>
        <input
          ref={input}
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="Search the docs…"
          className="w-full border-0 border-b border-line bg-transparent px-5 py-4 text-[16px] text-ink outline-none"
        />
        <ul className="m-0 max-h-[50vh] list-none overflow-y-auto p-2">
          {results.length === 0 && <li className="px-3 py-3 text-sm text-muted">No matches.</li>}
          {results.map((r, i) => (
            <li key={i}>
              <button type="button" onClick={() => go(r)} className="w-full cursor-pointer rounded-lg px-3 py-2.5 text-left text-[14px] hover:bg-wash focus:bg-wash focus:outline-none">
                <span className="font-medium text-ink">{r.page.title}</span>
                {r.section ? <span className="text-muted"> › {r.section}</span> : <span className="text-muted"> · {r.page.description}</span>}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

export default function Docs() {
  const { slug } = useParams();
  const { hash } = useLocation();
  const page = findDocPage(slug) ?? DOC_PAGES[0]!;
  const groups = docGroups();
  const idx = DOC_PAGES.indexOf(page);
  const prev = idx > 0 ? DOC_PAGES[idx - 1] : undefined;
  const next = idx < DOC_PAGES.length - 1 ? DOC_PAGES[idx + 1] : undefined;
  const ids = useMemo(() => page.sections.map((s) => s.id), [page]);
  const active = useScrollSpy(ids);
  const [searching, setSearching] = useState(false);
  const [menu, setMenu] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setSearching((v) => !v);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // A new page starts at its top, or at the linked section.
  useEffect(() => {
    const id = hash.slice(1);
    if (id) document.getElementById(id)?.scrollIntoView();
    else window.scrollTo(0, 0);
    setMenu(false);
  }, [page, hash]);

  const navLink = (p: DocPage) => (
    <Link
      key={p.slug}
      to={docHref(p)}
      aria-current={p === page ? "page" : undefined}
      className={cx(
        "-ml-px block border-l py-2.5 pl-3 text-[15px] no-underline transition-colors lg:py-1.5 lg:text-[14px]",
        p === page ? "border-ink font-medium text-ink" : "border-transparent text-muted hover:border-faint hover:text-ink",
      )}
    >
      {p.title}
    </Link>
  );

  return (
    <div className="min-h-screen bg-paper">
      <header className="sticky top-0 z-30 border-b-2 border-ink bg-paper/95 backdrop-blur">
        <div className="mx-auto flex max-w-[1280px] items-center gap-3 px-5 py-3">
          <Link to="/" className="mr-auto flex min-w-0 items-center gap-2.5 no-underline">
            <LogoMark className="h-7 w-7 shrink-0 sm:hidden" />
            <LogoWide className="hidden h-6 w-auto shrink-0 sm:block" />
            <span className="leading-none">
              <span className="block text-[17px] font-medium tracking-tight text-ink">Kalyptra</span>
              <span className="mono block text-[11px] text-muted">docs</span>
            </span>
          </Link>
          <button
            type="button"
            onClick={() => setSearching(true)}
            aria-label="Search the docs"
            className="mono flex cursor-pointer items-center gap-3 rounded-lg border border-line bg-card px-2.5 py-2 sm:py-1.5 text-[12.5px] text-muted hover:border-ink sm:px-3"
          >
            <svg viewBox="0 0 16 16" className="h-4 w-4 sm:hidden" aria-hidden="true">
              <circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" strokeWidth="1.6" />
              <path d="M10.5 10.5 L14 14" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
            </svg>
            <span className="hidden sm:inline">Search</span>
            <kbd className="hidden rounded border border-line px-1.5 text-[11px] sm:inline">Ctrl K</kbd>
          </button>
          <span className="hidden sm:block">
            <GitHubLink />
          </span>
          <button type="button" onClick={() => setMenu((v) => !v)} aria-expanded={menu} className="mono cursor-pointer rounded-lg border border-line px-3 py-2 text-[12.5px] sm:py-1.5 lg:hidden">
            {menu ? "Close" : "Menu"}
          </button>
          <Link to="/rounds" className="no-underline">
            <Button size="sm">
              <span className="hidden sm:inline">Launch app</span>
              <span className="sm:hidden">App</span> →
            </Button>
          </Link>
        </div>
      </header>

      <div className="mx-auto grid max-w-[1280px] gap-10 px-5 lg:grid-cols-[210px_minmax(0,1fr)_200px]">
        <aside className={cx("lg:sticky lg:top-20 lg:block lg:h-[calc(100vh-6rem)] lg:overflow-y-auto lg:pt-8", menu ? "block pt-6" : "hidden")}>
          {groups.map((g) => (
            <div key={g.label} className="mb-6">
              <p className="mono m-0 mb-2 text-[11.5px] font-medium tracking-wider text-ink uppercase">{g.label}</p>
              <div className="border-l border-line">{g.pages.map(navLink)}</div>
            </div>
          ))}
          <a href={REPO} className="mono text-[12.5px]">
            Source on GitHub ↗
          </a>
          <a href={VIDEO} className="mono mt-2 block text-[12.5px]">
            The 2-minute video ↗
          </a>
        </aside>

        <article className="min-w-0 pt-8 pb-24">
          <p className="mono m-0 text-[11.5px] tracking-wider text-muted uppercase">
            {page.group} <span className="px-1 text-faint">/</span> {page.title}
          </p>
          <h1 className="m-0 mt-3 text-[clamp(32px,4vw,44px)] leading-tight font-medium tracking-tight">{page.title}</h1>
          <p className="m-0 mt-3 max-w-[680px] text-[17px] leading-relaxed text-muted">{page.description}</p>
          <hr className="mt-7 mb-0 border-0 border-t-2 border-ink" />

          {page.sections.map((s) => (
            <section key={s.id} className="mt-10 max-w-[760px]">
              <h2 id={s.id} className="group m-0 mb-3 flex scroll-mt-24 items-center justify-between gap-3 text-[22px] font-medium tracking-tight">
                {s.title}
                <a href={`#${s.id}`} aria-label={`Link to ${s.title}`} className="mono text-[18px] text-faint no-underline opacity-60 group-hover:opacity-100 hover:text-ink">
                  #
                </a>
              </h2>
              {s.body}
            </section>
          ))}

          <nav className="mt-16 grid max-w-[760px] gap-3 sm:grid-cols-2" aria-label="Previous and next">
            {prev ? (
              <Link to={docHref(prev)} className="rounded-xl border border-line px-4 py-3 no-underline hover:border-ink">
                <span className="mono block text-[11.5px] text-muted">← Previous</span>
                <span className="mt-1 block font-medium text-ink">{prev.title}</span>
              </Link>
            ) : (
              <span />
            )}
            {next && (
              <Link to={docHref(next)} className="rounded-xl border border-line px-4 py-3 text-right no-underline hover:border-ink">
                <span className="mono block text-[11.5px] text-muted">Next →</span>
                <span className="mt-1 block font-medium text-ink">{next.title}</span>
              </Link>
            )}
          </nav>
        </article>

        <aside className="hidden lg:sticky lg:top-20 lg:block lg:h-fit lg:pt-8">
          <p className="mono m-0 mb-2 text-[11.5px] font-medium tracking-wider text-ink uppercase">On this page</p>
          <div className="border-l border-line">
            {page.sections.map((s) => (
              <a
                key={s.id}
                href={`#${s.id}`}
                className={cx(
                  "-ml-px block border-l py-1 pl-3 text-[13px] no-underline transition-colors",
                  s.id === active ? "border-ink text-ink" : "border-transparent text-muted hover:text-ink",
                )}
              >
                {s.title}
              </a>
            ))}
          </div>
        </aside>
      </div>

      {searching && <SearchModal onClose={() => setSearching(false)} />}
    </div>
  );
}
