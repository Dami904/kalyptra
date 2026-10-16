//! The local gateway: a small web server that the Kalyptra page talks to.
//!
//! A web page cannot hold Zcash keys or run a wallet, so this program runs next to them, on the
//! investor's own machine. It serves the page, and wraps what the `kalyptra` command line already
//! does (open and close rounds, submit sealed orders, check a round, settle an order) as JSON
//! under `/api`.
//!
//! * **Keys stay here.** Order tickets (which hold each order's one-time key) are written to the
//!   gateway's home directory and are never sent to the page.
//! * **Several wallets, kept apart.** A demo machine can hold several investors' wallets. Each
//!   request names the one it acts as (`?as=alice`); each has its own tickets folder.
//! * **Wallet commands run one at a time per wallet.** Each rescans the chain and can take tens
//!   of seconds, and two of them on one wallet database would collide.
//! * **Nothing here is trusted by the nodes.** Every answer about a round comes from the nodes
//!   and is checked by the public checker before the page shows it as verified.
//!
//! Limits: plain HTTP on the local machine, no login. Run it only on a machine you control, and
//! keep it on `127.0.0.1`.

use std::collections::BTreeMap;
use std::io::Read;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use kalyptra_matcher::order::Side;
use kalyptra_matcher::settlement::{leader, market_assets};
use serde::Deserialize;
use serde_json::{json, Value};
use tiny_http::{Header, Method, Request, Response};

use crate::bundle;
use crate::client::{self, Settled};
use crate::files::{read_json, read_json_or_default, write_json};
use crate::wallet::Wallet;
use crate::wire::{
    parse_side, valid_symbol, AssetsJ, ListingJ, ListingSigJ, RecordJ, RosterJ, RoundStatusJ,
    RoundSummaryJ, SettlementJ, StatusJ, TicketJ,
};

pub struct GatewayOptions {
    pub roster: RosterJ,
    /// Where tickets and settlement work files are kept.
    pub home: PathBuf,
    /// The assets file (symbol to asset id); listing a company adds to it.
    pub assets_path: PathBuf,
    /// Investors' wallets, by name. The first is the default. With none the page can still show
    /// rounds and place orders, but not settle, list a company or show balances.
    pub wallets: Vec<(String, Wallet)>,
    /// A separate wallet for the Audit screen.
    pub auditor: Option<Wallet>,
    /// A built page to serve at `/`.
    pub ui_dir: Option<PathBuf>,
}

struct Inner {
    roster: RosterJ,
    home: PathBuf,
    assets_path: PathBuf,
    assets: Mutex<AssetsJ>,
    wallets: BTreeMap<String, Wallet>,
    /// Who a request acts as when it does not say.
    default_actor: String,
    auditor: Option<Wallet>,
    ui_dir: Option<PathBuf>,
    /// One lock per wallet (and one for the auditor), held for the length of a wallet command.
    busy: BTreeMap<String, Mutex<()>>,
    /// Held from the funds check to the ticket write, so two orders arriving together cannot
    /// both pass a check that only one of them could.
    order_lock: Mutex<()>,
    /// Held while a company is listed (issue, pending record, assets file).
    listing_lock: Mutex<()>,
}

/// The lock name used for the auditor's wallet.
const AUDITOR: &str = "~auditor";

pub struct GatewayHandle {
    pub addr: SocketAddr,
    server: Arc<tiny_http::Server>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl GatewayHandle {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.server.unblock();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

type ApiResult = Result<Value, (u16, String)>;

fn bad(msg: impl ToString) -> (u16, String) {
    (400, msg.to_string())
}

fn fail(msg: impl ToString) -> (u16, String) {
    (502, msg.to_string())
}

fn phase_rank(p: &str) -> u8 {
    match p {
        "open" => 0,
        "closed" => 1,
        "committed" => 2,
        "voided" | "cleared" => 3,
        _ => 0,
    }
}

/// A ticket or round id that is safe to use in a file name.
fn safe_id(s: &str) -> Result<&str, (u16, String)> {
    if !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        Ok(s)
    } else {
        Err(bad("bad id"))
    }
}

/// An asset symbol such as `ACME`.
fn safe_symbol(s: &str) -> Result<&str, (u16, String)> {
    if !s.is_empty() && s.len() <= 12 && s.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(s)
    } else {
        Err(bad("a symbol is 1 to 12 letters or digits"))
    }
}

impl Inner {
    fn tickets_dir(&self, who: &str) -> PathBuf {
        self.home.join("tickets").join(who)
    }

