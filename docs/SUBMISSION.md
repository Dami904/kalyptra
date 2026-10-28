# Zecathon submission (draft)

Form fields, ready to paste. The description must stay under 2000 characters (check with the
counter on the form).

- **Project name:** Kalyptra
- **Track:** Private Markets
- **Code repository:** https://github.com/Dami904/kalyptra
- **Working demo:** https://kalyptra.vercel.app
- **Demo video (2 minutes):** <https://youtu.be/tFmisu23AHM>
- **Tick:** the repository will be public by the deadline (it is public already).

## Description

Kalyptra is a sealed-bid, uniform-price auction for private company shares on Zcash Shielded Assets. The price is public; the participants are not. A dark pool you do not have to trust: no operator reports the price; anyone can recompute it.

How it works: each order is threshold-encrypted to a key split across three matcher nodes (any 2 of 3 open it). No node can read an order before the round closes, drop one that two nodes receipted, or publish a price that does not follow from the orders. When the round closes, two nodes must sign the same final set before anything is decrypted; then every order clears at one price, and a public checker recomputes the price, proofs and signatures from published data, even offline. A cleared round settles as one atomic ZSA swap: every investor builds and signs their own part in their own wallet, and the transaction moves every filled order or none.

What stays private: who placed each order (a fresh one-time key per order; nothing links it to a person or wallet), every order until the round closes, and holdings, which are shielded on chain and visible only to an auditor the investor gives a viewing key. What is public by design: after the close, each order's side, size and limit (anonymous), because that is what lets anyone verify the price.

Execution: 154 tests; 51 defences each proven needed by switching it off; 1,000 rounds in 9 situations, and 109 in a row on a real Zcash chain (83 settled, every price and balance checked, each verifiable offline); 47 known-answer vectors re-derived independently; CI green. Settlement runs on QEDIT's ZSA swap prototype: one transaction, exact balances, failed and unknown outcomes handled, safe retry.

The hosted demo runs a full round in the browser on sample data, with a guided tour. The real system runs from the repo: a setup script, then one command. Limits are documented in the repo: regtest only (ZSA swaps are not on a public network), not audited.
