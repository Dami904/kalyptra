import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { DemoStrip } from "../components/Demo";
import { CountUp, GlyphField, Marquee, Reveal, Rotator, useInView } from "../components/motion";
import { Logo } from "../components/Shell";
import { Terminal, type Line } from "../components/Terminal";
import { Button, Card, Chip, cx } from "../components/ui";
import { USE_MOCK } from "../lib/api";

const features = [
  { n: "01", t: "Sealed until close", d: "Each order is encrypted to a key split across three nodes. No single node, and no one else, can read an order while the round is open." },
  { n: "02", t: "One public price", d: "When the round closes, two nodes open the orders together and a uniform auction finds one price. Everyone who trades gets it." },
  { n: "03", t: "Anyone can check", d: "The price is recomputed from published data by a checker you can run yourself, offline, trusting no node. A wrong price is caught." },
  { n: "04", t: "One atomic swap", d: "Every filled order settles in a single shielded transaction. Either all of it moves, or none of it does." },
];

const steps = [
  { n: "I", t: "Seal your order", d: "Pick a side, a size and a limit. Your wallet makes a one-time key for it, encrypts it to the group key, and gets signed receipts back." },
  { n: "II", t: "The nodes open it together", d: "When the round closes, the nodes agree which orders count, release shares, open everything and clear one price. Too few orders voids the round and nothing is ever opened." },
  { n: "III", t: "Settle in one transaction", d: "Each filled order builds its own part with its own wallet. The leader checks every amount and assembles one swap." },
];

const compare: [string, string, string][] = [
  ["While the round is open", "Every order is visible to anyone watching", "Only sealed envelopes and signed receipts"],
  ["Who can read an order", "The venue sees everything", "No single node. Any 2 of 3, together, after close"],
  ["Who is behind an order", "A wallet address that can be followed", "A one-time key that names no one"],
  ["The price", "Moves with each order, and can be front-run", "One price per round, the same for everyone"],
  ["Checking the result", "Trust the venue", "Recompute it yourself, offline"],
];

const assets = [
  { s: "ACME", d: "Fictional company equity", tag: "Supply capped" },
  { s: "BOLT", d: "Fictional company equity", tag: "Supply capped" },
  { s: "CRUX", d: "Fictional company equity", tag: "Supply capped" },
  { s: "vUSD", d: "Play-money quote asset. No peg, no redemption", tag: "Quote asset" },
];

const evidence = [
  ["149", "automated tests, passing on every push in CI"],
  ["44", "defences, each proven needed by switching it off and watching a test fail"],
  ["4", "protocol holes our own attack tests found, then fixed"],
  ["3", "separate node processes run a round over the network"],
];

type Seen = "yes" | "no" | "key" | string;
const seenBy: { what: string; you: Seen; node: Seen; pub: Seen; auditor: Seen }[] = [
  { what: "Your order, while the round is open", you: "yes", node: "no", pub: "no", auditor: "no" },
  { what: "Your order, after the round closes", you: "yes", node: "yes", pub: "yes", auditor: "yes" },
  { what: "Who placed an order", you: "yes", node: "no", pub: "no", auditor: "no" },
  { what: "The clearing price", you: "yes", node: "yes", pub: "yes", auditor: "yes" },
  { what: "Your holdings", you: "yes", node: "no", pub: "no", auditor: "key" },
];

const status = [
  ["Sealed orders and threshold opening", "Live", "Three separate node processes, 2 of 3"],
  ["Clearing price and public checker", "Live", "Every defence mutation-checked"],
  ["Offline verification of a saved round", "Live", "kalyptra verify, no network"],
  ["The page and local gateway", "Live on local regtest", "Orders, clearing, settlement, balances and audit, by hand in the browser"],
  ["Settlement as one atomic swap", "Live on local regtest", "QEDIT prototype node; not on any public network"],
  ["Auditor view with a viewing key", "Live on local regtest", "Holdings only, cannot spend"],
  ["Listing a company", "Live on local regtest", "Each node signs the symbol-to-asset binding; 2 of 3 make it count"],
];