    fn wallet(&self, who: &str) -> Result<&Wallet, (u16, String)> {
        if self.wallets.is_empty() {
            return Err((409, "this gateway has no wallet configured".to_string()));
        }
        self.wallets
            .get(who)
            .ok_or((404, format!("no wallet called {who}")))
    }

    fn with_wallet<T>(&self, who: &str, f: impl FnOnce() -> T) -> T {
        let _guard = self
            .busy
            .get(who)
            .map(|m| m.lock().unwrap_or_else(|e| e.into_inner()));
        f()
    }

    fn assets(&self) -> AssetsJ {
        self.assets
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Every node's list of rounds, merged: the most advanced phase and most orders seen.
    fn rounds(&self) -> Vec<RoundSummaryJ> {
        let mut merged: BTreeMap<u64, RoundSummaryJ> = BTreeMap::new();
        for url in self.roster.urls() {
            let Ok(list) = client::get::<Vec<RoundSummaryJ>>(&format!("{url}/v1/rounds")) else {
                continue;
            };
            for r in list {
                merged
                    .entry(r.round_id)
                    .and_modify(|m| {
                        if phase_rank(&r.phase) > phase_rank(&m.phase) {
                            m.phase = r.phase.clone();
                        }
                        m.orders = m.orders.max(r.orders);
                    })
                    .or_insert(r);
            }
        }
        merged.into_values().collect()
    }

    /// A round's phase and final record, from the first node that has one.
    fn status(&self, round: u64) -> Option<RoundStatusJ> {
        let mut best: Option<RoundStatusJ> = None;
        for url in self.roster.urls() {
            let Ok(s) = client::get::<RoundStatusJ>(&format!("{url}/v1/rounds/{round}")) else {
                continue;
            };
            let better = match &best {
                None => true,
                Some(b) => {
                    (s.record.is_some() && b.record.is_none())
                        || phase_rank(&s.phase) > phase_rank(&b.phase)
                }
            };
            if better {
                best = Some(s);
            }
        }
        best
    }

    fn settlement(&self, round: u64) -> Option<SettlementJ> {
        let ids: Vec<u16> = self.roster.nodes.iter().map(|n| n.id).collect();
        let lead = leader(&ids, round)?;
        let url = &self.roster.node(lead).ok()?.url;
        client::get(&format!("{url}/v1/rounds/{round}/settlement")).ok()
    }

    /// Ask the round's leader to assemble again after a failed or unknown attempt.
    fn retry_settlement(&self, round: u64) -> ApiResult {
        let ids: Vec<u16> = self.roster.nodes.iter().map(|n| n.id).collect();
        let lead = leader(&ids, round).ok_or_else(|| bad("no nodes"))?;
        let url = &self.roster.node(lead).map_err(fail)?.url;
        client::post::<_, Value>(&format!("{url}/v1/rounds/{round}/settle/retry"), &json!({}))
            .map_err(|e| match e.strip_prefix("HTTP 409: ") {
                // "nothing to retry" is the leader's answer, not a gateway failure
                Some(msg) => (409, msg.to_string()),
                None => fail(e),
            })
    }

    // ---- handlers --------------------------------------------------------------------------------

    fn get_status(&self) -> ApiResult {
        let mut nodes = Vec::new();
        let mut group = None;
        for n in &self.roster.nodes {
            let st = client::get::<StatusJ>(&format!("{}/v1/status", n.url)).ok();
            if group.is_none() {
                group = st.as_ref().and_then(|s| s.group.clone());
            }
            nodes.push(json!({
                "id": n.id,
                "url": n.url,
                "up": st.is_some(),
                "keys_ready": st.as_ref().map(|s| s.dkg_done).unwrap_or(false),
            }));
        }
        Ok(json!({
            "threshold": self.roster.threshold,
            "nodes": nodes,
            "group_key": group.map(|g| g.pk),
            "assets": self.assets(),
            "wallet": !self.wallets.is_empty(),
            "wallets": self.wallets.keys().collect::<Vec<_>>(),
            "default": self.default_actor,
            "auditor": self.auditor.is_some(),
        }))
    }

    fn get_rounds(&self) -> ApiResult {
        Ok(json!(self.rounds()))
    }

    fn get_round(&self, round: u64) -> ApiResult {
        let summary = self
            .rounds()
            .into_iter()
            .find(|r| r.round_id == round)
            .ok_or((404, format!("no round {round}")))?;
        let status = self.status(round);
        let record = status.as_ref().and_then(|s| s.record.clone());
        let done = matches!(summary.phase.as_str(), "cleared" | "voided");
        // The public checker runs on every finished round: the page shows a result as verified
        // only if this passes.
        let check = if done {
            Some(match client::check(&self.roster, round) {
                Ok(rep) => json!({
                    "clean": rep.is_clean(),
                    "voided": rep.voided,
                    "findings": rep.findings.iter().map(|f| format!("{f:?}")).collect::<Vec<_>>(),
                }),
                Err(e) => json!({ "clean": false, "voided": false, "findings": [e] }),
            })
        } else {
            None
        };
        let settlement = record
            .as_ref()
            .filter(|r| r.price.is_some())
            .and_then(|_| self.settlement(round));
        Ok(json!({
            "round": summary,
            "record": record,
            "check": check,
            "settlement": settlement,
            "nodes": self.node_progress(round),
        }))
    }

    /// What each node has done in a round, from the data the nodes published: how many orders
    /// it receipted, whether it voted on the committed set, how many shares it released, and
    /// whether it signed the result. A node that sees two matching commit votes commits without
    /// voting itself, so `committed` means "cast a vote", not "agreed".
    fn node_progress(&self, round: u64) -> Value {
        let Ok(public) = client::fetch_public(&self.roster, round) else {
            return Value::Null;
        };
        let nodes: Vec<Value> = self
            .roster
            .nodes
            .iter()
            .map(|n| {
                json!({
                    "id": n.id,
                    "receipts": public.receipts.iter().filter(|r| r.node == n.id).count(),
                    "committed": public.commit_votes.iter().any(|v| v.node == n.id),
                    // distinct orders: every node's copy of the same share is merged in
                    "shares": public
                        .partials
                        .iter()
                        .filter(|(_, p)| p.node_id == n.id)
                        .map(|(h, _)| *h)
                        .collect::<std::collections::BTreeSet<_>>()
                        .len(),
                    "signed": public.record_votes.iter().any(|v| v.node == n.id),
                })
            })
            .collect();
        json!({ "orders": public.envelopes.len(), "nodes": nodes })
    }

    fn open_round(&self, body: &Value) -> ApiResult {
        #[derive(Deserialize)]
        struct B {
            round_id: Option<u64>,
            market: String,
            min_orders: Option<usize>,
        }
        let b: B = serde_json::from_value(body.clone()).map_err(bad)?;
        let id = match b.round_id {
            Some(id) => id,
            None => self.rounds().iter().map(|r| r.round_id).max().unwrap_or(0) + 1,
        };
        client::open_round(&self.roster, id, &b.market, b.min_orders.unwrap_or(2)).map_err(fail)?;
        Ok(json!({ "round_id": id }))
    }

    fn close_round(&self, round: u64) -> ApiResult {
        client::close_round(&self.roster, round).map_err(fail)?;
        Ok(json!({ "round_id": round }))
    }

    /// What an order can cost at most: a sell gives up `size` of the base asset; a buy pays
    /// at most `size * limit` of the quote asset (the price is never above its limit).
    fn order_cost(
        market: &str,
        side: Side,
        size: u64,
        limit: u64,
    ) -> Result<(String, u64), (u16, String)> {
        let (base, quote) =
            market_assets(market).ok_or_else(|| bad(format!("bad market name {market}")))?;
        match side {
            Side::Sell => Ok((base.to_string(), size)),
            Side::Buy => size
                .checked_mul(limit)
                .map(|c| (quote.to_string(), c))
                .ok_or_else(|| bad("size times limit is too large")),
        }
    }

    /// Refuse an order the investor's wallet cannot pay for, counting what their other orders
    /// in rounds that are still open already commit. Without this, an order nobody can settle
    /// would still be counted in the round and could displace a real one at the cut.
    ///
    /// Skipped when the gateway has no wallet: then it cannot know. Orders in a round that has
    /// cleared but is not yet settled are not counted as committed.
    fn check_funds(
        &self,
        who: &str,
        round: u64,
        market: &str,
        side: Side,
        size: u64,
        limit: u64,
    ) -> Result<(), (u16, String)> {
        if self.wallets.is_empty() {
            return Ok(());
        }
        let wallet = self.wallet(who)?;
        let (symbol, needed) = Self::order_cost(market, side, size, limit)?;
        let assets = self.assets();
        let id = assets
            .get(&symbol)
            .ok_or_else(|| bad(format!("{symbol} is not a listed asset")))?
            .clone();
        let mut reserved: u64 = 0;
        let mut phases: BTreeMap<u64, Option<String>> = BTreeMap::new();
        for (_, t) in self.read_tickets(who)? {
            // One order per asset per round: each filled order's part spends the wallet's notes,
            // and two parts built from one wallet would spend the same note twice, which the
            // chain rejects (a duplicate nullifier) and the whole round's swap fails with it.
            if t.round_id == round {
                if let Ok(ts) = parse_side(&t.side) {
                    if Self::order_cost(&t.market, ts, t.size, t.limit)
                        .is_ok_and(|(s, _)| s == symbol)
                    {
                        return Err((
                            409,
                            format!(
                                "{who} already has an order in round {round} that spends {symbol}; a wallet can settle only one such order per round, because both would spend the same note"
                            ),
                        ));
                    }
                }
            }
            let phase = phases
                .entry(t.round_id)
                .or_insert_with(|| self.status(t.round_id).map(|s| s.phase))
                .clone();
            // A round no node will answer for is counted as still open: when unsure, reserve.
            if matches!(phase.as_deref(), Some("cleared" | "voided")) {
                continue;
            }
            let ts = parse_side(&t.side).map_err(|e| {
                (
                    503,
                    format!(
                        "a saved ticket for {who} is unreadable, so funds cannot be counted: {e}"
                    ),
                )
            })?;
            if let Ok((sym, c)) = Self::order_cost(&t.market, ts, t.size, t.limit) {
                if sym == symbol {
                    reserved = reserved.saturating_add(c);
                }
            }
        }
        let held = self
            .with_wallet(who, || wallet.balances(std::slice::from_ref(&id)))
            .map_err(fail)?
            .iter()
            .find(|(a, _)| *a == id)
            .map(|x| x.1)
            .unwrap_or(0);
        if held < reserved.saturating_add(needed) {
            return Err((
                422,
                format!(
                    "this order can cost up to {needed} {symbol}; {who} holds {held} and has {reserved} already committed to open orders"
                ),
            ));
        }
        Ok(())
    }

    fn place_order(&self, round: u64, body: &Value, who: &str) -> ApiResult {
        #[derive(Deserialize)]
        struct B {
            market: String,
            side: String,
            size: u64,
            limit: u64,
        }
        let b: B = serde_json::from_value(body.clone()).map_err(bad)?;
        if b.size == 0 || b.limit == 0 {
            return Err(bad("size and limit must be above zero"));
        }
        let side = parse_side(&b.side).map_err(bad)?;
        let _one_at_a_time = self.order_lock.lock().unwrap_or_else(|e| e.into_inner());
        self.check_funds(who, round, &b.market, side, b.size, b.limit)?;
        let sealed = client::seal_order(&self.roster, round, &b.market, side, b.size, b.limit)
            .map_err(fail)?;
        // Save the ticket (and so the order's one-time key) BEFORE anything is sent. If sending
        // is cut off, nodes may hold the order; without the key it could never be settled.
        let id = format!("r{round}-{}", &sealed.ticket.envelope[..8]);
        let path = self.tickets_dir(who).join(format!("{id}.json"));
        write_json(&path, &sealed.ticket, true).map_err(fail)?;
        let sent = match client::send_order(&self.roster, round, &sealed.envelope) {
            Ok(s) => s,
            Err(e) => {
                // nothing was sent: the ticket would only hold funds for an order nobody has
                let _ = std::fs::remove_file(&path);
                return Err(fail(e));
            }
        };
        let needed = self.roster.threshold as usize;
        let n = self.roster.nodes.len();
        // Only when every node answered with a refusal is it certain that no node holds the
        // order; then the ticket goes. Otherwise it is kept, even below `t` receipts: the
        // receipts are gossiped and the order can still count, and its funds stay reserved
        // until the round ends.
        let kept = !(sent.receipts.is_empty() && sent.refused == n);
        if !kept {
            let _ = std::fs::remove_file(&path);
        }
        Ok(json!({
            "receipts": sent.receipts.len(),
            "needed": needed,
            "counted": sent.receipts.len() >= needed,
            "ticket": if kept { Some(id) } else { None },
        }))
    }

    /// Every saved ticket of this investor. A folder that does not exist yet is no tickets; any
    /// other read or parse error is an error, never a shorter list: the funds check must not
    /// under-count what is already promised, and the page must not hide an order.
    fn read_tickets(&self, who: &str) -> Result<Vec<(String, TicketJ)>, (u16, String)> {
        let unreadable = |what: String| {
            crate::log::event(
                "error",
                "gateway",
                "read saved tickets",
                json!({ "as": who, "error": what }),
            );
            (503, format!("{who}'s saved tickets cannot be read: {what}"))
        };
        let mut out = Vec::new();
        let rd = match std::fs::read_dir(self.tickets_dir(who)) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(unreadable(e.to_string())),
        };
        for e in rd {
            let p = e.map_err(|e| unreadable(e.to_string()))?.path();
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let id = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            out.push((id, read_json::<TicketJ>(&p).map_err(unreadable)?));
        }
        out.sort_by(|a, b| a.1.round_id.cmp(&b.1.round_id).then(a.0.cmp(&b.0)));
        Ok(out)
    }

