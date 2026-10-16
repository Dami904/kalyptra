#!/usr/bin/env bash
# Run one swap scenario on a FRESH local regtest chain with FRESH wallet databases.
#
# Usage: run-test.sh <scenario>
#   test-asset-swaps   QEDIT's own two-party swap with a matcher
#   test-vibru-swaps   Vibru: a 4-leg auction round in one transaction, price and fee checks
#   test-vibru-keys    Vibru: six separate wallets, matcher assembles, two matcher attacks
#
# Every scenario assumes an empty chain, so the node container is (re)started first.
set -euo pipefail

SCENARIO=${1:?usage: run-test.sh <scenario>}
WORK=${VIBRU_WORK:-$HOME/vibru-spike}
TOOL="$WORK/txtool-swap"
LOG="$WORK/$SCENARIO.log"

docker rm -f zebra-swap >/dev/null 2>&1 || true
docker run -d --name zebra-swap -p 18232:18232 qedit/zebra-regtest-swap >/dev/null
for _ in $(seq 1 30); do
  sleep 2
  if curl -s -m 5 -X POST http://127.0.0.1:18232 -H 'content-type: application/json' \
       --data '{"jsonrpc":"1.0","id":"q","method":"getblockcount","params":[]}' | grep -q result; then
    echo "node up"; break
  fi
done

cd "$TOOL"
# Fresh wallet databases: the shared one, plus one per wallet for test-vibru-keys.
rm -f walletdb.sqlite vk_*.sqlite vk_*.env
python3 - <<'PY'
import sqlite3
sql = open("migrations/wallet_init/up.sql").read()
for name in ["walletdb"] + [f"vk_{n}" for n in ["issuer", "s1", "s2", "b1", "b2", "m"]]:
    db = sqlite3.connect(f"{name}.sqlite"); db.executescript(sql); db.commit()
PY

export ZCASH_NODE_ADDRESS=127.0.0.1 ZCASH_NODE_PORT=18232 ZCASH_NODE_PROTOCOL=http
set +e
./target/release/zcash_tx_tool "$SCENARIO" > "$LOG" 2>&1
RC=$?
set -e
echo "exit code: $RC (full log: $LOG)"
sed 's/\x1b\[[0-9;]*m//g' "$LOG" \
  | grep -E "=== |balance:|PASSED|BLOCKED|stopped|panicked|Message:" | tail -40
exit $RC