const faq: [string, ReactNode][] = [
  [
    "Why not a continuous order book?",
    "A continuous book has to read each order as it arrives, so the matcher sees everything, and every trade leaks a price and a time. A batch auction opens orders only after the round closes and publishes one price, which is how stock exchanges run their opening and closing auctions. The trade-off is real: you wait for the round instead of trading at once.",
  ],
  [
    "Is this on a real Zcash network?",
    "No. Zcash Shielded Assets are not on mainnet, and swaps of them are not on any public network yet. Settlement runs on a local regtest node built from QEDIT's prototype branches. Everything else (sealing, opening, clearing, checking) runs as separate processes you can start yourself.",
  ],
  [
    "What if a node cheats?",
    "One node can do nothing alone: it cannot read an order, drop one that two nodes receipted, or publish a wrong price. If two nodes collude they can read orders early, which is the security threshold, but they cannot hide a wrong price, a dropped order, a duplicated order or a wrongly voided round: the public checker flags each one from published data.",
  ],
  [
    "Who sees my identity?",
    "Each order is signed with a fresh one-time key, so an opened order shows what was asked for, not who asked. Network details such as who connected, and from where, are not hidden by anything here.",
  ],
  [
    "Can I list my own company?",
    "Yes. The List screen issues a token from your wallet and can cap its supply for good. The matcher nodes each sign that this symbol means this asset id, and refuse a symbol or asset already listed another way. Nothing checks that the company is real, so always compare asset ids, not names. Whether a company is real is a legal question outside the code.",
  ],
  [
    "Is it audited?",
    "No. The threshold-decryption design is our own composition of standard parts. It has had an AI review and extensive attack tests, not a human audit. Do not use it with real assets.",
  ],
];

const iconCls = "h-7 w-7 text-ink";

const icons = [
  // sealed: a padlock whose shackle lifts a little on hover
  <svg key="lock" viewBox="0 0 28 28" className={iconCls} fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <rect x="5" y="12" width="18" height="12" rx="3" />
    <path d="M9 12V9a5 5 0 0 1 10 0v3" className="origin-[14px_12px] transition-transform duration-300 group-hover:-translate-y-0.5 group-hover:-rotate-12" />
    <circle cx="14" cy="18" r="1.4" fill="currentColor" />
  </svg>,
  // one price: two lines meeting at a point
  <svg key="price" viewBox="0 0 28 28" className={iconCls} fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M4 22L14 12" />
    <path d="M4 6l10 6" />
    <path d="M14 12h10" />
    <circle cx="14" cy="12" r="2.4" fill="currentColor" className="origin-center transition-transform duration-300 group-hover:scale-125" />
  </svg>,
  // checkable: a shield with a tick that draws in on hover
  <svg key="check" viewBox="0 0 28 28" className={iconCls} fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M14 3l9 3.5v6.5c0 5.2-3.6 9.2-9 11.5C8.6 22.200 5 18.200 5 13V6.500z" />
    <path d="M10 14l3 3 5-6" strokeDasharray="14" strokeDashoffset="0" className="transition-[stroke-dashoffset] duration-500 group-hover:[stroke-dashoffset:-14]" />
  </svg>,
  // atomic: two arrows swapping, which turn on hover
  <svg key="swap" viewBox="0 0 28 28" className={iconCls} fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <g className="origin-center transition-transform duration-500 group-hover:rotate-180">
      <path d="M5 10h16l-4-4" />
      <path d="M23 18H7l4 4" />
    </g>
  </svg>,
];

