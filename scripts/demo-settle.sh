#!/usr/bin/env bash
# Kalyptra settlement demo: a sealed round, cleared by three matcher processes, then settled as
# ONE atomic swap transaction on a local Zcash regtest chain with Zcash Shielded Assets.
#
#   bash scripts/demo-settle.sh
#
# 1. A fresh regtest node starts (QEDIT's swap-enabled Zebra, in Docker).
# 2. Every party gets its OWN wallet, in its own directory, used only by its own processes:
#    an issuer, four investors (alice, bob, carol, dave) and one wallet per matcher node.
# 3. The issuer issues ACME (equity, then finalized: supply capped for good) and vUSD
#    (play-money quote), and funds the investors. Matcher wallets hold nothing.
# 4. Three matcher nodes run key generation and a sealed round, as in scripts/demo.sh:
#    it clears at 5, volume 14. The public checker passes.
# 5. Each investor checks the round publicly, builds its own part of the swap for exactly the
#    record's terms (in parallel, each in its own process), signs it with its order's one-time
#    key, and hands it to the nodes.
# 6. The round's leader checks every part's amounts, assembles one transaction and mines it.
# 7. Every wallet's balance is checked against the record.
# 8. An auditor, with its own wallet and only alice's viewing key, sees alice's holdings.
#
# Needs: Docker, Rust 1.85+, and QEDIT's zcash_tx_tool built with Kalyptra's patch
# (bash settlement/setup.sh). Ports 7101-7103 and 18232 must be free.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=${KALYPTRA_DEMO_DIR:-$(mktemp -d)}
TOOL=${KALYPTRA_TOOL:-${KALYPTRA_WORK:-$HOME/kalyptra-spike}/txtool-kalyptra/target/release/zcash_tx_tool}
IMAGE=${KALYPTRA_ZEBRA_IMAGE:-qedit/zebra-regtest-swap}
PIDS=()
cleanup() {
  for p in "${PIDS[@]:-}"; do kill "$p" 2>/dev/null || true; done
  wait 2>/dev/null || true
  docker rm -f kalyptra-regtest >/dev/null 2>&1 || true
}
trap cleanup EXIT

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
[ -x "$TOOL" ] || { echo "no wallet tool at $TOOL; run settlement/setup.sh first"; exit 1; }

# One wallet command; prints the field asked for from its result line.
wallet() {
  local field=$1; shift
  "$TOOL" kalyptra-wallet "$@" 2>>"$WORK/wallet.log" | grep KALYPTRA_RESULT | sed 's/^.*KALYPTRA_RESULT //' \
    | python3 -c "import json,sys; r=json.load(sys.stdin); assert r['ok'], r; print(r['$field'])"
}

say "building"
(cd "$ROOT" && cargo build --release -q -p kalyptra-node)
BIN=${CARGO_TARGET_DIR:-$ROOT/target}/release/kalyptra
cd "$WORK"
echo "work directory: $WORK"

say "1. a fresh regtest chain"
docker rm -f kalyptra-regtest >/dev/null 2>&1 || true
docker run -d --name kalyptra-regtest -p 18232:18232 "$IMAGE" >/dev/null
export ZCASH_NODE_ADDRESS=127.0.0.1 ZCASH_NODE_PORT=18232 ZCASH_NODE_PROTOCOL=http
for _ in $(seq 1 60); do
  curl -s -m 2 -X POST http://127.0.0.1:18232 -H 'content-type: application/json' \
    --data '{"jsonrpc":"1.0","id":"q","method":"getblockcount","params":[]}' | grep -q result && break
  sleep 1
done

say "2. one wallet per party"
for w in issuer alice bob carol dave node1 node2 node3; do
  addr=$(wallet address init --wallet "wallets/$w")
  printf '  %-7s %s...\n' "$w" "${addr:0:24}"
done

