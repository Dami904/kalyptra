# Kalyptra: demo run-of-show

**Thesis in one line:** the hard part of a private market is not hiding the orders; it is hiding
them *and* letting anyone prove the one published price is the right one.

Target: 3 minutes for the video, then Q&A if live. Feeling first (a sealed order), then proof
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

## Cold open: the problem (0:00-0:20)

> "Private company shares trade in the dark. If everyone sees the orders, everyone trades
> against them. If one operator hides them, you have to trust that operator not to peek, not to
> favour anyone, and not to quietly pick the price. Kalyptra removes that operator."

Show the landing page hero, then click **Launch app**.

## Act 1: the feel (0:20-1:20)

1. **Trade, as alice:** sell 10 ACME at 4. Point at "3 of 3 nodes receipted it (needs 2)":
   "The order is encrypted to a key no single node holds. Each node signs a receipt; it cannot
   read it."
2. **Switch Acting as to bob, carol, dave.** bob sells 4 at 5; carol buys 6 at 5; dave buys 8 at 6.
3. **The guard, once:** as alice, try a second sell of ACME in the same round. Read the refusal
   aloud: the gateway refuses because both parts would spend the same note. "It refuses what it
   cannot settle, before it counts."
4. **Rounds:** close round 1. Watch the node tracker: receipts, votes, shares released, signed.
   "Only after two nodes signed the final set does any node release its share."

## Act 2: the proof (1:20-2:30)

5. **The price:** cleared at **5**, volume **14**. "Dave bid 6 and pays 5: one price for everyone."
6. **The check:** the round shows **check clean**. "This is not the nodes saying so. The checker
   recomputes the price from what they published."
7. **Settle:** as each investor, press Settle. The leader assembles **one transaction**; show the
   block number and the txid. Portfolio as alice: ACME 90, vUSD 50.
8. **Cut to the terminal:** `bash scripts/demo.sh`, scrolled to step 5b. "Saved to a file, every
   node stopped, verified offline. Now I change the price in the file from 5 to 6." Let
   `VERIFICATION FAILED` sit on screen.

## Close: the thesis (2:30-3:00)

> "Orders sealed until close, one public price anyone can check, private participants, and
> settlement in a single atomic swap on Zcash Shielded Assets. 149 tests, every defence proven
> by switching it off. It runs on a local regtest chain today, because ZSA swaps are not on a
> public network yet. Thank you."

Stop talking.

---

## If something breaks

- **The stack is still starting or a wallet command is slow:** say "every wallet command
  rescans the chain; that is the regtest prototype, not the protocol", and cut to
  `bash scripts/demo.sh`, which needs no chain and shows the whole protocol in a minute.
- **The page will not load:** the gateway is not up. Show `scripts/demo.sh` output instead; the
  README has the same run pasted from 2026-10-15.
- **Settlement shows `failed` or `unknown`:** that is a real state, not a crash. Press **Try the
  settlement again** and say "a retry is safe: the chain refuses to spend the same notes twice".
  If it fails again, show the settled run in the README (block 14, txid `e19d96b4…432f`).
- **A command misbehaves on camera:** skip to step 8 (the offline verification). It is the
  strongest single proof and needs nothing running.
- **Everything dies:** screen-share the README's "core proof" block and `ATTACKS.md`: real output,
  plus every attack mapped to the test that proves it is stopped.
