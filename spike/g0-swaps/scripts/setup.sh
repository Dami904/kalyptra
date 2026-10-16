#!/usr/bin/env bash
# Reproduce the Vibru G0 swap spike from scratch.
#
# Builds QEDIT's swap-enabled Zebra regtest node (Docker) and QEDIT's zcash_tx_tool
# (zsa-swap branch) with Vibru's patch applied, and installs the Sapling parameters.
#
# Run inside Linux or WSL (tested on WSL Ubuntu 24.04). Needs: git, docker, rustup,
# python3, curl, and the packages pkg-config, libssl-dev, libsqlite3-dev.
#
# Expect roughly 15 minutes for the Zebra image and 6-11 minutes for the tx-tool.
set -euo pipefail

SPIKE_DIR=$(cd "$(dirname "$0")/.." && pwd)
WORK=${VIBRU_WORK:-$HOME/vibru-spike}

# QED-it/zebra, branch zsa-swap-multiple-ag: the commit the tx-tool's own CI pins.
ZEBRA_REF=0aef55cea41b83f17610e6ea708e59995f9e739f
# QED-it/zcash_tx_tool, branch zsa-swap (2026-09-24).
TXTOOL_REF=a344fde10418ac824bf66662bb3a39dba61b213f

mkdir -p "$WORK"
cd "$WORK"

echo "== Zebra (swap-enabled regtest node)"
if [ ! -d zebra-swap ]; then
  git clone --single-branch --branch zsa-swap-multiple-ag https://github.com/QED-it/zebra.git zebra-swap
fi
git -C zebra-swap checkout -q "$ZEBRA_REF"
docker build -f zebra-swap/testnet-single-node-deploy/dockerfile -t qedit/zebra-regtest-swap zebra-swap

echo "== zcash_tx_tool (zsa-swap) with the Vibru patch"
if [ ! -d txtool-swap ]; then
  git clone --single-branch --branch zsa-swap https://github.com/QED-it/zcash_tx_tool.git txtool-swap
fi
git -C txtool-swap checkout -q "$TXTOOL_REF"
if git -C txtool-swap apply --check "$SPIKE_DIR/patches/zcash_tx_tool-zsa-swap-vibru.patch" 2>/dev/null; then
  git -C txtool-swap apply "$SPIKE_DIR/patches/zcash_tx_tool-zsa-swap-vibru.patch"
  echo "patch applied"
else
  echo "patch already applied (or does not apply cleanly); continuing"
fi
(cd txtool-swap && cargo build --release --package zcash_tx_tool --bin zcash_tx_tool)

echo "== Sapling parameters (about 52 MB; the 726 MB Sprout file is not needed)"
mkdir -p "$HOME/.zcash-params"
fetch() {
  local f=$1 h=$2 p="$HOME/.zcash-params/$1"
  if [ -f "$p" ] && echo "$h  $p" | sha256sum -c - >/dev/null 2>&1; then echo "$f ok"; return; fi
  curl -fsSL --retry 5 -o "$p.part" "https://download.z.cash/downloads/$f"
  echo "$h  $p.part" | sha256sum -c - >/dev/null
  mv "$p.part" "$p"
  echo "$f downloaded and verified"
}
# Hashes are the ones pinned in zcash_tx_tool's zcutil/fetch-params.sh.
fetch sapling-spend.params 8e48ffd23abb3a5fd9c5589204f32d9c31285a04b78096ba40a79b75677efc13
fetch sapling-output.params 2f0ebbcbb9bb0bcffe95a397e7eba89c29eb4dde6191c339db88570e3f3fb0e4

echo "== done. Next: $SPIKE_DIR/scripts/run-test.sh test-vibru-keys"
