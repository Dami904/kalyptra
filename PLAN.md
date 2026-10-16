# Kalyptra: build plan

**Status (2026-10-15):** working prototype. G0, G1 and G2 passed; the whole app runs on a local regtest chain (`scripts/run.sh`). Closed on 2026-10-15: balance check at order time, re-voting for diverging receipts, node-signed listings, settlement retry, CI configured (not yet run on GitHub). Next: the repo location and first commit (the user decides), then the video (`docs/RUNOFSHOW.md`). See "Gap closing (2026-10-15)" and `docs/LIMITATIONS.md`.
**Track:** Private Markets (Zecathon, $15k track prize, plus the $20k Grand Prize on top). Expect a crowded track, so this plan goes for depth.
The track brief: "The dark-pool thesis. Sealed-bid auctions, order flow, settlement, anything where the price should be public and the participants should not."

## What Kalyptra is

A sealed-bid auction for private company equity, where **no single party can read a bid before the round closes**. Settled on Zcash Shielded Assets (ZSAs).

- Investors submit orders **encrypted to a round key that no one holds alone**. The key is split across n independent matcher nodes (demo: 2-of-3). Any t of them can decrypt, and fewer than t learn nothing.
- Matchers decrypt **only after the round closes**. Each node recomputes the same **clearing price**, and t of them co-sign the round record.
- Matching orders settle **atomically** through ZSA swaps (ZIP 228).
- **Public:** the clearing price, total volume, the committed order set, and the co-signed round record. Anyone can recompute the price.
- **Private:** who traded, who holds what, and every order's contents until the round closes.
- **Supporting pieces:** the equity token is issued and finalized (supply capped forever), and an auditor with a viewing key can verify holdings.

One line for the README: *sealed until close, public price, private participants, capped supply.*

## The core design: threshold-sealed orders

This is what separates Kalyptra from a thin app on QEDIT's tool and from Wraith and Tukar, which both have a single trusted matcher that sees every order.

### Actors

- **Investors:** each has an Ed25519 identity key (to sign orders) and holds ZSAs in a Zcash wallet.
- **Matcher nodes (n = 3, threshold t = 2 in the demo):** each holds one share of the round decryption key and its own signing key.
- **Settlement leader:** one matcher per round, rotating, that builds the ZIP 228 swap bundle. Any matcher can do it.
- **Auditor:** holds a viewing key.
- **Checker:** a standalone program anyone can run on the public round data.

### Round lifecycle

