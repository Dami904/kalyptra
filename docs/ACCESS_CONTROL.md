# Access control and proof integrity: review against the five patterns

Kalyptra has no smart contract: the chain enforces value (a ZSA swap balances or is rejected),
and the matcher nodes are the off-chain policy gate in front of that settlement step. This maps
the `onchain-access-control` skill's five patterns onto that gate. Reviewed 2026-10-15.

| Pattern | Where in Kalyptra | Status |
|---|---|---|
| **1. Default-deny** | A node accepts a settlement part only with the order owner's signature (`verify_part`); an unknown wallet name at the gateway is 404, not the default wallet; an unknown round is 404; a listing nobody signed is not listed; a node refuses a vote, receipt or listing from a key not in the roster. | Holds. One widening: a gateway request with **no** `?as=` acts as the first wallet. The gateway is loopback-only with no login, so anything that reaches it already controls every wallet it holds (see `THREAT_MODEL.md`). |
| **2. Read-then-settle gap** | A part's signature covers the hash of the exact `t`-signed round record, the order position and the part bytes (`settle_message`). The leader checks each part against the legs of that same record when it assembles. Listings are first-come and immutable, so the asset id read at assembly cannot be changed under it. | Holds. A record cannot change after `t` nodes signed it (a different one would be a second final record, which the checker flags). |
| **3. Recompute, don't trust the claim** | The leader recomputes each part's amounts from the record (value commitments and binding key) instead of trusting the owner; the node rejects an unbalanced transaction; the public checker recomputes the price, the committed set, every proof and signature from published data. Nothing is accepted because a node says so. | Holds. Tested: `a_part_that_does_not_match_is_named_dropped_and_can_be_replaced`, the forged-price tests, G0 attack B. |
| **4. Replay and expiry** | Replay: the signed message binds record hash + order + part hash, so a part cannot be reused for another order or round; the chain's nullifiers make any second spend of the same notes impossible, so a retried or replayed transaction moves nothing twice. Expiry: **none.** A part handed in late is still accepted while the round is collecting. | Replay holds. Expiry is not defended: a stale part simply fails at the chain if its notes were spent meanwhile. Recorded in `LIMITATIONS.md`. |
| **5. "Can append" separate from "can change the rules"** | Each node has one signing key that signs receipts, votes, records **and** company listings. Listings are the closest thing to policy (which asset a symbol means). There is no admin key that can change the threshold, the roster or past records: the roster is fixed at start. | Partly. Listing and routine signing share a key, so a stolen node key can sign listings as well as votes; one key alone still cannot reach `t`. A separate listing key is not built. |

## Hash-chained history

Not used. Ordering inside a round does not matter (the committed set is a sorted set, not a
sequence), and each round's record is signed by `t` nodes and saved as a self-contained bundle
that verifies offline. A chain across rounds would add tamper evidence for the *list* of rounds;
it is not built.
