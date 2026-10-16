#!/usr/bin/env bash
# Run the Kalyptra page against real matcher nodes.
#
#   bash scripts/demo-ui.sh
#
# Builds the page, starts three node processes and the local gateway, then waits. Open
# http://127.0.0.1:8787 and use the app: open a round, place sealed orders, close it, and watch
# it clear. Ctrl-C stops everything.
#
# This runs the protocol only. Settling, balances, listing a company and the Audit screen need a
# Zcash wallet and the regtest node: see settlement/README.md, then pass `--tool` and
# `--wallet` to `kalyptra serve` yourself. Without them the page says so instead of faking it.
#
# Needs Rust 1.85+ and Node 20+. Ports 7101-7103 and 8787 must be free.
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=${KALYPTRA_DEMO_DIR:-$(mktemp -d)}
PIDS=()
cleanup() {
  for p in "${PIDS[@]:-}"; do kill "$p" 2>/dev/null || true; done
  wait 2>/dev/null || true
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

say "building the page and the nodes"
build_ui
(cd "$ROOT" && cargo build --release -q -p kalyptra-node)
BIN=${CARGO_TARGET_DIR:-$ROOT/target}/release/kalyptra
cd "$WORK"

say "three nodes, a 2-of-3 roster, and key generation among them"
for i in 1 2 3; do "$BIN" init --id "$i" --url "http://127.0.0.1:710$i" --dir nodes >/dev/null; done
"$BIN" roster --threshold 2 --out roster.json nodes/node-*.pub.json
for i in 1 2 3; do
  "$BIN" node --id "$i" --dir nodes --roster roster.json 2>"node-$i.log" &
  PIDS+=($!)
done
"$BIN" keys --roster roster.json

# Placeholder asset ids: with no wallet there is nothing to settle, but the markets need names.
printf '{"ACME":"%s","BOLT":"%s","CRUX":"%s","vUSD":"%s"}\n' \
  "$(printf 'aa%.0s' $(seq 32))" "$(printf 'bb%.0s' $(seq 32))" \
  "$(printf 'cc%.0s' $(seq 32))" "$(printf 'dd%.0s' $(seq 32))" > assets.json

say "the gateway, serving the page"
"$BIN" serve --roster roster.json --assets assets.json --ui "$UI_DIST" &
PIDS+=($!)
sleep 1
echo
echo "Open http://127.0.0.1:8787"
echo "Ctrl-C to stop. Work files: $WORK"
wait
