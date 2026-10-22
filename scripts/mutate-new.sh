#!/usr/bin/env bash
# Switch off each of the defences added on 2026-10-15, one at a time, and check that a test
# fails. A defence whose removal changes no test result is not really tested.
#
# Run inside Linux or WSL from anywhere. It edits the sources in place and restores them, so do
# not run it while editing those files.
set -uo pipefail
source "$HOME/.cargo/env" 2>/dev/null || true
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$HOME/vibru-target}

fail=0
# mutate <name> <file> <perl substitution> <cargo test args...>
mutate() {
  local name=$1 file=$2 sub=$3
  shift 3
  cp "$file" "$file.orig"
  perl -0pi -e "$sub" "$file"
  if cmp -s "$file" "$file.orig"; then
    echo "NOT APPLIED  $name (the pattern did not match)"
    mv "$file.orig" "$file"
    fail=1
    return
  fi
  if cargo test "$@" >/tmp/mutate.out 2>&1; then
    echo "SURVIVED     $name  <-- no test noticed"
    fail=1
  elif grep -q "error\[" /tmp/mutate.out; then
    echo "DID NOT COMPILE $name"
    fail=1
  else
    echo "caught       $name"
  fi
  mv "$file.orig" "$file"
}

M=crates/kalyptra-matcher
N=crates/kalyptra-node

mutate "re-vote at any time (the dead-epoch test)" $M/src/node.rs \
  's/best \+ n\.saturating_sub\(seen\) < t/true/' \
  -p kalyptra-matcher --test rounds
mutate "no epoch cap on received votes" $M/src/round.rs \
  's/if self\.epoch > MAX_EPOCH \{/if false {/' \
  -p kalyptra-matcher --test rounds
mutate "checker ignores a premature re-vote" $M/src/checker.rs \
  's/if best \+ n\.saturating_sub\(seen\) >= t \{/if false {/' \
  -p kalyptra-matcher --test rounds
mutate "checker ignores a second final commit in a later epoch" $M/src/checker.rs \
  's/if tally\.iter\(\)\.any\(\|\(k, c\)\| \*c >= t && \*k != w\.key\(\)\) \{/if false {/' \
  -p kalyptra-matcher --test rounds
mutate "epoch not part of what a node signs" $M/src/round.rs \
  's/&epoch\.to_le_bytes\(\),\n//' \
  -p kalyptra-matcher --test rounds
mutate "no funds check at all" $N/src/gateway.rs \
  's/self\.check_funds\(who, round, &b\.market, side, b\.size, b\.limit\)\?;//' \
  -p kalyptra-node --test http
mutate "funds already promised are not counted" $N/src/gateway.rs \
  's/reserved = reserved\.saturating_add\(c\);//' \
  -p kalyptra-node --test http
mutate "a buy is costed as only its size, not size x limit" $N/src/gateway.rs \
  's/\.checked_mul\(limit\)/.checked_mul(1)/' \
  -p kalyptra-node --test http
mutate "a node lists one symbol as two assets" $N/src/server.rs \
  's/Some\(have\) if \*have != asset_id => \{/Some(have) if false \&\& *have != asset_id => {/' \
  -p kalyptra-node --test http
mutate "a node lists one asset under two symbols" $N/src/server.rs \
  's/if let Some\(\(other, _\)\) = self\.listings\.iter\(\)\.find\(\|\(_, a\)\| \*\*a == asset_id\)/if let Some((other, _)) = None::<(\&String, \&String)>/' \
  -p kalyptra-node --test http
mutate "the gateway counts a signature it did not verify" $N/src/gateway.rs \
  's/&& s\.verify\(&roster\)\.is_ok\(\)\n            \{\n                signers/{\n                signers/' \
  -p kalyptra-node --test http

mutate "the gateway listens on a non-loopback address" $N/src/main.rs \
  's/if !addr\.ip\(\)\.is_loopback\(\) \{/if false {/' \
  -p kalyptra-node --bin kalyptra

