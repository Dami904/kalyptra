//! Three matcher nodes on localhost ports, talking only over HTTP: distributed key generation,
//! a full round, and the public checker.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use kalyptra_matcher::Side;
use kalyptra_node::client::{self, Outcome};
use kalyptra_node::files::NodeIdentity;
use kalyptra_node::server::{spawn, NodeHandle, NodeOptions, SettleOptions};
use kalyptra_node::wire::{PublicRoundJ, RosterJ};
use rand_core::{OsRng, RngCore};

const MARKET: &str = "ACME/vUSD";

struct Cluster {
    roster: RosterJ,
    handles: Vec<Option<NodeHandle>>,
    dir: PathBuf,
}

fn temp_dir() -> PathBuf {
    let mut b = [0u8; 8];
    OsRng.fill_bytes(&mut b);
    std::env::temp_dir().join(format!("kalyptra-test-{}", hex::encode(b)))
}

fn spawn_node(
    s: tiny_http::Server,
    identity: NodeIdentity,
    roster: &RosterJ,
    dir: &Path,
) -> NodeHandle {
    spawn_settling(s, identity, roster, dir, None)
}

fn spawn_settling(
    s: tiny_http::Server,
    identity: NodeIdentity,
    roster: &RosterJ,
    dir: &Path,
    settle: Option<SettleOptions>,
) -> NodeHandle {
    spawn(
        s,
        NodeOptions {
            identity,
            roster: roster.clone(),
            data_dir: dir.to_path_buf(),
            verbose: false,
            settle,
        },
    )
    .unwrap()
}

fn start(n: u16, t: u16) -> Cluster {
    start_with(n, t, temp_dir(), None)
}

fn start_with(n: u16, t: u16, dir: PathBuf, settle: Option<SettleOptions>) -> Cluster {
    let servers: Vec<tiny_http::Server> = (0..n)
        .map(|_| tiny_http::Server::http("127.0.0.1:0").unwrap())
        .collect();
    let identities: Vec<NodeIdentity> = servers
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let addr = s.server_addr().to_ip().unwrap();
            NodeIdentity::generate(i as u16 + 1, &format!("http://{addr}"))
        })
        .collect();
    let roster = RosterJ {
        threshold: t,
        nodes: identities.iter().map(|i| i.public_entry()).collect(),
    };
    for i in &identities {
        i.save(&kalyptra_node::files::key_path(&dir, i.id)).unwrap();
    }
    let handles = servers
        .into_iter()
        .zip(identities)
        .map(|(s, identity)| Some(spawn_settling(s, identity, &roster, &dir, settle.clone())))
        .collect();
    Cluster {
        roster,
        handles,
        dir,
    }
}

