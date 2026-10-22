#!/usr/bin/env python3
"""Rounds on the real regtest chain, each in a different situation, driven through the gateway
exactly as the page drives it. After every settled round, every investor's balances are
reconciled against what the round's signed record says should have moved.

Start the stack first (bash scripts/run.sh, in another terminal), then:

    python3 scripts/soak_chain.py            # the scenarios below, then N repeat rounds
    python3 scripts/soak_chain.py --repeat 6
    python3 scripts/soak_chain.py --evidence evidence/2026-10-21   # also keep checkable proof

With --evidence, it saves only public data: each round's bundle (verify it offline with
`kalyptra verify`), each settlement transaction as the chain itself reports it (id, block,
block hash, raw bytes), the roster and the announced group key, and a README saying how to check
each one. No keys, no tickets, nothing that names an investor.

It prints one line per round and a Markdown table at the end (pasted into docs/EVIDENCE.md).
Stops and restarts a node to test outages: run it only against the stack run.sh started.
"""
import argparse
import json
import os
import signal
import subprocess
import time
import urllib.error
import urllib.request

GW = "http://127.0.0.1:8787/api"
CHAIN_RPC = "http://127.0.0.1:18232"
EVIDENCE = None
PROOF = []
WORK = os.environ.get("KALYPTRA_RUN_DIR", os.path.expanduser("~/kalyptra-run"))
TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.expanduser("~/vibru-target"))
BIN = os.path.join(TARGET, "release", "kalyptra")
TOOL = os.environ.get(
    "KALYPTRA_TOOL",
    os.path.expanduser("~/kalyptra-spike/txtool-kalyptra/target/release/zcash_tx_tool"),
)
INVESTORS = ["alice", "bob", "carol", "dave"]
ROWS = []
NOTES = []
STARTED = []


def call(method, path, who=None, body=None, timeout=1200):
    url = GW + path + (("?as=" + who) if who else "")
    data = json.dumps(body if body is not None else {}).encode() if method == "POST" else None
    req = urllib.request.Request(url, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, json.loads(r.read() or b"null")
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read() or b"null")


