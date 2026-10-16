//! `kalyptra`: run a matcher node, or act as investor, operator or checker.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use kalyptra_node::bundle::{self, BundleJ};
use kalyptra_node::client::{self, Outcome, Settled};
use kalyptra_node::files::{key_path, pub_path, read_json, write_json, NodeIdentity};
use kalyptra_node::gateway::{self, GatewayOptions};
use kalyptra_node::server::{spawn, NodeOptions, SettleOptions};
use kalyptra_node::wallet::Wallet;
use kalyptra_node::wire::{hx, parse_side, AssetsJ, RecordJ, RosterJ, RosterNodeJ, TicketJ};

#[derive(Parser)]
#[command(
    name = "kalyptra",
    about = "Kalyptra sealed-bid auction: matcher nodes, investor and checker"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a node's identity keys (secret file) and its public roster entry.
    Init {
        #[arg(long)]
        id: u16,
        /// Base URL other nodes and clients use to reach this node.
        #[arg(long)]
        url: String,
        #[arg(long, default_value = "nodes")]
        dir: PathBuf,
    },
    /// Combine public roster entries into a roster file.
    Roster {
        #[arg(long)]
        threshold: u16,
        #[arg(long, default_value = "roster.json")]
        out: PathBuf,
        /// The `node-<id>.pub.json` files.
        entries: Vec<PathBuf>,
    },
    /// Run a matcher node. Runs key generation with its peers on first start.
    Node {
        #[arg(long)]
        id: u16,
        #[arg(long, default_value = "nodes")]
        dir: PathBuf,
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
        /// Listen address; defaults to the host and port of this node's URL in the roster.
        #[arg(long)]
        listen: Option<String>,
        /// QEDIT's `zcash_tx_tool` with Kalyptra's wallet commands. With `--wallet` and
        /// `--assets`, lets this node lead settlement when its turn comes.
        #[arg(long, requires_all = ["wallet", "assets"])]
        tool: Option<PathBuf>,
        /// This node's own Zcash wallet directory (it holds no investor's assets).
        #[arg(long)]
        wallet: Option<PathBuf>,
        /// Asset ids by symbol (JSON), written when the assets were issued.
        #[arg(long)]
        assets: Option<PathBuf>,
    },
    /// Wait for key generation and print the group key.
    Keys {
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
    },
    /// Open a round on every node.
    Open {
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
        #[arg(long)]
        round: u64,
        #[arg(long, default_value = "ACME/vUSD")]
        market: String,
        #[arg(long, default_value_t = 2)]
        min_orders: usize,
    },
    /// Sign an order with a fresh key, seal it, and submit it to every node.
    Submit {
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
        #[arg(long)]
        round: u64,
        #[arg(long, default_value = "ACME/vUSD")]
        market: String,
        /// buy or sell
        #[arg(long)]
        side: String,
        #[arg(long)]
        size: u64,
        #[arg(long)]
        limit: u64,
        /// Where to keep the order's ticket (secret: it holds the order's key). Needed to settle.
        #[arg(long)]
        ticket: Option<PathBuf>,
    },
    /// Settle a filled order: check the round publicly, build this wallet's part of the swap
    /// for exactly the record's terms, sign it with the order's key and hand it to the nodes.
    Settle {
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
        #[arg(long)]
        ticket: PathBuf,
        #[arg(long)]
        tool: PathBuf,
        /// The investor's own Zcash wallet directory.
        #[arg(long)]
        wallet: PathBuf,
        #[arg(long, default_value = "assets.json")]
        assets: PathBuf,
    },
    /// Export a wallet's viewing key to a file, to hand to an auditor.
    ViewKey {
        #[arg(long)]
        tool: PathBuf,
        #[arg(long)]
        wallet: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// As an auditor: report the holdings behind a viewing key, using the auditor's own wallet.
    Audit {
        #[arg(long)]
        tool: PathBuf,
        /// The auditor's own wallet directory.
        #[arg(long)]
        wallet: PathBuf,
        #[arg(long)]
        view_key: PathBuf,
        #[arg(long, default_value = "assets.json")]
        assets: PathBuf,
    },
    /// Wait for a round's settlement and print it.
    Settlement {
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
        #[arg(long)]
        round: u64,
        #[arg(long, default_value_t = 300)]
        timeout: u64,
    },
    /// Close a round on every node.
    Close {
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
        #[arg(long)]
        round: u64,
    },
    /// Wait for a round's final result and print it.
    Wait {
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
        #[arg(long)]
        round: u64,
        #[arg(long, default_value_t = 60)]
        timeout: u64,
    },
    /// Run the local gateway: serves the Kalyptra page and lets it open and close rounds, place
    /// sealed orders, check a round and settle, with keys and wallets kept on this machine.
    Serve {
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
        /// Where tickets (they hold each order's one-time key) are kept.
        #[arg(long, default_value = "gateway")]
        home: PathBuf,
        #[arg(long, default_value = "assets.json")]
        assets: PathBuf,
        /// QEDIT's `zcash_tx_tool` with Kalyptra's wallet commands.
        #[arg(long)]
        tool: Option<PathBuf>,
        /// An investor's Zcash wallet, as `name=dir` (or just `dir`, named `me`). Repeat it for
        /// several investors; the page can switch between them. The first is the default.
        #[arg(long, requires = "tool")]
        wallet: Vec<String>,
        /// A separate wallet directory for the Audit screen.
        #[arg(long, requires = "tool")]
        auditor_wallet: Option<PathBuf>,
        /// The built page to serve (the `ui/dist` folder).
        #[arg(long)]
        ui: Option<PathBuf>,
        /// Must be a loopback address: the gateway has no login.
        #[arg(long, default_value = "127.0.0.1:8787")]
        listen: String,
    },
    /// Save a round (every node's view, merged, plus the group key) to one file.
    Export {
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
        #[arg(long)]
        round: u64,
        #[arg(long)]
        out: PathBuf,
    },
    /// Verify a saved round OFFLINE: no network, no node trusted. Needs the roster, and ideally
    /// the announced group key.
    Verify {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
        /// The group key (hex) the nodes announced. Without it the file's own key is used.
        #[arg(long)]
        group_key: Option<String>,
    },
    /// Verify a round from public data alone, trusting no node.
    Check {
        #[arg(long, default_value = "roster.json")]
        roster: PathBuf,
        #[arg(long)]
        round: u64,
    },
}

