# Repo instructions for Claude Code

## Mission
Kalyptra is a sealed-bid, uniform-price auction for private company shares on Zcash Shielded
Assets, for the Zecathon (Private Markets track; submission due 2026-10-28 23:59 UTC, internal
cutoff Oct 26). Orders are threshold-encrypted to a key split across three matcher nodes
(2 of 3), a public checker recomputes the price from published data, and a cleared round
settles as one atomic swap on a local regtest chain.

**The invariant that must never break: no order is readable by anyone before `t` nodes have
signed the final committed set, and no price is accepted that the public checker cannot
reproduce from published data.**

## Source of truth, in order
1. Behaviour you have actually reproduced (a test you ran, a regtest run, a response you logged).
2. The tests in this repo (`cargo test --workspace`), `ATTACKS.md`, and `PLAN.md`.
3. Current docs for the pinned upstreams: QEDIT's `zcash_tx_tool` (a344fde) and Zebra (0aef55c).
4. This file and the docs in `docs/`.
5. Model output and assumptions: lowest priority, check against 1-4 before shipping.

## Non-negotiable invariants
- A node releases partial decryptions only after `t` nodes signed the same final committed set,
  and never for a voided round.
- A node votes again (next epoch) only when the earlier epoch provably cannot reach `t`.
- Settlement amounts come only from the `t`-signed round record; a part that does not match is
  refused by name.
- Order tickets (each order's one-time key) never leave the gateway's home directory and are
  never sent to the page. The gateway listens on loopback only.
- Every defence ships with a test that fails when the defence is removed
  (`scripts/mutate-new.sh` proves it for the newest ones; `ATTACKS.md` lists the rest).
- No real keys or funds anywhere: regtest only.

## Engineering rules
- Write or update a failing test before changing behaviour.
- Do not suppress a failing test, lint or type error to get green.
- The wallet tool (`settlement/`) is a patch on QEDIT's code. Regenerate the patch from the
  built tree after changing it, and check it applies to a clean checkout.
- `spike/` keeps the old name "Vibru" as recorded evidence. Do not rename it.
- UI: the package manager is **npm** (`ui/package-lock.json`). The template rule "use pnpm" was
  not adopted: it would be a lockfile migration with 11 days left, for no change in behaviour.
- Do not read secrets or keystores. Never point anything at mainnet.
- Commits: the user allowed the first commit and push on 2026-10-16 (repo `Dami904/kalyptra`, private). Commit messages carry no co-author line.

## Required checks
Rust, in Linux or WSL (run via WSL on Windows; `CARGO_TARGET_DIR=$HOME/vibru-target`):
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
python3 tools/check_vectors.py      # independent re-derivation of the known-answer vectors
bash scripts/mutate-new.sh          # every newer defence still has a test that notices removal
bash scripts/guardrails.sh          # the rules below that can be checked as code
```
UI, in `ui/`:
```bash
npm ci && npm run build              # typecheck, then build
```
`.github/workflows/ci.yml` runs all of these on every push and PR. The regtest scripts
(`scripts/demo*.sh`, `scripts/run.sh`) need Docker and a 15-minute image build, so they are run
by hand, not in CI.

## Durable docs
- `docs/API_NOTES.md`: measured behaviour of the regtest node and the wallet tool.
- `docs/LIMITATIONS.md`: what is explicitly NOT handled yet.
- `docs/THREAT_MODEL.md`: who is trusted, and what happens if each key is compromised.
- `docs/ACCESS_CONTROL.md`: the settlement gate reviewed against the `onchain-access-control` patterns.
- `docs/RUNOFSHOW.md`: the scripted demo, with what to do when a step fails.
- `PLAN.md` (design, progress, decisions) and `ATTACKS.md` (every attack, who stops it, which test).

## Review gates
After a change, before calling it done, run the relevant subagent:
- failure handling, retries, state, logging: `reliability-auditor`
- CI, scripts, packaging, repo structure, onboarding: `dx-auditor`
- permissions, policy gates, anchoring proofs: consult the `onchain-access-control` skill first