const verifyLines: Line[] = [
  { kind: "cmd", text: "kalyptra export --round 1 --out round-1.json" },
  { kind: "out", text: "round 1 saved to round-1.json" },
  { kind: "cmd", text: "kalyptra verify --bundle round-1.json --group-key 84f17a5f…7756" },
  { kind: "out", text: "round 1: cleared at 5, volume 14" },
  { kind: "ok", text: "VERIFIED OFFLINE: everything checked from the file alone" },
  { kind: "gap", text: "" },
  { kind: "cmd", text: "kalyptra verify --bundle round-1-forged.json   # price edited 5 to 6" },
  { kind: "out", text: 'FINDING: RecordMismatch { field: "price" }' },
  { kind: "err", text: "error: VERIFICATION FAILED" },
];

const marqueeItems = [
  "SEALED UNTIL CLOSE",
  "ONE PRICE FOR EVERYONE",
  "ANYONE CAN CHECK IT",
  "ONE ATOMIC SWAP",
  "CAPPED SUPPLY",
  "NO ORDER READABLE EARLY",
  "PRIVATE PARTICIPANTS",
  "PUBLIC PRICE",
];

/// The three steps, joined by a line that draws itself across when the row scrolls into view.
function StepsRow() {
  const { ref, seen } = useInView<HTMLDivElement>();
  return (
    <div ref={ref} className="relative">
      <div aria-hidden="true" className="absolute top-[37px] right-[16%] left-[16%] hidden h-px bg-line md:block">
        <div className={cx("h-full origin-left bg-ink transition-transform duration-[1600ms] ease-out", seen ? "scale-x-100" : "scale-x-0")} />
      </div>
      <div className="relative grid gap-6 md:grid-cols-3">
        {steps.map((s, i) => (
          <Reveal key={s.n} delay={i * 140}>
            <Card className="h-full bg-paper p-6 transition-transform duration-300 hover:-translate-y-1">
              <span className="mono grid h-8 w-8 place-items-center rounded-full border border-ink bg-card text-sm">{s.n}</span>
              <h3 className="m-0 mt-4 text-[19px] font-medium">{s.t}</h3>
              <p className="m-0 mt-2 leading-relaxed text-muted">{s.d}</p>
            </Card>
          </Reveal>
        ))}
      </div>
    </div>
  );
}

function Seen({ v }: { v: Seen }) {
  if (v === "yes") return <Chip tone="up">sees it</Chip>;
  if (v === "no") return <Chip tone="neutral">cannot</Chip>;
  if (v === "key") return <Chip tone="warn">only with a viewing key</Chip>;
  return <Chip>{v}</Chip>;
}

function Section({ id, tint, children }: { id?: string; tint?: boolean; children: ReactNode }) {
  return (
    <section id={id} className={cx(tint && "border-y border-line bg-card")}>
      <div className="mx-auto max-w-[1180px] px-6 py-20">{children}</div>
    </section>
  );
}

function Head({ eyebrow, title, lead }: { eyebrow: string; title: string; lead?: string }) {
  return (
    <Reveal className="mb-10 max-w-[680px]">
      <div>
      <p className="eyebrow m-0">{eyebrow}</p>
      <h2 className="m-0 mt-2 text-[34px] leading-tight font-medium tracking-tight">{title}</h2>
      {lead && <p className="m-0 mt-3 text-[16px] leading-relaxed text-muted">{lead}</p>}
      </div>
    </Reveal>
  );
}

