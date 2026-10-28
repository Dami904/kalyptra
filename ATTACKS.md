# Attacks Kalyptra is tested against

Each row is something a cheater tries, who or what stops it, and the test that proves it. Paths are under `crates/`. A test name that is not a link is in the file named in the section heading.

**How to read "stops it"**
- **Prevented** means an honest node or the library refuses the move.
- **Caught** means the move can succeed against nobody honest, but the public checker flags it from published data. This is the answer when `t` nodes collude: they hold `t` signing keys, so no node can stop them, and the checker proves what they did.
- **Cannot** means the move is mathematically blocked, not just refused.

## Reading orders early (`kalyptra-tse/tests/threshold.rs`)

| Attack | Result | Test |
|---|---|---|
| `t - 1` nodes pool their shares to open an order, before or after close | Cannot | `fewer_than_t_partials_do_not_open_the_envelope`, `attacker_forcing_interpolation_with_t_minus_1_shares_gets_nothing`, `one_node_alone_cannot_open_even_with_its_own_partial` |
| A key generated with a low threshold is presented as a higher one | Prevented: the group key checks its own degree | `a_group_cannot_claim_a_higher_threshold_than_it_was_generated_with` |
| Copy a victim's envelope point into a new envelope to get it decrypted | Prevented: the sealer proof fails, so no node produces a partial | `copying_a_victims_ephemeral_point_into_a_new_envelope_gets_no_partials` |
| Shift a victim's point by a known amount | Prevented, same | `shifting_a_victims_ephemeral_point_by_a_known_amount_gets_no_partials` |
| Move an envelope into another round | Prevented: the proof binds the round | `an_envelope_cannot_be_moved_into_another_round`, `every_part_of_the_sealer_proof_is_checked` |
| A node skips the sealer check | The combiner still refuses | `even_if_a_node_skips_the_sealer_check_open_still_refuses_the_forged_envelope` |
| An envelope sealed with a zero point | Prevented | `an_envelope_sealed_with_r_zero_is_rejected` |
| Release partials before close, or before the commit is final | Prevented | `no_partials_before_close_or_before_the_commit_is_final` (`kalyptra-matcher/tests/rounds.rs`) |
| Release partials for a voided round | Prevented by nodes; caught by the checker if one does it anyway | `a_voided_round_never_releases_partials_and_the_checker_confirms_it`, `checker_catches_partials_published_for_a_voided_round` |

## Wrong decryption (`kalyptra-tse/tests/threshold.rs`, `kalyptra-matcher/tests/rounds.rs`)

| Attack | Result | Test |
|---|---|---|
| A node sends a wrong partial decryption | Prevented: its proof fails, it is dropped, the round still clears | `a_wrong_partial_is_caught_and_excluded_and_the_honest_nodes_still_open_it`, `a_node_sending_wrong_partials_is_rejected_and_the_round_still_clears` |
| A partial with a forged proof, or a replayed one under another node's name | Prevented | `a_partial_from_a_random_point_with_a_forged_proof_is_rejected`, `a_proof_cannot_be_replayed_under_another_nodes_identity` |
| One node's partial counted twice | Prevented | `duplicate_partials_do_not_count_twice` |
| A node withholds its share | The round completes if `t` honest nodes remain; otherwise it stalls | `a_cheater_can_stall_only_if_too_few_honest_nodes_remain`, `a_round_completes_with_one_node_offline` |
| Nodes see different receipts at close, so no `t` votes match | Recovered: each node votes again (next epoch) once the earlier epoch provably cannot win | `nodes_that_closed_on_different_receipts_vote_again_and_the_round_goes_through` (`rounds.rs`) |
| A node votes again early, to push a different set | Caught: the checker flags `PrematureRevote`; and a later epoch can never overturn an earlier one that reached `t` | `checker_flags_a_later_epoch_used_while_the_first_could_still_have_won`, `a_second_epoch_cannot_overturn_a_first_epoch_commit`, `a_node_does_not_vote_again_while_the_first_vote_could_still_win` |
| A node floods later-epoch votes | Prevented: votes past epoch 16 are refused | `a_vote_beyond_the_epoch_limit_is_refused` |
| Too few partials | The round stalls **visibly**, reveals nothing, and recovers when another node releases | `with_too_few_partials_the_round_stalls_visibly_and_reveals_nothing` (`kalyptra-matcher/tests/adversarial.rs`) |

## Censorship and injection (`kalyptra-matcher/tests/`)

