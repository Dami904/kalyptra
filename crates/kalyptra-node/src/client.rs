//! Client side: what an investor, an operator and a checker do against running nodes.

use std::path::Path;
use std::time::{Duration, Instant};

use ed25519_dalek::SigningKey;
use kalyptra_matcher::round::envelope_hash;
use kalyptra_matcher::settlement::{leader, legs, market_assets, sign_part, Leg};
use kalyptra_matcher::{
    check as check_round, round_aad, CheckReport, Order, Receipt, RoundRecord, Side,
};
use kalyptra_tse::{seal, GroupKey};
use rand_core::{OsRng, RngCore};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::wallet::Wallet;
use crate::wire::{
    hx, parse_side, unhex_n, AssetsJ, ConfigJ, ErrorJ, GroupJ, PublicRoundJ, ReceiptJ, RecordJ,
    RosterJ, RoundStatusJ, SettleJ, SettlementJ, StatusJ, SubmitJ, TicketJ, WireResult,
};

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build()
}

fn err_text(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            let msg = serde_json::from_str::<ErrorJ>(&body)
                .map(|e| e.error)
                .unwrap_or(body);
            format!("HTTP {code}: {msg}")
        }
        other => other.to_string(),
    }
}

pub fn get<T: DeserializeOwned>(url: &str) -> WireResult<T> {
    agent()
        .get(url)
        .call()
        .map_err(err_text)?
        .into_json()
        .map_err(|e| e.to_string())
}

pub fn post<B: Serialize, T: DeserializeOwned>(url: &str, body: &B) -> WireResult<T> {
    agent()
        .post(url)
        .send_json(serde_json::to_value(body).map_err(|e| e.to_string())?)
        .map_err(err_text)?
        .into_json()
        .map_err(|e| e.to_string())
}

/// The group key, as reported by the nodes. Requires at least `t` nodes to answer, and every
/// answering node to report exactly the same key, so one lying node cannot hand an investor a
/// key it controls.
pub fn group_key(roster: &RosterJ) -> WireResult<GroupKey> {
    let mut seen: Option<GroupJ> = None;
    let mut count = 0;
    for url in roster.urls() {
        let Ok(s) = get::<StatusJ>(&format!("{url}/v1/status")) else {
            continue;
        };
        let Some(g) = s.group else { continue };
        match &seen {
            None => seen = Some(g),
            Some(prev) if *prev != g => {
                return Err("nodes disagree on the group key: refusing to seal".into())
            }
            Some(_) => {}
        }
        count += 1;
    }
    if count < roster.threshold as usize {
        return Err(format!(
            "only {count} nodes report a group key; need at least {}",
            roster.threshold
        ));
    }
    seen.expect("count > 0").to_group()
}