impl Drop for Cluster {
    fn drop(&mut self) {
        for h in self.handles.iter_mut() {
            if let Some(h) = h.take() {
                h.stop();
            }
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Wait until every node has seen every envelope and every receipt.
fn wait_gossip(c: &Cluster, round: u64, envelopes: usize) {
    let n = c.roster.nodes.len();
    for _ in 0..200 {
        let settled = c.roster.urls().iter().all(|u| {
            client::get::<PublicRoundJ>(&format!("{u}/v1/rounds/{round}/public"))
                .map(|p| p.envelopes.len() == envelopes && p.receipts.len() == envelopes * n)
                .unwrap_or(false)
        });
        if settled {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("gossip did not settle");
}

#[test]
fn three_nodes_over_http_generate_a_key_run_a_round_and_the_checker_agrees() {
    let c = start(3, 2);
    let group = client::wait_for_keys(&c.roster, Duration::from_secs(60)).unwrap();
    assert_eq!(group.threshold(), 2);
    // the share files exist and no node needed a dealer
    for id in 1..=3 {
        assert!(c.dir.join(format!("node-{id}.share.json")).exists());
    }

    client::open_round(&c.roster, 1, MARKET, 2).unwrap();
    for (side, size, limit) in [
        (Side::Sell, 10, 4),
        (Side::Sell, 4, 5),
        (Side::Buy, 8, 6),
        (Side::Buy, 6, 5),
    ] {
        let (receipts, _) = client::submit_order(&c.roster, 1, MARKET, side, size, limit).unwrap();
        assert_eq!(receipts.len(), 3, "every node receipts a valid order");
    }
    wait_gossip(&c, 1, 4);
    client::close_round(&c.roster, 1).unwrap();

    match client::wait_final(&c.roster, 1, Duration::from_secs(60)).unwrap() {
        Outcome::Cleared(r) => {
            assert_eq!(r.price, Some(5));
            assert_eq!(r.volume, 14);
            assert_eq!(r.orders.len(), 4);
        }
        Outcome::Voided => panic!("round should clear"),
    }
    let report = client::check(&c.roster, 1).unwrap();
    assert!(report.is_clean(), "{:?}", report.findings);
    assert_eq!(report.recomputed.unwrap().price, Some(5));
}

#[test]
fn a_voided_round_over_http_decrypts_nothing() {
    let c = start(3, 2);
    client::wait_for_keys(&c.roster, Duration::from_secs(60)).unwrap();
    client::open_round(&c.roster, 7, MARKET, 3).unwrap();
    client::submit_order(&c.roster, 7, MARKET, Side::Buy, 5, 5).unwrap();
    client::submit_order(&c.roster, 7, MARKET, Side::Sell, 5, 4).unwrap();
    wait_gossip(&c, 7, 2);
    client::close_round(&c.roster, 7).unwrap();
    assert!(matches!(
        client::wait_final(&c.roster, 7, Duration::from_secs(60)).unwrap(),
        Outcome::Voided
    ));
    for u in c.roster.urls() {
        let p: PublicRoundJ = client::get(&format!("{u}/v1/rounds/7/public")).unwrap();
        assert!(
            p.partials.is_empty(),
            "no partials anywhere for a voided round"
        );
    }
    let report = client::check(&c.roster, 7).unwrap();
    assert!(report.voided && report.is_clean(), "{:?}", report.findings);
}

#[test]
fn orders_for_a_closed_or_unknown_round_get_no_receipts() {
    let c = start(3, 2);
    client::wait_for_keys(&c.roster, Duration::from_secs(60)).unwrap();
    client::open_round(&c.roster, 1, MARKET, 1).unwrap();
    client::close_round(&c.roster, 1).unwrap();
    let (late, _) = client::submit_order(&c.roster, 1, MARKET, Side::Buy, 1, 1).unwrap();
    assert!(late.is_empty());
    let (unknown, _) = client::submit_order(&c.roster, 99, MARKET, Side::Buy, 1, 1).unwrap();
    assert!(unknown.is_empty());
}

#[test]
fn a_restarted_node_reloads_its_share_and_keeps_working() {
    let mut c = start(3, 2);
    let before = client::wait_for_keys(&c.roster, Duration::from_secs(60)).unwrap();
    // stop node 3, then start it again on the same address from its saved files only
    let addr = c.handles[2].as_ref().unwrap().addr;
    c.handles[2].take().unwrap().stop();
    let server = (0..50)
        .find_map(|_| {
            std::thread::sleep(Duration::from_millis(100));
            tiny_http::Server::http(addr).ok()
        })
        .expect("rebind node 3's address");
    let identity = NodeIdentity::load(&kalyptra_node::files::key_path(&c.dir, 3)).unwrap();
    c.handles[2] = Some(spawn_node(server, identity, &c.roster, &c.dir));
    let after = client::wait_for_keys(&c.roster, Duration::from_secs(30)).unwrap();
    assert_eq!(
        before, after,
        "same group key after the restart, no new key generation"
    );

    // and node 3 still takes part in a round
    client::open_round(&c.roster, 2, MARKET, 2).unwrap();
    client::submit_order(&c.roster, 2, MARKET, Side::Sell, 5, 3).unwrap();
    client::submit_order(&c.roster, 2, MARKET, Side::Buy, 5, 4).unwrap();
    wait_gossip(&c, 2, 2);
    client::close_round(&c.roster, 2).unwrap();
    assert!(matches!(
        client::wait_final(&c.roster, 2, Duration::from_secs(60)).unwrap(),
        Outcome::Cleared(_)
    ));
    let s3: kalyptra_node::wire::RoundStatusJ =
        client::get(&format!("{}/v1/rounds/2", c.roster.nodes[2].url)).unwrap();
    assert!(
        s3.record.is_some(),
        "the restarted node reached the final record too"
    );
}

#[test]
fn a_round_opens_closes_and_clears_with_any_one_node_down() {
    let mut c = start(3, 2);
    client::wait_for_keys(&c.roster, Duration::from_secs(60)).unwrap();
    for (round, down) in [(1u64, 0usize), (2, 2)] {
        // node 1 first: the operator commands used to stop at the first node that did not answer
        let addr = c.handles[down].as_ref().unwrap().addr;
        c.handles[down].take().unwrap().stop();
        client::open_round(&c.roster, round, MARKET, 2).expect("2 of 3 nodes can open a round");
        client::submit_order(&c.roster, round, MARKET, Side::Sell, 5, 3).unwrap();
        client::submit_order(&c.roster, round, MARKET, Side::Buy, 5, 4).unwrap();
        // as a real round does by its close: both orders have reached both live nodes. (Closed
        // on different receipts with the third node away, two nodes cannot prove the other's
        // vote dead, so the round would wait for it: a documented limit, not what this tests.)
        wait_envelopes(&c, round, 2);
        client::close_round(&c.roster, round).expect("2 of 3 nodes can close it");
        assert!(matches!(
            client::wait_final(&c.roster, round, Duration::from_secs(60)).unwrap(),
            Outcome::Cleared(_)
        ));
        start_again(&mut c, down, addr);
    }
    // with two nodes down, the operator is told plainly
    c.handles[0].take().unwrap().stop();
    c.handles[1].take().unwrap().stop();
    let err = client::open_round(&c.roster, 3, MARKET, 2).unwrap_err();
    assert!(err.contains("1 of 3"), "{err}");
}

// ---- many rounds under different situations ------------------------------------------------

/// Stop node `i` (0-based) and start it again on the same address from its saved files only.
fn restart(c: &mut Cluster, i: usize) {
    let addr = c.handles[i].as_ref().unwrap().addr;
    c.handles[i].take().unwrap().stop();
    start_again(c, i, addr);
}

fn start_again(c: &mut Cluster, i: usize, addr: SocketAddr) {
    let server = (0..100)
        .find_map(|_| {
            std::thread::sleep(Duration::from_millis(50));
            tiny_http::Server::http(addr).ok()
        })
        .expect("rebind the node's address");
    let identity =
        NodeIdentity::load(&kalyptra_node::files::key_path(&c.dir, i as u16 + 1)).unwrap();
    c.handles[i] = Some(spawn_node(server, identity, &c.roster, &c.dir));
}

/// Wait until every node that is up has seen `envelopes` envelopes.
fn wait_envelopes(c: &Cluster, round: u64, envelopes: usize) {
    for _ in 0..400 {
        let up: Vec<String> = c
            .handles
            .iter()
            .zip(c.roster.urls())
            .filter(|(h, _)| h.is_some())
            .map(|(_, u)| u)
            .collect();
        if up.iter().all(|u| {
            client::get::<PublicRoundJ>(&format!("{u}/v1/rounds/{round}/public"))
                .map(|p| p.envelopes.len() == envelopes)
                .unwrap_or(false)
        }) {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("round {round}: envelopes did not reach every live node");
}

fn rnd(below: u64) -> u64 {
    OsRng.next_u64() % below
}

/// The clearing price found a second way, written apart from `auction::clear`: the candidate
/// prices are the orders' own limits (published rule 1), and the winner has the most volume,
/// then the least imbalance, then the lowest price. (A first version also tried the prices
/// between limits. That is a different rule: between two limits, demand is the upper limit's
/// and supply the lower one's, so an in-between price can tie on volume with less imbalance.
/// The soak run caught the difference; the published rule only ever clears at a stated limit.)
fn reference_clearing(orders: &[(Side, u64, u64)]) -> (Option<u64>, u64) {
    let mut limits: Vec<u64> = orders.iter().map(|o| o.2).collect();
    limits.sort_unstable();
    limits.dedup();
    let mut best: Option<(u128, u128, u64)> = None;
    for p in limits {
        let d: u128 = orders
            .iter()
            .filter(|o| o.0 == Side::Buy && o.2 >= p)
            .map(|o| o.1 as u128)
            .sum();
        let s: u128 = orders
            .iter()
            .filter(|o| o.0 == Side::Sell && o.2 <= p)
            .map(|o| o.1 as u128)
            .sum();
        let key = (d.min(s), d.abs_diff(s), p);
        let better = match best {
            None => true,
            Some((v, i, _)) => key.0 > v || (key.0 == v && key.1 < i),
        };
        if better {
            best = Some(key);
        }
    }
    match best {
        Some((v, _, p)) if v > 0 => (Some(p), v as u64),
        _ => (None, 0),
    }
}

/// Every rule a cleared record must keep, checked from the record alone.
fn check_fills(r: &kalyptra_node::wire::RecordJ) {
    let price = r.price;
    let mut bought = 0u64;
    let mut sold = 0u64;
    for f in &r.fills {
        let (_, o) = r
            .orders
            .iter()
            .find(|(id, _)| *id == f.id)
            .expect("every fill is a known order");
        assert_eq!(f.side, o.side, "a fill keeps its order's side");
        assert!(
            f.quantity > 0 && f.quantity <= o.size,
            "a fill is within its order"
        );
        let p = price.expect("a fill needs a price");
        if o.side == "buy" {
            assert!(o.limit >= p, "no buyer pays above its limit");
            bought += f.quantity;
        } else {
            assert!(o.limit <= p, "no seller receives below its limit");
            sold += f.quantity;
        }
    }
    assert_eq!(bought, r.volume, "bought = volume");
    assert_eq!(sold, r.volume, "sold = volume");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Situation {
    Normal,
    ManyTies,
    NothingCrosses,
    TooFewOrders,
    GarbageEnvelope,
    SentTwice,
    NodeDownBeforeOpen,
    NodeDownBeforeClose,
    NodeRestarted,
}

const SITUATIONS: [Situation; 9] = [
    Situation::Normal,
    Situation::ManyTies,
    Situation::NothingCrosses,
    Situation::TooFewOrders,
    Situation::GarbageEnvelope,
    Situation::SentTwice,
    Situation::NodeDownBeforeOpen,
    Situation::NodeDownBeforeClose,
    Situation::NodeRestarted,
];

/// Many rounds through three real node processes, each round in a randomly chosen situation,
/// each result checked against the reference clearing, the fill rules and the public checker.
/// `SOAK_ROUNDS=1000 cargo test --release -p kalyptra-node --test http soak -- --ignored --nocapture`
#[test]
#[ignore = "long: run by hand for evidence (docs/EVIDENCE.md)"]
fn soak_many_rounds_in_different_situations() {
    let rounds: u64 = std::env::var("SOAK_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);
    let mut c = start(3, 2);
    client::wait_for_keys(&c.roster, Duration::from_secs(60)).unwrap();
    let started = std::time::Instant::now();
    // situation -> (rounds, cleared, voided, orders, volume)
    let mut tally: BTreeMap<Situation, (u64, u64, u64, u64, u64)> = BTreeMap::new();
    let mut total_orders = 0u64;

    for round in 1..=rounds {
        let sit = SITUATIONS[rnd(SITUATIONS.len() as u64) as usize];
        let down = rnd(3) as usize;
        let mut stopped: Option<SocketAddr> = None;
        if sit == Situation::NodeDownBeforeOpen {
            let addr = c.handles[down].as_ref().unwrap().addr;
            c.handles[down].take().unwrap().stop();
            stopped = Some(addr);
        }

        let n = match sit {
            Situation::TooFewOrders => 1 + rnd(3),
            _ => 2 + rnd(23),
        };
        let min_orders = if sit == Situation::TooFewOrders {
            n as usize + 1
        } else {
            2
        };
        client::open_round(&c.roster, round, MARKET, min_orders)
            .unwrap_or_else(|e| panic!("round {round} ({sit:?}): open failed: {e}"));

        let mut orders = Vec::new();
        for _ in 0..n {
            let side = if rnd(2) == 0 { Side::Buy } else { Side::Sell };
            let limit = match (sit, side) {
                (Situation::NothingCrosses, Side::Buy) => 1 + rnd(5),
                (Situation::NothingCrosses, Side::Sell) => 10 + rnd(5),
                (Situation::ManyTies, _) => 4 + rnd(3),
                _ => 1 + rnd(20),
            };
            let size = 1 + rnd(100);
            let sealed = client::seal_order(&c.roster, round, MARKET, side, size, limit).unwrap();
            let sent = client::send_order(&c.roster, round, &sealed.envelope).unwrap();
            assert!(
                sent.receipts.len() >= 2,
                "round {round} ({sit:?}): an order got {} receipts",
                sent.receipts.len()
            );
            if sit == Situation::SentTwice {
                // the same envelope again: it must count once
                client::send_order(&c.roster, round, &sealed.envelope).unwrap();
            }
            orders.push((side, size, limit));
        }
        if sit == Situation::GarbageEnvelope {
            let mut junk = vec![0u8; 64 + rnd(200) as usize];
            OsRng.fill_bytes(&mut junk);
            let sent = client::send_order(&c.roster, round, &junk).unwrap();
            assert!(sent.receipts.is_empty(), "random bytes get no receipt");
        }
        wait_envelopes(&c, round, orders.len());

        if sit == Situation::NodeDownBeforeClose {
            let addr = c.handles[down].as_ref().unwrap().addr;
            c.handles[down].take().unwrap().stop();
            stopped = Some(addr);
        }
        if sit == Situation::NodeRestarted {
            restart(&mut c, down);
        }
        client::close_round(&c.roster, round)
            .unwrap_or_else(|e| panic!("round {round} ({sit:?}): close failed: {e}"));

        let outcome = client::wait_final(&c.roster, round, Duration::from_secs(60))
            .unwrap_or_else(|e| panic!("round {round} ({sit:?}): {e}"));
        let report = client::check(&c.roster, round).unwrap();
        assert!(
            report.is_clean(),
            "round {round} ({sit:?}): the public check found {:?}",
            report.findings
        );
        let t = tally.entry(sit).or_default();
        t.0 += 1;
        t.3 += n;
        total_orders += n;
        match outcome {
            Outcome::Voided => {
                assert_eq!(
                    sit,
                    Situation::TooFewOrders,
                    "round {round} voided ({sit:?})"
                );
                assert!(report.voided);
                for u in c.roster.urls() {
                    if let Ok(p) =
                        client::get::<PublicRoundJ>(&format!("{u}/v1/rounds/{round}/public"))
                    {
                        assert!(p.partials.is_empty(), "a voided round releases nothing");
                    }
                }
                t.2 += 1;
            }
            Outcome::Cleared(r) => {
                assert_ne!(sit, Situation::TooFewOrders, "round {round} should void");
                assert_eq!(
                    r.orders.len(),
                    orders.len(),
                    "round {round}: every order counted once"
                );
                let (price, volume) = reference_clearing(&orders);
                assert_eq!(
                    (r.price, r.volume),
                    (price, volume),
                    "round {round} ({sit:?}): differs from the reference clearing"
                );
                if sit == Situation::NothingCrosses {
                    assert_eq!(r.volume, 0);
                }
                check_fills(&r);
                let rec = report.recomputed.expect("the checker recomputed the round");
                assert_eq!((rec.price, rec.volume), (r.price, r.volume));
                t.1 += 1;
                t.4 += r.volume;
            }
        }
        if let Some(addr) = stopped {
            start_again(&mut c, down, addr);
        }
    }

    let secs = started.elapsed().as_secs_f64();
    eprintln!("\nSOAK: {rounds} rounds, {total_orders} orders, {secs:.1} s");
    eprintln!("| Situation | Rounds | Cleared | Voided | Orders | Volume |");
    eprintln!("|---|---|---|---|---|---|");
    for (s, (n, cl, vo, o, v)) in &tally {
        eprintln!("| {s:?} | {n} | {cl} | {vo} | {o} | {v} |");
    }
}

// ---- the local gateway: a round driven entirely through its JSON ---------------------------

mod gw {
    use super::*;
    use kalyptra_node::bundle::{self, BundleJ};
    use kalyptra_node::gateway::{self, GatewayHandle, GatewayOptions};
    use kalyptra_node::wallet::Wallet;
    use serde_json::{json, Value};

    pub fn start_gateway(
        c: &Cluster,
        wallets: Vec<(&str, Wallet)>,
        assets: &[(&str, &str)],
    ) -> (GatewayHandle, String) {
        start_gateway_with(c, c.roster.clone(), wallets, assets)
    }

    /// Like [`start_gateway`], but talking to the nodes listed in `roster`.
    pub fn start_gateway_with(
        c: &Cluster,
        roster: kalyptra_node::wire::RosterJ,
        wallets: Vec<(&str, Wallet)>,
        assets: &[(&str, &str)],
    ) -> (GatewayHandle, String) {
        let home = c.dir.join("gateway");
        let assets_path = c.dir.join("assets.json");
        let map: kalyptra_node::wire::AssetsJ = assets
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        kalyptra_node::files::write_json(&assets_path, &map, false).unwrap();
        let h = gateway::spawn(
            tiny_http::Server::http("127.0.0.1:0").unwrap(),
            GatewayOptions {
                roster,
                home,
                assets_path,
                auditor: wallets.first().map(|(_, w)| w.clone()),
                wallets: wallets
                    .into_iter()
                    .map(|(n, w)| (n.to_string(), w))
                    .collect(),
                ui_dir: None,
            },
        )
        .unwrap();
        let base = format!("http://{}", h.addr);
        (h, base)
    }

    pub fn get(base: &str, path: &str) -> Result<Value, String> {
        client::get(&format!("{base}{path}"))
    }

    pub fn post(base: &str, path: &str, body: Value) -> Result<Value, String> {
        client::post(&format!("{base}{path}"), &body)
    }

    /// Poll until `f` returns something, or fail after about 60 seconds.
    pub fn until<T>(what: &str, mut f: impl FnMut() -> Option<T>) -> T {
        for _ in 0..600 {
            if let Some(v) = f() {
                return v;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("timed out waiting for {what}");
    }

    /// Open a round and place the given orders, all through the gateway.
    pub fn run_round(
        base: &str,
        c: &Cluster,
        round: Option<u64>,
        orders: &[(&str, u64, u64)],
    ) -> (u64, Vec<String>) {
        let opened = post(
            base,
            "/api/rounds",
            json!({ "round_id": round, "market": MARKET, "min_orders": 2 }),
        )
        .unwrap();
        let id = opened["round_id"].as_u64().unwrap();
        let mut tickets = Vec::new();
        for (side, size, limit) in orders {
            let r = post(
                base,
                &format!("/api/rounds/{id}/orders"),
                json!({ "market": MARKET, "side": side, "size": size, "limit": limit }),
            )
            .unwrap();
            assert_eq!(r["counted"], json!(true), "{r}");
            tickets.push(r["ticket"].as_str().unwrap().to_string());
        }
        wait_gossip(c, id, orders.len());
        post(base, &format!("/api/rounds/{id}/close"), json!({})).unwrap();
        until("the round to clear", || {
            let r = get(base, &format!("/api/rounds/{id}")).ok()?;
            (r["round"]["phase"] == "cleared" && r["check"].is_object()).then_some(())
        });
        (id, tickets)
    }

    #[test]
    fn the_order_key_is_saved_before_sending_and_dropped_only_when_every_node_refuses() {
        let c = start(3, 2);
        client::wait_for_keys(&c.roster, Duration::from_secs(60)).unwrap();
        // node 3 is listed at an address where nothing answers
        let mut roster = c.roster.clone();
        let dead = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        roster.nodes[2].url = format!("http://{}", dead.local_addr().unwrap());
        drop(dead);
        let (gw, base) = start_gateway_with(&c, roster, vec![], &[]);
        let tickets = c.dir.join("gateway").join("tickets").join("me");
        let saved = || std::fs::read_dir(&tickets).map(|d| d.count()).unwrap_or(0);
        let order = |round: u64| {
            post(
                &base,
                &format!("/api/rounds/{round}/orders"),
                json!({ "market": MARKET, "side": "buy", "size": 1, "limit": 1 }),
            )
            .unwrap()
        };
        // round 4 is closed on the two live nodes; node 3 never answers: not certain nobody
        // holds it, so the ticket (and its key) is kept
        client::open_round(&c.roster, 4, MARKET, 1).unwrap();
        for n in &c.roster.nodes[..2] {
            client::post::<_, Value>(&format!("{}/v1/rounds/4/close", n.url), &json!({})).unwrap();
        }
        let r = order(4);
        assert_eq!(r["counted"], false, "{r}");
        assert!(r["ticket"].is_string(), "{r}");
        assert_eq!(saved(), 1);
        gw.stop();
        // with every node answering and every one refusing, nothing is kept
        let (gw, base) = start_gateway_with(&c, c.roster.clone(), vec![], &[]);
        let r = post(
            &base,
            "/api/rounds/4/orders",
            json!({ "market": MARKET, "side": "buy", "size": 1, "limit": 1 }),
        )
        .unwrap();
        assert_eq!(r["receipts"], 0, "{r}");
        assert!(r["ticket"].is_null(), "{r}");
        assert_eq!(saved(), 1, "only the earlier, uncertain ticket remains");
        gw.stop();
    }

    /// Send one raw request, so the test can set the `Host` and `Origin` a hostile page
    /// would send. Returns the status code and the response headers, lower-cased.
    fn raw(
        addr: &str,
        method: &str,
        path: &str,
        host: &str,
        origin: Option<&str>,
    ) -> (u16, String) {
        use std::io::{Read, Write};
        let mut s = std::net::TcpStream::connect(addr).unwrap();
        let origin = origin
            .map(|o| format!("Origin: {o}\r\n"))
            .unwrap_or_default();
        let body = if method == "POST" { "{}" } else { "" };
        write!(
            s,
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\n{origin}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        // Read the head only: the server may keep the connection open after answering.
        s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        let mut out = Vec::new();
        let mut byte = [0u8; 1];
        while !out.ends_with(b"\r\n\r\n") && s.read(&mut byte).unwrap_or(0) == 1 {
            out.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&out).to_ascii_lowercase();
        let code = head
            .split(' ')
            .nth(1)
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);
        (code, head)
    }

    #[test]
    fn a_web_page_from_another_site_cannot_read_or_drive_the_gateway() {
        let c = start(3, 2);
        let (gw, _base) = start_gateway(&c, vec![], &[]);
        let addr = gw.addr.to_string();
        let port = gw.addr.port();
        let own = format!("127.0.0.1:{port}");

        // The page itself, and the development server's proxy, are served from this machine.
        let (code, head) = raw(&addr, "GET", "/api/tickets", &own, None);
        assert_eq!(code, 200, "a same-origin read works: {head}");
        assert!(
            !head.contains("access-control-allow-origin"),
            "no response tells a browser that other sites may read it: {head}"
        );
        let local = format!("http://localhost:{port}");
        assert_eq!(raw(&addr, "GET", "/api/tickets", &own, Some(&local)).0, 200);

        // Any other site, from the user's own browser: refused, read or write, and the
        // preflight grants nothing.
        let evil = Some("https://evil.example");
        assert_eq!(raw(&addr, "GET", "/api/tickets", &own, evil).0, 403);
        assert_eq!(raw(&addr, "POST", "/api/rounds/1/close", &own, evil).0, 403);
        let (code, head) = raw(&addr, "OPTIONS", "/api/tickets", &own, evil);
        assert!(
            code == 403 && !head.contains("access-control-allow-origin"),
            "a preflight from another site is refused: {code} {head}"
        );

        // DNS rebinding: another site's name pointed at this machine. The browser then
        // thinks it is same-origin, but the Host header still carries that name.
        let (code, _) = raw(
            &addr,
            "GET",
            "/api/tickets",
            &format!("evil.example:{port}"),
            None,
        );
        assert_eq!(code, 403, "a request for another host name is refused");
        gw.stop();
    }

    #[test]
    fn a_round_runs_through_the_gateway_and_the_page_never_sees_ticket_keys() {
        let c = start(3, 2);
        client::wait_for_keys(&c.roster, Duration::from_secs(60)).unwrap();
        let (gw, base) = start_gateway(&c, vec![], &[]);

        let st = get(&base, "/api/status").unwrap();
        assert_eq!(st["threshold"], 2);
        assert_eq!(st["nodes"].as_array().unwrap().len(), 3);
        assert!(st["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| n["up"] == true && n["keys_ready"] == true));
        assert!(st["group_key"].is_string());

        // the round id is chosen by the gateway when none is given
        let (id, tickets) = run_round(
            &base,
            &c,
            None,
            &[
                ("sell", 10, 4),
                ("sell", 4, 5),
                ("buy", 8, 6),
                ("buy", 6, 5),
            ],
        );
        assert_eq!(id, 1);
        let r = get(&base, "/api/rounds/1").unwrap();
        assert_eq!(r["check"]["clean"], true, "{}", r["check"]);
        assert_eq!(r["record"]["price"], 5);
        assert_eq!(r["record"]["volume"], 14);
        assert_eq!(
            get(&base, "/api/rounds").unwrap().as_array().unwrap().len(),
            1
        );
        // what each node did, from the published data: every node receipted all four orders,
        // released a share for each order and signed the record
        assert_eq!(r["nodes"]["orders"], 4);
        let nodes = r["nodes"]["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 3);
        for n in nodes {
            assert_eq!(n["receipts"], 4, "{n}");
            assert_eq!(n["shares"], 4, "{n}");
            assert_eq!(n["signed"], true, "{n}");
        }
        // A node that sees two matching commit votes commits without voting itself, so only
        // the threshold is guaranteed to have voted.
        let voted = nodes.iter().filter(|n| n["committed"] == true).count();
        assert!(voted >= 2, "{voted} nodes voted");

        // my tickets: all four filled at 5, and nothing secret in what the page receives
        let mine = get(&base, "/api/tickets").unwrap();
        let mine = mine.as_array().unwrap();
        assert_eq!(mine.len(), 4);
        for t in mine {
            assert_eq!(t["price"], 5);
            assert_eq!(t["filled"], t["size"], "{t}");
            assert!(t.get("key").is_none(), "a ticket's key reached the page");
            assert!(tickets.contains(&t["id"].as_str().unwrap().to_string()));
        }
        let raw = std::fs::read_to_string(
            c.dir
                .join("gateway/tickets/me")
                .join(format!("{}.json", tickets[0])),
        )
        .unwrap();
        let key = serde_json::from_str::<Value>(&raw).unwrap()["key"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(!get(&base, "/api/tickets")
            .unwrap()
            .to_string()
            .contains(&key));

        // the saved round verifies offline
        let b: BundleJ =
            serde_json::from_value(get(&base, "/api/rounds/1/bundle").unwrap()).unwrap();
        assert!(bundle::verify(&b, &c.roster, None)
            .unwrap()
            .report
            .is_clean());

        // refusals
        let order = |side: &str, size: u64| json!({ "market": MARKET, "side": side, "size": size, "limit": 5 });
        let e = post(&base, "/api/rounds/1/orders", order("buy", 0)).unwrap_err();
        assert!(e.contains("400"), "{e}");
        let e = post(&base, "/api/rounds/1/orders", order("hold", 1)).unwrap_err();
        assert!(e.contains("400"), "{e}");
        assert!(get(&base, "/api/nope").unwrap_err().contains("404"));
        assert!(get(&base, "/api/rounds/99").unwrap_err().contains("404"));
        let e = post(&base, "/api/tickets/..%2fsecret/settle", json!({})).unwrap_err();
        assert!(e.contains("400"), "{e}");
        // no wallet configured: settling and balances are refused, not faked
        let e = post(
            &base,
            &format!("/api/tickets/{}/settle", tickets[0]),
            json!({}),
        )
        .unwrap_err();
        assert!(e.contains("409"), "{e}");
        assert!(get(&base, "/api/balances").unwrap_err().contains("409"));
        // with no page directory, a page request is a 404
        assert!(client::get::<Value>(&format!("{base}/"))
            .unwrap_err()
            .contains("404"));
        gw.stop();
    }
}

// ---- a saved round, verified offline ---------------------------------------------------------

#[test]
fn a_saved_round_verifies_offline_and_every_tampering_is_caught() {
    use kalyptra_matcher::Finding;
    use kalyptra_node::bundle::{self, BundleJ};

    let c = start(3, 2);
    let group = client::wait_for_keys(&c.roster, Duration::from_secs(60)).unwrap();
    client::open_round(&c.roster, 1, MARKET, 2).unwrap();
    for (side, size, limit) in [
        (Side::Sell, 10, 4),
        (Side::Sell, 4, 5),
        (Side::Buy, 8, 6),
        (Side::Buy, 6, 5),
    ] {
        client::submit_order(&c.roster, 1, MARKET, side, size, limit).unwrap();
    }
    wait_gossip(&c, 1, 4);
    client::close_round(&c.roster, 1).unwrap();
    client::wait_final(&c.roster, 1, Duration::from_secs(60)).unwrap();

    // save it to a file, then shut every node down: what follows needs no network
    let roster = c.roster.clone();
    // outside the cluster's directory, which is deleted when the cluster is dropped
    let saved_dir = temp_dir();
    std::fs::create_dir_all(&saved_dir).unwrap();
    let path = saved_dir.join("round-1.json");
    kalyptra_node::files::write_json(&path, &bundle::export(&roster, 1).unwrap(), false).unwrap();
    let pk = kalyptra_node::wire::hx(group.public_key().compress().as_bytes());
    drop(c);
    let saved: BundleJ = kalyptra_node::files::read_json(&path).unwrap();
    let _ = std::fs::remove_dir_all(&saved_dir);

    let ok = bundle::verify(&saved, &roster, Some(&pk)).unwrap();
    assert!(ok.report.is_clean(), "{:?}", ok.report.findings);
    assert!(ok.pinned);
    assert_eq!(ok.report.recomputed.unwrap().price, Some(5));
    // without a pin it still passes, and says the key came from the file
    assert!(!bundle::verify(&saved, &roster, None).unwrap().pinned);

    // a wrong pin is refused outright
    assert!(bundle::verify(&saved, &roster, Some(&"00".repeat(32))).is_err());

    // edit the saved file: a different price in the published record
    let mut json: serde_json::Value = serde_json::to_value(&saved).unwrap();
    json["round"]["record"]["price"] = serde_json::json!(6);
    let forged: BundleJ = serde_json::from_value(json).unwrap();
    let report = bundle::verify(&forged, &roster, Some(&pk)).unwrap().report;
    assert!(report
        .findings
        .contains(&Finding::RecordMismatch { field: "price" }));

    // swap in another group key: real envelopes were sealed to the real one
    let other = start(3, 2);
    let other_group = client::wait_for_keys(&other.roster, Duration::from_secs(60)).unwrap();
    let mut swapped = saved.clone();
    swapped.group = kalyptra_node::wire::GroupJ::from_group(&other_group);
    // the roster and group must describe the same nodes, which they do: same ids and threshold
    let report = bundle::verify(&swapped, &roster, None).unwrap().report;
    assert!(
        report
            .findings
            .iter()
            .any(|f| matches!(f, Finding::BadEnvelope { .. })),
        "{:?}",
        report.findings
    );

    // drop every partial from the file: the orders cannot be opened
    let mut stripped: serde_json::Value = serde_json::to_value(&saved).unwrap();
    stripped["round"]["partials"] = serde_json::json!([]);
    let stripped: BundleJ = serde_json::from_value(stripped).unwrap();
    let report = bundle::verify(&stripped, &roster, Some(&pk))
        .unwrap()
        .report;
    assert!(report
        .findings
        .iter()
        .any(|f| matches!(f, Finding::CannotOpen { .. })));
}

// ---- settlement, with a stand-in for the Zcash wallet tool -----------------------------------
//
// These tests cover the protocol around settlement (who may hand in a part, when the leader
// assembles, what happens to a bad part) without a Zcash node. The real wallet tool and a real
// regtest chain are exercised by `scripts/demo-settle.sh`.

#[cfg(unix)]
mod settle {
    use super::*;
    use kalyptra_matcher::settlement::sign_part;
    use kalyptra_node::client::Settled;
    use kalyptra_node::wallet::Wallet;
    use kalyptra_node::wire::{hx, unhex_n, AssetsJ, SettleJ, SettlementJ, TicketJ};

    /// A stand-in for `zcash_tx_tool kalyptra-wallet`. `leg` writes a part naming its amounts;
    /// `assemble` reports any part containing `BAD` as not matching, else a fixed txid.
    const FAKE_TOOL: &str = r#"#!/bin/sh
shift
op=$1; shift
if [ "$op" = balances ]; then
  found=""; take=0
  for a in "$@"; do
    case "$a" in
      --asset) take=1;;
      --*) take=0;;
      *) if [ $take = 1 ]; then found="$found${found:+,}\"$a\":42"; fi;;
    esac
  done
  echo "KALYPTRA_RESULT {\"ok\":true,\"balances\":{$found}}"
  exit 0
fi
out=""; plan=""; send=""; recv=""
while [ $# -gt 0 ]; do
  case "$1" in
    --finalize) shift;;
    --out) out=$2; shift 2;;
    --plan) plan=$2; shift 2;;
    --send) send=$2; shift 2;;
    --recv) recv=$2; shift 2;;
    *) shift 2;;
  esac
done
case "$op" in
  issue)
    echo 'KALYPTRA_RESULT {"ok":true,"asset":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}';;
  balance)
    echo 'KALYPTRA_RESULT {"ok":true,"balance":42}';;
  view-key)
    echo 'KALYPTRA_RESULT {"ok":true,"fvk":"abcd"}';;
  leg)
    printf 'part:%s:%s' "$send" "$recv" > "$out"
    echo 'KALYPTRA_RESULT {"ok":true,"bytes":12}';;
  assemble)
    # one-shot markers next to the leader's work folder: a rejected block, or a crash
    d=$(dirname "$plan")
    if [ -f "$d.reject-once" ]; then
      rm "$d.reject-once"
      echo 'KALYPTRA_RESULT {"ok":false,"error":"the node rejected the settlement: Block rejected"}'
      exit 3
    fi
    if [ -f "$d.rpcfail-once" ]; then
      rm "$d.rpcfail-once"
      echo 'KALYPTRA_RESULT {"ok":false,"error":"the node rejected the settlement: connection reset by peer"}'
      exit 3
    fi
    if [ -f "$d.crash-once" ]; then
      rm "$d.crash-once"
      echo "thread main panicked: connection reset" >&2
      exit 101
    fi
    bad=""
    for f in "$(dirname "$plan")"/order-*.part; do
      if grep -q BAD "$f"; then
        n=$(basename "$f" .part); n=${n#order-}
        bad="$bad${bad:+,}\"order $n\""
      fi
    done
    if [ -n "$bad" ]; then
      echo "KALYPTRA_RESULT {\"ok\":false,\"error\":\"groups do not match their legs\",\"bad\":[$bad]}"
      exit 3
    fi
    echo 'KALYPTRA_RESULT {"ok":true,"txid":"ab12cd","height":7}';;
esac
"#;

    struct Env {
        c: Cluster,
        tool: PathBuf,
        assets: AssetsJ,
    }

    fn env() -> Env {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let tool = dir.join("fake-tool.sh");
        std::fs::write(&tool, FAKE_TOOL).unwrap();
        std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
        let assets: AssetsJ = [
            ("ACME".to_string(), "aa".repeat(32)),
            ("vUSD".to_string(), "bb".repeat(32)),
        ]
        .into_iter()
        .collect();
        let settle = SettleOptions {
            wallet: Wallet::new(&tool, dir.join("leader-wallet")),
            assets: assets.clone(),
            assets_file: None,
            takeover_after: Duration::from_millis(500),
        };
        let c = start_with(3, 2, dir, Some(settle));
        client::wait_for_keys(&c.roster, Duration::from_secs(60)).unwrap();
        Env { c, tool, assets }
    }

    /// Run a round with these orders; return each order's ticket.
    fn round(e: &Env, r: u64, orders: &[(Side, u64, u64)]) -> Vec<TicketJ> {
        client::open_round(&e.c.roster, r, MARKET, 2).unwrap();
        let tickets: Vec<TicketJ> = orders
            .iter()
            .map(|(side, size, limit)| {
                client::submit_order(&e.c.roster, r, MARKET, *side, *size, *limit)
                    .unwrap()
                    .1
            })
            .collect();
        wait_gossip(&e.c, r, orders.len());
        client::close_round(&e.c.roster, r).unwrap();
        assert!(matches!(
            client::wait_final(&e.c.roster, r, Duration::from_secs(60)).unwrap(),
            Outcome::Cleared(_)
        ));
        tickets
    }

    fn settle(e: &Env, t: &TicketJ, name: &str) -> Settled {
        let w = Wallet::new(&e.tool, e.c.dir.join(name));
        client::settle_order(&e.c.roster, t, &w, &e.assets, &e.c.dir.join(name)).unwrap()
    }

    fn view(e: &Env, r: u64, node: usize) -> SettlementJ {
        let url = &e.c.roster.nodes[node].url;
        client::get(&format!("{url}/v1/rounds/{r}/settlement")).unwrap()
    }

    #[test]
    fn a_round_whose_leader_never_saw_it_is_settled_by_the_next_node() {
        let mut e = env();
        let r = 5; // leader: node ids [1, 2, 3][5 % 3] = node 3; the next one is node 1
                   // node 3 is down from before the round opens, so it never learns the round at all
        let addr = e.c.handles[2].as_ref().unwrap().addr;
        e.c.handles[2].take().unwrap().stop();
        client::open_round(&e.c.roster, r, MARKET, 2).unwrap();
        let tickets: Vec<TicketJ> = [(Side::Sell, 6, 5), (Side::Buy, 6, 5)]
            .iter()
            .map(|(side, size, limit)| {
                client::submit_order(&e.c.roster, r, MARKET, *side, *size, *limit)
                    .unwrap()
                    .1
            })
            .collect();
        wait_envelopes(&e.c, r, 2);
        client::close_round(&e.c.roster, r).unwrap();
        assert!(matches!(
            client::wait_final(&e.c.roster, r, Duration::from_secs(60)).unwrap(),
            Outcome::Cleared(_)
        ));
        for (i, t) in tickets.iter().enumerate() {
            assert!(matches!(
                settle(&e, t, &format!("w{i}")),
                Settled::Handed { accepted: 2, .. }
            ));
        }
        // node 3 comes back, still without the round: the next node has already taken it up
        start_again(&mut e.c, 2, addr);
        let s = client::wait_settlement(&e.c.roster, r, Duration::from_secs(30)).unwrap();
        assert_eq!(s.status, "settled", "{s:?}");
        assert_eq!(s.leader, 1, "node 1, next in turn, took the round over");
        assert_eq!(s.txid.as_deref(), Some("ab12cd"));
        // and only one node assembled it
        assert_eq!(view(&e, r, 1).status, "collecting");
    }

    #[test]
    fn a_round_settles_once_every_filled_order_hands_in_its_part() {
        let e = env();
        let r = 3; // leader: node ids [1, 2, 3][3 % 3] = node 1
        let tickets = round(
            &e,
            r,
            &[
                (Side::Sell, 10, 4),
                (Side::Sell, 4, 5),
                (Side::Buy, 8, 6),
                (Side::Buy, 6, 5),
                (Side::Buy, 3, 1), // below the clearing price: not filled
            ],
        );
        let v = view(&e, r, 1);
        assert_eq!(v.leader, 1);
        assert_eq!(v.status, "collecting");
        assert_eq!(v.legs.len(), 4, "only filled orders have legs");
        assert!(v.legs.iter().all(|l| !l.received && l.quote == l.base * 5));

        assert!(matches!(
            settle(&e, &tickets[4], "w4"),
            Settled::NothingToSettle
        ));
        for (i, t) in tickets[..3].iter().enumerate() {
            match settle(&e, t, &format!("w{i}")) {
                Settled::Handed { accepted, .. } => assert_eq!(accepted, 3),
                Settled::NothingToSettle => panic!("order {i} was filled"),
            }
        }
        // one part missing: the leader waits
        std::thread::sleep(Duration::from_millis(300));
        let v = view(&e, r, 0);
        assert_eq!(v.status, "collecting");
        assert_eq!(v.legs.iter().filter(|l| l.received).count(), 3);

        settle(&e, &tickets[3], "w3");
        let s = client::wait_settlement(&e.c.roster, r, Duration::from_secs(30)).unwrap();
        assert_eq!(s.status, "settled");
        assert_eq!(s.txid.as_deref(), Some("ab12cd"));
        assert_eq!(s.height, Some(7));
        assert!(s.legs.iter().all(|l| l.received));

        // the leader received the exact terms: a seller's part sends base, receives quote
        let parts = std::fs::read_dir(e.c.dir.join(format!("settle-node1-round{r}")))
            .unwrap()
            .filter_map(|f| f.ok())
            .filter(|f| f.file_name().to_string_lossy().ends_with(".part"))
            .count();
        assert_eq!(parts, 4);

        // after settlement no part is accepted
        let late = client::post::<_, serde_json::Value>(
            &format!("{}/v1/rounds/{r}/settle", e.c.roster.nodes[0].url),
            &SettleJ {
                order: s.legs[0].order,
                part: "00".into(),
                sig: "00".repeat(64),
            },
        );
        assert!(late.is_err());

        // the next node in turn (node 2) held every part too, but the leader settled: past the
        // takeover wait, node 2 has not assembled anything
        std::thread::sleep(Duration::from_millis(1500));
        assert_eq!(view(&e, r, 1).status, "collecting");
        assert!(!e.c.dir.join(format!("settle-node2-round{r}")).exists());
    }

    #[test]
    fn a_company_is_listed_and_a_round_settled_through_the_gateway() {
        use serde_json::json;
        let e = env();
        let investor = Wallet::new(&e.tool, e.c.dir.join("investor"));
        let acme = e.assets["ACME"].clone();
        let vusd = e.assets["vUSD"].clone();
        let other = Wallet::new(&e.tool, e.c.dir.join("other"));
        let (gw, base) = super::gw::start_gateway(
            &e.c,
            vec![("alice", investor), ("bob", other)],
            &[("ACME", &acme), ("vUSD", &vusd)],
        );
        let st = super::gw::get(&base, "/api/status").unwrap();
        assert_eq!(st["wallets"], json!(["alice", "bob"]));
        assert_eq!(st["default"], "alice");

        // list a company: issue (here a stand-in), add it to the assets, and persist
        let listed = super::gw::post(
            &base,
            "/api/assets",
            json!({ "symbol": "NOVA", "amount": 1000, "finalize": true }),
        )
        .unwrap();
        assert_eq!(listed["symbol"], "NOVA");
        assert_eq!(listed["finalized"], true);
        assert_eq!(listed["asset_id"], "cc".repeat(32));
        // the nodes signed it: every node's own list shows NOVA, signed by that node
        assert_eq!(listed["signed_by"].as_array().unwrap().len(), 3);
        assert_eq!(listed["needed"], 2);
        let roster = e.c.roster.roster().unwrap();
        for n in &e.c.roster.nodes {
            let list: Vec<kalyptra_node::wire::ListingSigJ> =
                client::get(&format!("{}/v1/listings", n.url)).unwrap();
            let nova = list.iter().find(|l| l.symbol == "NOVA").expect("listed");
            assert_eq!(nova.node, n.id);
            assert_eq!(nova.asset_id, "cc".repeat(32));
            nova.verify(&roster).unwrap();
        }
        // a node will not list the same symbol as another asset, nor the asset as another symbol
        let node_url = format!("{}/v1/listings", e.c.roster.nodes[0].url);
        let clash = client::post::<_, serde_json::Value>(
            &node_url,
            &json!({ "symbol": "NOVA", "asset_id": "dd".repeat(32) }),
        );
        assert!(clash.unwrap_err().contains("409"));
        let reuse = client::post::<_, serde_json::Value>(
            &node_url,
            &json!({ "symbol": "OTHER", "asset_id": "cc".repeat(32) }),
        );
        assert!(reuse.unwrap_err().contains("409"));
        // asking again is answered the same way, and a bad symbol is refused
        let again = client::post::<_, serde_json::Value>(
            &node_url,
            &json!({ "symbol": "NOVA", "asset_id": "cc".repeat(32) }),
        )
        .unwrap();
        assert_eq!(again["node"], 1);
        let bad = client::post::<_, serde_json::Value>(
            &node_url,
            &json!({ "symbol": "no way", "asset_id": "ee".repeat(32) }),
        );
        assert!(bad.unwrap_err().contains("400"));
        let assets = super::gw::get(&base, "/api/assets").unwrap();
        assert_eq!(assets["NOVA"], "cc".repeat(32));
        let saved = std::fs::read_to_string(e.c.dir.join("assets.json")).unwrap();
        assert!(saved.contains("NOVA"), "the listing was not saved");
        // refusals
        let dup = super::gw::post(
            &base,
            "/api/assets",
            json!({ "symbol": "NOVA", "amount": 5 }),
        );
        assert!(dup.unwrap_err().contains("409"));
        let bad = super::gw::post(
            &base,
            "/api/assets",
            json!({ "symbol": "no way", "amount": 5 }),
        );
        assert!(bad.unwrap_err().contains("400"));
        let zero = super::gw::post(
            &base,
            "/api/assets",
            json!({ "symbol": "ZERO", "amount": 0 }),
        );
        assert!(zero.unwrap_err().contains("400"));
        let b = super::gw::get(&base, "/api/balances").unwrap();
        assert_eq!(b["ACME"], 42);
        assert_eq!(b["NOVA"], 42);
        assert_eq!(
            super::gw::get(&base, "/api/viewkey").unwrap()["fvk"],
            "abcd"
        );

        // a round, then every investor settles through the gateway
        let (id, tickets) =
            super::gw::run_round(&base, &e.c, Some(3), &[("sell", 5, 3), ("buy", 5, 4)]);
        assert_eq!(id, 3);
        // the orders were placed as the default wallet (alice): bob sees none of them, and
        // cannot settle them
        let bobs = super::gw::get(&base, "/api/tickets?as=bob").unwrap();
        assert_eq!(bobs.as_array().unwrap().len(), 0);
        let e404 = super::gw::post(
            &base,
            &format!("/api/tickets/{}/settle?as=bob", tickets[0]),
            json!({}),
        );
        assert!(e404.unwrap_err().contains("404"));
        assert!(
            super::gw::get(&base, "/api/tickets?as=carol").is_ok(),
            "an unknown wallet just has no tickets"
        );
        assert!(super::gw::get(&base, "/api/balances?as=carol")
            .unwrap_err()
            .contains("404"));
        assert!(super::gw::get(&base, "/api/tickets?as=../x")
            .unwrap_err()
            .contains("400"));
        for t in &tickets {
            let r = super::gw::post(&base, &format!("/api/tickets/{t}/settle"), json!({})).unwrap();
            assert_eq!(r["settled"], true, "{r}");
            assert_eq!(r["handed_to"], 3);
        }
        let done = super::gw::until("settlement", || {
            let r = super::gw::get(&base, "/api/rounds/3").ok()?;
            (r["settlement"]["status"] == "settled").then_some(r)
        });
        assert_eq!(done["settlement"]["txid"], "ab12cd");
        assert_eq!(done["check"]["clean"], true);
        // nothing to retry once settled: the leader's 409 reaches the page as a 409
        let again = super::gw::post(&base, "/api/rounds/3/settle/retry", json!({})).unwrap_err();
        assert!(
            again.contains("409") && again.contains("nothing to retry"),
            "{again}"
        );
        gw.stop();
    }

    #[test]
    fn an_order_the_wallet_cannot_pay_for_is_refused() {
        use serde_json::json;
        let e = env();
        let investor = Wallet::new(&e.tool, e.c.dir.join("investor"));
        let acme = e.assets["ACME"].clone();
        let vusd = e.assets["vUSD"].clone();
        let (gw, base) = super::gw::start_gateway(
            &e.c,
            vec![("alice", investor)],
            &[("ACME", &acme), ("vUSD", &vusd)],
        );
        // the stand-in wallet holds 42 of every asset
        super::gw::post(
            &base,
            "/api/rounds",
            json!({ "round_id": 6, "market": super::MARKET }),
        )
        .unwrap();
        super::gw::post(
            &base,
            "/api/rounds",
            json!({ "round_id": 7, "market": super::MARKET }),
        )
        .unwrap();
        let order_in = |round: u64, side: &str, size: u64, limit: u64| {
            super::gw::post(
                &base,
                &format!("/api/rounds/{round}/orders"),
                json!({ "market": super::MARKET, "side": side, "size": size, "limit": limit }),
            )
        };
        let order = |side: &str, size: u64, limit: u64| order_in(6, side, size, limit);
        // more than it holds, in either asset
        let e1 = order("sell", 43, 3).unwrap_err();
        assert!(e1.contains("422") && e1.contains("ACME"), "{e1}");
        let e2 = order("buy", 5, 9).unwrap_err();
        assert!(e2.contains("422") && e2.contains("vUSD"), "{e2}");
        // within its means, and then the same funds cannot be promised twice
        assert_eq!(order("sell", 30, 3).unwrap()["counted"], true);
        // a second order spending the same asset in the same round would spend the same note
        // twice, so it is refused outright
        let e4 = order("sell", 5, 3).unwrap_err();
        assert!(e4.contains("409") && e4.contains("same note"), "{e4}");
        // the other asset is a separate pool, so a buy in the same round is fine
        assert_eq!(order("buy", 6, 7).unwrap()["counted"], true);
        // in another open round the 30 already promised still count
        let e3 = order_in(7, "sell", 13, 3).unwrap_err();
        assert!(
            e3.contains("422") && e3.contains("already committed"),
            "{e3}"
        );
        // and the exact remainder is allowed
        assert_eq!(order_in(7, "sell", 12, 3).unwrap()["counted"], true);
        gw.stop();
    }

    /// A "node" that answers every listing with a signature made by the wrong key.
    fn lying_node(real: &kalyptra_node::wire::RosterNodeJ) -> String {
        use kalyptra_node::wire::{hx, listing_msg, unhex_n, ListingJ, ListingSigJ};
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr().to_ip().unwrap());
        let id = real.id;
        std::thread::spawn(move || {
            let wrong = ed25519_dalek::SigningKey::generate(&mut OsRng);
            for mut req in server.incoming_requests() {
                let mut body = String::new();
                std::io::Read::read_to_string(req.as_reader(), &mut body).unwrap();
                let reply = match serde_json::from_str::<ListingJ>(&body) {
                    Ok(l) => {
                        let msg = listing_msg(&l.symbol, &unhex_n::<32>(&l.asset_id).unwrap());
                        let sig = ed25519_dalek::Signer::sign(&wrong, &msg);
                        serde_json::to_string(&ListingSigJ {
                            symbol: l.symbol,
                            asset_id: l.asset_id,
                            node: id,
                            sig: hx(&sig.to_bytes()),
                        })
                        .unwrap()
                    }
                    Err(_) => "[]".to_string(),
                };
                let _ = req.respond(tiny_http::Response::from_string(reply));
            }
        });
        url
    }

    #[test]
    fn a_listing_signature_that_does_not_verify_is_not_counted() {
        use serde_json::json;
        let e = env();
        let acme = e.assets["ACME"].clone();
        let vusd = e.assets["vUSD"].clone();
        let mut roster = e.c.roster.clone();
        roster.nodes[2].url = lying_node(&e.c.roster.nodes[2]);
        let (gw, base) = super::gw::start_gateway_with(
            &e.c,
            roster,
            vec![("alice", Wallet::new(&e.tool, e.c.dir.join("investor")))],
            &[("ACME", &acme), ("vUSD", &vusd)],
        );
        let listed = super::gw::post(
            &base,
            "/api/assets",
            json!({ "symbol": "NOVA", "amount": 10 }),
        )
        .unwrap();
        // two honest signatures are enough (t = 2); the wrong-key one is not among them
        assert_eq!(listed["signed_by"], json!([1, 2]));
        gw.stop();
    }

    #[test]
    fn a_damaged_pending_file_stops_a_listing_instead_of_issuing_twice() {
        use serde_json::json;
        let e = env();
        let acme = e.assets["ACME"].clone();
        let vusd = e.assets["vUSD"].clone();
        let (gw, base) = super::gw::start_gateway(
            &e.c,
            vec![("alice", Wallet::new(&e.tool, e.c.dir.join("investor")))],
            &[("ACME", &acme), ("vUSD", &vusd)],
        );
        let pending = e.c.dir.join("gateway").join("pending-listings.json");
        std::fs::create_dir_all(pending.parent().unwrap()).unwrap();
        std::fs::write(&pending, "{ half a fi").unwrap();
        let err = super::gw::post(
            &base,
            "/api/assets",
            json!({ "symbol": "NOVA", "amount": 5 }),
        )
        .unwrap_err();
        assert!(
            err.contains("502") && err.contains("pending-listings"),
            "{err}"
        );
        // nothing was issued or listed, and the damaged file is left for a person to look at
        assert_eq!(std::fs::read_to_string(&pending).unwrap(), "{ half a fi");
        assert!(super::gw::get(&base, "/api/assets")
            .unwrap()
            .get("NOVA")
            .is_none());
        gw.stop();
    }

    #[test]
    fn two_orders_arriving_together_cannot_both_spend_the_same_funds() {
        use serde_json::json;
        let e = env();
        let acme = e.assets["ACME"].clone();
        let vusd = e.assets["vUSD"].clone();
        let (gw, base) = super::gw::start_gateway(
            &e.c,
            vec![("alice", Wallet::new(&e.tool, e.c.dir.join("investor")))],
            &[("ACME", &acme), ("vUSD", &vusd)],
        );
        super::gw::post(
            &base,
            "/api/rounds",
            json!({ "round_id": 8, "market": super::MARKET }),
        )
        .unwrap();
        // the stand-in wallet holds 42 ACME: two sells of 30 cannot both be accepted
        let handles: Vec<_> = (0..6)
            .map(|_| {
                let base = base.clone();
                std::thread::spawn(move || {
                    super::gw::post(
                        &base,
                        "/api/rounds/8/orders",
                        json!({ "market": super::MARKET, "side": "sell", "size": 30, "limit": 3 }),
                    )
                })
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let accepted = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(accepted, 1, "{results:?}");
        gw.stop();
    }

    #[test]
    fn a_failed_or_unknown_settlement_can_be_retried_by_the_leader() {
        let e = env();
        let r = 6; // leader: node ids [1, 2, 3][6 % 3] = node 1
        let work = e.c.dir.join(format!("settle-node1-round{r}"));
        let marker = |kind: &str| {
            let mut p = work.clone().into_os_string();
            p.push(format!(".{kind}-once"));
            std::fs::write(std::path::PathBuf::from(p), "").unwrap();
        };
        let retry = || {
            client::post::<_, serde_json::Value>(
                &format!("{}/v1/rounds/{r}/settle/retry", e.c.roster.nodes[0].url),
                &serde_json::json!({}),
            )
        };
        let tickets = round(&e, r, &[(Side::Sell, 5, 3), (Side::Buy, 5, 4)]);
        // nothing has failed yet: there is nothing to retry, and only the leader may
        assert!(retry().unwrap_err().contains("409"));
        let not_leader = client::post::<_, serde_json::Value>(
            &format!("{}/v1/rounds/{r}/settle/retry", e.c.roster.nodes[1].url),
            &serde_json::json!({}),
        );
        assert!(not_leader.unwrap_err().contains("not the leader"));

        // 1. the node rejects the block: a definite failure, nothing moved
        marker("reject");
        for (i, t) in tickets.iter().enumerate() {
            settle(&e, t, &format!("w{i}"));
        }
        let s = client::wait_settlement(&e.c.roster, r, Duration::from_secs(30)).unwrap();
        assert_eq!(s.status, "failed", "{s:?}");
        assert!(s.error.as_deref().unwrap().contains("Block rejected"));

        // 2. retried, and this time the tool crashes: the outcome is unknown, not "failed"
        marker("crash");
        assert_eq!(retry().unwrap()["leader"], 1);
        let s = client::wait_settlement(&e.c.roster, r, Duration::from_secs(30)).unwrap();
        assert_eq!(s.status, "unknown", "{s:?}");
        assert!(s.error.as_deref().unwrap().contains("no result"));

        // 3. retried again with the parts already held: settled
        retry().unwrap();
        let s = client::wait_settlement(&e.c.roster, r, Duration::from_secs(30)).unwrap();
        assert_eq!(s.status, "settled", "{s:?}");
        assert_eq!(s.txid.as_deref(), Some("ab12cd"));
        // and a settled round cannot be retried
        assert!(retry().unwrap_err().contains("409"));
    }

    #[test]
    fn a_rejection_after_an_unknown_attempt_stays_unknown() {
        let e = env();
        let r = 9; // leader: node ids [1, 2, 3][9 % 3] = node 1
        let work = e.c.dir.join(format!("settle-node1-round{r}"));
        let marker = |kind: &str| {
            let mut p = work.clone().into_os_string();
            p.push(format!(".{kind}-once"));
            std::fs::write(std::path::PathBuf::from(p), "").unwrap();
        };
        let retry = || {
            client::post::<_, serde_json::Value>(
                &format!("{}/v1/rounds/{r}/settle/retry", e.c.roster.nodes[0].url),
                &serde_json::json!({}),
            )
        };
        let tickets = round(&e, r, &[(Side::Sell, 5, 3), (Side::Buy, 5, 4)]);
        // the first attempt fails without the node saying "Block rejected" (a dropped connection):
        // it may have landed
        marker("rpcfail");
        for (i, t) in tickets.iter().enumerate() {
            settle(&e, t, &format!("w{i}"));
        }
        let s = client::wait_settlement(&e.c.roster, r, Duration::from_secs(30)).unwrap();
        assert_eq!(s.status, "unknown", "{s:?}");
        // the retry is rejected, as it would be if the first attempt had spent the notes: that
        // proves nothing moved only if no earlier attempt could have landed, so it stays unknown
        marker("reject");
        retry().unwrap();
        let s = client::wait_settlement(&e.c.roster, r, Duration::from_secs(30)).unwrap();
        assert_eq!(s.status, "unknown", "{s:?}");
        assert!(
            s.error
                .as_deref()
                .unwrap()
                .contains("may already have settled"),
            "{s:?}"
        );
    }

    #[test]
    fn an_unreadable_saved_ticket_stops_new_orders_instead_of_under_counting() {
        use serde_json::json;
        let e = env();
        let acme = e.assets["ACME"].clone();
        let vusd = e.assets["vUSD"].clone();
        let (gw, base) = super::gw::start_gateway(
            &e.c,
            vec![("alice", Wallet::new(&e.tool, e.c.dir.join("investor")))],
            &[("ACME", &acme), ("vUSD", &vusd)],
        );
        super::gw::post(
            &base,
            "/api/rounds",
            json!({ "round_id": 10, "market": super::MARKET }),
        )
        .unwrap();
        let tickets = e.c.dir.join("gateway").join("tickets").join("alice");
        std::fs::create_dir_all(&tickets).unwrap();
        std::fs::write(tickets.join("r10-broken.json"), "{ not a ticket").unwrap();
        let err = super::gw::post(
            &base,
            "/api/rounds/10/orders",
            json!({ "market": super::MARKET, "side": "sell", "size": 1, "limit": 3 }),
        )
        .unwrap_err();
        assert!(
            err.contains("503") && err.contains("cannot be read"),
            "{err}"
        );
        // the page is told too, instead of being shown a shorter list
        assert!(super::gw::get(&base, "/api/tickets")
            .unwrap_err()
            .contains("503"));
        gw.stop();
    }

    #[test]
    fn only_the_orders_owner_can_hand_in_its_part() {
        let e = env();
        let r = 4;
        let tickets = round(&e, r, &[(Side::Sell, 5, 3), (Side::Buy, 5, 4)]);
        let record = client::verified_record(&e.c.roster, r).unwrap();
        let order = record
            .set
            .iter()
            .position(|h| hx(h) == tickets[0].envelope)
            .unwrap() as u32;
        let part = b"part:5:15".to_vec();
        let post = |key: &ed25519_dalek::SigningKey, order: u32| {
            client::post::<_, serde_json::Value>(
                &format!("{}/v1/rounds/{r}/settle", e.c.roster.nodes[1].url),
                &SettleJ {
                    order,
                    part: hx(&part),
                    sig: hx(&sign_part(key, &record, order, &part)),
                },
            )
        };
        let stranger = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let err = post(&stranger, order).unwrap_err();
        assert!(err.contains("403"), "{err}");
        // the other order's owner cannot hand in a part for this order either
        let other = ed25519_dalek::SigningKey::from_bytes(&unhex_n(&tickets[1].key).unwrap());
        assert!(post(&other, order).unwrap_err().contains("403"));
        // the owner can
        let owner = ed25519_dalek::SigningKey::from_bytes(&unhex_n(&tickets[0].key).unwrap());
        post(&owner, order).unwrap();
        assert_eq!(view(&e, r, 1).legs.iter().filter(|l| l.received).count(), 1);
        // a ticket's envelope hash is how its owner finds the order
        assert_eq!(hx(&record.set[order as usize]), tickets[0].envelope);
    }

    #[test]
    fn a_part_that_does_not_match_is_named_dropped_and_can_be_replaced() {
        let e = env();
        let r = 5; // leader: node 3
        let tickets = round(&e, r, &[(Side::Sell, 5, 3), (Side::Buy, 5, 4)]);
        settle(&e, &tickets[0], "s");
        // the buyer hands in a part that does not match its leg
        let record = client::verified_record(&e.c.roster, r).unwrap();
        let order = record
            .set
            .iter()
            .position(|h| hx(h) == tickets[1].envelope)
            .unwrap() as u32;
        let key = ed25519_dalek::SigningKey::from_bytes(&unhex_n(&tickets[1].key).unwrap());
        let part = b"BAD part".to_vec();
        for n in &e.c.roster.nodes {
            client::post::<_, serde_json::Value>(
                &format!("{}/v1/rounds/{r}/settle", n.url),
                &SettleJ {
                    order,
                    part: hx(&part),
                    sig: hx(&sign_part(&key, &record, order, &part)),
                },
            )
            .unwrap();
        }
        let s = client::wait_settlement(&e.c.roster, r, Duration::from_secs(30)).unwrap();
        assert_eq!(s.leader, 3);
        assert_eq!(s.status, "failed");
        assert_eq!(s.bad, vec![order], "the bad part is named");
        assert!(s.txid.is_none());
        // the bad part was dropped, the good one kept; an honest part then settles the round
        let v = view(&e, r, 2);
        assert_eq!(v.legs.iter().filter(|l| l.received).count(), 1);
        settle(&e, &tickets[1], "b");
        let s = client::wait_settlement(&e.c.roster, r, Duration::from_secs(30)).unwrap();
        assert_eq!(s.status, "settled");
    }
}
