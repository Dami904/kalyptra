# API notes: measured behaviour of what Kalyptra's settlement depends on

Kalyptra's own interfaces are documented by their tests. This file is about the two upstreams
we do not control, and what was actually observed, not assumed. Pinned versions: QEDIT's
`zcash_tx_tool` branch `zsa-swap` at `a344fde`, and QEDIT's Zebra branch `zsa-swap-multiple-ag`
at `0aef55c` (Docker image `qedit/zebra-regtest-swap`).

## The regtest node (Zebra, JSON-RPC on port 18232)

| Observed | What it means for Kalyptra |
|---|---|
| Submitting a transaction on regtest mines a block directly. A rejected block comes back as the tool error `the node rejected the settlement: Block rejected`, and the reason is only in the node's log (`docker logs kalyptra-regtest`). | The leader's job reports `failed` with that text. The real cause must be read from the node log. |
| 2026-10-15: two parts built from one wallet's single ACME note were rejected: `DuplicateOrchardNullifier`. | Two orders spending the same asset from one wallet cannot settle in one round. The gateway refuses the second at placement. |
| `getblock` for a height not yet mined returns `block height not in best chain` (logged at INFO). | Normal while a wallet polls for its transaction; not an error. |
| A transaction is atomic: an unbalanced one (value balance does not match) is rejected by the node, not by the leader. | Spike G0, attack B: a leader that tries to take more than the agreed amounts is rejected. |

## The wallet tool (`zcash_tx_tool`, with `settlement/zcash_tx_tool-kalyptra.patch`)

| Observed | What it means |
|---|---|
| Every command rescans the chain from the start. | Slow on a long chain; the gateway holds one lock per wallet so two commands never touch one wallet database at once. |
| `kalyptra-wallet balances` reads every asset in one pass; per-asset `balance` rescans once per asset. | Portfolio went from about 25 s to 0.3 s on a fresh chain. |
| A wallet's address must be derived before `sync_from_height`, or its notes are not found after a rescan. | Fixed in the patch. |
| Commands print one line `KALYPTRA_RESULT <json>`; a non-zero exit with `{"ok":false,"error":...,"bad":[...]}` names parts that do not match their legs. | `crates/kalyptra-node/src/wallet.rs` parses exactly this line. |
| A part is about 9.3 KB: QEDIT's action-group bytes plus the value balance and binding key. | Parts cross processes as bytes, wrapped with the `KLG1` magic. |

## Not yet measured
- Behaviour on a chain with thousands of blocks (rescan time).
- Any behaviour on a public network: ZIP 228 swaps are not on one.
