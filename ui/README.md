# Kalyptra: the page

A light, editorial React page for Kalyptra: `React 19`, `Vite`, `Tailwind 4`, `TypeScript`. It has no keys and no wallet. It talks to the **local gateway** (`kalyptra serve`), which runs next to your wallet and wraps the `kalyptra` command line as JSON.

## Screens

| Screen | What it does |
|---|---|
| `/` Landing | The idea in order: a looping animated walk through one round (sealed, closed, opened, priced), what you get, how it works, a private-versus-open-book comparison, a who-sees-what trust table, the four assets, an evidence strip with a copyable offline-verify example, a table of what is live and what is not, a FAQ, the honest limits, and a footer |
| `/rounds` | The round list, a step bar (Open, Closed, Committed, Cleared, Settled), the price and volume, the public check result, the opened orders, settlement status, and a button to save the round for offline verification |
| `/trade` | Pick a market with an open round, seal an order, and see its receipts. Below it, **My orders** (only the investor you are acting as), with a Settle button once an order is filled |
| `/list` | List a company: issue a token from your wallet, optionally capping its supply forever |
| `/portfolio` | Your own balances, read from your own wallet |
| `/audit` | See holdings through a viewing key, or export yours |

## Run it

It is one app. Run all of it, for real, from the repository root:

```bash
bash scripts/run.sh
```

Then open <http://127.0.0.1:8787>. Use **Acting as** in the header to switch between the investors' wallets.

**Only for working on the look**, there is a sample-data mode with no nodes or wallets behind it. The header then says "Sample data". It is not a way to run Kalyptra:

```bash
cd ui
npm ci
VITE_USE_MOCK=true npm run dev
```

With the gateway running on `:8787`, `npm run dev` (without the mock flag) serves the page on `:5173` and proxies `/api` to it.

## Motion and design

The look is Ztocks' (calm, off-white, thin type, monospace labels) with details taken from the Kombai gallery: an italic serif accent word in the headline, and node-and-dot trackers like its approval dashboards. Every effect has a job, and every one switches off under `prefers-reduced-motion`.

| Where | Effect | Job |
|---|---|---|
| Landing hero | A canvas of sealed glyphs drifting around a ring; now and then one opens into a green digit and seals again | The idea of the product as a backdrop |
| Landing hero | The word before "stay sealed" rotates (Orders, Bids, Limits, Sizes) and its slot slides to each word's width | Says what is hidden |
| Landing | A slow marquee of the product's claims | Rhythm between sections |
| Landing | An animated round: sealed blocks arrive, the round closes, they open, one price lands | Shows the product in ten seconds |
| Landing | Sections and cards fade up as they scroll in; the three steps are joined by a line that draws itself | Pacing |
| Landing | Evidence numbers count up; the offline-verify terminal types its commands, then prints the result | Evidence, shown live |
| Rounds | The step bar's line fills and the current step pings | Where the round is |
| Rounds | **Per-node tracker**: for each node, four dots (receipted, voted, shared, signed) filled from the nodes' real published data | What each node actually did |
| Rounds | Opened orders **decrypt**: cipher characters resolve into the real side, size and limit | The "sealed, then opened" moment |
| Rounds | Price, volume and order count count up | Draws the eye to the result |
| App | Screens fade in; loading shows shimmering skeletons, not spinners | Feels responsive |

The per-node tracker is not decoration. A node that sees two matching commit votes commits without voting itself, so a node can show a hollow "Voted" dot and still be fine. The page shows that as it is.

## How it is built

- `src/lib/types.ts`: the shapes the gateway returns, and the `Api` interface.
- `src/lib/real.ts`: the real client. `src/lib/mock.ts`: an in-memory pretend backend with the same rules (a real uniform-price auction), used when `VITE_USE_MOCK=true`. The header shows "Sample data" when it is on.
- `src/components/`: the small kit (button, card, chip, notice, table, the step bars) and the shell.
- `src/pages/`: one file per screen.

## What the page promises, and what it does not

- **Keys never reach the page.** An order's one-time key stays in the gateway's tickets folder. The gateway tests assert that no ticket key appears in anything the page receives.
- **A result is shown as verified only if the public checker passed.** The gateway runs the checker on every finished round.
- **Errors are shown, not hidden.** With no wallet the page says so instead of faking balances or settlement.
- **The gateway has no login.** It only listens on `127.0.0.1`. Run it only on a machine you control.
