#!/usr/bin/env bash
# Kalyptra demo: three matcher node PROCESSES on localhost run a sealed-bid round end to end.
#
#   bash scripts/demo.sh
#
# 1. Each node creates its identity; a 2-of-3 roster is assembled.
# 2. The nodes start and run distributed key generation among themselves (no dealer).
# 3. Round 1 opens for ACME/vUSD. Four investors each sign with a FRESH key, seal their order
#    to the group key, and submit it to every node.
# 4. The round closes. Nodes commit, release partial decryptions, clear the auction and sign
#    the record. Expected: clearing price 5, volume 14.
# 5. The public checker re-verifies everything from public data, trusting no node. The round
#    is then saved to a file, every node is stopped, and the file is verified OFFLINE; a copy
#    with the price edited is rejected.
# 6. A second round with too few orders is voided: nothing in it is ever decrypted.
#
# Needs Rust (1.85+). Ports 7101-7103 must be free. Work files go to a temp directory.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=${KALYPTRA_DEMO_DIR:-$(mktemp -d)}
PIDS=()
cleanup() {
  for p in "${PIDS[@]:-}"; do kill "$p" 2>/dev/null || true; done
  wait 2>/dev/null || true
}
trap cleanup EXIT

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }

say "building"
(cd "$ROOT" && cargo build --release -q -p kalyptra-node)
BIN=${CARGO_TARGET_DIR:-$ROOT/target}/release/kalyptra
cd "$WORK"

say "1. node identities and a 2-of-3 roster (in $WORK)"
for i in 1 2 3; do "$BIN" init --id "$i" --url "http://127.0.0.1:710$i" --dir nodes; done
"$BIN" roster --threshold 2 --out roster.json nodes/node-*.pub.json

say "2. start three node processes; they run key generation among themselves"
for i in 1 2 3; do
  "$BIN" node --id "$i" --dir nodes --roster roster.json 2>"node-$i.log" &
  PIDS+=($!)
done
"$BIN" keys --roster roster.json | tee keys.txt
GROUP_KEY=$(awk '{print $3}' keys.txt) # the key every investor seals to; auditors pin it

say "3. round 1: four investors submit sealed orders"
"$BIN" open --roster roster.json --round 1 --min-orders 2
"$BIN" submit --roster roster.json --round 1 --side sell --size 10 --limit 4
"$BIN" submit --roster roster.json --round 1 --side sell --size 4 --limit 5
"$BIN" submit --roster roster.json --round 1 --side buy --size 8 --limit 6
"$BIN" submit --roster roster.json --round 1 --side buy --size 6 --limit 5
sleep 1 # let receipts gossip between nodes before closing

say "4. close; nodes commit, open the orders together, clear and sign"
"$BIN" close --roster roster.json --round 1
"$BIN" wait --roster roster.json --round 1

say "5. anyone can check the round from public data"
"$BIN" check --roster roster.json --round 1

say "5b. save the round to a file, STOP every node, and verify the file offline"
"$BIN" export --roster roster.json --round 1 --out round-1.json
for p in "${PIDS[@]}"; do kill "$p" 2>/dev/null || true; done
wait 2>/dev/null || true
PIDS=()
"$BIN" verify --bundle round-1.json --roster roster.json --group-key "$GROUP_KEY"
echo "tampering with the saved file (price 5 -> 6) must fail:"
sed 's/"price": *5/"price":6/' round-1.json > round-1-forged.json
if "$BIN" verify --bundle round-1-forged.json --roster roster.json --group-key "$GROUP_KEY"; then
  echo "THE FORGED FILE WAS ACCEPTED"; exit 1
else
  echo "(rejected, as it should be)"
fi

say "5c. start the nodes again from their saved key shares"
for i in 1 2 3; do
  "$BIN" node --id "$i" --dir nodes --roster roster.json 2>>"node-$i.log" &
  PIDS+=($!)
done
sleep 2

say "6. round 2 has too few orders, so it is voided and never decrypted"
"$BIN" open --roster roster.json --round 2 --min-orders 3
"$BIN" submit --roster roster.json --round 2 --side buy --size 1 --limit 9
sleep 1
"$BIN" close --roster roster.json --round 2
"$BIN" wait --roster roster.json --round 2
"$BIN" check --roster roster.json --round 2

say "node logs"
tail -n 6 node-1.log node-2.log node-3.log
say "done"
