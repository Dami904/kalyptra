# Kalyptra: demo run-of-show

**Thesis in one line:** the hard part of a private market is not hiding the orders; it is hiding
them *and* letting anyone prove the one published price is the right one.

Target: **2 minutes** (the Zecathon form asks for a two-minute video), then Q&A if live. Feeling first (a sealed order), then proof
(the price checked, a forged price rejected), then the close. Every number on screen is live
from a real run; if a step fails, the contingency section says what to show instead.

---

## Pre-flight (before recording)

- [ ] `bash scripts/run.sh` finished in WSL and printed `Kalyptra is running: http://127.0.0.1:8787`
      (15 to 20 minutes on a cold start: start it well before recording).
- [ ] Browser at <http://127.0.0.1:8787>, zoom 110 to 125 %, light theme, one tab.
- [ ] A second terminal in WSL, large font, at the repo root, for `bash scripts/demo.sh`
      (stop the stack first, or run the demo on another machine: both use ports 7101-7103).
- [ ] The stack is fresh: round 1 (ACME/vUSD) open, no orders yet. If not, Ctrl-C and rerun.
- [ ] Backups open in other tabs: this repo's `README.md` (the pasted real run), `ATTACKS.md`.
- [ ] The one moment to let breathe: **the forged price being rejected.** Stop talking for two
      seconds when `VERIFICATION FAILED` appears.

## Before you press record (saves 30 seconds on camera)

Place bob's, carol's and dave's orders first (bob sells 4 at 5; carol buys 6 at 5; dave buys 8 at 6),
so on camera you only place alice's. Edit out any wait while wallets work: a cut is fine, a
spinner is not.

## Cold open: the problem (0:00-0:15)

> "Private company shares trade in the dark. Show the orders and everyone trades against them;
> hide them with one operator and you have to trust that operator. Kalyptra removes the operator:
> the price is public, the participants are not."

Landing page hero, then **Launch app**.

## The order, sealed (0:15-0:45)

1. **Trade, as alice:** sell 10 ACME at 4. Point at "3 of 3 nodes receipted it (needs 2)":
   "Encrypted to a key split across three nodes. Each signs a receipt; none can read it."
2. **The guard:** a second ACME sell in the same round is refused, before it is counted.

## The close and the proof (0:45-1:35)

3. **Rounds:** close the round. The node tracker fills: receipts, votes, shares, signatures.
   "Nothing opens until two nodes sign the same final set."
4. **The price:** cleared at **5**, volume **14**. "Dave bid 6 and pays 5: one price for everyone."
   **Public check: Passed.** "Not the nodes saying so: anyone recomputes it from published data."
5. **Settle** (cut the waiting): one transaction, block number and txid on screen. Portfolio:
   alice ACME 90, vUSD 50.

## What stays private, and the close (1:35-2:00)

6. **The refusal (the beat a sharp judge would try).** `bash scripts/demo.sh` step 5b: the saved
   round with the price changed from 5 to 6, checked offline with every node stopped:
   `VERIFICATION FAILED`. "Nobody has to trust the nodes for the price, and a forged one is
   caught." Let it sit for two seconds.

> "Who placed each order is never published: every order has its own one-time key, and holdings
> stay shielded on chain. Only the anonymous orders are opened after the close, so the price can
> be checked. 154 tests, every defence proven by switching it off, settlement on a real Zcash
> regtest chain."

Then say the limit out loud, before a judge finds it: "Today the three nodes run on one machine,
on a local regtest chain; real use puts each node with a different operator. Kalyptra."

Stop.

---

## If something breaks

- **The stack is still starting or a wallet command is slow:** say "every wallet command
  rescans the chain; that is the regtest prototype, not the protocol", and cut to
  `bash scripts/demo.sh`, which needs no chain and shows the whole protocol in a minute.
- **No local stack at all (or it will not start):** open the hosted sample-data demo at <https://kalyptra.vercel.app> and press "Take the 2-minute tour"; it walks the same flow without a chain. Say plainly that it is sample data.
- **The page will not load:** the gateway is not up. Show `scripts/demo.sh` output instead; the
  README has the same run pasted from 2026-10-15.
- **Settlement shows `failed` or `unknown`:** that is a real state, not a crash. Press **Try the
  settlement again** and say "a retry is safe: the chain refuses to spend the same notes twice".
  If it fails again, show the settled run in the README (block 14, txid `e19d96b4…432f`).
- **A command misbehaves on camera:** skip to step 8 (the offline verification). It is the
  strongest single proof and needs nothing running.
- **Everything dies:** screen-share the README's "core proof" block and `ATTACKS.md`: real output,
  plus every attack mapped to the test that proves it is stopped.