    fn get_tickets(&self, who: &str) -> ApiResult {
        let mut cache: BTreeMap<u64, Option<RoundStatusJ>> = BTreeMap::new();
        let mut out = Vec::new();
        for (id, t) in self.read_tickets(who)? {
            let st = cache
                .entry(t.round_id)
                .or_insert_with(|| self.status(t.round_id))
                .clone();
            let phase = st.as_ref().map(|s| s.phase.clone());
            let record: Option<&RecordJ> = st.as_ref().and_then(|s| s.record.as_ref());
            let (filled, price) = match record {
                Some(r) => {
                    let pos = r.set.iter().position(|h| *h == t.envelope);
                    let filled: u64 = pos
                        .map(|p| {
                            r.fills
                                .iter()
                                .filter(|f| f.id as usize == p)
                                .map(|f| f.quantity)
                                .sum()
                        })
                        .unwrap_or(0);
                    (Some(filled), r.price)
                }
                None => (None, None),
            };
            // never include the ticket's key
            out.push(json!({
                "id": id,
                "round_id": t.round_id,
                "market": t.market,
                "side": t.side,
                "size": t.size,
                "limit": t.limit,
                "phase": phase,
                "filled": filled,
                "price": price,
            }));
        }
        Ok(json!(out))
    }

    fn settle_ticket(&self, id: &str, who: &str) -> ApiResult {
        let id = safe_id(id)?;
        let wallet = self.wallet(who)?;
        let ticket: TicketJ =
            read_json(&self.tickets_dir(who).join(format!("{id}.json"))).map_err(|e| (404, e))?;
        let assets = self.assets();
        let work = self.home.join("parts").join(who);
        let done = self
            .with_wallet(who, || {
                client::settle_order(&self.roster, &ticket, wallet, &assets, &work)
            })
            .map_err(fail)?;
        Ok(match done {
            Settled::NothingToSettle => json!({ "settled": false, "reason": "not filled" }),
            Settled::Handed { leg, accepted } => json!({
                "settled": true,
                "handed_to": accepted,
                "order": leg.order,
                "base": leg.base,
                "quote": leg.quote,
                "sells": leg.sends_base(),
            }),
        })
    }