1. **Setup (once per matcher set): distributed key generation.** The nodes run a Pedersen/FROST-style DKG on ristretto255 and end with a group public key `PK` and one secret share each. No node ever holds the full key. **We run a separate DKG for the encryption key; signing keys are separate and never reused.**
2. **Round opens.** The board publishes `PK`, the round id, the close time, and the minimum number of bidders.
3. **Submit.** An investor builds an order `{side, size, limit, round_id, salt}`, signs it with their identity key, and encrypts it to `PK` with hybrid encryption: an ephemeral ristretto key agreement with `PK`, then ChaCha20-Poly1305, with the round id as authenticated data. The envelope carries a **proof that the sealer knows its ephemeral secret**, bound to the round id and the ciphertext, so nobody can lift someone else's envelope into another round. They post the envelope to the **bulletin board**.
4. **Bulletin board.** An append-only, hash-chained log replicated on all nodes. An order is accepted once **t nodes sign a receipt** over its ciphertext hash. A single node therefore cannot silently censor an order; it takes t colluding nodes.
5. **Close.** At close time the board stops accepting orders. Each node signs the final log head. **t matching signatures commit the order set.** If fewer than the minimum number of bidders took part, the round is voided and nothing is decrypted.
6. **Threshold decrypt.** For each envelope, each node first checks the sealer's proof against **this** round id (and refuses otherwise), then computes its partial decryption `s_i * R` with a **DLEQ (Chaum-Pedersen) proof** that it used its real share. Nodes never release partials before close or for a voided round. Anyone can verify each proof. Any t valid partials combine by Lagrange interpolation. A node that sends a wrong partial is caught and excluded.
7. **Clear.** The call auction is a deterministic function, so every node computes the same clearing price from the same decrypted set. **t nodes co-sign the round record**: price, volume, committed set hash.
8. **Reveal and verify.** The anonymised orders (side, size, limit, no identities) are published. The checker recomputes the price and verifies the committed set, every DLEQ proof and every signature.
9. **Settle.** For each match, the two owners settle one fixed-term swap at the clearing price, using whichever swap route passes G0. The leader coordinates (as the mailbox for the pair's messages, or as the matcher that builds the bundle) and submits. The leader's spread is its fee.

### Build ladder (there is always a working version)

| Level | What it is | Fallback value |
|---|---|---|
| L0 | Commit-reveal: bidders post a hash first, reveal after close | Matcher cannot use contents while the round is open |
| L1 | One matcher, orders encrypted to a round key | Outsiders cannot read the board |
| **L2** | **Threshold 2-of-3 with DKG, DLEQ-proven partial decryptions, co-signed record (the target)** | No single node can read or censor |
| L3 (stretch) | Timelock backup (drand `tlock`) so orders decrypt at time T even if the committee fails; 3-of-5 | Liveness fallback |

**L2 was built directly (2026-10-08 and 09), so L0 and L1 remain only as fallbacks.** Gate G1 (Oct 16): L2 must pass its standalone tests, or Kalyptra ships L1 plus L0 and says so in the README. **The library side of G1 is met; see G1 progress.**

### Building blocks (verified to exist and be maintained)

- **Distributed key generation and shares:** `frost-ristretto255` (Zcash Foundation FROST, partially audited by NCC; updated Oct 2026). It provides DKG, trusted-dealer setup, share refresh and repair. It does **not** provide threshold decryption.
- **Curve and encryption:** `curve25519-dalek` (ristretto), `chacha20poly1305`, `hkdf`, `sha2`.
- **What we write ourselves (the novel, unaudited part):** threshold ECDH decryption with DLEQ proofs, the sealer's proof of knowledge, Lagrange combination, the bulletin board, the round state machine. This is the main security risk and gets the heaviest testing.
- **Timelock (L3):** the `tlock` crate (v0.0.10, early) and drand's Go library.

## Assets we issue ourselves (four ZSAs)

Everything is a test asset that we issue. There is no faucet, no real company and no real money.

| Asset | What it is | Notes |
|---|---|---|
| **ACME** | Equity token of a fictional company | Issued, then **finalized** (supply capped) |
| **BOLT** | Equity token of a second fictional company | Same |
| **CRUX** | Equity token of a third fictional company | Same |
| **vUSD** | Stable quote asset every equity is priced in | A unit of account only. **No peg and no redemption**; it is play money we issue |

- **Why three equities:** each auction round is for one equity against vUSD, so three show several markets running side by side, and investors hold different positions (a larger, more realistic anonymity set). Extra tokens cost almost nothing: each issue takes seconds.
- **Every trade is an equity/vUSD pair.** There are no equity-for-equity trades.
- **Investors:** a handful of test wallets funded with vUSD and some equity by shielded transfers before the demo.
- **The demo shows one or two rounds**, not all three, but the other two markets exist and are visible to the auditor.

## What the demo shows (2-3 minutes)

1. Three matcher nodes start. A DKG runs and they publish one group key. No node holds it.
2. Four investors submit sealed orders. The board shows only ciphertexts and receipts. Nothing is readable.
3. The round closes. A last-second attempt to decrypt with one node's share **fails**.
4. Two nodes release proven partial decryptions. The orders open, the clearing price is computed, and two nodes co-sign it.
5. The matching orders settle atomically through ZSA swaps. The unmatched orders return untouched.
6. **Anyone runs the checker** on the public data and gets the same price.
7. **A cheating matcher is caught on screen:** a node sends a bad partial decryption, or lies about the price, and the checker flags it.
8. The auditor, with a viewing key, sees the real holdings. The public sees only price and volume.
9. Bonus: the founder's issue of more ACME is rejected on-chain, because the supply is finalized.

## Trust table (kept in the README)

| What a single matcher can do | What it cannot do |
|---|---|
| Go offline (liveness; the others carry on while t remain) | Read any order before the round closes |
| Send a wrong partial decryption | Get it accepted (the DLEQ proof fails and it is excluded) |
| Refuse to sign a receipt | Silently censor an order (t signatures are needed to refuse) |
| Take the settlement-leader spread when it leads | Publish a wrong price (t co-signatures are needed, and the checker recomputes it) |

| What t colluding matchers can do | Why we say so |
|---|---|
| Decrypt orders early | This is the security threshold. Below t, nothing leaks. At t, everything does |
| Censor or reorder orders | Detectable by receipts and the committed set, but not prevented |
| Co-sign a wrong price | Detectable by the checker, not prevented |

**Honest limits, stated in the README:**
- The demo runs all three nodes on one machine under one operator. It **demonstrates the mechanism**; it is not real decentralization.
- Security needs fewer than t colluding nodes. We use a small committee, so the threshold is small.
- The threshold-decryption protocol is **our own composition of standard, widely used primitives** (FROST's key generation is partially audited by NCC; the rest is not) **and has not been independently audited**. An AI review found a high-severity flaw, which is fixed; that review is not an audit. Do not use it with real assets.
- After close, orders are revealed in anonymised form. This leaks order sizes and limits (not identities).
- The DKG assumes authenticated, private channels between nodes. The demo uses local connections.
- ZSAs are testnet-only. Swaps run on a local regtest node.

## What we are NOT building

- Mainnet, KYC, or a legal wrapper.
- Our own zero-knowledge circuits (backed-order proofs and eligibility proofs are out of scope; see the stretch list).
- A bridge, NEAR Intents, or a general exchange UI.
- Real multi-party operation. Three nodes under one operator is a demonstration.

## What is already proven (spikes, 2026-10-07 and 08)

- `zcash_tx_tool` builds in WSL and talks to the public ZSA testnet (`rpc.test-zsa.org`).
- Issue, shielded transfer, burn and finalize all work. A second issue after finalize is rejected, which is the cap guarantee.
- **There is no native swap primitive on the public ZSA testnet** (checked 2026-10-08). The public node `rpc.test-zsa.org` runs transaction v6. ZIP 228 swaps need transaction v7, which QEDIT has not shipped (their open issue "move ZSA to transaction v7", updated 2026-10-07). QEDIT's separate "swaps testnet" host still resolves but sits at block 0, so it is dead. The public testnet was also **reset on Sep 10 and restarted on Sep 25**, so it can be wiped again.
- **G0 route 1 PASSED (2026-10-08).** QEDIT's `zsa-swap` tx-tool against a locally built `zsa-swap-multiple-ag` Zebra regtest node ran `test-asset-swaps` to completion (exit 0). Issuer gave 10 of asset A and Alice gave 6 of asset B in **one transaction** built by a third matcher account, with a spread of 1 unit of each asset kept as the matcher's fee. Balances after the swap: asset A issuer 990 and Alice 9; asset B issuer 5 and Alice 4. Follow-up transfers also worked. The node validated the transaction (the block was accepted). Bundles of more than two orders and the matcher's fee balance were tested next (below); partial fills are still untested. Setup notes: Zebra image built in WSL Docker (about 15 minutes), tx-tool built with Rust 1.85 (6 minutes), Sapling params downloaded (52 MB), wallet DB created by applying `migrations/wallet_init/up.sql`, and the node must be restarted for a fresh chain before each scenario run.
- **G0 extended tests PASSED (2026-10-08).** A new scenario (`test-vibru-swaps`, in our copy of the tx-tool) added a generalized batch-swap builder: any number of party legs plus a matcher group that collects fees in any assets, all in ONE transaction, with a per-asset balance assertion. Assets were ACME (equity) and vUSD (quote); accounts 0 issuer, 1-2 sellers, 3-4 buyers, 5 matcher. Every step's balances were checked against expectations by the tool's own checker.
  - **Test 1, a whole round in one transaction:** 4 legs at a uniform price of 5. Sellers sold 10 and 4 ACME, buyers bought 8 and 6 ACME. Sellers got 49 and 19 vUSD, buyers paid 40 and 30, the matcher kept 2 vUSD. Final: ACME 90, 96, 8, 6 and vUSD 49, 19, 960, 970 for accounts 1-4.
  - **Test 2, a different price and odd size:** price 3, size 7, with change outputs. Seller got 19 vUSD, buyer paid 21, matcher kept 2 more vUSD (total 4).
  - **Test 3, the matcher really gets the fees:** account 5 held 2, then 4 vUSD, as expected.
  - **Test 4, fees in both assets at once:** the matcher collected 1 ACME and 1 vUSD; final matcher balance ACME 1, vUSD 5.
  - **Separate-keys flow PASSED (2026-10-08, `test-vibru-keys`).** Six independent wallets (issuer, two sellers, two buyers, matcher), each with its own random seed and its own database file. Each party built ONLY its own action group using only its own wallet. The matcher wallet, holding none of the parties' keys or assets, only assembled the finished groups into one transaction, which the node accepted. Balances matched exactly (sellers 90 and 96 ACME, buyers 8 and 6 ACME; matcher held only its 2 vUSD fee).
    - **Attack A blocked:** the matcher tried to build a group spending ACME it does not hold. The tool refused ("insufficient inputs: required 10 but found 0").
    - **Attack B blocked by the node:** the matcher took a fee of 2 vUSD where the parties had agreed 1. The node **rejected the block**, and nothing moved.
    - **Recovery:** the honest version of that round (fee 1) then settled normally.
  - **Caveats on the separate-keys test** (caveat (a) was closed by G2 on 2026-10-09: parts now move between processes as bytes): (a) the hand-off of finished groups and their binding-key material to the matcher happens inside one process; I did not find ready-made byte serialization for action groups in the pinned libraries, so a networked version needs a serializer we write or the library gains. (b) Handing the matcher each group's binding key means it can work out each group's **net amounts**, which it already knows from the order terms; fine for our trust model, but say so in the README. (c) Parties' groups were built against the same chain tip; groups built at different tips (different anchors) are allowed by the design but untested.
  - **Still untested:** how many legs fit in one transaction (size and time limits); partial fills with unmatched residuals; proving time grows with legs (2 legs about 20 s, 4 legs about 36 s on this machine), and each investor proves their own group on their own device.
- Two routes to atomic swaps exist:
  1. **QEDIT's tx-tool branch `zsa-swap`** (action groups, matcher bundle). Needs a custom local Zebra node (branch `zsa-swap-multiple-ag`) in Docker. Prototype. **Run and passed (above); this is the chosen route.**
  2. **Cachet's pairwise swap** (github.com/cachet-zec/cachet, MIT, merged 2026-10-07; design in its `docs/adr/004-atomic-swaps.md`). Both sides' spends and outputs go in one v6 OrchardZSA bundle, so it is atomic by construction. It uses a patched, vendored `orchard`. It claims to run on local regtest and the public testnet, but I found no on-chain evidence. It is pairwise: the maker parks units in a swap slot and publishes an offer, the taker builds and proves the whole transaction, and the maker countersigns. **Backup only; not run.**

## G1 progress: `kalyptra-tse` (built 2026-10-08 and 09, reviewed 2026-10-09; gate is Oct 16)

`crates/kalyptra-tse` is the threshold-sealed-envelope library, standalone and independent of Zcash. **L2 is implemented directly.**

**What it does**
- **Key generation:** FROST distributed key generation on ristretto255, run node by node over byte messages (`Participant`), so no one ever holds the group secret. A node with a bad round 1 proof or a bad round 2 share is named (`DkgBlame`); a message impersonating this node is rejected.
- **Sealing:** hybrid encryption to the group key (HKDF-SHA256, then ChaCha20-Poly1305, with the round id as authenticated data), plus a **Schnorr proof that the sealer knows its ephemeral secret**, bound to the group key, the round id and the ciphertext (the TDH1 construction of Shoup and Gennaro).
- **Partial decryption:** a node checks the sealer's proof for this round before computing `s_i * R`, and attaches a Chaum-Pedersen proof (node id bound in, hedged nonce).
- **Opening:** drops invalid, unknown and duplicate partials and reports them; Lagrange combination at zero.
- **Group keys** are checked for consistency, for the identity point, and for a claimed threshold higher than the one the shares were generated with.
- **Wire formats:** 98-byte partials and `R || proof || ciphertext` envelopes, with strict parsing of points and canonical scalars.
- Test-only entry points (fixed randomness, attack simulations) exist only with the `test-vectors` feature. A downstream build without it cannot call them (checked).

**Review (2026-10-09).** A fresh AI reviewer attacked the crate; separately I reviewed it myself. **This is not an audit.**
- **HIGH, fixed:** a partial decryption depended only on the envelope's ephemeral point `R`. An attacker could copy `R` from someone else's envelope (for example from a voided round) into their own, or submit `R + k*G` for a known `k`, get the nodes to decrypt it in a later round, and open the victim's order. The reviewer confirmed both attacks with working tests on the old code. **Fix:** the sealer's proof of knowledge above; nodes refuse any envelope whose proof does not verify for the round. Tests for both attacks, for moving an envelope between rounds, and for a node that skips the check (`open` still refuses) now pass.
- **MEDIUM, fixed:** a group key could claim a higher threshold than its shares were generated with (a 2-of-5 key presented as 4-of-5 passed). **Fix:** the shares must not also fit a lower threshold.
- **LOW, fixed:** identity points accepted (as `R`, as a verifying share, as the group key); proof nonces purely from the random number generator and secret copies not wiped; test-only functions public; own-id DKG messages silently skipped.
- **LOW, covered by the fix above:** one ciphertext valid under two round ids. The sealer's proof binds one round id.
- **Accepted and documented:** a DKG blame is a local judgement (no complaint protocol, sender ids not authenticated); a failure inside FROST ends that participant's run.

**Evidence**
- **58 tests pass**, on five repeated runs, including:
  - every subset for 2-of-3 and 3-of-5;
  - t-1 shares never opening, even when an attacker skips every check;
  - the copy-`R`, shifted-`R` and round-moving attacks;
  - forged, replayed, duplicate and wrong partials;
  - exhaustive single-bit flips on partials and envelopes;
  - 4,000-case fuzz-style tests on every byte-accepting function;
  - DKG over threads with bad actors.
- **Known-answer vectors** (`tests/vectors/v3.txt`, version 3: regenerated on 2026-10-13 when the project was renamed, since every domain string changed) are re-derived by `tools/check_vectors.py` with **libsodium plus Python's standard library**, sharing no code with the crate: **47 of 47 checks pass**. The checker was shown to fail on a corrupted vector, a changed KDF salt and a changed proof domain.
- **Mutation checks:**
  - With the original five bugs injected, each one was caught.
  - With the sealer-proof check disabled, nine attack and tamper tests fail.
  - With the threshold-degree check removed, the inflated-threshold test fails.
- Clippy with warnings as errors (with and without the feature), rustfmt and rustdoc are clean. The crate builds on Rust 1.85, as its manifest claims.

**Still open**
- The matcher's round logic, which decides when partials may be released (never before close, never for a voided round).
- DKG across separate processes or machines (so far, threads in one process).
- Coverage-guided fuzzing (so far, property and exhaustive-mutation tests).
- **A human audit before any real use.**

## Matcher node progress (`crates/kalyptra-matcher`, started 2026-10-09)

The node's logic, transport-agnostic, tested with three nodes in one process (2-of-3).

**What it does**
- **Signed orders:** an Ed25519 signature by the investor, with the round id and market inside the signed order **and** in the envelope's authenticated data.
- **Bulletin board:** a node accepts and receipts an envelope only while the round is open and only if its sealer proof verifies for this round. Garbage, envelopes from other rounds and lifted ephemeral points are refused at the door.
- **Round state machine:** open, then closed, then committed once `t` nodes sign the same set of envelopes receipted by `t` nodes, or voided if that set is smaller than the round's minimum.
  - Partials are released **only after a final commit and never for a voided round**, enforced in code.
  - Signatures are checked against the roster.
  - A node that signs two different votes is reported as equivocating.
- **Clearing:** a uniform-price call auction.
  - It maximises volume, then minimises imbalance, then takes the lowest price.
  - Buyers fill by highest limit first and sellers by lowest limit first; equal limits go by position in the set; the marginal order can be partly filled.
  - An envelope whose order is invalid for the round is committed but excluded and listed.
  - `t` nodes sign the round record.
- **Public checker:** re-derives a round from published data alone. It flags:
  - bad signatures and equivocation;
  - no final commit;
  - **censorship** (an envelope with `t` receipts left out of the set);
  - **injection** (a set member without `t` receipts);
  - missing or unopenable envelopes;
  - any difference between the published record and the recomputed one (price, volume, fills, orders);
  - a record signed by fewer than `t` nodes;
  - partials or a record published for a voided round.

**Evidence:** 38 tests pass (96 across the workspace, on three repeated runs). They include:
- the G0 round clearing at 5 with volume 14, with every node reaching the same record and the checker agreeing;
- a round completing with one node offline;
- no partials before close or before the commit is final;
- voided rounds never releasing;
- a single node unable to censor;
- wrong partials rejected while the round still clears;
- equivocation detected;
- the checker catching censorship, injection, a wrong price **even when `t` nodes sign it**, too few signatures, and partials for a voided round;
- three auction properties checked against brute force over 3,000 random books.

**Mutation checks:** six safety rules were disabled one at a time, and each made a specific test fail. The six are:
- releasing before the final commit;
- committing with `t - 1` votes;
- releasing in a voided round;
- skipping the censorship check;
- skipping the price comparison;
- accepting envelopes without the round's sealer proof.

**Design decisions made here**
- Committed orders are opened publicly after the round, so the price can be checked. **Investors must therefore use a fresh signing key for every order**; an opened order then shows side, size and limit, not who placed it.
- The committed set is a set (sorted by envelope hash), not an arrival order, so nodes that saw submissions in different orders still agree.

**Known limits**
- **Diverging views at close:** solved on 2026-10-15, see "Gap closing (2026-10-15)" below. Before that, nodes with different receipts at close could leave a round stuck for good.
- **Tie-breaks can be gamed:** equal limit prices are ordered by envelope hash, so an investor can grind salts for priority. Pro-rata allocation would remove that.

## Networking progress (`crates/kalyptra-node`, 2026-10-09)

A `kalyptra` binary runs a matcher node as an HTTP server and is also the investor, operator and checker command line.

**What it does**
- **Distributed key generation between separate processes.**
  - Round 1 messages are signed broadcasts.
  - Round 2 messages hold secret shares, so each is **sealed to its recipient's key** (with the same envelope code) and signed.
  - Each node saves its share; later starts load it and skip key generation.
- **Gossip** of envelopes, receipts, commit votes, partials and record votes.
  - Every message is self-authenticating: a sealer proof, a roster signature or a partial's proof. The transport needs no authentication for these.
  - Messages that arrive early (a partial before the commit) are held and retried.
- **Rounds advance by themselves.**
  - A node that sees `t` matching commit votes commits, even if it had not closed yet.
  - It then releases its partials, clears once it can open every envelope, and signs the record.
  - Close is idempotent.
- **The client never trusts one node for the group key.** Every answering node must report the same key, and at least `t` must answer, so one lying node cannot hand an investor a key it controls.
- **Commands:** `init`, `roster`, `node`, `keys`, `open`, `submit` (fresh key per order), `close`, `wait`, `check`.

**Evidence**
- **4 HTTP tests**, each with three nodes on real localhost ports talking only over HTTP: key generation plus a full round with a clean public check; a voided round with no partials anywhere; orders for closed or unknown rounds get no receipts; a node **restarted from its saved files** keeps the same group key and takes part in the next round. They pass on repeated runs.
- **Workspace total: 102 tests.** Clippy (warnings as errors), rustfmt, rustdoc and the independent vector check are clean.
- **`scripts/demo.sh` ran with three separate OS processes (2026-10-09):**
  - key generation among the processes, with no dealer;
  - four sealed orders, each receipted by 3 of 3 nodes;
  - clearing at **5, volume 14**, and `CHECK PASSED`;
  - a second round with too few orders **voided, nothing decrypted**, and `CHECK PASSED`.

**Bug found by the tests and fixed:** `close` failed on a node that had already committed because its two peers' votes arrived first. Close is now idempotent.

**Known limits**
- **Transport:** plain HTTP with no TLS.
- **Access control:** the operator endpoints (open and close a round) are not access-controlled. This is a local demo.
- **Retry window:** gossip retries for up to 2 minutes, then gives up.
- **Restarting key generation:** a node that loses its share file cannot rejoin a key generation the others have finished. The whole group must re-run it.

## Settlement progress (G2, `settlement/`, 2026-10-09)

**Gate G2 passed:** a sealed round, cleared by three matcher processes, settles as **one atomic swap transaction** on local regtest. Every party acts in its own process with its own wallet.

**How it works** (details in `settlement/README.md`)
- **Terms come from the co-signed record.** A seller sends `fill` and receives `fill × price` vUSD; a buyer does the reverse. `kalyptra-matcher::settlement::legs` derives them and checks they balance.
- **Public orders now carry the order's one-time key** (`owner`). It names no one, since keys are fresh per order, but only its holder can authorise that order's settlement. It is part of the signed record encoding.
- **Each investor checks the round, then builds its part.** It runs the public checker itself, finds its order by its envelope hash, builds its action group for exactly the record's terms with its own wallet, signs `(record hash, order, part hash)` with the order key, and hands the part to every node.
- **Nodes accept a part only from the order's owner.** Parts signed by anyone else are refused (403).
- **The leader rotates** (`ids[round % n]`). Once every filled order has a part, it checks each part's amounts and assembles one transaction.
  - **The amount check:** the sum of a part's value commitments, less `[bsk]R`, must equal the commitment to exactly what that part should move.
  - A part that does not match is named and dropped, and its owner can hand in another. The rest are kept.
- **Parts cross processes as bytes.** This closes the G0 caveat on the networked hand-off. The format is QEDIT's in-transaction action-group layout plus the value balance and binding key; a part is about 9.3 KB.
- **Auditor view.** An investor exports a full viewing key. An auditor, with its own wallet and only that key, sees the holdings and cannot spend them.

**Evidence**
- **`scripts/demo-settle.sh`** (about 3.5 minutes):
  - wallets for an issuer, four investors and three nodes;
  - ACME issued and finalized, vUSD issued;
  - the round clears at 5, volume 14, and `CHECK PASSED`;
  - the four investors build their parts in parallel, each in its own process;
  - leader node 2 settles all four legs in **one transaction**;
  - all 14 balances match the record, and the node wallets hold 0;
  - the auditor sees alice's 90 ACME and 50 vUSD from her viewing key.
- **By hand, with real parts:** a plan claiming 60 vUSD where the parts moved 50 was refused before anything was sent, naming both parts. The honest plan then settled.
- **7 new library tests** (`crates/kalyptra-matcher/tests/settlement.rs`):
  - the legs balance;
  - only the owner can authorise a part, and the signature binds the record, the order and the exact bytes;
  - unbalanced, flipped, duplicated, unknown or overflowing fills are refused.
- **3 new HTTP tests**, with a stand-in for the wallet tool:
  - the leader waits for every part, then settles;
  - a stranger, or another order's owner, is refused;
  - a bad part is named, dropped and replaceable.
- **Workspace total: 112 tests**, passing on three runs; clippy, rustfmt, rustdoc and the vector check are clean.
- **Reproducible from scratch:** `settlement/setup.sh` cloned QEDIT's tool, applied `zcash_tx_tool-kalyptra.patch` cleanly, built it, and the demo ran on that build.

**Known limits**
- **A missing part stalls the round, visibly.** Re-clearing without a defaulting order is not built.
- **The leader learns each part's amounts** through its binding key. It knows them anyway from the public record.
- **Fees and mining:** the leader pays the transaction fee and mines the block (regtest).
- **Slow rescans:** each wallet command rescans the chain from the start. That is fine on regtest and slow on a long chain.
- **Licence:** the patch modifies QEDIT code whose repository has no LICENSE file.

## Adversarial suite progress (2026-10-09)

Every attack in the list above now has a test, mapped one by one in `ATTACKS.md`.

**Four protocol holes found and fixed.** The first three were proven with failing tests before the fix.
1. **Double-counted orders.** `t` colluding nodes could list one envelope twice in the committed set. The auction would then count that order twice, and the checker passed it. Now honest nodes refuse a set that is unsorted or repeats an envelope, and the checker flags it (`MalformedCommit`).
2. **Unsigned minimum.** The round's minimum number of orders was not signed. `t` nodes could therefore void a round that had enough orders (censoring everyone in it), and the checker could not tell. Commit votes now sign the minimum and the set's length. Honest nodes refuse a vote whose minimum or void decision is wrong. The checker flags a wrong decision (`WrongVoidDecision`) and a relabelled minimum (`ConfigMismatch`).
3. **Record-vote equivocation** was detected by nodes but not by the checker. It is now.
4. **No honest-majority rule.** With `2t <= n`, two conflicting commits could both be final. Nodes now refuse such a threshold, and the checker flags conflicting commits.

**Also added**
- **Settlement checks limits.** `legs` refuses a fill at a price worse than the order's own limit.
- **Offline verification.** `kalyptra export` saves a round (every node's view, merged, plus the group key) to one file. `kalyptra verify` re-checks it with no network and no trust in any node, and can pin the announced group key. `scripts/demo.sh` now saves round 1, **stops every node**, verifies the file offline, and shows that a copy with the price edited from 5 to 6 is rejected.
- **Randomized tampering.** 600 random tamperings of public data, and 600 forgeries signed by `t` colluding nodes, assert one invariant: **a clean check always means the honest result.** More than half of the cases are flagged, so the test is not vacuous.
- **Mutation checks.** All 23 defences in the checker, the node and settlement are load-bearing (and 21 more added on 2026-10-15, see ATTACKS.md): switching any one off fails a test. The first pass found six defences no test exercised; each now has a test that breaks exactly that rule.

**Evidence**
- **Workspace: 130 tests**, passing on three runs. Clippy (warnings as errors), rustfmt and rustdoc are clean, it builds with Rust 1.85, and the vector check passes 47/47.
- **Both demos pass after the consensus change:**
  - `scripts/demo.sh`, with offline verification;
  - `scripts/demo-settle.sh`, settled in one transaction on regtest, with every balance matching.

**Wire change:** commit votes carry `min_orders`. Nodes from before this change cannot talk to nodes after it (nothing is deployed, so nothing breaks).

**Still open**
- **CI.** Nothing runs the tests automatically yet.
- **Diverging views at close.** This limit stands: no reconciliation protocol.

## UI progress (`ui/`, `kalyptra serve`, 2026-10-12)

A light, editorial page in the style of Ztocks (off-white, thin type, monospace labels, black pill buttons, green and red only for status), with Wraith's module split. Five screens: **Rounds**, **Trade**, **List a company**, **Portfolio** and **Audit**, behind a landing page.

**How it fits together.** A web page cannot hold Zcash keys or run a wallet, so a small local program does: `kalyptra serve` (crate `kalyptra-node`, `src/gateway.rs`). It serves the page and wraps what the command line already does as JSON under `/api`.
- **Keys stay local.** Order tickets (each holds an order's one-time key) live in the gateway's folder. The gateway tests assert that no ticket key appears in anything the page receives.
- **Wallet commands run one at a time,** because each rescans the chain and two on one database would collide. Each request has its own thread, so a slow one does not block the page.
- **The page shows a result as verified only if the public checker passed.** The gateway runs the checker on every finished round.
- **Loopback only.** The gateway has no login, so it refuses any listen address that is not loopback.
- **Nodes gained `GET /v1/rounds`** (a list of their rounds), so the page can list rounds.
- **Mock mode** (`VITE_USE_MOCK=true`) runs the page on a pretend backend with the same auction rule, for work on the look. The header says "Sample data" when it is on.

**Evidence**
- **2 new gateway tests** (workspace: **132**, passing on three runs; clippy, rustfmt and rustdoc clean):
  - a whole round driven through the gateway's JSON, with the check passing, tickets showing all four orders filled at 5, a saved round verifying offline, and every refusal (zero size, bad side, unknown paths, a path-traversal ticket id, no wallet) returning the right error;
  - listing a company, balances and view key, then settling a round through the gateway, with a stand-in for the wallet tool.
- **By eye against real nodes (2026-10-12):** `kalyptra serve` on three real node processes. The page showed "Nodes 3 of 3" and the real group key. A round was opened from the page, four sealed orders were receipted 3 of 3, the round was closed from the page, and it cleared at **5, volume 14** with **Public check: Passed**.
- **Mock mode** was clicked through for all five screens, including placing an order.
- **Landing page (2026-10-12):** an animated round walk-through, a private-versus-open-book comparison, a who-sees-what trust table, the four assets, an evidence strip, the live/not-live table, a FAQ and honest limits. Looked at section by section in the browser. The animation respects reduced-motion.
- **Motion and design pass (2026-10-12):** a drifting glyph field behind the hero, an italic serif accent, a rotating word, a marquee, scroll reveals, counting numbers, a typing terminal, an animated step bar, and an orders table that decrypts into view. All switch off under reduced motion. Table in `ui/README.md`.
- **Per-node tracker on Rounds:** the gateway's `GET /api/rounds/{r}` now includes what each node did (orders receipted, whether it voted, shares released, whether it signed), computed from the nodes' published data. The test asserts every node receipted and signed and released a share for each of the four orders. Writing it showed that a node can commit without ever voting (it saw two matching votes first), so only the threshold is asserted to have voted, and the page shows the gap honestly.

**Known limits**
- **Listing a company:** now signed by the nodes, see "Gap closing (2026-10-15)". Nothing checks the company is real.
- **Settle, balances and Audit were exercised only with a stand-in wallet tool** in the tests, not yet by hand in the browser against the regtest chain. The real wallet commands are proven from the command line (`scripts/demo-settle.sh`).
- **Wallet commands are slow.** Each rescans the chain. The page shows a spinner and a note.
- **Not tested on a phone-sized screen,** and no automated browser tests yet.

## One app, run for real (2026-10-12)

**`scripts/run.sh` starts everything behind one page:**
- a fresh regtest chain;
- wallets for an issuer, four investors, three nodes and an auditor;
- ACME, BOLT and CRUX (finalized) and vUSD, with the investors funded;
- three matcher nodes;
- the gateway serving the page.

**What it needed**
- **Several wallets in one gateway.** Each request names the wallet it acts as (`?as=alice`). Each wallet has its own tickets folder and its own lock, so two investors can settle at once. The page has an **Acting as** switch and forgets a remembered name the gateway doesn't have. Tests cover the separation: one investor sees none of another's tickets and cannot settle them; an unknown wallet has no balance; a bad name is refused.
- **Nodes re-read the assets file before each settlement,** so a company listed from the page can settle.
- **One scan for all balances.** A new `balances` wallet command reads every asset in one pass. The Portfolio screen went from about 25 s to 0.3 s on a fresh chain. The patch was refreshed and still reproduces the built tool exactly from a clean checkout.
- **The page build no longer breaks across systems.** `node_modules` installed on Windows has Windows binaries, so building from WSL failed. The scripts reuse an up-to-date `ui/dist`, or build in a private copy.

**Evidence: by hand, in the browser, on the real chain**
- **Round 1, ACME/vUSD:**
  - Alice sealed a sell of 10 at 4 from the Trade screen, receipted 3 of 3. Bob, Carol and Dave placed theirs through the same API with `?as=`, and each saw only their own ticket.
  - The round was closed from the page. It cleared at **5, volume 14**, the public check passed, and the node tracker showed node 3 committing without voting.
  - Alice settled from the Settle button; the other three settled in parallel.
  - **One transaction, block 14.**
- **Portfolio:** Alice showed ACME 90, BOLT 100, CRUX 0, vUSD 50.
- **Audit:** the separate auditor wallet, given Alice's viewing key exported on the same screen, showed the same four numbers.
- **Listing:**
  - The issuer listed **NOVA** (500, capped), then sold 50 at 3 in round 2. Carol bid 50 at 4.
  - It cleared at **3, volume 50**, and **settled in one transaction, block 17.**
  - Carol's balances afterwards: ACME 8, NOVA 50, vUSD 1810 (2000 − 40 − 150).
- **Workspace: 132 tests,** passing on three runs. Clippy, rustfmt and rustdoc are clean. (149 after 2026-10-15; see "Gap closing".)

**Known limits**
- **Balance check at order time:** added on 2026-10-15 (see below). Before that an order was accepted even if the investor could not cover it.
- **One machine holds every investor's wallet.** That is a demo setup; in real use each investor runs their own gateway.
- **Linux or WSL only.** No `docker compose` yet.

## Key technical risks

1. **Which swap mechanism, and single clearing price vs fixed-term swaps.** Each swap has fixed terms, so a uniform-price auction runs in two phases: the matchers compute the price off-chain, then each matched pair settles one fixed-term swap at that price. Orders that do not confirm drop out. **Gate G0 tests both routes** (QEDIT `zsa-swap` on a local node, and Cachet's pairwise swap on local regtest) and picks the one that works. **Result (2026-10-08): QEDIT's route works for whole rounds.** A batch of several sellers and buyers at one clearing price settled in ONE transaction (see the G0 results above), so whole-round bundling is back in the plan. Cachet's route (pairwise, matcher as mailbox) is now only a backup.
2. **Threshold decryption is our own protocol.** Mitigations done: standard primitives, known-answer vectors checked by an independent libsodium implementation, property, mutation and fuzz-style tests, and an adversarial AI review whose findings are fixed. Still needed before any real use: a human audit.
3. **Schedule.** This is a lot of work for one person in 21 days. The build ladder and gates exist so there is always a shippable version.
4. **Authentication of orders.** Orders are signed with an investor identity key, separate from Zcash keys. Tying that identity to the holder of the ZSA notes is out of scope; the swap itself proves ownership at settlement.

## Architecture

| Piece | What it does | Built on |
|---|---|---|
| Node | Local Zebra regtest with swap support, in Docker | QEDIT `zsa-swap-multiple-ag` |
| Tx layer | Issue, transfer, finalize, build parts, check and assemble swaps | QEDIT `zcash_tx_tool` (`zsa-swap` branch) plus our `kalyptra-wallet` commands (`settlement/`), driven by the `kalyptra` binary |
| `kalyptra-tse` (library) | DKG wrapper, hybrid encryption, partial decryption with DLEQ proofs, combination | `frost-ristretto255`, `curve25519-dalek`, `chacha20poly1305` |
| Matcher node (x3) | Bulletin board, receipts, round state machine, threshold decrypt, clearing, co-signing | New (Rust, with the library) |
| Checker | Recomputes the price and verifies every proof and signature on the public data | New (standalone binary) |
| Auditor view | Reads holdings with a viewing key | `kalyptra-wallet view-key` / `audit` (our patch), `kalyptra audit` |
| UI | Order entry, round status, public record, checker result, auditor view | Web app |

## Adversarial test suite (a deliverable)

Each case runs misbehaving nodes and asserts the system catches them:

- **t-1 shares cannot decrypt** any order, before or after close (property test).
- A node sends a **wrong partial decryption** -> DLEQ verification fails and it is excluded; the round still completes if t honest nodes remain.
- A node **withholds its share** -> the round completes with the others if t remain; otherwise it stalls visibly, never leaks.
- A node tries to **release a share before close** -> refused by the node; and a test shows what t colluding nodes could do, so the README's claim matches reality.
- **Equivocation:** a node signs two different log heads -> detected by head-hash mismatch.
- A node **drops a receipted order** -> the receipts prove the omission.
- A node **injects an order** after the committed set -> the revealed set no longer matches.
- Wrong **clearing price** published -> the checker's recomputation fails.
- **Replay** of an old round's orders -> rejected (round id and expiry in every order).
- **Lifting an envelope into another round** (copying or shifting its ephemeral point, or replaying it under a new round id) -> nodes refuse to decrypt. Already covered by the library tests.
- Settling an order the owner **never signed**, or **worse than the owner's limit** -> the swap fails.
- **Voided round:** too few bidders -> nothing is decrypted.

These run in CI against the local regtest node where possible, and as pure logic tests where not.

## Schedule and gates

| Dates | Goal | Done when |
|---|---|---|
| Oct 7-9 | Local swap node plus `test-asset-swaps`; test two-order and multi-order bundles | **G0 (Oct 10): PASSED Oct 8** |
| Oct 10-12 | Tx-layer wrapper; scripted issue, transfer and swap flow; start the `kalyptra-tse` spec | One command runs the whole ZSA flow |
| Oct 13-16 | `kalyptra-tse` library: DKG, encryption, partial decryption with DLEQ, combination, test vectors, property tests | **G1 (Oct 16): library done and reviewed Oct 9**; the freed time moves to the matcher node |
| Oct 17-19 | Matcher node: bulletin board, receipts, round state machine, close, threshold decrypt, clearing, co-signed record; three nodes locally | A full sealed round runs end to end without settlement. **Done Oct 9, in-process and over HTTP** |
| Oct 20-21 | Settlement through ZSA swaps; auditor view | **G2 (Oct 21): PASSED Oct 9** |
| Oct 22-23 | Checker and the adversarial suite | All cases caught, tests in CI. **Done Oct 9** (CI configured 2026-10-15 in `.github/workflows/ci.yml`; not yet run on GitHub) |
| Oct 24-25 | UI and a reproducible `docker compose up` with three matcher containers | A stranger can run it from the README |
| Oct 26 | README, spec, trust and limits tables, 2-minute video | Everything in draft |
| Oct 27-28 | Buffer; repo public; final submission | Submitted before Oct 28 23:59 UTC |

## Fallbacks

- **(Not needed: G0 passed.) If swaps had not run locally by Oct 10 (G0):** the matcher quorum coordinates sequential transfers (non-atomic, disclosed), and everything else in the plan stays.
- **(Not needed so far: the L2 library passed.) If L2 is not ready by Oct 16 (G1):** ship L1 (single matcher, encrypted) plus L0 (commit-reveal) plus the checker, and state it. Keep the three-node structure for receipts and co-signing if it works.
- **If bundles are limited to two orders:** settle pair by pair at the same clearing price.
- **If the auditor view is not possible with the tool:** show holdings from the wallet's own view and state the limit.
- **If the demo cannot be made reproducible:** record the full run on video and publish the logs.

## Gap closing (2026-10-15)

Four of the known limits are closed.

- **Orders are checked against the wallet at placement.** The gateway refuses an order its wallet cannot pay for: a sell needs `size` of the base asset, a buy needs up to `size × limit` of the quote asset (the price is never above the limit). It also counts what the same investor's other orders in rounds still open (open, closed or committed) already commit, so the same funds cannot be promised twice. Not counted: orders in a round that has cleared but not yet settled. Skipped when the gateway has no wallet. Test: `an_order_the_wallet_cannot_pay_for_is_refused`.
- **A round with diverging receipts no longer sticks.** A commit vote now carries an **epoch** (signed). A node votes again, in the next epoch, only when the earlier epoch is **provably dead**: the largest group of matching votes plus every vote not yet seen is still below `t`. Votes only add up, so a dead epoch stays dead, and two different sets can never both become final (`2t > n` is already enforced). The first epoch in which `t` nodes agree decides; later epochs are capped at 16. The public checker understands epochs and flags `PrematureRevote` if a later epoch decided while an earlier one could still have won. Tests: three nodes that each saw a different third of the receipts vote three ways, then vote again and the round clears with a clean check; no re-vote while the first vote could still win; a later epoch cannot overturn an earlier commit; the epoch cap; the checker's premature-re-vote finding. The vote format changed (the epoch is signed), so commit votes from before this are not compatible.
- **Company listings are signed by the nodes.** Listing a company now asks every node to sign that this symbol means this asset id. A node refuses a symbol already listed as another asset, or an asset already listed as another symbol. `t` valid signatures make the listing count. Gateways pick up listings that `t` nodes signed elsewhere, and the settlement leader uses its own accepted listings. A listing that got fewer than `t` signatures is kept and can be retried without issuing a second token. The signature says "the node accepted this binding"; it does not say the supply is real. Test: the company-listing test now checks all three signatures verify against the roster and the conflicts are refused.
- **Phone-sized screens checked** at 375 px on every screen: no horizontal overflow. The header's tabs now scroll sideways without a scrollbar and no longer wrap. Looked at by eye in a browser pane only, not on a real phone.

- **Found by running it for real: one wallet cannot settle two orders that spend the same asset in one round.** On the regtest chain, alice sold ACME in two orders in the same round; both parts were built from her one ACME note, and the node refused the block (`DuplicateOrchardNullifier`). The gateway now refuses the second such order at placement ("a wallet can settle only one such order per round"). A buy and a sell in the same round are fine, because they spend different assets. Test: `an_order_the_wallet_cannot_pay_for_is_refused`.
- **A failed assembly can now be retried** (also 2026-10-15). Outcomes are confirmed, failed (the tool reported a rejection) or unknown (no answer from the tool). The leader retries with the parts it holds, via `POST /v1/rounds/{r}/settle/retry` or the page; safe because the chain refuses a second spend of the same notes. Test: `a_failed_or_unknown_settlement_can_be_retried_by_the_leader`.

- **The settlement outcomes checked on the real regtest chain (2026-10-15).** Confirmed: a 4-order round settled in one transaction at block 14, balances exact. Unknown: the leader's wallet tool was killed the moment it started assembling; the round showed `unknown`, balances had not moved; a retry settled it at block 16 and the balances moved exactly once. Failed: two sells from one wallet in one round, submitted from the command line past the gateway's guard; the node answered "Block rejected" (`DuplicateOrchardNullifier` in its log), the round showed `failed`, and two retries (one from the page's button) were rejected the same way with nothing moved. Also on that chain: order tickets saved owner-only before sending, a company listing signed by 3 of 3 nodes, and gateway and node failures logged as JSON lines.

**Not covered**
- The new re-vote path is proven in-process (the matcher's network of nodes). The HTTP nodes call it, but a divergence race over real HTTP is not forced by a test.
- Late receipts: a node that re-votes before a peer's receipt has reached it just joins a further epoch; the receipts only ever grow, so it converges, but there is no timer.

## Stretch goals (only after G2)

- L3: a drand timelock backup so orders decrypt at a set time even if the committee fails.
- An off-chain eligibility proof (approved investor, not sanctioned), adapted from AnchorShield's Circom circuit and verified by the matchers. It needs a trusted issuer and a better setup than one operator.
- 3-of-5 committee. (Whole-round bundling is no longer a stretch goal: G0 showed it works.)

## Decisions (2026-10-08)

1. **Demo on a local regtest node** (swaps do not run on the public testnet). Yes.
2. **Rust** for the library and the matcher nodes. Yes.
3. **Name:** Kalyptra. Yes.
4. **Internal cutoff Oct 26**, with Oct 27-28 only as buffer. Yes.

- **Renamed Vibru to Kalyptra (2026-10-13).** From the Greek *kalyptra*, a veil: what conceals. Everything was renamed, including the protocol's domain-separation strings, so the known-answer vectors moved to v3 and were re-checked independently (47 of 47). `spike/` keeps the old name as recorded evidence. Logo: the clearing point (two lines meeting at one price).

## Still to decide

1. **Licence:** MIT, chosen 2026-10-15 (`LICENSE`). Still to do: ask QEDIT to add the LICENSE file their README promises, since the patch modifies their code.
2. **Where the repo lives** (for example a private GitHub repo under `Dami904`, made public by Oct 28).
