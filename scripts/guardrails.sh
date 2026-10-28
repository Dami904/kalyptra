#!/usr/bin/env bash
# Repo rules that would otherwise live only as sentences in CLAUDE.md, checked as code.
# Fails (exit 1) and names the line if any rule is broken. Runs in CI and anywhere with bash.
#
#   1. Code and scripts talk only to this machine, plus a short list of download hosts used to
#      build the prototype (no live network, no mainnet endpoint, no hosted RPC). The page may
#      also link to this project's own repository.
#   2. Nothing reads a .env file or a keystore.
#   3. Nothing in the scripts or code selects Zcash mainnet (the page may say the word: it is prose).
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0
bad() {
  echo "GUARDRAIL: $1"
  echo "$2" | sed 's/^/    /'
  fail=1
}

code_files() {
  {
    git ls-files --cached --others --exclude-standard 2>/dev/null || find . -type f -not -path './.git/*'
  } | grep -E '^(\./)?(scripts|settlement|crates|ui/src|tools)/.*\.(sh|rs|ts|tsx|py)$|^(\./)?ui/vite\.config\.ts$' |
    grep -v '^scripts/guardrails\.sh$'
}

# 1. every URL is loopback, one of the hosts the build downloads from, or this project's repo.
#    Tests may also name a host under `.example` (reserved by RFC 2606: it never resolves).
#    The docs page may also link (for a reader to open, never for code to call) this project's
#    video and the two installers its setup guide names: rustup and nvm.
allowed='^https?://(127\.0\.0\.1|localhost|\[::1\]|[a-z0-9.-]+\.example)([:/]|$)|^https://(download\.z\.cash|github\.com/QED-it|github\.com/ZcashFoundation|github\.com/Dami904/kalyptra)(/|$)|^https://(youtu\.be/tFmisu23AHM|sh\.rustup\.rs|github\.com/nvm-sh/nvm)$'
urls=$(code_files | xargs grep -noE 'https?://[^ "'"'"'`)<>{}\\]+' 2>/dev/null |
  awk -F: '{ url=$0; sub(/^[^:]*:[^:]*:/, "", url); print $1":"$2" "url }' |
  while read -r where url; do
    case "$url" in
      *'${'*|*'{'*) continue ;; # built at run time from a variable (checked by rule 1 at that site)
    esac
    echo "$url" | grep -Eq "$allowed" || echo "$where $url"
  done)
[ -z "$urls" ] || bad "a URL that is not this machine or an allowed download host" "$urls"

# 2. no .env files or keystores read
envs=$(code_files | xargs grep -nE '(^|[^a-zA-Z_])\.env([^a-zA-Z_]|$)|keystore' 2>/dev/null || true)
[ -z "$envs" ] || bad "a .env file or keystore is referenced" "$envs"

# 3. no mainnet
main=$(code_files | grep -v '^ui/' | xargs grep -niE 'mainnet|--network[ =]main\b|Network::Main' 2>/dev/null || true)
[ -z "$main" ] || bad "mainnet is referenced in code or scripts" "$main"

[ "$fail" = 0 ] && echo "guardrails: all rules hold" || echo "guardrails: FAILED"
exit $fail