/// Wait until every node has finished key generation.
pub fn wait_for_keys(roster: &RosterJ, timeout: Duration) -> WireResult<GroupKey> {
    let start = Instant::now();
    loop {
        let ready = roster
            .urls()
            .iter()
            .filter(|u| {
                get::<StatusJ>(&format!("{u}/v1/status"))
                    .map(|s| s.dkg_done)
                    .unwrap_or(false)
            })
            .count();
        if ready == roster.nodes.len() {
            return group_key(roster);
        }
        if start.elapsed() > timeout {
            return Err(format!(
                "key generation not finished: {ready} of {} nodes ready",
                roster.nodes.len()
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

pub fn open_round(
    roster: &RosterJ,
    round_id: u64,
    market: &str,
    min_orders: usize,
) -> WireResult<()> {
    let c = ConfigJ {
        round_id,
        market: market.into(),
        min_orders,
    };
    to_every_node(roster, "open the round", |url| {
        post::<_, serde_json::Value>(&format!("{url}/v1/rounds"), &c).map(|_| ())
    })
}

/// Send an operator command to every node, and succeed when at least `t` took it: one node
/// being down must not stop a round that the other `t` can run. Fewer than `t` is an error
/// that names each node that did not take it.
fn to_every_node(
    roster: &RosterJ,
    what: &str,
    send: impl Fn(&str) -> WireResult<()>,
) -> WireResult<()> {
    let urls = roster.urls();
    let mut failed = Vec::new();
    for url in &urls {
        if let Err(e) = send(url) {
            failed.push(format!("{url}: {e}"));
        }
    }
    let ok = urls.len() - failed.len();
    if ok >= roster.threshold as usize {
        Ok(())
    } else {
        Err(format!(
            "could not {what}: only {ok} of {} nodes took it (need {}): {}",
            urls.len(),
            roster.threshold,
            failed.join("; ")
        ))
    }
}

/// Sign an order with a FRESH key, seal it to the group for this round, and submit it to every
/// node. Returns the valid receipts (fewer than `t` means the order may not count) and the
/// ticket needed to settle the order later. The ticket holds the order's key: keep it secret.
/// A sealed order, not yet sent: the ticket (with the order's one-time key) and the envelope.
pub struct SealedOrder {
    pub ticket: TicketJ,
    pub envelope: Vec<u8>,
}

/// What sending a sealed order produced.
pub struct Sent {
    /// Valid receipts from roster nodes.
    pub receipts: Vec<Receipt>,
    /// Nodes that answered with a refusal (an HTTP error status): those definitely did not take
    /// the order. Nodes that did not answer at all are counted in neither.
    pub refused: usize,
}

/// Seal an order to the nodes' group key with a fresh one-time key. Nothing is sent, so the
/// caller can save the ticket first: if sending is cut off, the key is not lost.
pub fn seal_order(
    roster: &RosterJ,
    round_id: u64,
    market: &str,
    side: Side,
    size: u64,
    limit: u64,
) -> WireResult<SealedOrder> {
    let group = group_key(roster)?;
    let key = SigningKey::generate(&mut OsRng);
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let order = Order {
        side,
        size,
        limit,
        round_id,
        market: market.into(),
        salt,
        investor: key.verifying_key().to_bytes(),
    };
    let signed = order.sign(&key).map_err(|e| e.to_string())?;
    let env = seal(
        group.public_key(),
        &round_aad(round_id, market),
        &signed,
        &mut OsRng,
    );
    let env_bytes = env.to_bytes();
    let ticket = TicketJ {
        round_id,
        market: market.into(),
        side: match side {
            Side::Buy => "buy".into(),
            Side::Sell => "sell".into(),
        },
        size,
        limit,
        envelope: hx(&envelope_hash(&env_bytes)),
        key: hx(key.as_bytes()),
    };
    Ok(SealedOrder {
        ticket,
        envelope: env_bytes,
    })
}

/// Send a sealed order to every node and collect the valid receipts.
pub fn send_order(roster: &RosterJ, round_id: u64, envelope: &[u8]) -> WireResult<Sent> {
    let checker = roster.roster()?;
    let body = SubmitJ {
        envelope: hx(envelope),
    };
    let mut receipts = Vec::new();
    let mut refused = 0;
    for url in roster.urls() {
        match post::<_, ReceiptJ>(&format!("{url}/v1/rounds/{round_id}/submit"), &body) {
            Ok(rj) => {
                if let Ok(r) = rj.receipt() {
                    if r.verify(&checker).is_ok() {
                        receipts.push(r);
                    }
                }
            }
            Err(e) if e.starts_with("HTTP ") => refused += 1,
            Err(_) => {}
        }
    }
    Ok(Sent { receipts, refused })
}

/// Seal and send in one step (the command line). The gateway seals, saves the ticket, then sends.
pub fn submit_order(
    roster: &RosterJ,
    round_id: u64,
    market: &str,
    side: Side,
    size: u64,
    limit: u64,
) -> WireResult<(Vec<Receipt>, TicketJ)> {
    let s = seal_order(roster, round_id, market, side, size, limit)?;
    let sent = send_order(roster, round_id, &s.envelope)?;
    Ok((sent.receipts, s.ticket))
}

pub fn close_round(roster: &RosterJ, round_id: u64) -> WireResult<()> {
    to_every_node(roster, "close the round", |url| {
        post::<_, serde_json::Value>(
            &format!("{url}/v1/rounds/{round_id}/close"),
            &serde_json::json!({}),
        )
        .map(|_| ())
    })
}

pub enum Outcome {
    Cleared(RecordJ),
    Voided,
}

/// Wait until `t` nodes report the same final record, or the round is voided.
pub fn wait_final(roster: &RosterJ, round_id: u64, timeout: Duration) -> WireResult<Outcome> {
    let start = Instant::now();
    loop {
        let statuses: Vec<RoundStatusJ> = roster
            .urls()
            .iter()
            .filter_map(|u| get(&format!("{u}/v1/rounds/{round_id}")).ok())
            .collect();
        if statuses.iter().filter(|s| s.phase == "voided").count() >= roster.threshold as usize {
            return Ok(Outcome::Voided);
        }
        let records: Vec<&RecordJ> = statuses.iter().filter_map(|s| s.record.as_ref()).collect();
        if let Some(r) = records.first() {
            if records.iter().filter(|x| **x == *r).count() >= roster.threshold as usize {
                return Ok(Outcome::Cleared((*r).clone()));
            }
        }
        if start.elapsed() > timeout {
            return Err(format!("round {round_id} not final after {timeout:?}"));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Every node's view of a round, merged. Trusts no node: a node that answers with something
/// wrong shows up as a finding when the checker runs on the merged data.
pub fn fetch_public(roster: &RosterJ, round_id: u64) -> WireResult<kalyptra_matcher::PublicRound> {
    let mut merged: Option<kalyptra_matcher::PublicRound> = None;
    for url in roster.urls() {
        let Ok(p) = get::<PublicRoundJ>(&format!("{url}/v1/rounds/{round_id}/public")) else {
            continue;
        };
        let p = p.public()?;
        match merged.as_mut() {
            None => merged = Some(p),
            Some(m) => {
                m.envelopes.extend(p.envelopes);
                m.receipts.extend(p.receipts);
                m.commit_votes.extend(p.commit_votes);
                m.partials.extend(p.partials);
                m.record_votes.extend(p.record_votes);
                if m.record.is_none() {
                    m.record = p.record;
                }
            }
        }
    }
    let mut m = merged.ok_or("no node returned the round")?;
    m.envelopes.sort();
    m.envelopes.dedup();
    m.receipts.sort_by_key(|r| (r.envelope, r.node));
    m.receipts.dedup();
    m.commit_votes.sort_by_key(|v| (v.epoch, v.node));
    m.commit_votes.dedup();
    m.record_votes.sort_by_key(|v| v.node);
    m.record_votes.dedup();
    Ok(m)
}

/// Fetch the round from the nodes and run the public checker on it.
pub fn check(roster: &RosterJ, round_id: u64) -> WireResult<CheckReport> {
    let group = group_key(roster)?;
    let checker = roster.roster()?;
    let m = fetch_public(roster, round_id)?;
    Ok(check_round(&m, &group, &checker))
}

/// The round's record, only if the public checker passes on it: the investor settles against
/// a record it has verified itself, not one a node handed it.
pub fn verified_record(roster: &RosterJ, round_id: u64) -> WireResult<RoundRecord> {
    let report = check(roster, round_id)?;
    if !report.is_clean() {
        return Err(format!(
            "round {round_id} does not pass the public check: {:?}",
            report.findings
        ));
    }
    report
        .recomputed
        .ok_or_else(|| format!("round {round_id} has no record (voided?)"))
}

pub enum Settled {
    /// The order was not filled: there is nothing to settle and nothing moves.
    NothingToSettle,
    /// The order's part was built and handed to `accepted` nodes.
    Handed { leg: Leg, accepted: usize },
}

/// Settle one filled order: verify the round, find this order in it, build this wallet's part
/// for exactly the record's terms, sign it with the order's key, and hand it to every node.
pub fn settle_order(
    roster: &RosterJ,
    ticket: &TicketJ,
    wallet: &Wallet,
    assets: &AssetsJ,
    work: &Path,
) -> WireResult<Settled> {
    let record = verified_record(roster, ticket.round_id)?;
    let key = SigningKey::from_bytes(&unhex_n::<32>(&ticket.key)?);
    let env: [u8; 32] = unhex_n(&ticket.envelope)?;
    let order = record
        .set
        .iter()
        .position(|h| *h == env)
        .ok_or("this order is not in the committed set")? as u32;
    let mine = record.orders.iter().find(|(i, _)| *i == order);
    match mine {
        Some((_, o)) if o.owner == key.verifying_key().to_bytes() => {}
        _ => return Err("the record does not hold this ticket's order".into()),
    }
    if parse_side(&ticket.side)? != mine.expect("checked").1.side {
        return Err("the record's side differs from the ticket".into());
    }
    let Some(leg) = legs(&record)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|l| l.order == order)
    else {
        return Ok(Settled::NothingToSettle);
    };
    let (base_sym, quote_sym) = market_assets(&record.market).ok_or("bad market name")?;
    let id = |sym: &str| {
        assets
            .get(sym)
            .cloned()
            .ok_or_else(|| format!("no asset id for {sym}"))
    };
    let (base, quote) = (id(base_sym)?, id(quote_sym)?);
    let (sa, ra) = if leg.sends_base() {
        (&base, &quote)
    } else {
        (&quote, &base)
    };
    std::fs::create_dir_all(work).map_err(|e| e.to_string())?;
    let out = work.join(format!("round{}-order{order}.part", record.round_id));
    let part = wallet.leg(sa, leg.send_amount(), ra, leg.recv_amount(), &out)?;
    let body = SettleJ {
        order,
        part: hx(&part),
        sig: hx(&sign_part(&key, &record, order, &part)),
    };
    let mut accepted = 0;
    let mut errors = Vec::new();
    for url in roster.urls() {
        match post::<_, serde_json::Value>(
            &format!("{url}/v1/rounds/{}/settle", record.round_id),
            &body,
        ) {
            Ok(_) => accepted += 1,
            Err(e) => errors.push(format!("{url}: {e}")),
        }
    }
    if accepted == 0 {
        return Err(format!("no node accepted the part: {}", errors.join("; ")));
    }
    Ok(Settled::Handed { leg, accepted })
}

/// How a round's settlement stands, from the node settling it: the round's leader, or the next
/// node in turn when that one took it over (a leader that was down when the round opened never
/// learns the round). Both may have assembled; the chain lands at most one transaction, and
/// the other node then reports "failed". So the view that is furthest along wins, whichever
/// node holds it: a settled round always shows as settled.
pub fn settlement_view(roster: &RosterJ, round_id: u64) -> WireResult<SettlementJ> {
    let ids: Vec<u16> = roster.nodes.iter().map(|n| n.id).collect();
    let lead = leader(&ids, round_id).ok_or("empty roster")?;
    let next = leader(&ids, round_id.wrapping_add(1)).ok_or("empty roster")?;
    let ask = |id: u16| -> Option<SettlementJ> {
        let url = &roster.node(id).ok()?.url;
        get(&format!("{url}/v1/rounds/{round_id}/settlement")).ok()
    };
    furthest([lead, next].into_iter().filter_map(ask).collect())
        .ok_or_else(|| format!("neither node {lead} nor node {next} answers for round {round_id}"))
}

/// The view that is furthest along: settled (or nothing to settle), then unknown, failed,
/// assembling, collecting. On a tie, the first (the round's own leader).
fn furthest(views: Vec<SettlementJ>) -> Option<SettlementJ> {
    let rank = |s: &str| match s {
        "settled" | "nothing" => 5,
        "unknown" => 4,
        "failed" => 3,
        "assembling" => 2,
        _ => 1,
    };
    let mut best: Option<SettlementJ> = None;
    for v in views {
        if best
            .as_ref()
            .is_none_or(|b| rank(&v.status) > rank(&b.status))
        {
            best = Some(v);
        }
    }
    best
}

/// Wait for the node settling the round to report it settled or failed.
pub fn wait_settlement(
    roster: &RosterJ,
    round_id: u64,
    timeout: Duration,
) -> WireResult<SettlementJ> {
    let start = Instant::now();
    loop {
        if let Ok(s) = settlement_view(roster, round_id) {
            if matches!(
                s.status.as_str(),
                "settled" | "failed" | "unknown" | "nothing"
            ) {
                return Ok(s);
            }
            if start.elapsed() > timeout {
                return Ok(s);
            }
        } else if start.elapsed() > timeout {
            return settlement_view(roster, round_id);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::furthest;
    use crate::wire::SettlementJ;

    fn v(leader: u16, status: &str) -> SettlementJ {
        SettlementJ {
            round_id: 5,
            leader,
            legs: vec![],
            status: status.into(),
            txid: (status == "settled").then(|| "ab12".into()),
            height: None,
            bad: vec![],
            error: None,
        }
    }

    #[test]
    fn a_settled_view_wins_over_a_losing_nodes_failed_one() {
        // the leader lost the race (the chain refused its second spend); the backup settled
        let got = furthest(vec![v(3, "failed"), v(1, "settled")]).unwrap();
        assert_eq!((got.leader, got.status.as_str()), (1, "settled"));
        let got = furthest(vec![v(3, "unknown"), v(1, "settled")]).unwrap();
        assert_eq!(got.leader, 1);
        let got = furthest(vec![v(3, "assembling"), v(1, "failed")]).unwrap();
        assert_eq!(got.status, "failed");
        // a tie goes to the round's own leader, listed first
        let got = furthest(vec![v(3, "collecting"), v(1, "collecting")]).unwrap();
        assert_eq!(got.leader, 3);
        assert!(furthest(vec![]).is_none());
    }
}