    fn get_assets(&self) -> ApiResult {
        self.sync_listings();
        Ok(json!(self.assets()))
    }

    /// List a company: issue its token from this wallet and add it to the assets file.
    ///
    /// This listing is local to this gateway and its nodes' assets file. It is not yet signed
    /// by the nodes, and the page says so.
    fn list_company(&self, body: &Value, who: &str) -> ApiResult {
        #[derive(Deserialize)]
        struct B {
            symbol: String,
            amount: u64,
            finalize: Option<bool>,
        }
        let b: B = serde_json::from_value(body.clone()).map_err(bad)?;
        let symbol = safe_symbol(&b.symbol)?.to_string();
        self.sync_listings();
        if b.amount == 0 {
            return Err(bad("supply must be above zero"));
        }
        // One listing at a time: issuing, the pending record and the assets file go together.
        let _listing = self.listing_lock.lock().unwrap_or_else(|e| e.into_inner());
        if self.assets().contains_key(&symbol) {
            return Err((409, format!("{symbol} is already listed")));
        }
        let wallet = self.wallet(who)?;
        let finalize = b.finalize.unwrap_or(true);
        // A listing the nodes did not all sign yet is kept, and asking again asks them again,
        // rather than issuing a second token under the same symbol.
        let pending_path = self.home.join("pending-listings.json");
        // A missing file is an empty list; one that cannot be read is an error, never "empty",
        // or a damaged file would make the next request issue the token a second time.
        let mut pending: AssetsJ = read_json_or_default(&pending_path).map_err(fail)?;
        let id = match pending.get(&symbol) {
            Some(id) => id.clone(),
            None => {
                let id = self
                    .with_wallet(who, || wallet.issue(&symbol, b.amount, finalize))
                    .map_err(fail)?;
                pending.insert(symbol.clone(), id.clone());
                write_json(&pending_path, &pending, false).map_err(|e| {
                    // the token exists on chain: say which one, so it is not lost
                    fail(format!("{symbol} was issued as asset {id}, but this gateway could not record it: {e}"))
                })?;
                id
            }
        };
        let signers = self.collect_listing_signatures(&symbol, &id);
        let needed = self.roster.threshold as usize;
        if signers.len() < needed {
            return Err(fail(format!(
                "{symbol} was issued, but only {} of the {needed} node signatures it needs arrived; ask again to retry the listing",
                signers.len()
            )));
        }
        {
            // the file first: the list in memory only changes once the file has
            let mut assets = self.assets.lock().unwrap_or_else(|e| e.into_inner());
            let mut next = assets.clone();
            next.insert(symbol.clone(), id.clone());
            write_json(&self.assets_path, &next, false).map_err(fail)?;
            *assets = next;
        }
        pending.remove(&symbol);
        if let Err(e) = write_json(&pending_path, &pending, false) {
            crate::log::event(
                "warn",
                "gateway",
                "clear a pending listing",
                json!({ "symbol": symbol, "asset_id": id, "error": e }),
            );
        }
        Ok(json!({
            "symbol": symbol,
            "asset_id": id,
            "finalized": finalize,
            "signed_by": signers,
            "needed": needed,
        }))
    }