def chain(method, params):
    """Ask the regtest node itself (Zebra's JSON-RPC), not anything of ours."""
    body = json.dumps({"jsonrpc": "2.0", "id": "kalyptra", "method": method,
                       "params": params}).encode()
    req = urllib.request.Request(CHAIN_RPC, data=body, method="POST",
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as r:
        out = json.loads(r.read())
    if out.get("error"):
        raise RuntimeError(f"{method}: {out['error']}")
    return out["result"]


def save(name, data):
    path = os.path.join(EVIDENCE, name)
    with open(path, "w", encoding="utf-8") as f:
        if isinstance(data, str):
            f.write(data)
        else:
            json.dump(data, f, indent=2)
            f.write("\n")
    return path


def keep_round(rid, settlement):
    """Save the round's bundle and, if it settled, its transaction as the chain reports it."""
    code, bundle = call("GET", f"/rounds/{rid}/bundle")
    if code != 200:
        print(f"    evidence: no bundle for round {rid}: {code} {bundle}")
        return
    save(f"round-{rid}.bundle.json", bundle)
    entry = {"round": rid, "bundle": f"round-{rid}.bundle.json"}
    txid = (settlement or {}).get("txid")
    if txid:
        try:
            raw = chain("getrawtransaction", [txid, 0])
            tx = chain("getrawtransaction", [txid, 1])
            height = tx.get("height", (settlement or {}).get("height"))
            block = chain("getblock", [str(height), 1])
            hashes = [t if isinstance(t, str) else t.get("txid") for t in block.get("tx", [])]
            entry.update({
                "txid": txid, "height": height, "block_hash": block.get("hash"),
                "in_block": txid in hashes, "confirmations": tx.get("confirmations"),
                "raw_bytes": len(raw) // 2, "raw": f"round-{rid}.tx.hex",
            })
            save(f"round-{rid}.tx.hex", raw + "\n")
            save(f"round-{rid}.tx.json", {k: v for k, v in entry.items() if k != "bundle"})
            print(f"    evidence: tx {txid[:16]}… in block {height} "
                  f"({block.get('hash', '')[:16]}…): {'found' if entry['in_block'] else 'NOT FOUND'}")
        except Exception as e:  # keep going: the bundle is still evidence
            entry["chain_error"] = str(e)
            print(f"    evidence: could not read tx {txid[:16]}… from the chain: {e}")
    PROOF.append(entry)


def verify_bundles(group_key):
    """Check every saved bundle offline, as a reviewer would: no node, no chain."""
    for e in PROOF:
        out = subprocess.run(
            [BIN, "verify", "--bundle", os.path.join(EVIDENCE, e["bundle"]),
             "--roster", os.path.join(EVIDENCE, "roster.json"), "--group-key", group_key],
            capture_output=True, text=True)
        text = out.stdout + out.stderr
        e["offline_check"] = ("verified offline" if "VERIFIED OFFLINE" in text
                              else "voided, checked" if "voided" in text and out.returncode == 0
                              else "FAILED: " + text.strip().splitlines()[-1] if text.strip()
                              else "FAILED")
        print(f"    round {e['round']}: {e['offline_check']}")


def write_readme(group_key):
    lines = [
        "# Evidence from a run on the regtest chain",
        "",
        "Saved by `scripts/soak_chain.py --evidence` while the chain was running. Public data only.",
        "",
        "Check any round offline, trusting nobody (no node, no chain, no network):",
        "",
        "```bash",
        "kalyptra verify --bundle round-1.bundle.json --roster roster.json \\",
        f"  --group-key {group_key}",
        "```",
        "",
        "It recomputes the price from the published orders and checks every receipt, vote, "
        "decryption share and signature. Change one number in a bundle and it fails.",
        "",
        "Each `round-N.tx.json` is the settlement as the chain's own node reported it "
        "(`getrawtransaction`, `getblock`), and `round-N.tx.hex` is the transaction's raw "
        "bytes: the swap is on chain, and its amounts and parties are shielded.",
        "",
        "| Round | Bundle checked offline | Transaction id | Block | Block hash | In that block |",
        "|---|---|---|---|---|---|",
    ]
    for e in PROOF:
        lines.append("| {} | {} | {} | {} | {} | {} |".format(
            e["round"], e.get("offline_check", "not checked"),
            f"`{e['txid']}`" if e.get("txid") else "none (nothing moved)",
            e.get("height", ""), f"`{e['block_hash']}`" if e.get("block_hash") else "",
            "yes" if e.get("in_block") else ("" if not e.get("txid") else "NO")))
    save("README.md", "\n".join(lines) + "\n")


def balances():
    return {w: call("GET", "/balances", w)[1] for w in INVESTORS}


def order(rid, market, who, side, size, limit):
    code, out = call("POST", f"/rounds/{rid}/orders", who,
                     {"market": market, "side": side, "size": size, "limit": limit})
    note = out.get("ticket") if code == 200 else out.get("error")
    print(f"    {who} {side} {size} @ {limit}: {code} {note}")
    return code, out


def wait_round(rid, want, timeout=600):
    t0 = time.time()
    while time.time() - t0 < timeout:
        code, r = call("GET", f"/rounds/{rid}")
        phase = (r or {}).get("round", {}).get("phase")
        if phase in want and (phase != "cleared" or (r or {}).get("check") is not None):
            return r
        time.sleep(2)
    raise SystemExit(f"round {rid}: not {want} after {timeout}s")


def node_cmd(i):
    return [BIN, "node", "--id", str(i), "--dir", "nodes", "--roster", "roster.json",
            "--tool", TOOL, "--wallet", f"wallets/node{i}", "--assets", "assets.json"]


def stop_node(i):
    subprocess.run(["pkill", "-f", f"node --id {i} --dir nodes"], check=False)
    time.sleep(2)
    print(f"    node {i} stopped")


def start_node(i):
    log = open(os.path.join(WORK, f"node-{i}-restarted.log"), "a")
    p = subprocess.Popen(node_cmd(i), cwd=WORK, stdout=log, stderr=log)
    STARTED.append(p)
    time.sleep(3)
    print(f"    node {i} started again from its saved files")


def settle_all(rid, timeout=1800, mid=None):
    """Every investor with a filled order in this round hands in its part; wait for the chain.
    `mid`, if given, runs a minute after the parts are in (after noting the status then)."""
    owners = []
    for w in INVESTORS:
        for t in call("GET", "/tickets", w)[1]:
            if t["round_id"] == rid and (t.get("filled") or 0) > 0:
                owners.append((w, t))
    for w, t in owners:
        code, out = call("POST", f"/tickets/{t['id']}/settle", w)
        print(f"    {w} settles {t['id']}: {code} {out.get('error') or 'part handed in'}")
    if mid:
        time.sleep(60)
        s = (call("GET", f"/rounds/{rid}")[1] or {}).get("settlement") or {}
        print(f"    one minute after the parts: settlement {s.get('status') or 'not started'}")
        NOTES.append(f"round {rid}: one minute after every part was in, with the leader down, "
                     f"settlement was '{s.get('status') or 'not started'}'")
        mid()
    t0 = time.time()
    while time.time() - t0 < timeout:
        s = (call("GET", f"/rounds/{rid}")[1] or {}).get("settlement") or {}
        if s.get("status") in ("settled", "failed", "unknown"):
            return owners, s
        time.sleep(3)
    return owners, {"status": "still waiting"}


def expected(before, owners, market, price):
    base, quote = market.split("/")
    want = json.loads(json.dumps(before))
    for w, t in owners:
        f = t["filled"]
        sign = -1 if t["side"] == "sell" else 1
        want[w][base] = want[w].get(base, 0) + sign * f
        want[w][quote] = want[w].get(quote, 0) - sign * f * price
    return want


def run(rid, market, situation, orders, min_orders=2, expect="settled", before_close=None,
        before_open=None, after=None, mid_settle=None):
    print(f"\nround {rid} [{situation}] {market}")
    if before_open:
        before_open()
    if rid != 1:  # run.sh opens round 1 itself
        code, out = call("POST", "/rounds", None,
                         {"round_id": rid, "market": market, "min_orders": min_orders})
        print(f"    open: {code} {out}")
    before = balances()
    refused = 0
    for who, side, size, limit, ok in orders:
        code, _ = order(rid, market, who, side, size, limit)
        if (code == 200) != ok:
            raise SystemExit(f"round {rid}: {who}'s order answered {code}, expected "
                             f"{'accepted' if ok else 'refused'}")
        refused += code != 200
    if before_close:
        before_close()
    print(f"    close: {call('POST', f'/rounds/{rid}/close')[0]}")
    r = wait_round(rid, {"cleared", "voided"})
    phase = r["round"]["phase"]
    rec = r.get("record") or {}
    check = r.get("check") or {}
    clean = check.get("clean") is True or (check.get("findings") == [] if check else False)
    price, volume = rec.get("price"), rec.get("volume", 0)
    print(f"    {phase}: price {price}, volume {volume}, public check "
          f"{'clean' if clean else check}")
    if phase == "voided":
        settled, recon = "nothing opened", balances() == before
    elif not price:
        settled, recon = "nothing to settle", balances() == before
    else:
        owners, s = settle_all(rid, mid=mid_settle)
        settled = (f"{s.get('status')} at block {s.get('height')}, "
                   f"tx {str(s.get('txid', ''))[:16]}…") if s.get("txid") else s.get("status")
        recon = balances() == expected(before, owners, market, price)
        if not recon:
            print("    MISMATCH", balances(), expected(before, owners, market, price))
    if after:
        after()
    print(f"    balances reconciled: {recon}")
    if EVIDENCE:
        keep_round(rid, (call("GET", f"/rounds/{rid}")[1] or {}).get("settlement"))
    ok = (phase == ("voided" if expect == "voided" else "cleared")) and clean and recon and (
        expect != "settled" or settled.startswith("settled"))
    ROWS.append((rid, situation, market, len(orders) - refused, refused,
                 "voided" if phase == "voided" else f"{price} / {volume}",
                 "clean" if clean else "FINDINGS", settled, "yes" if recon else "NO",
                 "as expected" if ok else "NOT AS EXPECTED"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repeat", type=int, default=4, help="extra plain rounds at the end")
    ap.add_argument("--evidence", help="a directory to keep checkable proof of every round in")
    a = ap.parse_args()
    A, B, C = "ACME/vUSD", "BOLT/vUSD", "CRUX/vUSD"
    global EVIDENCE
    group_key = ""
    if a.evidence:
        EVIDENCE = a.evidence
        os.makedirs(EVIDENCE, exist_ok=True)
        with open(os.path.join(WORK, "roster.json"), encoding="utf-8") as f:
            save("roster.json", json.load(f))
        with open(os.path.join(WORK, "assets.json"), encoding="utf-8") as f:
            save("assets.json", json.load(f))
        group_key = call("GET", "/status")[1]["group_key"]
        save("group-key.txt", group_key + "\n")
    try:
        run(1, A, "plain round, four investors", [
            ("alice", "sell", 10, 4, True), ("bob", "sell", 4, 5, True),
            ("carol", "buy", 8, 6, True), ("dave", "buy", 6, 5, True)])
        run(2, B, "orders the wallet cannot cover, or a second order in one asset", [
            ("alice", "sell", 500, 3, False), ("alice", "sell", 20, 3, True),
            ("alice", "sell", 5, 2, False), ("carol", "buy", 15, 4, True),
            ("dave", "buy", 10, 3, True)])
        run(3, C, "too few orders: voided", [("bob", "sell", 5, 2, True)], min_orders=2,
            expect="voided")
        run(4, A, "nothing crosses", [("bob", "sell", 5, 9, True), ("dave", "buy", 5, 2, True)],
            expect="no trade")
        run(5, A, "node 3 (this round's settlement leader) down, back after the parts", [
            ("alice", "sell", 6, 5, True), ("carol", "buy", 6, 5, True)],
            before_open=lambda: stop_node(3), mid_settle=lambda: start_node(3))
        run(6, C, "node 2 restarted between orders and close", [
            ("bob", "sell", 10, 7, True), ("carol", "buy", 10, 8, True)],
            before_close=lambda: (stop_node(2), start_node(2)))
        run(7, B, "partial fill of the marginal order", [
            ("alice", "sell", 9, 2, True), ("carol", "buy", 5, 5, True),
            ("dave", "buy", 7, 3, True)])
        markets = [A, B, C]
        sellers = {A: ["alice", "bob"], B: ["alice"], C: ["bob"]}
        for k in range(a.repeat):
            m = markets[k % 3]
            s = sellers[m][k % len(sellers[m])]
            run(8 + k, m, "plain round, repeated", [
                (s, "sell", 3 + k, 4, True), ("carol", "buy", 2 + k, 5, True),
                ("dave", "buy", 2, 4, True)])
    finally:
        for p in STARTED:
            p.send_signal(signal.SIGTERM)
        print("\n| Round | Situation | Market | Orders counted | Refused | Price / volume "
              "| Public check | Settlement | Balances reconciled | Result |")
        print("|---|---|---|---|---|---|---|---|---|---|")
        for row in ROWS:
            print("| " + " | ".join(str(x) for x in row) + " |")
        for n in NOTES:
            print("- " + n)
        if EVIDENCE and PROOF:
            print("\nchecking every saved bundle offline:")
            verify_bundles(group_key)
            write_readme(group_key)
            print(f"evidence saved in {EVIDENCE}")


if __name__ == "__main__":
    main()
