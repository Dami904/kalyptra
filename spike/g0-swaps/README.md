# G0 spike: can Vibru settle a sealed auction round as a real atomic swap?

> **Name note.** The project was called **Vibru** when this spike ran (2026-10-07 and 08), and was renamed **Kalyptra** on 2026-10-13. This folder is recorded evidence, so its logs, patch, scripts and command names (`test-vibru-keys`, `test-vibru-swaps`) are kept exactly as they were. The settlement code that grew out of it lives in `settlement/`, under the new name.

**Answer: yes, on a local regtest node.** Run 2026-10-07 and 2026-10-08 on WSL Ubuntu 24.04.

This folder is evidence and a way to reproduce it. It is not Vibru's product code.

## What was tested

| # | Scenario | Network | Result | Log |
|---|---|---|---|---|
| 1 | `test-issue-one`: issue one ZSA | Public ZSA testnet (`rpc.test-zsa.org`) | Passed: balance 0 to 1 | `logs/01-public-testnet-issue-one.log` |
| 2 | `test-asset-swaps` (QEDIT's own): two parties swap two assets through a matcher | Local regtest | Passed | `logs/02-local-qedit-asset-swaps.log` |
| 3 | `test-vibru-swaps` (ours): a 4-leg auction round in **one transaction**; a different price and size; the matcher's fee; fees in both assets | Local regtest | Passed | `logs/03-local-vibru-batch-swaps.log` |
| 4 | `test-vibru-keys` (ours): six **separate wallets**, each building only its own part; the matcher assembles; two matcher attacks | Local regtest | Passed, both attacks blocked | `logs/04-local-vibru-separate-keys.log` |

`test-orchard-zsa` (issue, transfer, burn, finalize, then an issue after finalize is rejected) was also run on the public testnet, by hand. Its log is not saved here.

### Test 4 in detail (the one that matters most)

- **Setup:** issuer, two sellers, two buyers and a matcher. Each has its own random seed and its own database file.
- **Round:** each party built only its own action group with only its own wallet. The matcher wallet, holding none of their keys or assets, assembled one transaction. The node accepted it, and every balance matched.
- **Attack A:** the matcher tried to spend a seller's ACME. Refused while building ("insufficient inputs").
- **Attack B:** the matcher took a fee of 2 vUSD where the parties had signed for 1. **The node rejected the block.** Nothing moved.
- **Recovery:** the honest version of that round then settled.

## What is NOT proven yet

- **Networked hand-off.** Groups were handed to the matcher inside one process. No ready-made byte serialization for action groups was found in the pinned libraries.
- **Binding keys.** The matcher receives each group's binding signing key, so it can work out each group's net amounts. It knows them anyway from the order terms.
- **Limits:** how many legs fit in one transaction, and partial fills.
- **Speed:** proving time. About 20 s for 2 legs and 36 s for 4 legs on this machine.
- **Public network.** Swaps do not run on the public testnet: there, ZIP 228 needs transaction v7, which QEDIT has not shipped. Swaps here use QEDIT's prototype node branch.

## Reproduce

Inside Linux or WSL:

```bash
bash spike/g0-swaps/scripts/setup.sh
```

```bash
bash spike/g0-swaps/scripts/run-test.sh test-vibru-keys
```

`setup.sh` clones QEDIT's Zebra (`zsa-swap-multiple-ag` at `0aef55c`) and `zcash_tx_tool` (`zsa-swap` at `a344fde`), applies `patches/zcash_tx_tool-zsa-swap-vibru.patch`, builds the Docker node image and the tool, and fetches the two Sapling parameter files with pinned SHA-256 hashes.

## What the patch adds to QEDIT's tool

- `create_batch_swap_transaction`: any number of legs plus a matcher fee group in one transaction, with a per-asset balance assertion.
- `party_build_leg`, `matcher_build_fee_group`, `matcher_assemble`: the separate-keys split.
- `create_transfer_to_address`: send to another wallet's address.
- Two scenarios: `test-vibru-swaps` and `test-vibru-keys`.
- Four existing private helpers made `pub(crate)`.

## Third-party code and licence

The patch modifies and partly copies code from QEDIT's `zcash_tx_tool`. That project's README states it is MIT-licensed, but the repository ships no LICENSE file on `main` or `zsa-swap` (checked 2026-10-08). Before Vibru depends on this code, confirm the licence with QEDIT or keep our own code separate from theirs.