    /// Ask every node to accept a listing, and return the ids of the nodes whose signature
    /// checks out against the roster.
    fn collect_listing_signatures(&self, symbol: &str, asset_id: &str) -> Vec<u16> {
        let Ok(roster) = self.roster.roster() else {
            return Vec::new();
        };
        let body = ListingJ {
            symbol: symbol.to_string(),
            asset_id: asset_id.to_string(),
        };
        let mut signers = Vec::new();
        for n in &self.roster.nodes {
            let Ok(s) = client::post::<_, ListingSigJ>(&format!("{}/v1/listings", n.url), &body)
            else {
                continue;
            };
            if s.node == n.id
                && s.symbol == symbol
                && s.asset_id.eq_ignore_ascii_case(asset_id)
                && s.verify(&roster).is_ok()
            {
                signers.push(n.id);
            }
        }
        signers
    }

    /// Pick up listings made elsewhere: any symbol that at least `t` nodes have signed for one
    /// asset id is listed here too. A symbol already mapped to a different id is left alone.
    fn sync_listings(&self) {
        let Ok(roster) = self.roster.roster() else {
            return;
        };
        let mut signed: BTreeMap<(String, String), std::collections::BTreeSet<u16>> =
            BTreeMap::new();
        for n in &self.roster.nodes {
            let Ok(list) = client::get::<Vec<ListingSigJ>>(&format!("{}/v1/listings", n.url))
            else {
                continue;
            };
            for s in list {
                if s.node == n.id && valid_symbol(&s.symbol) && s.verify(&roster).is_ok() {
                    signed
                        .entry((s.symbol.clone(), s.asset_id.to_lowercase()))
                        .or_default()
                        .insert(s.node);
                }
            }
        }
        let mut assets = self.assets.lock().unwrap_or_else(|e| e.into_inner());
        let before = assets.clone();
        let mut changed = false;
        for ((symbol, id), by) in signed {
            if by.len() >= self.roster.threshold as usize && !assets.contains_key(&symbol) {
                assets.insert(symbol, id);
                changed = true;
            }
        }
        if changed {
            // keep the file and the memory the same: undo the memory change if the file fails
            if let Err(e) = write_json(&self.assets_path, &*assets, false) {
                crate::log::event(
                    "error",
                    "gateway",
                    "save listings picked up from the nodes",
                    json!({ "error": e }),
                );
                *assets = before;
            }
        }
    }

