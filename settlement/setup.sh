#!/usr/bin/env bash
# Build what Kalyptra's settlement needs: QEDIT's swap-enabled Zebra regtest node (Docker) and
# QEDIT's zcash_tx_tool with Kalyptra's patch (the `kalyptra-wallet` commands).
#
# Run inside Linux or WSL (tested on WSL Ubuntu 24.04). Needs: git, docker, rustup, python3,
# curl, and the packages pkg-config, libssl-dev, libsqlite3-dev.
#
# Expect roughly 15 minutes for the Zebra image (skipped if it exists) and 6-11 minutes for
# the tool. Then: bash scripts/demo-settle.sh
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
WORK=${KALYPTRA_WORK:-$HOME/kalyptra-spike}
PATCH="$HERE/zcash_tx_tool-kalyptra.patch"

# QED-it/zebra, branch zsa-swap-multiple-ag: the commit the tx-tool's own CI pins.
ZEBRA_REF=0aef55cea41b83f17610e6ea708e59995f9e739f
# QED-it/zcash_tx_tool, branch zsa-swap (2026-09-24).
TXTOOL_REF=a344fde10418ac824bf66662bb3a39dba61b213f
IMAGE=qedit/zebra-regtest-swap

mkdir -p "$WORK"
cd "$WORK"

echo "== Zebra (swap-enabled regtest node)"
if docker image inspect "$IMAGE" >/dev/null 2>&1; then
  echo "image $IMAGE exists"
else
  if [ ! -d zebra-swap ]; then
    git clone --single-branch --branch zsa-swap-multiple-ag https://github.com/QED-it/zebra.git zebra-swap
  fi
  git -C zebra-swap checkout -q "$ZEBRA_REF"
  docker build -f zebra-swap/testnet-single-node-deploy/dockerfile -t "$IMAGE" zebra-swap
fi

echo "== zcash_tx_tool (zsa-swap at ${TXTOOL_REF:0:7}) with Kalyptra's patch, in $WORK/txtool-kalyptra"
if [ ! -d txtool-kalyptra ]; then
  git clone --single-branch --branch zsa-swap https://github.com/QED-it/zcash_tx_tool.git txtool-kalyptra
fi
git -C txtool-kalyptra checkout -q "$TXTOOL_REF"
if git -C txtool-kalyptra apply --check "$PATCH" 2>/dev/null; then
  git -C txtool-kalyptra apply "$PATCH"
  echo "patch applied"
elif git -C txtool-kalyptra apply --reverse --check "$PATCH" 2>/dev/null; then
  echo "patch already applied"
else
  echo "the patch does not apply to $WORK/txtool-kalyptra; remove that directory and re-run"
  exit 1
fi
(cd txtool-kalyptra && cargo build --release --package zcash_tx_tool --bin zcash_tx_tool)

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

echo "== done. Wallet tool: $WORK/txtool-kalyptra/target/release/zcash_tx_tool"
echo "   Next: bash scripts/demo-settle.sh"