fn print_record(r: &RecordJ) {
    match r.price {
        Some(p) => println!("round {}: cleared at {p}, volume {}", r.round_id, r.volume),
        None => println!("round {}: nothing crossed", r.round_id),
    }
    for (i, o) in &r.orders {
        let filled: u64 = r
            .fills
            .iter()
            .filter(|f| f.id == *i)
            .map(|f| f.quantity)
            .sum();
        println!(
            "  order {i}: {} {} @ {} -> filled {filled}",
            o.side, o.size, o.limit
        );
    }
    if !r.invalid.is_empty() {
        println!("  invalid envelopes (excluded): {:?}", r.invalid);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.cmd {
        Cmd::Init { id, url, dir } => {
            let ident = NodeIdentity::generate(id, &url);
            let kp = key_path(&dir, id);
            if kp.exists() {
                return Err(format!("{} already exists; not overwriting", kp.display()));
            }
            ident.save(&kp)?;
            write_json(&pub_path(&dir, id), &ident.public_entry(), false)?;
            println!(
                "node {id}: wrote {} (secret) and {}",
                kp.display(),
                pub_path(&dir, id).display()
            );
        }
        Cmd::Roster {
            threshold,
            out,
            entries,
        } => {
            let mut nodes: Vec<RosterNodeJ> = entries
                .iter()
                .map(|p| read_json(p))
                .collect::<Result<_, _>>()?;
            nodes.sort_by_key(|n| n.id);
            let roster = RosterJ { threshold, nodes };
            roster.roster()?;
            write_json(&out, &roster, false)?;
            println!(
                "wrote {} ({} nodes, threshold {threshold})",
                out.display(),
                roster.nodes.len()
            );
        }
        Cmd::Node {
            id,
            dir,
            roster,
            listen,
            tool,
            wallet,
            assets,
        } => {
            let settle = match (tool, wallet, assets) {
                (Some(tool), Some(wallet), Some(assets)) => Some(SettleOptions {
                    wallet: Wallet::new(tool, wallet),
                    assets: read_json::<AssetsJ>(&assets)?,
                    assets_file: Some(assets),
                }),
                _ => None,
            };
            let roster: RosterJ = read_json(&roster)?;
            let identity = NodeIdentity::load(&key_path(&dir, id))?;
            let url = roster.node(id)?.url.clone();
            let listen = listen.unwrap_or_else(|| {
                url.trim_start_matches("http://")
                    .trim_end_matches('/')
                    .to_string()
            });
            let server = tiny_http::Server::http(&listen)
                .map_err(|e| format!("cannot listen on {listen}: {e}"))?;
            let handle = spawn(
                server,
                NodeOptions {
                    identity,
                    roster,
                    data_dir: dir,
                    verbose: true,
                    settle,
                },
            )?;
            eprintln!("[node {id}] listening on {}", handle.addr);
            loop {
                std::thread::park();
            }
        }
        Cmd::Keys { roster } => {
            let roster: RosterJ = read_json(&roster)?;
            let g = client::wait_for_keys(&roster, Duration::from_secs(120))?;
            println!(
                "group key {} ({}-of-{})",
                hx(g.public_key().compress().as_bytes()),
                g.threshold(),
                g.node_ids().len()
            );
        }
        Cmd::Open {
            roster,
            round,
            market,
            min_orders,
        } => {
            client::open_round(&read_json(&roster)?, round, &market, min_orders)?;
            println!("round {round} open on every node ({market}, at least {min_orders} orders)");
        }
        Cmd::Submit {
            roster,
            round,
            market,
            side,
            size,
            limit,
            ticket,
        } => {
            let roster: RosterJ = read_json(&roster)?;
            let side = parse_side(&side)?;
            let (receipts, t) = client::submit_order(&roster, round, &market, side, size, limit)?;
            if let Some(path) = ticket {
                write_json(&path, &t, true)?;
            }
            println!(
                "sealed order submitted: {} of {} nodes receipted it (needs {})",
                receipts.len(),
                roster.nodes.len(),
                roster.threshold
            );
            if receipts.len() < roster.threshold as usize {
                return Err("too few receipts: this order may not be counted".into());
            }
        }
        Cmd::Settle {
            roster,
            ticket,
            tool,
            wallet,
            assets,
        } => {
            let t: TicketJ = read_json(&ticket)?;
            let work = wallet.join("parts");
            match client::settle_order(
                &read_json(&roster)?,
                &t,
                &Wallet::new(tool, wallet),
                &read_json(&assets)?,
                &work,
            )? {
                Settled::NothingToSettle => {
                    println!(
                        "round {}: this order was not filled; nothing moves",
                        t.round_id
                    )
                }
                Settled::Handed { leg, accepted } => println!(
                    "round {}: order {} part built ({} {} for {} {}), handed to {accepted} nodes",
                    t.round_id,
                    leg.order,
                    if leg.sends_base() { "sell" } else { "buy" },
                    leg.base,
                    leg.quote,
                    t.market.split_once('/').map_or("quote", |m| m.1),
                ),
            }
        }
        Cmd::ViewKey { tool, wallet, out } => {
            let fvk = Wallet::new(tool, wallet).view_key()?;
            write_json(&out, &serde_json::json!({ "fvk": fvk }), true)?;
            println!("viewing key written to {}", out.display());
        }
        Cmd::Audit {
            tool,
            wallet,
            view_key,
            assets,
        } => {
            let v: serde_json::Value = read_json(&view_key)?;
            let fvk = v["fvk"].as_str().ok_or("no fvk in the viewing-key file")?;
            let assets: AssetsJ = read_json(&assets)?;
            let ids: Vec<String> = assets.values().cloned().collect();
            let balances = Wallet::new(tool, wallet).audit(fvk, &ids)?;
            println!("holdings seen through the viewing key:");
            for (sym, id) in &assets {
                let b = balances
                    .iter()
                    .find(|(a, _)| a == id)
                    .map(|x| x.1)
                    .unwrap_or(0);
                println!("  {sym:<5} {b}");
            }
        }
        Cmd::Settlement {
            roster,
            round,
            timeout,
        } => {
            let s =
                client::wait_settlement(&read_json(&roster)?, round, Duration::from_secs(timeout))?;
            for l in &s.legs {
                println!(
                    "  order {}: {} {} for {} vUSD{}",
                    l.order,
                    l.side,
                    l.base,
                    l.quote,
                    if l.received { "" } else { " (no part yet)" }
                );
            }
            match s.status.as_str() {
                "settled" => println!(
                    "round {round}: SETTLED by node {} in one transaction {} (block {})",
                    s.leader,
                    s.txid.unwrap_or_default(),
                    s.height.unwrap_or_default()
                ),
                "nothing" => println!("round {round}: nothing to settle"),
                other => {
                    return Err(format!(
                        "round {round}: settlement {other}{}{}",
                        s.error.map(|e| format!(": {e}")).unwrap_or_default(),
                        if s.bad.is_empty() {
                            String::new()
                        } else {
                            format!(" (orders {:?})", s.bad)
                        }
                    ))
                }
            }
        }
        Cmd::Close { roster, round } => {
            client::close_round(&read_json(&roster)?, round)?;
            println!("round {round} closed on every node");
        }
        Cmd::Wait {
            roster,
            round,
            timeout,
        } => match client::wait_final(&read_json(&roster)?, round, Duration::from_secs(timeout))? {
            Outcome::Cleared(r) => print_record(&r),
            Outcome::Voided => {
                println!("round {round}: voided (too few orders); nothing was decrypted")
            }
        },
        Cmd::Serve {
            roster,
            home,
            assets,
            tool,
            wallet,
            auditor_wallet,
            ui,
            listen,
        } => {
            let addr = gateway_addr(&listen)?;
            let server = tiny_http::Server::http(addr)
                .map_err(|e| format!("cannot listen on {addr}: {e}"))?;
            let handle = gateway::spawn(
                server,
                GatewayOptions {
                    roster: read_json(&roster)?,
                    home,
                    assets_path: assets,
                    wallets: match &tool {
                        None => Vec::new(),
                        Some(t) => wallet
                            .iter()
                            .map(|w| match w.split_once('=') {
                                Some((name, dir)) => (name.to_string(), Wallet::new(t, dir)),
                                None => ("me".to_string(), Wallet::new(t, w)),
                            })
                            .collect(),
                    },
                    auditor: tool.zip(auditor_wallet).map(|(t, w)| Wallet::new(t, w)),
                    ui_dir: ui,
                },
            )?;
            eprintln!("Kalyptra gateway on http://{}", handle.addr);
            loop {
                std::thread::park();
            }
        }
        Cmd::Export { roster, round, out } => {
            let b = bundle::export(&read_json(&roster)?, round)?;
            write_json(&out, &b, false)?;
            println!("round {round} saved to {}", out.display());
        }
        Cmd::Verify {
            bundle,
            roster,
            group_key,
        } => {
            let b: BundleJ = read_json(&bundle)?;
            let v = bundle::verify(&b, &read_json(&roster)?, group_key.as_deref())?;
            if !v.pinned {
                println!(
                    "note: group key {} is taken from the file; pass --group-key to pin it",
                    v.group_key
                );
            }
            if let Some(r) = &v.report.recomputed {
                print_record(&RecordJ::from(r));
            }
            if v.report.voided {
                println!("round {}: voided", b.round.config.round_id);
            }
            if v.report.is_clean() {
                println!("VERIFIED OFFLINE: everything checked from the file alone");
            } else {
                for f in &v.report.findings {
                    println!("FINDING: {f:?}");
                }
                return Err("VERIFICATION FAILED".into());
            }
        }
        Cmd::Check { roster, round } => {
            let report = client::check(&read_json(&roster)?, round)?;
            if let Some(r) = &report.recomputed {
                print_record(&RecordJ::from(r));
            }
            if report.voided {
                println!("round {round}: voided");
            }
            if report.is_clean() {
                println!("CHECK PASSED: everything verified from public data");
            } else {
                for f in &report.findings {
                    println!("FINDING: {f:?}");
                }
                return Err("CHECK FAILED".into());
            }
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The address the gateway may listen on. It has no login, so only a loopback address is allowed.
fn gateway_addr(listen: &str) -> Result<std::net::SocketAddr, String> {
    let addr: std::net::SocketAddr = listen
        .parse()
        .map_err(|e| format!("bad listen address {listen}: {e}"))?;
    if !addr.ip().is_loopback() {
        return Err(
            "the gateway has no login, so it only listens on a loopback address (127.0.0.1)".into(),
        );
    }
    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::gateway_addr;

    #[test]
    fn the_gateway_refuses_any_address_that_is_not_loopback() {
        assert!(gateway_addr("127.0.0.1:8787").is_ok());
        assert!(gateway_addr("[::1]:8787").is_ok());
        for bad in [
            "0.0.0.0:8787",
            "192.168.1.5:8787",
            "[::]:8787",
            "8.8.8.8:80",
        ] {
            assert!(
                gateway_addr(bad).unwrap_err().contains("loopback"),
                "{bad} was allowed"
            );
        }
        assert!(gateway_addr("not an address").is_err());
    }
}