    fn get_balances(&self, who: &str) -> ApiResult {
        let wallet = self.wallet(who)?;
        let assets = self.assets();
        let ids: Vec<String> = assets.values().cloned().collect();
        // One scan of the chain for every asset, not one per asset.
        let found = self
            .with_wallet(who, || wallet.balances(&ids))
            .map_err(fail)?;
        let out: BTreeMap<String, u64> = assets
            .iter()
            .map(|(sym, id)| {
                let b = found
                    .iter()
                    .find(|(a, _)| a == id)
                    .map(|x| x.1)
                    .unwrap_or(0);
                (sym.clone(), b)
            })
            .collect();
        Ok(json!(out))
    }

    fn get_viewkey(&self, who: &str) -> ApiResult {
        let wallet = self.wallet(who)?;
        let fvk = self.with_wallet(who, || wallet.view_key()).map_err(fail)?;
        Ok(json!({ "fvk": fvk }))
    }

    fn audit(&self, body: &Value) -> ApiResult {
        let fvk = body["fvk"]
            .as_str()
            .ok_or_else(|| bad("a viewing key is needed"))?;
        let auditor = self.auditor.as_ref().ok_or((
            409,
            "this gateway has no auditor wallet configured".to_string(),
        ))?;
        let assets = self.assets();
        let ids: Vec<String> = assets.values().cloned().collect();
        let found = self
            .with_wallet(AUDITOR, || auditor.audit(fvk.trim(), &ids))
            .map_err(fail)?;
        let by_symbol: BTreeMap<String, u64> = assets
            .iter()
            .map(|(sym, id)| {
                (
                    sym.clone(),
                    found
                        .iter()
                        .find(|(a, _)| a == id)
                        .map(|x| x.1)
                        .unwrap_or(0),
                )
            })
            .collect();
        Ok(json!(by_symbol))
    }