say "3. issue ACME (finalized) and vUSD; fund the investors"
ACME=$(wallet asset issue --wallet wallets/issuer --name ACME --amount 1000 --finalize)
VUSD=$(wallet asset issue --wallet wallets/issuer --name vUSD --amount 100000)
printf '{"ACME": "%s", "vUSD": "%s"}\n' "$ACME" "$VUSD" > assets.json
echo "  ACME $ACME (finalized: no more can ever be issued)"
echo "  vUSD $VUSD"
fund() { wallet txid send --wallet wallets/issuer --to "$(wallet address address --wallet "wallets/$1")" --asset "$2" --amount "$3" >/dev/null; }
fund alice "$ACME" 100
fund bob "$ACME" 100
fund carol "$VUSD" 1000
fund dave "$VUSD" 1000
echo "  alice 100 ACME, bob 100 ACME, carol 1000 vUSD, dave 1000 vUSD"

say "4. three matcher nodes; a sealed round"
for i in 1 2 3; do "$BIN" init --id "$i" --url "http://127.0.0.1:710$i" --dir nodes >/dev/null; done
"$BIN" roster --threshold 2 --out roster.json nodes/node-*.pub.json
for i in 1 2 3; do
  "$BIN" node --id "$i" --dir nodes --roster roster.json \
    --tool "$TOOL" --wallet "wallets/node$i" --assets assets.json 2>"node-$i.log" &
  PIDS+=($!)
done
"$BIN" keys --roster roster.json
"$BIN" open --roster roster.json --round 1 --min-orders 2
"$BIN" submit --roster roster.json --round 1 --side sell --size 10 --limit 4 --ticket alice.ticket
"$BIN" submit --roster roster.json --round 1 --side sell --size 4 --limit 5 --ticket bob.ticket
"$BIN" submit --roster roster.json --round 1 --side buy --size 8 --limit 6 --ticket carol.ticket
"$BIN" submit --roster roster.json --round 1 --side buy --size 6 --limit 5 --ticket dave.ticket
sleep 1
"$BIN" close --roster roster.json --round 1
"$BIN" wait --roster roster.json --round 1
"$BIN" check --roster roster.json --round 1

say "5. each investor builds and hands in its own part (in parallel, own wallet, own process)"
SPIDS=()
for who in alice bob carol dave; do
  "$BIN" settle --roster roster.json --ticket "$who.ticket" --tool "$TOOL" \
    --wallet "wallets/$who" --assets assets.json 2>>"$WORK/wallet.log" | sed "s/^/  $who: /" &
  SPIDS+=($!)
done
for p in "${SPIDS[@]}"; do wait "$p"; done

say "6. the leader checks every part and settles the round in one transaction"
"$BIN" settlement --roster roster.json --round 1

say "7. balances, each read by its owner's wallet"
FAIL=0
expect() {
  local who=$1 sym=$2 id=$3 want=$4 got
  got=$(wallet balance balance --wallet "wallets/$who" --asset "$id")
  if [ "$got" = "$want" ]; then mark=ok; else mark="EXPECTED $want"; FAIL=1; fi
  printf '  %-6s %-5s %6s  %s\n' "$who" "$sym" "$got" "$mark"
}
expect alice ACME "$ACME" 90;  expect alice vUSD "$VUSD" 50
expect bob   ACME "$ACME" 96;  expect bob   vUSD "$VUSD" 20
expect carol ACME "$ACME" 8;   expect carol vUSD "$VUSD" 960
expect dave  ACME "$ACME" 6;   expect dave  vUSD "$VUSD" 970
for i in 1 2 3; do expect "node$i" ACME "$ACME" 0; expect "node$i" vUSD "$VUSD" 0; done
[ "$FAIL" = 0 ] || { echo "BALANCES DO NOT MATCH THE RECORD"; exit 1; }

say "8. an auditor, given only alice's viewing key, sees her holdings (and cannot spend them)"
"$BIN" view-key --tool "$TOOL" --wallet wallets/alice --out alice.viewkey
wallet address init --wallet wallets/auditor >/dev/null
"$BIN" audit --tool "$TOOL" --wallet wallets/auditor --view-key alice.viewkey --assets assets.json \
  | tee audit.txt
grep -Eq '^  ACME +90$' audit.txt && grep -Eq '^  vUSD +50$' audit.txt \
  || { echo "THE AUDITOR'S VIEW DOES NOT MATCH ALICE'S WALLET"; exit 1; }

say "SETTLED: one atomic transaction moved exactly what the co-signed record says"