export default function Landing() {
  return (
    <div className="min-h-screen">
      <header className="mx-auto flex max-w-[1180px] items-center justify-between px-6 py-4">
        <Logo />
        <nav className="hidden items-center gap-8 text-[14px] text-muted md:flex" aria-label="Page">
          {[
            ["#how", "How it works"],
            ["#compare", "Compare"],
            ["#evidence", "Evidence"],
            ["#faq", "FAQ"],
            ["/docs", "Docs"],
          ].map(([h, l]) => (
            <a key={h} href={h} className="mono no-underline hover:text-ink">
              {l}
            </a>
          ))}
        </nav>
        <Link to="/rounds" className="no-underline">
          <Button size="sm">Launch app</Button>
        </Link>
      </header>

      <section className="relative overflow-hidden">
        <div className="grid-lines absolute inset-0 opacity-70" aria-hidden="true" />
        <GlyphField />
        <div className="relative mx-auto max-w-[1180px] px-6 pt-20 pb-24">
          <p className="eyebrow m-0 flex items-center gap-3">
            <span className="h-px w-10 bg-faint" /> SEALED-BID AUCTIONS FOR PRIVATE COMPANY SHARES, ON ZCASH
          </p>
          <h1 className="rise m-0 mt-6 max-w-[860px] text-[clamp(40px,7vw,76px)] leading-[1.02] font-medium tracking-tight">
            Trade private <span className="accent">shares</span> without showing your <span className="accent">hand</span>.
          </h1>
          <p className="m-0 mt-7 max-w-[640px] text-[19px] leading-relaxed text-muted">
            <Rotator words={["Orders", "Bids", "Limits", "Sizes"]} className="font-medium text-ink" /> stay sealed until the round closes. Then one price is published for everyone, anyone can check it, and the people behind the orders stay private.
          </p>
          <div className="mt-9 flex flex-wrap gap-3">
            <Link to="/rounds" className="no-underline">
              <Button>Launch app →</Button>
            </Link>
            {USE_MOCK && (
              <Link to="/rounds?tour=1" className="no-underline">
                <Button variant="outline">Take the 2-minute tour</Button>
              </Link>
            )}
            <a href="#how" className="no-underline">
              <Button variant="outline">How it works</Button>
            </a>
          </div>
          <dl className="m-0 mt-16 grid max-w-[760px] grid-cols-3 gap-6 border-t border-line pt-6">
            {[
              [3, "nodes hold the key, any 2 open the orders"],
              [1, "price per round, the same for everyone"],
              [0, "orders readable before the round closes"],
            ].map(([v, l]) => (
              <div key={l}>
                <dt className="num text-[40px] leading-none">
                  <CountUp to={Number(v)} duration={900} />
                </dt>
                <dd className="m-0 mt-2 text-sm text-muted">{l}</dd>
              </div>
            ))}
          </dl>
        </div>
      </section>

      <div className="border-y border-line bg-card py-3.5">
        <Marquee items={marqueeItems} />
      </div>

      <Section tint>
        <Head eyebrow="ONE ROUND, IN TEN SECONDS" title="Watch an auction happen." lead="The same four orders as our real demo. They stay sealed until the round closes, then clear at one price." />
        <Reveal>
          <DemoStrip />
        </Reveal>
      </Section>

      <Section>
        <Head eyebrow="WHAT YOU GET" title="The price is public. The participants are not." />
        <Reveal>
          <div className="grid gap-px overflow-hidden rounded-2xl border border-line bg-line md:grid-cols-2">
            {features.map((f, i) => (
              <div key={f.n} className="group bg-card p-7 transition-colors duration-300 hover:bg-wash/70">
                <div className="flex items-center justify-between">
                  {icons[i]}
                  <span className="mono text-xs text-faint">{f.n}</span>
                </div>
                <h3 className="m-0 mt-4 text-[19px] font-medium">{f.t}</h3>
                <p className="m-0 mt-2 leading-relaxed text-muted">{f.d}</p>
              </div>
            ))}
          </div>
        </Reveal>
      </Section>

      <Section id="how" tint>
        <Head eyebrow="HOW IT WORKS" title="Three steps. No order is readable until the second." />
        <StepsRow />
      </Section>

      <Section id="compare">
        <Head eyebrow="PRIVATE VS PUBLIC" title="An open order book shows your hand. A sealed round doesn't." lead="Tokenized private-company shares are starting to trade on open order books. They have thin liquidity, wide spreads, and every order and position in view." />
        <Reveal>
        <div className="overflow-x-auto rounded-2xl border border-line bg-card">
          <table className="w-full min-w-[640px] border-collapse text-sm">
            <thead>
              <tr className="bg-wash">
                <th className="eyebrow px-5 py-3 text-left font-normal" />
                <th className="eyebrow px-5 py-3 text-left font-normal">An open order book</th>
                <th className="eyebrow px-5 py-3 text-left font-normal">Kalyptra</th>
              </tr>
            </thead>
            <tbody>
              {compare.map(([a, b, c]) => (
                <tr key={a}>
                  <td className="border-t border-line px-5 py-4 font-medium">{a}</td>
                  <td className="border-t border-line px-5 py-4 text-muted">{b}</td>
                  <td className="border-t border-line px-5 py-4">{c}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        </Reveal>
        <p className="m-0 mt-4 max-w-[680px] text-sm text-muted">
          The trade-off: a sealed round is periodic, not instant. You place an order and wait for the round to close. We think that is the right trade for thinly traded private shares.
        </p>
      </Section>

      <Section tint>
        <Head eyebrow="WHO SEES WHAT" title="The trust model, in one table." />
        <Reveal>
        <div className="overflow-x-auto rounded-2xl border border-line bg-paper">
          <table className="w-full min-w-[720px] border-collapse text-sm">
            <thead>
              <tr className="bg-wash">
                <th className="eyebrow px-5 py-3 text-left font-normal" />
                {["You", "One matcher node", "The public", "An auditor"].map((h) => (
                  <th key={h} className="eyebrow px-5 py-3 text-left font-normal">
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {seenBy.map((r) => (
                <tr key={r.what}>
                  <td className="border-t border-line px-5 py-3.5 font-medium">{r.what}</td>
                  {[r.you, r.node, r.pub, r.auditor].map((v, i) => (
                    <td key={i} className="border-t border-line px-5 py-3.5">
                      <Seen v={v} />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        </Reveal>
        <p className="m-0 mt-4 max-w-[720px] text-sm text-muted">
          Two matcher nodes working together can read orders early. That is the security threshold, and the reason there are three nodes and not one. They still cannot hide what they did: the public checker flags it. Network details (who connected, from where) are not hidden by anything here.
        </p>
      </Section>

      <Section>
        <Head eyebrow="WHAT YOU CAN TRADE" title="Four assets, all issued for the demo." lead="There is no faucet, no real company and no real money. Anyone can list a new company from the app." />
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {assets.map((a, i) => (
            <Reveal key={a.s} delay={i * 90}>
              <Card className="h-full p-5 transition-transform duration-300 hover:-translate-y-1">
                <div className="flex items-start justify-between gap-2">
                  <p className="m-0 text-[22px] font-medium tracking-tight">{a.s}</p>
                  <Chip tone={a.s === "vUSD" ? "warn" : "up"}>{a.tag}</Chip>
                </div>
                <p className="m-0 mt-2 text-sm text-muted">{a.d}</p>
              </Card>
            </Reveal>
          ))}
        </div>
        <p className="m-0 mt-4 max-w-[680px] text-sm text-muted">Capped means the network itself refuses any new shares, forever. A token's identity is its asset id, not its name, so the app always shows the id.</p>
      </Section>

      <Section id="evidence" tint>
        <Head eyebrow="EVIDENCE" title="Built to be attacked, then checked again." lead="We wrote the attacks before trusting the defences, and switched each defence off to prove a test needs it." />
        <Reveal>
          <dl className="m-0 grid gap-px overflow-hidden rounded-2xl border border-line bg-line sm:grid-cols-2 lg:grid-cols-4">
            {evidence.map(([v, l]) => (
              <div key={l} className="bg-card p-6">
                <dt className="num text-[44px] leading-none">
                  <CountUp to={Number(v)} />
                </dt>
                <dd className="m-0 mt-3 text-sm leading-relaxed text-muted">{l}</dd>
              </div>
            ))}
          </dl>
        </Reveal>
        <div className="mt-8 grid gap-6 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
          <div>
            <h3 className="m-0 text-[19px] font-medium">Check a round yourself</h3>
            <p className="m-0 mt-2 leading-relaxed text-muted">
              Save any finished round to one file. Then verify it with no network and no node trusted, pinning the group key the nodes announced. Edit the price in the file and the check fails.
            </p>
          </div>
          <Terminal lines={verifyLines} />
        </div>
      </Section>

      <Section id="status">
        <Head eyebrow="WHAT'S LIVE" title="Every part, and how real it is." />
        <div className="overflow-x-auto rounded-2xl border border-line bg-card">
          <table className="w-full min-w-[640px] border-collapse text-sm">
            <thead>
              <tr className="bg-wash">
                {["Part", "Status", "Detail"].map((h) => (
                  <th key={h} className="eyebrow px-5 py-3 text-left font-normal">
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {status.map(([a, b, c]) => (
                <tr key={a}>
                  <td className="border-t border-line px-5 py-3.5 font-medium">{a}</td>
                  <td className="border-t border-line px-5 py-3.5">
                    <Chip tone={b === "Live" ? "up" : b === "Local" ? "warn" : "neutral"}>{b}</Chip>
                  </td>
                  <td className="border-t border-line px-5 py-3.5 text-muted">{c}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Section>

      <Section id="faq" tint>
        <Head eyebrow="QUESTIONS" title="What people ask first." />
        <div className="grid max-w-[820px] gap-2">
          {faq.map(([q, a], i) => (
            <Reveal key={q} delay={i * 60}>
            <details className="group rounded-xl border border-line bg-paper px-5 py-4 transition-colors open:bg-card">
              <summary className="flex cursor-pointer list-none items-center justify-between gap-4 text-[16px] font-medium [&::-webkit-details-marker]:hidden">
                {q}
                <span className="mono text-muted transition-transform group-open:rotate-45" aria-hidden="true">
                  +
                </span>
              </summary>
              <p className="m-0 mt-3 leading-relaxed text-muted">{a}</p>
            </details>
            </Reveal>
          ))}
        </div>
      </Section>

      <Section id="limits">
        <Head eyebrow="HONEST LIMITS" title="What this is not." />
        <div className="grid gap-x-12 gap-y-4 text-[15px] leading-relaxed text-muted md:grid-cols-2">
          <p className="m-0">Zcash Shielded Assets are not on mainnet, and swaps of them are not on any public network. Settlement here runs on a local regtest node built from QEDIT's prototype branches.</p>
          <p className="m-0">The threshold-decryption design is our own composition of standard parts. It has had an AI review, not a human audit. Do not use it with real assets.</p>
          <p className="m-0">Rounds are periodic by design, not continuous. A batch auction gives one fair price and hides order flow, and that rhythm is the trade-off.</p>
          <p className="m-0">After a round, every order is published with its side, size and limit (and a one-time key that names no one), because the checker needs them to recompute the price.</p>
        </div>
      </Section>

      <footer className="border-t border-line bg-card">
        <div className="mx-auto grid max-w-[1180px] gap-10 px-6 py-12 md:grid-cols-[1.3fr_1fr]">
          <div>
            <Logo />
            <p className="m-0 mt-4 max-w-[340px] text-sm leading-relaxed text-muted">Sealed-bid auctions for private company shares on Zcash Shielded Assets. Built for the Zecathon, Private Markets track.</p>
          </div>
          <div>
            <p className="eyebrow m-0 mb-3">APP</p>
            <ul className="m-0 grid list-none gap-2 p-0 text-sm">
              {[
                ["/rounds", "Rounds"],
                ["/trade", "Trade"],
                ["/list", "List a company"],
                ["/portfolio", "Portfolio"],
                ["/audit", "Audit"],
              ].map(([to, l]) => (
                <li key={to}>
                  <Link to={to} className="text-muted no-underline hover:text-ink">
                    {l}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        </div>
        <div className="border-t border-line">
          <div className="mx-auto flex max-w-[1180px] flex-wrap items-center justify-between gap-3 px-6 py-5 text-xs text-muted">
            <span className="mono">Sealed until close · public price · private participants · capped supply</span>
            <span>Local regtest only. Not audited. Not for real assets.</span>
          </div>
        </div>
      </footer>
    </div>
  );
}