    fn get_bundle(&self, round: u64) -> ApiResult {
        let b = bundle::export(&self.roster, round).map_err(fail)?;
        serde_json::to_value(b).map_err(fail)
    }

    fn route(&self, method: &Method, path: &str, body: &Value, who: &str) -> ApiResult {
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
        let round =
            |s: &str| -> Result<u64, (u16, String)> { s.parse().map_err(|_| bad("bad round id")) };
        match (method, parts.as_slice()) {
            (Method::Get, ["api", "status"]) => self.get_status(),
            (Method::Get, ["api", "rounds"]) => self.get_rounds(),
            (Method::Post, ["api", "rounds"]) => self.open_round(body),
            (Method::Get, ["api", "rounds", r]) => self.get_round(round(r)?),
            (Method::Post, ["api", "rounds", r, "close"]) => self.close_round(round(r)?),
            (Method::Post, ["api", "rounds", r, "orders"]) => {
                self.place_order(round(r)?, body, who)
            }
            (Method::Get, ["api", "rounds", r, "bundle"]) => self.get_bundle(round(r)?),
            (Method::Get, ["api", "tickets"]) => self.get_tickets(who),
            (Method::Post, ["api", "tickets", id, "settle"]) => self.settle_ticket(id, who),
            (Method::Post, ["api", "rounds", r, "settle", "retry"]) => {
                self.retry_settlement(round(r)?)
            }
            (Method::Get, ["api", "assets"]) => self.get_assets(),
            (Method::Post, ["api", "assets"]) => self.list_company(body, who),
            (Method::Get, ["api", "balances"]) => self.get_balances(who),
            (Method::Get, ["api", "viewkey"]) => self.get_viewkey(who),
            (Method::Post, ["api", "audit"]) => self.audit(body),
            _ => Err((404, "no such endpoint".into())),
        }
    }
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).expect("valid header")
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// A file from the built page, or `index.html` for any path that is not a file (the page does its
/// own routing). Paths that try to leave the directory are refused.
fn serve_static(dir: &Path, url_path: &str) -> Option<(Vec<u8>, &'static str)> {
    let rel = Path::new(url_path.trim_start_matches('/'));
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    let candidate = dir.join(rel);
    let file = if candidate.is_file() {
        candidate
    } else if rel.extension().is_none() {
        dir.join("index.html")
    } else {
        return None;
    };
    let bytes = std::fs::read(&file).ok()?;
    Some((bytes, content_type(&file)))
}

