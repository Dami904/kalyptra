# Threat model

Kalyptra is a local-regtest prototype. This file states who is trusted and what each key can
do, so the claims can be checked. Every attack listed as stopped has a named test in
[`ATTACKS.md`](../ATTACKS.md).

## Trusted parties and keys

| Key or role | What it can do |
|---|---|
| **Group decryption key** (split 2-of-3 across the matcher nodes) | Opens sealed orders. No node holds it whole. Any `t = 2` shares together can open every order of a round. |
| **A matcher node's decryption share** | One of the `t` pieces needed to open orders. Alone it opens nothing. |
| **A matcher node's signing key** (Ed25519, separate from its share) | Signs receipts, commit votes, record votes, and company listings. |
| **An order's one-time key** (made fresh per order, kept in the investor's gateway) | Authorises the settlement part of that one order. Nothing else. |
| **An investor's wallet** | Holds that investor's assets. Builds and spends only its own part of a swap. |
| **The settlement leader's wallet** (rotates, `ids[round % n]`; the next node in turn takes over if the leader has not started) | Pays the fee and assembles the transaction. Holds none of the investors' assets, and cannot change an amount: the node rejects an unbalanced transaction. |
| **An auditor's viewing key** (given by an investor) | Reads that investor's holdings. Cannot spend them. |
| **The gateway** (`kalyptra serve`, one per investor machine) | Holds the investor's tickets and drives their wallet. Listens on loopback only. No login. |

## If each is compromised

- **One node's share or signing key:** nothing is readable, and no price or commit can be forged,
  because both need `t = 2`. That node can withhold its vote or share (liveness), and sign a
  wrong partial, which fails its proof and is dropped.
- **Two nodes' shares (`t` colluding nodes):** they can read orders early. This is the security
  threshold, not a bug. They can also censor, void, or co-sign a wrong price, but the public
  checker flags each one from published data (censorship, injected orders, a wrong price, a
  wrongly voided round). It proves them; it does not prevent them.
- **An order's one-time key:** an attacker can hand in a settlement part for that one order. The
  part must still match the record's amounts exactly or it is rejected.
- **An investor's wallet:** that investor's assets. Nothing else.
- **The leader's wallet:** its own fee balance only.
- **The gateway machine:** the tickets on it, so the one-time keys of that investor's open
  orders, and control of the wallets it holds. There is no authentication on the gateway; it
  must stay on `127.0.0.1` on a machine the investor controls.

## Not defended, plainly

- **`t` colluding nodes can read orders early.** The demo runs all three nodes on one machine
  under one operator. It shows the mechanism; it is not real decentralisation.
- **Network metadata.** Who connected, from where and when is not hidden by anything here.
- **Orders are published after a round** (side, size, limit, a one-time key that names no one),
  because the checker needs them to recompute the price. That leaks sizes and limits.
- **Salt grinding.** Equal limit prices are filled in envelope-hash order, so an investor can
  grind a salt to win a tie.
- **The node-to-node channels are plain HTTP.** Messages are signed, but nothing is encrypted in
  transit, and the key generation assumes authenticated, private channels.
- **Company listings:** a node's signature says it accepted the symbol-to-asset binding. It does
  not say the company or its supply is real; the nodes cannot see inside a shielded issuance.
- **The threshold-decryption design is our own composition of standard parts.** It has had an AI
  review (which found one high-severity flaw, since fixed), not a human audit. Do not use it with
  real assets.

## Weakest points, most exploitable first

1. Two colluding nodes on a single operator's machine: the demo's real trust boundary is that
   operator.
2. The gateway has no login; any program on the machine that can reach its port acts as every
   investor wallet it holds. (Web pages from other sites cannot: the gateway refuses another
   site's `Origin` and any `Host` that is not this machine.)
3. A settlement in the `unknown` state is not reconciled automatically; an operator must check balances before retrying (see [`LIMITATIONS.md`](LIMITATIONS.md)).
4. Salt grinding for tie priority.

Found something else? That is the point of writing this down.
