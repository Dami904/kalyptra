# Evidence: many rounds, in different situations

Written so a reviewer can check what was run, what happened, and what was fixed because of it.
Every number here comes from a run on 2026-10-21 (section 1, and the first 11 chain rounds) or 2026-10-23 (the 109 chain rounds); each section says how to repeat it.

## 1. A thousand rounds through three real node processes

Three node processes talk only over HTTP on localhost: key generation, receipts, votes,
partial decryptions, records. Each round is given one of nine situations at random, and every
result is checked three ways: against a separately written clearing (the published rules,
implemented again in the test), against the fill rules read from the record alone (nobody pays
above their limit or sells below it, bought = sold = volume, a fill never exceeds its order),
and by the public checker recomputing the round from published data. No chain is involved;
settlement is section 2.

```bash
SOAK_ROUNDS=1000 cargo test --release -p kalyptra-node --test http soak -- --ignored --nocapture
```

**1,000 rounds, 11,502 orders, 748 s. Every round as expected; the public check was clean on all 1,000.**

| Situation | Rounds | Cleared | Voided | Orders | Volume |
|---|---|---|---|---|---|
| Normal (2 to 24 orders, limits 1 to 20) | 102 | 102 | 0 | 1,336 | 16,093 |
| Many tied limits (limits 4 to 6) | 114 | 114 | 0 | 1,413 | 18,805 |
| Nothing crosses (every bid below every ask) | 114 | 114 | 0 | 1,393 | 0 |
| Too few orders (fewer than the round's minimum) | 115 | 0 | 115 | 237 | 0 |
| Random bytes sent as an extra order | 110 | 110 | 0 | 1,312 | 14,496 |
| Every order sent twice | 94 | 94 | 0 | 1,201 | 13,444 |
| One node down from before the round opens | 102 | 102 | 0 | 1,263 | 14,185 |
| One node down from after the orders, before the close | 121 | 121 | 0 | 1,679 | 19,591 |
| One node restarted from its files mid-round | 128 | 128 | 0 | 1,668 | 18,796 |

What each situation showed:

- **Voided rounds release nothing:** in all 115, no node published a single partial decryption.
- **Random bytes get no receipt** from any node, and the round clears without them.
- **An order sent twice counts once** (the record holds exactly as many orders as were sealed).
- **With any one node down, or restarted mid-round,** the other two open, receipt, close, open
  and clear the round, and the checker passes it.

## 2. Rounds on the real chain, each in a different situation

`scripts/soak_chain.py` drives the gateway exactly as the page does, against the full stack from
`scripts/run.sh` (QEDIT's ZSA regtest node in Docker, four investor wallets and the issuer, three
nodes). After every round it checks the nodes' price and volume against its own computation from
the orders it placed, and reconciles every trader's balances against what the round's signed
record says should have moved.

```bash
bash scripts/run.sh                      # terminal 1: wait for "Kalyptra is running"
python3 scripts/soak_chain.py --random 98 --evidence evidence/<date>   # terminal 2
```

### 109 rounds in a row (2026-10-23)

The 11 scenarios below, then 98 rounds each in a randomly chosen situation, built from what the
wallets held at that moment, on one fresh chain. Two new companies were listed during the run
(issued on chain, signed by all three nodes) and then traded. Full table, one line per round:
[`evidence/2026-10-23/rounds.md`](../evidence/2026-10-23/rounds.md); every round's bundle and
every settlement as the chain reported it: [`evidence/2026-10-23/`](../evidence/2026-10-23/).

| Check | Result |
|---|---|
| Rounds as expected | **109 of 109** |
| Price and volume equal to the independent computation | **109 of 109** |
| Public check clean | **109 of 109** |
| Every trader's balances reconciled after the round | **109 of 109** |
| Settled on chain, one transaction each | **83** (blocks 14 to 100); 10 voided and 16 with nothing crossing, as built |
| Settlement transactions found in their block by the chain's own node | **83 of 83** |
| Round bundles verified offline (`kalyptra verify`, no node, no chain) | **109 of 109** |
| Orders | 323 counted; 16 that the wallet could not cover refused, as built |
| Companies listed mid-run, then traded | NOVA (17 rounds), ORBT (2 rounds) |

| Situation | Rounds |
|---|---|
| Plain (2 to 4 orders, random sizes and limits) | 31 |
| An order the wallet cannot cover, refused | 15 |
| The settlement leader down, back after the parts are in | 13 |
| All orders at one limit (ties) | 9 |
| Too few orders (voided) | 10 |
| A node restarted between the orders and the close | 9 |
| A seller selling everything it holds | 7 |
| A node other than the leader down for the whole round | 7 |
| Nothing crosses | 5 |
| A partial fill of the marginal order | 1 (and many more inside the random rounds) |
| A new company's first trade | 2 |

The median round took 67 s; the whole run 3.3 hours. One round (48) took 56 minutes: the
regtest node stopped answering for 54 minutes (its own log: "chain updates have stalled, state
height has not increased for 54 minutes") and then carried on; the round then cleared and settled
as expected. Getting to a clean run took four fixes, findings 7 to 10 below: two in QEDIT's
wallet tool and regtest node (wrong balances on a missed request; a crash on a resubmitted
transaction) and two in Kalyptra's own code (a takeover that resubmitted a settled round; funds
reserved forever once nodes forgot a round).

### The first 11 rounds, walked through (2026-10-21)

**11 rounds on the fixed build, every one as expected: 9 settled in one transaction each
(blocks 14 to 22), all four investors' balances reconciled exactly after every round, and the
proof of each round kept in [`evidence/2026-10-21/`](../evidence/2026-10-21/).**

| Round | Situation | Market | Orders counted | Refused | Price / volume | Public check | Settlement | Balances reconciled |
|---|---|---|---|---|---|---|---|---|
| 1 | A plain round, four investors | ACME/vUSD | 4 | 0 | 5 / 14 | clean | block 14 | yes |
| 2 | An order the wallet cannot cover, and a second order in the same asset | BOLT/vUSD | 3 | 2 | 3 / 20 | clean | block 15 | yes |
| 3 | Too few orders | CRUX/vUSD | 1 | 0 | voided | clean | nothing opened | yes (unchanged) |
| 4 | Nothing crosses | ACME/vUSD | 2 | 0 | none / 0 | clean | nothing to settle | yes (unchanged) |
| 5 | Node 3, this round's settlement leader, down from before the round opened | ACME/vUSD | 2 | 0 | 5 / 6 | clean | block 16, by node 1 | yes |
| 6 | Node 2 restarted between the orders and the close | CRUX/vUSD | 2 | 0 | 7 / 10 | clean | block 17 | yes |
| 7 | The marginal order partly filled (dave: 4 of 7) | BOLT/vUSD | 3 | 0 | 2 / 9 | clean | block 18 | yes |
| 8 | A plain round, repeated | ACME/vUSD | 3 | 0 | 4 / 3 | clean | block 19 | yes |
| 9 | A plain round, repeated | BOLT/vUSD | 3 | 0 | 4 / 4 | clean | block 20 | yes |
| 10 | A plain round, repeated | CRUX/vUSD | 3 | 0 | 4 / 5 | clean | block 21 | yes |
| 11 | A plain round, repeated | ACME/vUSD | 3 | 0 | 4 / 6 | clean | block 22 | yes |

### The proof, kept so anyone can check it

The chain is a local regtest chain (ZSA swaps are not on a public network), so no explorer can
show these transactions. Instead, while the chain was running, `soak_chain.py --evidence` saved
public data only into [`evidence/2026-10-21/`](../evidence/2026-10-21/):

- **`round-N.bundle.json`**: everything the nodes published for the round. All 11 were then
  checked **offline**, with every node and the chain stopped:
  `kalyptra verify --bundle round-N.bundle.json --roster roster.json --group-key $(cat group-key.txt)`
  recomputes the price from the published orders and checks every receipt, vote, decryption
  share and signature. **11 of 11: verified offline.** Change one number and it fails.
- **`round-N.tx.json`**: the settlement as the chain's own node reported it
  (`getrawtransaction`, `getblock`): transaction id, block, block hash, and whether the
  transaction is in that block. **9 of 9: in their blocks.**
- **`round-N.tx.hex`**: the raw transaction. The swap is on chain; its amounts and parties are
  shielded.
- `roster.json` (node public keys), `group-key.txt` (the key the nodes announced), and
  `assets.json` (the asset ids on that chain). No private keys, tickets or investor names.

| Round | Transaction id | Block | Block hash |
|---|---|---|---|
| 1 | `a082c0f3abc7d263fc13a19babce70ce458deaba23a3d2b6fa41738a6ae3eae8` | 14 | `2934a5a45fad0c574cb171e93e36a9e7aedc27b8cf7872c06dea174245ff4258` |
| 2 | `78af9b478b062623b9f591d82fa2ed3221e431507010a7218358464a25f0e64a` | 15 | `dc85ae0d6cda849731653e6e8b5f5340bf346efd5499bf07540dc8e2508d81d0` |
| 5 | `e7af9be2185757e5985415626a1d05a6a0ff34bad6f89ed373fa4bb8a3398bd5` | 16 | `c9167573f5751b9122b009ecc10e8e0c383e2b23d286a86eb6a4f6f041810475` |
| 6 | `5852bf79f7b376a4f1d4fdec5be52699c29183c91232d7f4ad9bb73e9fa96431` | 17 | `627ad8cfd189a21d4a86bee40d6c67dcb579dfe4a2232158870aa6f3ee3235e8` |
| 7 | `d1519e30b815366fce051c0ed866adbfb2fcdb9009ea90ba1826de0295e6940c` | 18 | `01c56c52a630503a9c0935003cd1e40d4c41e2e4f4c58333b2946c697b70f0e3` |
| 8 | `25dbf84e5c4bb863f6400edbe2307d4fa933cdd6e216c295275a7616ab6aed58` | 19 | `89290aa9319b569e1784b78788668601b5109744c74e05ec8e9cd4245b3f8eda` |
| 9 | `de42146e761c82633d27f46650d537a6d549b321843331db999007b60486d7a3` | 20 | `4cb598d70d827b7031d4473c9a30a26212a14cd9179a95664794ccc666a45af1` |
| 10 | `278084ba5c5482279529b748053651d3ecb683121569c04ebfabd33c953dc86e` | 21 | `dfb98aaff135b9a25855356fe7f9f20fe797b5e498750a56b60120dff8067199` |
| 11 | `e832d2abe6274e738c646f7a340e4e64101a2c7da2dd76f4ad4de78075ad521f` | 22 | `9b71fca84debf9e08b09d1fb6d84361922aee817a2b6f5b3424bdc156029e77c` |

An earlier run of the same 11 rounds the same evening (before the evidence step existed) gave
the same results; its chain was not kept, so its transaction ids are recorded only in the node
logs and are not repeated here.

Round 5, from node 1's log:

```text
[node 1] round 5: taking the settlement over (the leader does not answer: http://127.0.0.1:7103/v1/rounds/5/settlement: Connection Failed: Connect error: Connection refused (os error 111))
[node 1] round 5: every part is in; assembling the swap
round 5: SETTLED in one transaction e7af9be2185757e5…a3398bd5 (block 16)
```

The same round on the build before the fix (finding 2 below): it cleared on nodes 1 and 2, both
parts were handed in, and settlement never started; node 3 came back without the round, and
the round stayed unsettled until the run was stopped.

## 3. The real page, clicked through in a real browser

Sections 1 and 2 drive the gateway's API with scripts. On 2026-10-22 the same round was also done
**by clicking the real page** (the one served by `scripts/run.sh`, not the sample-data demo) in
Chrome, against a fresh chain, as the four investors in turn ("Acting as" switch).

| Step on the page | What it showed |
|---|---|
| Trade, as alice: Sell 10 at 4 | "Sealed. Receipted by 3 of 3 nodes." |
| The same as bob (Sell 4 at 5), carol (Buy 8 at 6), dave (Buy 6 at 5) | Each: "Sealed. Receipted by 3 of 3 nodes." |
| Rounds, before closing | "4 sealed"; every node "4 of 4" receipted; nothing opened |
| Close round | Cleared: **price 5, volume 14, 4 orders, public check Passed**. Node 3's "Voted" dot was hollow (it committed without voting itself; documented, and fine) |
| Trade, as each investor: Settle | alice "Filled 10 at 5", bob "Filled 4 at 5", carol "Filled 8 at 5", dave "Filled 6 at 5"; each: "Your part is built and handed to 3 nodes" |
| Rounds, settlement | "Settled in one transaction", `35b6c4b0c940cbc6ddb29fac6917396581b5d4b08dddd35418b16f71e7461403`, block 14: "Either every order moved, or none did" |
| Portfolio | alice ACME 90 / vUSD 50; carol ACME 8 / vUSD 1960 on the page; the gateway's balances for all four: alice 90/50, bob 96/20, carol 8/1960, dave 6/1970 (the same as every earlier run) |

This also checked that the gateway's origin check (finding 7 below) lets the page's own requests
through: every call above was made by the page from `http://127.0.0.1:8787`.

**What was not shown in a browser.** A page from another site (`https://example.com`) tried to read
alice's orders from the gateway. The gateway logged no refusal because **the request never reached
it: Chrome held it back** (recent Chrome versions stop a public website from reaching a local
address without asking). So in this browser Chrome's own protection came first; the gateway's
check is proven by `a_web_page_from_another_site_cannot_read_or_drive_the_gateway` (raw requests
with a foreign `Origin` and `Host`, including preflight) and by two mutation checks. It is the
defence for browsers without that protection, and for DNS rebinding, where the browser believes
the request is same-origin.

## 4. What was found, and what was done

| # | Found by | What happened | Cause | Done |
|---|---|---|---|---|
| 1 | Section 1, round 2 | With node 1 down, a round could not be opened, although two nodes are enough to run it. With node 3 down it opened, but the command still reported an error. | The operator commands (open, close: the command line and the page's buttons) stopped at the first node that did not answer. | **Fixed.** They now go to every node and succeed when at least `t` take them; with fewer, the error names each node that did not. Test: `a_round_opens_closes_and_clears_with_any_one_node_down`; mutation check: reverting it fails that test. |
| 2 | Section 2, round 5 | With node 3 down from before round 5 opened, the round cleared on nodes 1 and 2, every part was handed in, and settlement never started. Node 3 came back without the round. **The round could never settle** (nothing moved, so nobody lost funds, but the trade was stuck). | Settlement is assembled by one leader per round, `ids[round % 3]`; node 3 led round 5 and never learned the round. | **Fixed.** The next node in turn (the backup) holds every part too; once it has them all, it waits (20 s) and asks the leader. If the leader does not answer, does not have the round, or has not started, the backup takes the settlement over. The gateway and client ask whichever node is settling. Two nodes assembling the same round cannot both land: the chain refuses to spend the same notes twice. Test: `a_round_whose_leader_never_saw_it_is_settled_by_the_next_node`; mutation check: removing the takeover fails it. Rerun on the chain: section 2. |
| 3 | Section 1, round 27 | The test's own reference price disagreed with the nodes (19 against 13, same volume). | **A mistake in the test, not in Kalyptra.** The first reference also tried prices between the orders' limits. Between two limits, demand is the upper limit's and supply the lower one's, so an in-between price can tie on volume with less imbalance. The published rule clears only at a stated limit. | Reference corrected to the published rule; the explanation is kept beside it in the test. |
| 4 | The full test run | One older test (`nodes_running_in_separate_threads…`, key generation over channels) hung for 11 minutes at 0 % CPU. | **A race in the test:** a fast node's round-2 message could reach a slow node still collecting round 1, which treated it as an error; the other threads then waited forever. | The test now keeps an early round-2 message for later, as a real node does, and gives up after 30 s instead of hanging. Passed 300 runs in a row. |
| 5 | The full test run, after finding 1's fix | The new test for finding 1 failed once: its round never finished. | **A timing gap in the test.** It closed the round straight after sending two orders, with a node down; the two live nodes could close on different receipts, and with the third node away neither can prove the other vote dead, so the round waits for it. That wait is a documented limit (LIMITATIONS: "A round can stay closed"); a real round closes after its trading period, when every order has long reached every node. | The test now waits until both orders are on both live nodes before closing, as the 1,000-round soak does. Passed 5 runs in a row. |
| 6 | The reliability review of fix 2 | If the leader and its backup both assembled (a slow leader), the chain landed one transaction and the other node reported `failed`; the page could show that `failed` for a round that had settled, and send a retry to the wrong node. | The page took the first status that was not `collecting`, leader first. | **Fixed.** The most advanced status wins across nodes (settled, then unknown, failed, assembling, collecting); a retry is refused once anything settled; every takeover is logged with its reason. Unit test `a_settled_view_wins_over_a_losing_nodes_failed_one`; the settlement test now also checks a live leader is never taken over; both have mutation checks. |
| 7 | Reading a similar project's code (Umbra, a Canton dark pool), which had closed the same hole | The local gateway answered every request with `Access-Control-Allow-Origin: *` and picked the investor from `?as=`. A web page from any site, open in the investor's own browser, could read their open orders (side, size, limit) before the close, or place, close and settle orders. | Loopback-only binding keeps other machines out, not another site's page in the same browser. | **Fixed.** No CORS header at all; a request whose `Origin` is another site, or whose `Host` is not this machine (DNS rebinding), is refused with 403, preflight included. Test `a_web_page_from_another_site_cannot_read_or_drive_the_gateway` (raw requests with a foreign `Origin` and `Host`); two mutation checks; the real page still works (section 3). |
| 8 | The first 360-round chain run, round 5 (it stopped there) | alice's order was refused: "alice holds 0 ACME", although the round-4 reconciliation had read her 90. | **In QEDIT's wallet tool, which Kalyptra patches:** every command rescans the chain, and the scan stops at the first error from the node as if it had reached the end. One missed request, and the wallet answers from part of the chain, as a success. Shown on the chain: with the node stopped, `/api/balances` answered alice as all zero with HTTP 200, and an order got the same "holds 0" refusal. | **Fixed in the patch** (`settlement/zcash_tx_tool-kalyptra.patch`): every scan must reach the node's best block; a short one is wiped and redone up to 5 times, then the command fails. On the chain afterwards, the node stopped gives HTTP 502 "the chain node does not answer, so this wallet's balances cannot be known", not zeros. Five unit tests on the decision; the first chain test of the fix also caught a byte-order quirk in the tool's client (fixed). The patch still applies to a clean QEDIT checkout. |
| 9 | The 360-round runs again: three attempts each died at round 5, with the regtest node's own log kept | The chain node crashed (exit 139): "transactions must be unique within a single chain". Block 16 held round 2's settlement a second time. | **Fix 2's takeover, together with the regtest node.** Node 3 settled round 2, then the run stopped node 3 for round 5; node 1, round 2's backup, checked at that moment, saw no answer, took over and submitted the same transaction again. This node does not reject a block holding a transaction it already has: it crashes. ("The chain refuses a second spend" is true for a different transaction over the same notes, not for the same one.) Not the device: 7.3 GB of memory free, no out-of-memory, the same round and block every time. | **Fixed in the patch:** before submitting a settlement, the wallet tool asks the node whether it already has that transaction; if so it reports the block and submits nothing. Proven on the chain by replaying a settled round twice: both times "already on chain, block 14", the same transaction id, the chain node still up, balances unchanged. The first version of the check sent the transaction id in the wrong byte order and missed; a deliberate replay caught that before any long run counted it. Unit test `a_settlement_already_in_a_block_is_found_not_resubmitted`. |
| 10 | The 109-round run, round 24 (it stopped there; rounds 1 to 23 were all as expected) | bob tried to sell all 83 CRUX he held and was refused: "bob holds 83 and has 5 already committed to open orders". His only other CRUX order was in round 3, voided long before. | Nodes keep rounds in memory (only their key share and listings are on disk). By round 24 every node had been restarted once, so none could say round 3 had ended, and the gateway's funds check counts an unanswered round as still open (when unsure, reserve). Safe, but money from finished rounds stayed locked after node restarts. | **Fixed in the gateway:** a round's final phase (cleared or voided) never changes, so the gateway saves it the first time it sees it (`finished-rounds.json` in its home) and uses that when no node remembers the round. Test `funds_from_a_finished_round_stay_free_after_every_node_restarts` (reproduces it: 30 in a voided round, all three nodes restarted, then 40 of 42 sold); a mutation check proves the test needs the fix. |
| 11 | The mutation check after the 109-round run | One mutation survived once: with the 2-of-3 rule for operator commands removed, its test still passed. Run by hand, the same mutation was caught. | **A timing gap in the test:** it stopped a node and opened the round at once, and a node that is still finishing can answer; then every node takes the command and the mutated code is never exercised. | The test now waits until the stopped node refuses requests. Passed 3 of 3 normally; with the mutation it failed 5 of 5. |
| 12 | Later full runs of the mutation script | Different mutations "survived" from one run to the next, though each was caught when applied by hand (3 of 3). | **The script, not the tests:** on a Windows drive under WSL, cargo sometimes did not see the mutated file as changed and ran the tests against the old build. | **Fixed in `scripts/mutate-new.sh`:** it builds first and insists the mutated crate was recompiled (dropping the crate's build fingerprint if needed); otherwise it reports "not rebuilt" instead of a result. 28 of 28 caught, locally and in CI. |
| 13 | CI, 2026-10-25 | `a_round_opens_closes_and_clears_with_any_one_node_down` timed out on GitHub's runner: the round never became final. Reproduced locally in 3 of 78 full-suite runs on two pinned cores. | **The test closed too early:** it waited until both live nodes had the orders, but a node votes on orders `t` nodes have *receipted*; under load a receipt was still in flight, the two live nodes voted on different sets, and with the third node down neither could prove the vote dead (the limit in `LIMITATIONS.md`). | The test now also waits for the receipts. 0 failures in 60 full-suite runs under the same load; CI green. |
| 14 | A clean machine, 2026-10-28: a new WSL Ubuntu 24.04 with nothing on it, following only the docs' "Run the real system" guide | Every step worked as written: the apt line, Rust, Node, the clone, `setup.sh`, `run.sh`, the first round on the page (price 5, volume 14, check passed), each investor's Settle (alice 90 ACME and +50 vUSD, bob 96 and +20, carol 8 ACME and -40, dave 6 and -30), then `kalyptra check`, `export` and `verify` (all passed). Three things in the guide were wrong or missing: apt can be locked for minutes on a fresh Ubuntu; a 184 MB Docker download failed twice mid-way; and `setup.sh` took 67 minutes (42 of them compiling Zebra), with an 8 GB image, against the guide's 20-30 minutes and 6 GB. | The guide was written on a machine that already had the Docker layers and build caches. | The guide now says so: an apt-lock note, a troubleshooting row (run `setup.sh` again, or `docker pull rust:1.81.0` first), 30-70 minutes and about 8 GB. |

Still open, and stated in [`LIMITATIONS.md`](LIMITATIONS.md): if a round's leader **and** its
backup are both unavailable, its settlement waits until one of them is back with the round. A
node that was down when a round opened does not catch up on it; the other two run it.