fn handle(inner: &Inner, mut req: Request) {
    let method = req.method().clone();
    let url = req.url().to_string();
    let path = url.split('?').next().unwrap_or("").to_string();
    // `?as=name`: which of this gateway's wallets the request acts for.
    let asked = url.split_once('?').and_then(|(_, q)| {
        q.split('&')
            .find_map(|kv| kv.strip_prefix("as=").map(str::to_string))
    });
    // The page is served from this same origin; the open CORS header only matters for the
    // development server, which runs on another port of the same machine.
    let cors = [
        header("Access-Control-Allow-Origin", "*"),
        header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
        header("Access-Control-Allow-Headers", "Content-Type"),
    ];
    if method == Method::Options {
        let mut r = Response::empty(204);
        for h in cors {
            r.add_header(h);
        }
        let _ = req.respond(r);
        return;
    }
    if !path.starts_with("/api/") {
        let served = inner
            .ui_dir
            .as_deref()
            .and_then(|d| serve_static(d, &path))
            .map(|(b, ct)| Response::from_data(b).with_header(header("Content-Type", ct)));
        let _ = match served {
            Some(r) => req.respond(r),
            None => req.respond(Response::from_string("not found").with_status_code(404)),
        };
        return;
    }
    let mut text = String::new();
    if req
        .as_reader()
        .take(1 << 20)
        .read_to_string(&mut text)
        .is_err()
    {
        crate::log::event(
            "warn",
            "gateway",
            "api request",
            json!({ "method": method.to_string(), "path": path, "status": 400, "error": "unreadable body" }),
        );
        let _ = req.respond(
            Response::from_string("{\"error\":\"unreadable body\"}").with_status_code(400),
        );
        return;
    }
    let body: Value = if text.trim().is_empty() {
        Value::Null
    } else {
        match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                crate::log::event(
                    "warn",
                    "gateway",
                    "api request",
                    json!({ "method": method.to_string(), "path": path, "status": 400, "error": format!("bad JSON: {e}") }),
                );
                let msg = json!({ "error": format!("bad JSON: {e}") }).to_string();
                let _ = req.respond(Response::from_string(msg).with_status_code(400));
                return;
            }
        }
    };
    let who = match asked {
        None => Ok(inner.default_actor.clone()),
        Some(n) => safe_id(&n).map(str::to_string),
    };
    let result = match &who {
        Ok(who) => inner.route(&method, &path, &body, who),
        Err(e) => Err(e.clone()),
    };
    let (code, out) = match result {
        Ok(v) => (200, v.to_string()),
        Err((c, m)) => {
            if c != 404 {
                crate::log::event(
                    if c >= 500 { "error" } else { "warn" },
                    "gateway",
                    "api request",
                    json!({
                        "method": method.to_string(),
                        "path": path,
                        "as": who.as_deref().unwrap_or("?"),
                        "status": c,
                        "error": m,
                    }),
                );
            }
            (c, json!({ "error": m }).to_string())
        }
    };
    let mut r = Response::from_string(out)
        .with_status_code(code)
        .with_header(header("Content-Type", "application/json"));
    for h in cors {
        r.add_header(h);
    }
    let _ = req.respond(r);
}

/// Start the gateway on an already-bound server (bind first so tests can use port 0). Each
/// request is handled on its own thread, so a slow wallet command does not block the page.
pub fn spawn(server: tiny_http::Server, opts: GatewayOptions) -> Result<GatewayHandle, String> {
    let addr = server
        .server_addr()
        .to_ip()
        .ok_or("server is not listening on an IP address")?;
    let assets: AssetsJ = if opts.assets_path.exists() {
        read_json(&opts.assets_path)?
    } else {
        AssetsJ::new()
    };
    let default_actor = opts
        .wallets
        .first()
        .map(|(n, _)| n.clone())
        .unwrap_or_else(|| "me".to_string());
    for (name, _) in &opts.wallets {
        safe_id(name).map_err(|_| format!("bad wallet name {name}"))?;
    }
    std::fs::create_dir_all(opts.home.join("tickets")).map_err(|e| e.to_string())?;
    let mut busy: BTreeMap<String, Mutex<()>> = opts
        .wallets
        .iter()
        .map(|(n, _)| (n.clone(), Mutex::new(())))
        .collect();
    busy.insert(AUDITOR.to_string(), Mutex::new(()));
    let inner = Arc::new(Inner {
        roster: opts.roster,
        home: opts.home,
        assets_path: opts.assets_path,
        assets: Mutex::new(assets),
        wallets: opts.wallets.into_iter().collect(),
        default_actor,
        auditor: opts.auditor,
        ui_dir: opts.ui_dir,
        busy,
        order_lock: Mutex::new(()),
        listing_lock: Mutex::new(()),
    });
    let server = Arc::new(server);
    let stop = Arc::new(AtomicBool::new(false));
    let (srv, stp) = (server.clone(), stop.clone());
    let thread = thread::spawn(move || {
        while !stp.load(Ordering::SeqCst) {
            let Ok(req) = srv.recv() else { break };
            let inner = inner.clone();
            thread::spawn(move || handle(&inner, req));
        }
    });
    Ok(GatewayHandle {
        addr,
        server,
        stop,
        thread: Some(thread),
    })
}