mutate "a damaged pending file is read as empty" $N/src/gateway.rs \
  's/read_json_or_default\(&pending_path\)\.map_err\(fail\)\?/read_json_or_default(&pending_path).unwrap_or_default()/' \
  -p kalyptra-node --test http
mutate "the checker only looks at epochs that have votes" $M/src/checker.rs \
  's/let epochs = 0\.\.=commit_votes\.keys\(\)\.map\(\|\(e, _\)\| \*e\)\.max\(\)\.unwrap_or\(0\);/let epochs: BTreeSet<u32> = commit_votes.keys().map(|(e, _)| *e).collect();/' \
  -p kalyptra-matcher --test rounds
mutate "orders are not serialised (the race)" $N/src/gateway.rs \
  's/let _one_at_a_time = self\.order_lock\.lock\(\)\.unwrap_or_else\(\|e\| e\.into_inner\(\)\);//' \
  -p kalyptra-node --test http

mutate "a crash of the wallet tool counts as a definite failure" $N/src/server.rs \
  's/entry\.status = SettleStatus::Unknown \{ error: e\.clone\(\) \};/entry.status = SettleStatus::Failed { error: e.clone(), bad: vec![] };/' \
  -p kalyptra-node --test http
mutate "any node may retry a settlement, leader or not" $N/src/server.rs \
  's/if self\.leader_of\(round\) != self\.identity\.id \{\n            return Err\(\(\n                409,/if false {\n            return Err((\n                409,/' \
  -p kalyptra-node --test http

mutate "any failure the tool reports counts as nothing moved" $N/src/wallet.rs \
  's/if error\.contains\("Block rejected"\) \{/if true {/' \
  -p kalyptra-node --test http
mutate "a rejection after an unknown attempt counts as nothing moved" $N/src/server.rs \
  's/Ok\(Assembled::Rejected\(e\)\) if entry\.was_unknown => \{/Ok(Assembled::Rejected(e)) if false => {/' \
  -p kalyptra-node --test http
mutate "an unreadable ticket is skipped" $N/src/gateway.rs \
  's/out\.push\(\(id, read_json::<TicketJ>\(&p\)\.map_err\(unreadable\)\?\)\);/if let Ok(t) = read_json::<TicketJ>(\&p) { out.push((id, t)); }/' \
  -p kalyptra-node --test http

mutate "an order's ticket is dropped when a node gave no answer" $N/src/gateway.rs \
  's/let kept = !\(sent\.receipts\.is_empty\(\) && sent\.refused == n\);/let kept = !sent.receipts.is_empty();/' \
  -p kalyptra-node --test http

mutate "a request for another host name is served (DNS rebinding)" $N/src/gateway.rs \
  's/Some\(h\) if is_loopback\(&h\) => \{\}/Some(_) => {}/' \
  -p kalyptra-node --test http a_web_page
mutate "a page from another site may call the gateway" $N/src/gateway.rs \
  's/if !local \{/if false {/' \
  -p kalyptra-node --test http a_web_page

mutate "an operator command needs every node, not t of them" $N/src/client.rs \
  's/if ok >= roster\.threshold as usize \{/if failed.is_empty() {/' \
  -p kalyptra-node --test http any_one_node_down
mutate "nobody takes a round over when its leader never saw it" $N/src/server.rs \
  's/        self\.maybe_watch_leader\(round\);\n//' \
  -p kalyptra-node --test http leader_never_saw_it
mutate "the backup takes over a leader that is alive and done" $N/src/server.rs \
  's/            Ok\(_\) => None,/            Ok(_) => Some("x".to_string()),/' \
  -p kalyptra-node --test http a_round_settles_once
mutate "the first view that is not collecting wins" $N/src/client.rs \
  's/if best\n            \.as_ref\(\)\n            \.is_none_or\(\|b\| rank\(&v\.status\) > rank\(&b\.status\)\)/if best.is_none()/' \
  -p kalyptra-node --lib
[ "$fail" = 0 ] && echo "every new defence is load-bearing" || echo "SOME DEFENCES ARE NOT TESTED"
exit $fail