| Attack | Result | Test |
|---|---|---|
| One node refuses to receipt an order | Prevented: the others still receipt it | `a_single_node_refusing_to_receipt_cannot_censor_an_order` |
| `t` nodes drop a receipted order from the committed set | Caught: the receipts prove it | `checker_proves_censorship_when_t_nodes_drop_a_receipted_order` |
| `t` nodes add an order that was never receipted | Caught | `checker_flags_an_order_injected_into_the_set_without_receipts` |
| `t` nodes list one order twice, to give it double weight | **Prevented** by honest nodes; caught by the checker | `a_duplicated_envelope_in_the_committed_set_is_refused_and_flagged`, `an_unsorted_committed_set_is_refused_and_flagged` |
| `t` nodes void a round that has enough orders (censoring everyone in it) | **Prevented** by honest nodes; caught by the checker | `t_colluding_nodes_voiding_a_viable_round_are_caught` |
| `t` nodes commit a lone order below the minimum, to open it | Caught | `t_colluding_nodes_committing_a_lone_order_are_caught` |
| A vote signs a lower minimum than the round has | Prevented | `a_commit_signing_a_different_minimum_is_refused` |
| A published round is given a different minimum afterwards | Caught: the signed votes carry the minimum | `a_published_round_with_a_relabelled_minimum_is_flagged` |
| Two conflicting commits each gather `t` votes | Prevented: a node refuses a threshold of half the nodes or less; caught by the checker if it happens | `two_conflicting_commits_cannot_both_be_final` |
| A node signs two different commits or records | Detected | `equivocation_on_the_commit_is_detected`, `a_node_signing_two_different_records_is_flagged` |
| An order claims another round inside, or is sealed for another | Refused at the door, or committed but excluded | `an_envelope_sealed_for_another_round_is_refused_at_the_door`, `an_order_naming_another_round_inside_is_committed_but_excluded_from_the_auction` |
| Replay a cleared round's envelope in the next round | Prevented | `a_cleared_rounds_envelope_replayed_in_the_next_round_is_refused` |
| Submit after close | Prevented | `submissions_after_close_are_refused` |
| Forged or unknown signatures, garbage envelopes | Prevented | `forged_or_unknown_signatures_are_rejected`, `garbage_is_refused_at_the_door`, `checker_flags_a_bad_receipt_signature` |

## The price (`kalyptra-matcher/tests/rounds.rs`, `auction.rs`)

| Attack | Result | Test |
|---|---|---|
| `t` nodes sign a wrong clearing price | Caught: the checker re-runs the auction | `checker_catches_a_wrong_price_even_when_t_nodes_sign_it` |
| A record signed by fewer than `t` nodes | Caught | `checker_catches_a_record_signed_by_too_few_nodes` |
| Overflow in sizes or prices | Prevented | `huge_sizes_do_not_overflow` |

## Settlement (`kalyptra-matcher/tests/settlement.rs`, `kalyptra-node/tests/http.rs`, `scripts/demo-settle.sh`)

| Attack | Result | Test |
|---|---|---|
| Settle an order the owner never authorised | Prevented: a part needs the order's one-time key | `only_the_orders_owner_can_authorise_its_part`, `only_the_orders_owner_can_hand_in_its_part` |
| Reuse an owner's signature for other bytes, another order, or another round | Prevented | `only_the_orders_owner_can_authorise_its_part`, `a_signature_is_bound_to_the_exact_record` |
| Settle with a record whose fills do not balance, repeat a fill, or name a stranger | Prevented | `records_whose_fills_do_not_balance_are_refused` |
| Settle a fill outside the order's own limit | Prevented | `a_fill_outside_its_orders_limit_is_refused` |
| An investor hands in a part that moves the wrong amounts | The leader names it, drops it, and the owner can replace it | `a_part_that_does_not_match_is_named_dropped_and_can_be_replaced`; by hand on regtest in `settlement/README.md` |
| The leader takes more than the agreed amounts | The node rejects the transaction | Spike G0, attack B (`spike/g0-swaps/`) |
| The leader spends an investor's assets | Cannot: it holds none of their keys | Spike G0, attack A |

## Orders and listings at the gateway (`kalyptra-node/tests/http.rs`)

