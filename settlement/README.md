# Settlement: a cleared round becomes one atomic swap

A cleared Kalyptra round settles as **one transaction** on Zcash Shielded Assets (local regtest). Either every filled order moves, or nothing does.

## Who does what

| Who | Does | With |
|---|---|---|
| Each investor | Checks the round with the public checker. Finds its own order by its envelope hash. Builds **its own** part of the swap (an action group) for exactly the record's terms. Signs the part with the order's one-time key. Hands it to every node. | Its own wallet, in its own process (`kalyptra settle`) |
| Every node | Accepts a part only if the order's owner signed it, for this record, for these exact bytes | `POST /v1/rounds/{r}/settle` |
| The round's leader (rotates: `ids[round % n]`) | Once every filled order has a part, checks each part's amounts, assembles one transaction and mines it | Its own wallet, which holds no investor's assets |
| An auditor | Given an investor's viewing key, sees that investor's holdings. Cannot spend them. | Its own wallet (`kalyptra audit`) |

Terms come from the round record, which `t` nodes co-signed and anyone can recompute: a seller sends `fill` of the equity and receives `fill × price` vUSD, and a buyer does the reverse. The leader cannot change them.

## Why a cheating party cannot move the wrong amounts

- **A part's amounts are checked before assembly.** A part's value commitments add up to the amount of each asset it moves, plus a blinding term the binding key removes. So the leader can check what a part moves without seeing a single note. A part that does not match its leg is named, dropped, and its owner can hand in a new one. The other parts are kept. Without this, one bad part would sink the whole transaction with no culprit.
- **The node rejects anything that does not balance.** A leader that takes more than agreed is rejected by the node itself. This is G0 attack B.
- **Only the owner can hand in a part for an order.** The part is signed with the order's one-time key, which the co-signed record names.
- **A wallet holds only its own keys.** The leader cannot build a part that spends an investor's notes. This is G0 attack A.

## What is here

- `zcash_tx_tool-kalyptra.patch` adds these `kalyptra-wallet` commands to QEDIT's `zcash_tx_tool` (branch `zsa-swap` at `a344fde`):

  | Command | Does |
  |---|---|
  | `init` | Create a wallet directory: a seed and a database, read at every start |
  | `address`, `balance` | Show the wallet's address and its balance of one asset |
  | `issue` (`--finalize`), `send` | Issue an asset (and cap its supply for good); send to another wallet |
  | `leg` | Build this wallet's part and write it to a file |
  | `assemble` | Check each part's amounts against a plan, then build one transaction and mine it |
  | `view-key`, `audit` | Export a viewing key; report the holdings behind one |

  Each command prints one `KALYPTRA_RESULT <json>` line. The patch also carries the G0 spike's additions: batch swaps and separate-key parts.
- **Part file format** (`src/components/kalyptra_wire.rs` in the patch): the action-group layout QEDIT's librustzcash uses inside a swap transaction, rebuilt from that crate's public helpers, plus the group's value balance and binding key. A 1-in, 1-out part is about 9.3 KB.
- `setup.sh` builds the Zebra regtest image and the patched tool, and fetches the Sapling parameters.

## Run it

```bash
bash settlement/setup.sh
```

```bash
bash scripts/demo-settle.sh
```

## Evidence (2026-10-09, WSL Ubuntu 24.04, local regtest)

`scripts/demo-settle.sh`, about 3.5 minutes end to end:

1. **Wallets:** an issuer, four investors and three matcher nodes, each with its own wallet directory.
2. **Assets:** ACME issued and **finalized**, vUSD issued; the investors funded.
3. **Round:** three node processes clear it at **5, volume 14**, and `CHECK PASSED`.
4. **Parts:** the four investors build their parts **in parallel, each in its own process**, and each part is accepted by 3 of 3 nodes.
5. **Settlement:** node 2, the leader for round 1, settles all four legs in **one transaction**.
6. **Balances:** all 14 match the record: alice 90 ACME and 50 vUSD, bob 96 and 20, carol 8 and 960, dave 6 and 970, and every node wallet 0.
7. **Audit:** an auditor with only alice's viewing key sees ACME 90 and vUSD 50.

**By hand, with real parts:**
- A plan claiming 60 vUSD where the parts moved 50 was refused before anything was sent, and both parts were named.
- The honest plan then settled.

The protocol around this (who may hand in a part, when the leader assembles, what happens to a bad part) is covered by `crates/kalyptra-node/tests/http.rs`. Those tests use a stand-in for the wallet tool.

## Honest limits

- **Prototype software.** QEDIT's swap node branch and libraries, local regtest only. ZIP 228 swaps are not on any public network: they need transaction v7, which QEDIT has not shipped.
- **The leader learns each part's amounts** through its binding key. It knows them anyway from the public record. It does not learn which notes or addresses are involved.
- **The leader pays the transaction fee and mines.** On regtest the "submit" step mines a block directly.
- **A missing part stalls the round.** If a filled order's owner never hands in its part, the round waits, visibly (`kalyptra settlement`). Re-clearing without that order is not built.
- **Each wallet rescans the chain from the start on every command.** That is fine for regtest and slow for a long chain.
- **Licence.** The patch modifies and partly copies QEDIT's code. QEDIT's README says MIT, but the repository has no LICENSE file. Confirm before relying on it.
