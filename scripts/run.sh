#!/usr/bin/env bash
# Run all of Kalyptra, for real, behind one page.
#
#   bash scripts/run.sh
#
# Starts, in order:
#   1. a fresh local Zcash regtest chain (QEDIT's swap-enabled Zebra, in Docker);
#   2. one wallet per party: an issuer, four investors (alice, bob, carol, dave), one per
#      matcher node, and an auditor;
#   3. the assets: ACME, BOLT and CRUX (each finalized: supply capped for good) and vUSD
#      (play money), with the investors funded;
#   4. three matcher nodes, which run key generation among themselves;
#   5. the gateway, serving the page with every investor's wallet behind an "Acting as" switch.
#
# Then open http://127.0.0.1:8787. A first round (ACME/vUSD) is already open.
# Ctrl-C stops everything, including the chain.
#
# Needs: Docker, Rust 1.85+, Node 20.19+, and QEDIT's tool built with Kalyptra's patch
# (bash settlement/setup.sh). Ports 7101-7103, 8787 and 18232 must be free.
# Startup takes 15 to 20 minutes: every wallet command rescans the chain.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=${KALYPTRA_RUN_DIR:-$HOME/kalyptra-run}
TOOL=${KALYPTRA_TOOL:-${KALYPTRA_WORK:-$HOME/kalyptra-spike}/txtool-kalyptra/target/release/zcash_tx_tool}
IMAGE=${KALYPTRA_ZEBRA_IMAGE:-qedit/zebra-regtest-swap}
PIDS=()
cleanup() {
  echo
  echo "stopping…"
  for p in "${PIDS[@]:-}"; do kill "$p" 2>/dev/null || true; done
  wait 2>/dev/null || true
  docker rm -f kalyptra-regtest >/dev/null 2>&1 || true
}
trap cleanup EXIT

# The page: reuse ui/dist if it is newer than every source file; otherwise build it in a private
# copy, so a node_modules installed for another system (Windows next to WSL) is never touched.
build_ui() {
  local dist="$ROOT/ui/dist"
  if [ -f "$dist/index.html" ] &&
    [ -z "$(find "$ROOT/ui/src" "$ROOT/ui/index.html" "$ROOT/ui/package.json" -newer "$dist/index.html" -print -quit)" ]; then
    UI_DIST=$dist
    echo "  page: using the existing build"
    return
  fi
  local b=${XDG_CACHE_HOME:-$HOME/.cache}/kalyptra-ui-build
  mkdir -p "$b"
  rm -rf "$b/src" "$b/dist"
  (cd "$ROOT/ui" && tar cf - --exclude=node_modules --exclude=dist .) | (cd "$b" && tar xf -)
  (cd "$b" && npm ci --silent && npm run build --silent)
  UI_DIST=$b/dist
  echo "  page: built in $b"
}

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
need() { command -v "$1" >/dev/null || { echo "missing: $1"; exit 1; }; }
need docker
need cargo
need npm
need python3
[ -x "$TOOL" ] || { echo "no wallet tool at $TOOL; run: bash settlement/setup.sh"; exit 1; }

# One wallet command; prints one field of its result.
wallet() {
  local field=$1
  shift
  "$TOOL" kalyptra-wallet "$@" 2>>"$WORK/wallet.log" | grep KALYPTRA_RESULT | sed 's/^.*KALYPTRA_RESULT //' |
    python3 -c "import json,sys; r=json.load(sys.stdin); assert r['ok'], r; print(r['$field'])"
}

say "building the page and the nodes"
build_ui
(cd "$ROOT" && cargo build --release -q -p kalyptra-node)
BIN=${CARGO_TARGET_DIR:-$ROOT/target}/release/kalyptra

# A fresh chain needs fresh wallets: start from an empty folder every time.
rm -rf "$WORK"
mkdir -p "$WORK"
cd "$WORK"
echo "work folder: $WORK"

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
for w in issuer alice bob carol dave node1 node2 node3 auditor; do
  wallet address init --wallet "wallets/$w" >/dev/null
  echo "  $w"
done

say "3. issue the assets and fund the investors"
ACME=$(wallet asset issue --wallet wallets/issuer --name ACME --amount 1000 --finalize)
BOLT=$(wallet asset issue --wallet wallets/issuer --name BOLT --amount 1000 --finalize)
CRUX=$(wallet asset issue --wallet wallets/issuer --name CRUX --amount 1000 --finalize)
VUSD=$(wallet asset issue --wallet wallets/issuer --name vUSD --amount 100000)
printf '{"ACME":"%s","BOLT":"%s","CRUX":"%s","vUSD":"%s"}\n' "$ACME" "$BOLT" "$CRUX" "$VUSD" > assets.json
echo "  ACME, BOLT, CRUX: 1000 each, supply capped. vUSD: 100000."
fund() {
  wallet txid send --wallet wallets/issuer --to "$(wallet address address --wallet "wallets/$1")" \
    --asset "$2" --amount "$3" >/dev/null
  echo "  $1: $3 $4"
}
fund alice "$ACME" 100 ACME
fund alice "$BOLT" 100 BOLT
fund bob "$ACME" 100 ACME
fund bob "$CRUX" 100 CRUX
fund carol "$VUSD" 2000 vUSD
fund dave "$VUSD" 2000 vUSD

say "4. three matcher nodes"
for i in 1 2 3; do "$BIN" init --id "$i" --url "http://127.0.0.1:710$i" --dir nodes >/dev/null; done
"$BIN" roster --threshold 2 --out roster.json nodes/node-*.pub.json
for i in 1 2 3; do
  "$BIN" node --id "$i" --dir nodes --roster roster.json \
    --tool "$TOOL" --wallet "wallets/node$i" --assets assets.json 2>"node-$i.log" &
  PIDS+=($!)
done
"$BIN" keys --roster roster.json

say "5. the gateway and the page"
"$BIN" serve --roster roster.json --assets assets.json --ui "$UI_DIST" --tool "$TOOL" \
  --wallet alice=wallets/alice --wallet bob=wallets/bob --wallet carol=wallets/carol \
  --wallet dave=wallets/dave --wallet issuer=wallets/issuer \
  --auditor-wallet wallets/auditor 2>gateway.log &
PIDS+=($!)
for _ in $(seq 1 30); do curl -s -m 2 http://127.0.0.1:8787/api/status >/dev/null && break; sleep 0.5; done
"$BIN" open --roster roster.json --round 1 --market ACME/vUSD --min-orders 2 >/dev/null

cat <<EOF

Kalyptra is running:  http://127.0.0.1:8787

Who holds what:
  alice   100 ACME, 100 BOLT        (a seller)
  bob     100 ACME, 100 CRUX        (a seller)
  carol   2000 vUSD                 (a buyer)
  dave    2000 vUSD                 (a buyer)
  issuer  the rest; can list a new company from the page

A first round, ACME/vUSD, is open. Try:
  1. Trade, acting as alice: sell 10 at 4.   As bob: sell 4 at 5.
     As carol: buy 8 at 6.                   As dave: buy 6 at 5.
  2. Rounds: close round 1. It clears at 5, volume 14, and the public check passes.
  3. Trade, as each investor in turn: Settle. Once all four have, one transaction settles it.
  4. Portfolio: balances moved. Audit: paste a viewing key to see holdings.

Logs: $WORK (node-*.log, gateway.log, wallet.log). Ctrl-C to stop.
EOF
wait