| Attack | Result | Test |
|---|---|---|
| Place an order the wallet cannot pay for (so it is counted but can never settle) | Prevented: refused at placement, counting funds the same investor already committed to open orders | `an_order_the_wallet_cannot_pay_for_is_refused` |
| List a symbol as two different assets, or one asset under two symbols | Prevented: each node refuses the second listing | `a_company_is_listed_and_a_round_settled_through_the_gateway` |
| One node is down when a round is opened or closed | The other `t` run the round: the command succeeds when `t` nodes take it, and names any that did not | `a_round_opens_closes_and_clears_with_any_one_node_down`; 1,000-round soak (`docs/EVIDENCE.md`) |
| A round's settlement leader is down, or never learned the round | The next node in turn holds every part too, checks on the leader and takes the settlement over; the chain refuses a second spend if both assemble | `a_round_whose_leader_never_saw_it_is_settled_by_the_next_node`; on the regtest chain in `docs/EVIDENCE.md` |
| A node answers a listing with a signature from the wrong key | Not counted: every signature is checked against the roster | `a_listing_signature_that_does_not_verify_is_not_counted` |
| A web page from another site, open in the investor's browser, reads their open orders (side, size, limit) from the gateway on `127.0.0.1`, or places, closes or settles orders | Prevented: no CORS header is ever sent, and a request whose `Origin` is another site is refused (403), preflight included | `a_web_page_from_another_site_cannot_read_or_drive_the_gateway` |
| The same through DNS rebinding (another site's name pointed at `127.0.0.1`, so the browser treats it as same-origin) | Prevented: a request whose `Host` is not this machine is refused | `a_web_page_from_another_site_cannot_read_or_drive_the_gateway` |
| Every node restarts and forgets a finished round, so its funds look committed forever (found on the chain, round 24 of the 109) | Prevented: the gateway saves each round's final phase the first time it sees it, and uses that when no node remembers the round | `funds_from_a_finished_round_stay_free_after_every_node_restarts` |

## The checker itself

| Claim | Test |
|---|---|
| **A clean check always means the honest result.** Over 600 random tamperings of the published data (drop, duplicate, flip or swap receipts, votes, envelopes and partials; edit the record or the configuration), no case passes with a different result. More than half are flagged, so the test is not vacuous. | `tampering_with_public_data_never_yields_a_clean_check_of_a_different_result` |
| The same with `t` colluding nodes signing forged votes and records: 600 cases. | `t_colluding_nodes_signing_forged_votes_never_yield_a_clean_check_of_a_different_result` |
| A saved round verifies offline, and an edited price, a swapped group key, a wrong pin and stripped partials are each caught | `a_saved_round_verifies_offline_and_every_tampering_is_caught` (`kalyptra-node/tests/http.rs`) |
| Each check in the checker and the node is load-bearing: switching any one off makes a test fail | Mutation checks, see below |

## Mutation checks (2026-10-09)

A passing test only proves something if it can fail. So each defence was switched off, one at a time, and the whole matcher suite run. **All 23 are load-bearing:** removing any one makes at least one test fail.

- **Checker (14):** malformed commits, wrong void decision, relabelled minimum, conflicting commits, record-vote equivocation, censorship, injected orders, record comparison of price, fills and orders, too few record signatures, the envelope sealer proof, and partials or a record for a voided round.
- **Node (4):** malformed sets, a different minimum, a wrong void decision, a threshold of half the nodes or less.
- **Settlement (5):** order limits, balance, a fill above its order's size, an order filled twice, the owner's signature.

The first pass found **six defences no test exercised**: the record comparison of fills and of orders, the checker's own envelope-proof check (the library repeats it, so the finding was only less precise), a record published for a voided round, and two settlement rules whose test inputs broke several rules at once. Each now has a test that breaks exactly that rule.

### Second run (2026-10-15)

The defences added on 2026-10-15 were checked the same way, with `scripts/mutate-new.sh`: **all 21 are load-bearing.** They are the re-vote rule (voting again only when the earlier epoch is dead), the epoch cap, the checker's premature re-vote and second-final-commit findings, the epoch being part of what a node signs, the funds check, counting funds already committed, costing a buy as size times limit, the two listing conflicts, verifying listing signatures, the gateway refusing a non-loopback address, a damaged pending file not being read as empty, the checker catching a skipped epoch, orders being serialised, a wallet-tool crash counting as "unknown" rather than "failed", only the leader retrying a settlement, only the node's own "Block rejected" counting as "nothing moved", a rejection after an unknown attempt staying unknown, an unreadable ticket stopping new orders, and an order's ticket being kept unless every node refused it. The first pass again found gaps: three of the first eleven had no test that broke exactly that rule. Each now has one.

### Third run (2026-10-22 to 25)

The defences added while running rounds on the real chain went into the same script, which now holds **28, all load-bearing** (23 + 28 = 51 in all). The seven new ones: the gateway refusing another site's page (by `Origin`) and a request for another host name (DNS rebinding), operator commands needing `t` nodes rather than all of them, the backup taking over a round whose leader never saw it, the backup leaving a live leader alone, the most advanced settlement view winning across nodes, and the gateway remembering rounds it saw finish so their funds stay free after every node restarts.

The script now also insists that cargo really rebuilt the mutated crate before it believes a result. On a Windows drive under WSL, cargo sometimes missed the edit and tested the old build, which read as "survived"; such a run is now reported as "not rebuilt", never as a pass or a survivor.

## What is NOT stopped

- **`t` colluding nodes can read orders early.** That is the security threshold, not a bug. They can also stall or void a round; honest nodes refuse a bad void and the checker proves it, but `t` colluders still hold `t` signatures.
- **A bundle can be missing data.** Someone who hands you a file can leave receipts out. `kalyptra export` merges every node's view to avoid this; a verifier using one file must trust it has them all.
- **Salt grinding.** Equal limit prices are filled in the order of the sealed envelopes' hashes, so an investor can grind to win a tie.
- **Unfilled orders are published** (side, size, limit, one-time key) after the round, because the checker needs them. They name no one.
- **Network metadata.** Who connected, from where and when is not hidden by anything here.
- **The threshold-decryption design is our own composition** of standard parts and has had an AI review, not a human audit.
