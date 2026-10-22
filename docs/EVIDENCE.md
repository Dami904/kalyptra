# Evidence: many rounds, in different situations

Written so a reviewer can check what was run, what happened, and what was fixed because of it.
Every number here comes from a run on 2026-10-21; each section says how to repeat it.

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
`scripts/run.sh` (QEDIT's ZSA regtest node in Docker, four investor wallets, three nodes). After
each settled round it reconciles every investor's ACME, BOLT, CRUX and vUSD against what the
round's signed record says should have moved.

```bash
bash scripts/run.sh                      # terminal 1: wait for "Kalyptra is running"
python3 scripts/soak_chain.py --evidence evidence/<date>   # terminal 2
```

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

## 3. What the runs found, and what was done

| # | Found by | What happened | Cause | Done |
|---|---|---|---|---|
| 1 | Section 1, round 2 | With node 1 down, a round could not be opened, although two nodes are enough to run it. With node 3 down it opened, but the command still reported an error. | The operator commands (open, close: the command line and the page's buttons) stopped at the first node that did not answer. | **Fixed.** They now go to every node and succeed when at least `t` take them; with fewer, the error names each node that did not. Test: `a_round_opens_closes_and_clears_with_any_one_node_down`; mutation check: reverting it fails that test. |
| 2 | Section 2, round 5 | With node 3 down from before round 5 opened, the round cleared on nodes 1 and 2, every part was handed in, and settlement never started. Node 3 came back without the round. **The round could never settle** (nothing moved, so nobody lost funds, but the trade was stuck). | Settlement is assembled by one leader per round, `ids[round % 3]`; node 3 led round 5 and never learned the round. | **Fixed.** The next node in turn (the backup) holds every part too; once it has them all, it waits (20 s) and asks the leader. If the leader does not answer, does not have the round, or has not started, the backup takes the settlement over. The gateway and client ask whichever node is settling. Two nodes assembling the same round cannot both land: the chain refuses to spend the same notes twice. Test: `a_round_whose_leader_never_saw_it_is_settled_by_the_next_node`; mutation check: removing the takeover fails it. Rerun on the chain: section 2. |
| 3 | Section 1, round 27 | The test's own reference price disagreed with the nodes (19 against 13, same volume). | **A mistake in the test, not in Kalyptra.** The first reference also tried prices between the orders' limits. Between two limits, demand is the upper limit's and supply the lower one's, so an in-between price can tie on volume with less imbalance. The published rule clears only at a stated limit. | Reference corrected to the published rule; the explanation is kept beside it in the test. |
| 4 | The full test run | One older test (`nodes_running_in_separate_threads…`, key generation over channels) hung for 11 minutes at 0 % CPU. | **A race in the test:** a fast node's round-2 message could reach a slow node still collecting round 1, which treated it as an error; the other threads then waited forever. | The test now keeps an early round-2 message for later, as a real node does, and gives up after 30 s instead of hanging. Passed 300 runs in a row. |
| 5 | The full test run, after finding 1's fix | The new test for finding 1 failed once: its round never finished. | **A timing gap in the test.** It closed the round straight after sending two orders, with a node down; the two live nodes could close on different receipts, and with the third node away neither can prove the other vote dead, so the round waits for it. That wait is a documented limit (LIMITATIONS: "A round can stay closed"); a real round closes after its trading period, when every order has long reached every node. | The test now waits until both orders are on both live nodes before closing, as the 1,000-round soak does. Passed 5 runs in a row. |
| 6 | The reliability review of fix 2 | If the leader and its backup both assembled (a slow leader), the chain landed one transaction and the other node reported `failed`; the page could show that `failed` for a round that had settled, and send a retry to the wrong node. | The page took the first status that was not `collecting`, leader first. | **Fixed.** The most advanced status wins across nodes (settled, then unknown, failed, assembling, collecting); a retry is refused once anything settled; every takeover is logged with its reason. Unit test `a_settled_view_wins_over_a_losing_nodes_failed_one`; the settlement test now also checks a live leader is never taken over; both have mutation checks. |

Still open, and stated in [`LIMITATIONS.md`](LIMITATIONS.md): if a round's leader **and** its
backup are both unavailable, its settlement waits until one of them is back with the round. A
node that was down when a round opened does not catch up on it; the other two run it.
