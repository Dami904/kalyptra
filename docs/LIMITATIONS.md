# Limitations

What is explicitly NOT handled. Dates are when each was last checked against the code.

## Settlement and chain (checked 2026-10-15)
- **Local regtest only.** Zcash Shielded Assets are not on mainnet, and ZSA swaps are not on any
  public network. Settlement uses QEDIT's prototype Zebra and `zcash_tx_tool` branches, pinned
  to specific commits (see [`settlement/README.md`](../settlement/README.md)).
- **A wallet can settle one order per asset per round.** Two orders from one wallet that spend
  the same asset would be built from the same note, and the node rejects the block
  (`DuplicateOrchardNullifier`, found on a real run). The gateway refuses the second order at
  placement. A buy and a sell in the same round are fine.
- **Settlement outcomes are confirmed, failed or unknown.** Only the node's own "Block rejected"
  answer is `failed` (nothing moved), and only when no earlier attempt was unknown; any other
  failure, no answer, or a tool that runs past its time limit (15 minutes,
  `KALYPTRA_TOOL_TIMEOUT_SECS`) is `unknown` (it may have gone out). Either can be
  retried by the leader (`POST /v1/rounds/{r}/settle/retry`, or the button on Rounds): the chain
  refuses to spend the same notes twice, so at most one attempt lands. Nothing reconciles an
  `unknown` automatically; check balances first.
- **No expiry on settlement parts.** A part handed in late is accepted while the round is still
  collecting; a stale one fails at the chain if its notes were spent meanwhile.
- **One node key signs everything,** votes and company listings alike. A stolen node key can sign
  listings too (one key still cannot reach `t`). See [`ACCESS_CONTROL.md`](ACCESS_CONTROL.md).
- **A missing part stalls the round.** If a filled order's owner never hands in its part, the
  round waits, visibly. Re-clearing without that order is not built.
- **Orders cleared but not yet settled are not counted** when the gateway checks an investor's
  funds for a new order.
- **Every wallet command rescans the chain.** Fine on regtest; startup of the full stack takes
  15-20 minutes for that reason.

## Protocol
- **`t` colluding nodes can read orders early** (the security threshold) and can stall or void a
  round. See [`THREAT_MODEL.md`](THREAT_MODEL.md).
- **Late receipts.** A node that votes again before a peer's receipts reach it simply joins a
  further epoch; receipts only grow, so it converges, but there is no timer. The re-vote path is
  tested in-process, and not forced over real HTTP by a test.
- **Salt grinding at equal prices.** Pro-rata allocation would remove it; not built.
- **Plain HTTP** between nodes and for the gateway. Fine on loopback, not elsewhere.
- **Not audited.** AI review only.

## App
- **One machine holds every demo investor's wallet.** In real use each investor runs their own
  gateway.
- **Listing a company** is signed by the nodes but nothing checks the company is real.
- **Linux or WSL only** for the full stack. There is no `docker compose`.
- **Phone-size layout** was checked only in a browser pane at 375 px (no horizontal overflow on
  any screen), not on a real phone, and there are no automated browser tests.
- **CI** (`.github/workflows/ci.yml`) passed all six jobs on GitHub on 2026-10-16 (tests, lint, docs,
  the Rust 1.85 build, vectors, UI, guardrails, mutation check). The regtest scripts are not in CI.

## Reliability gaps found by the 2026-10-15 review, not fixed
- **A crash between issuing a token and recording it can double-issue.** The asset id is
  written to `pending-listings.json` only after the wallet returns it. If the gateway dies in
  that window, the token exists on chain and the next listing request issues another. When the
  record cannot be written, the error now names the asset id so it is not lost.
- **A round can stay closed.** With one node offline and the other two disagreeing, the vote can
  never be proven dead; after 16 epochs a node stops voting again. Message delivery drops a peer's
  4xx or 5xx reply without retrying (a node that was not ready can lose a vote).
- **Nodes keep rounds and votes in memory only.** A restarted node forgets its rounds, and could
  sign a different first vote if the round were reopened. The gateway counts an investor's
  tickets in a round no node knows as still reserving funds, so after a node restart those funds
  stay reserved. The demo stack is always started fresh.
- **No node-level pull sync** for votes or receipts, and no operator command to force a void.
- **One lock for all orders at a gateway.** The funds check and the ticket write are serialised
  across every investor on that gateway, and the lock is held while the wallet rescans, so one
  slow order delays the others. Fine for a demo machine.

Fixed in the same review: file writes are atomic (a sibling file, flushed, then renamed); a
damaged `pending-listings.json` or node listings file is an error, not "empty"; the order check
and ticket write are serialised, so two orders arriving together cannot both spend the same
funds; a node signs a listing only after saving it; the checker flags a skipped epoch; the
gateway logs every error it returns as a one-line JSON record; an unreadable saved ticket stops
new orders (503) instead of being skipped; a wallet command that hangs is stopped after its
time limit; the order's ticket (and one-time key) is saved before the order is sent, and kept
unless every node definitely refused it, so an order a node may hold never loses its key and its
funds stay reserved until the round ends.
