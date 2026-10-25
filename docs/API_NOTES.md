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
| A scan stops at the first error from the node and treats it as the end of the chain (`sync_from_height`). Measured 2026-10-22: with the node stopped, `balances` answered every asset as 0, as a success, and the gateway refused an order with "holds 0". | Kalyptra's patch checks every scan reached the node's best block (`sync_to_tip`): a short scan is wiped and redone, up to 5 times, then the command fails with "the chain node does not answer" (the gateway answers 502). |
| `getbestblockhash` comes back byte-reversed from the client, `getblock` hashes do not. | The tip check turns the best hash round before comparing; without that it never matches once the chain has blocks. |
| Measured 2026-10-22: a block holding a transaction the chain already has makes the node panic ("transactions must be unique within a single chain", exit 139) instead of rejecting the block. | The patch checks `getrawtransaction` before submitting a settlement and reports the block it is already in. The transaction id must be sent byte-reversed from `txid()`, like the block hashes. |
| Measured 2026-10-23, during a 109-round run: the node stopped answering for 54 minutes, then carried on by itself; its log at that moment: "chain updates have stalled, state height has not increased for 54 minutes". | Nothing was wrong afterwards, but the wallet tool's HTTP client has no timeout, so a wallet command waits as long as the node does. A request timeout in the patch would turn that into a clear error; not done yet. |
| Every command rescans the chain from the start. | Slow on a long chain; the gateway holds one lock per wallet so two commands never touch one wallet database at once. |
| `kalyptra-wallet balances` reads every asset in one pass; per-asset `balance` rescans once per asset. | Portfolio went from about 25 s to 0.3 s on a fresh chain. |
| A wallet's address must be derived before `sync_from_height`, or its notes are not found after a rescan. | Fixed in the patch. |
| Commands print one line `KALYPTRA_RESULT <json>`; a non-zero exit with `{"ok":false,"error":...,"bad":[...]}` names parts that do not match their legs. | `crates/kalyptra-node/src/wallet.rs` parses exactly this line. |
| A part is about 9.3 KB: QEDIT's action-group bytes plus the value balance and binding key. | Parts cross processes as bytes, wrapped with the `KLG1` magic. |

## Not yet measured
- Behaviour on a chain with thousands of blocks (rescan time).
- Any behaviour on a public network: ZIP 228 swaps are not on one.
