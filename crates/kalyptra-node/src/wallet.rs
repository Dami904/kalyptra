//! A Zcash wallet, driven through the `kalyptra-wallet` commands Kalyptra adds to QEDIT's
//! `zcash_tx_tool` (see `settlement/`).
//!
//! The Zcash side (wallets, action groups, the swap transaction) lives in that tool because it
//! needs QEDIT's prototype libraries. This module only runs it: each call is one process that
//! opens one wallet directory, does one thing, and prints one `KALYPTRA_RESULT <json>` line.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::wire::WireResult;

#[derive(Clone, Debug)]
pub struct Wallet {
    /// The `zcash_tx_tool` binary.
    pub tool: PathBuf,
    /// This wallet's directory (`seed.txt`, `wallet.sqlite`).
    pub dir: PathBuf,
}

/// What the assembler reports.
pub enum Assembled {
    Settled {
        txid: String,
        height: u32,
    },
    /// These plan labels' parts did not match their legs; nothing was sent.
    BadParts(Vec<String>),
    /// The node itself answered "Block rejected": a definite failure, nothing moved. Any other
    /// failure from the tool is returned as an error, so the outcome counts as unknown. An error from [`Wallet::assemble`] instead means the outcome is unknown: the tool
    /// gave no result, so the transaction may or may not have been sent.
    Rejected(String),
}

/// How long one wallet command may run before it is stopped and reported as an error (and so,
/// for settlement, as an unknown outcome). `KALYPTRA_TOOL_TIMEOUT_SECS` overrides it.
fn tool_timeout() -> Duration {
    std::env::var("KALYPTRA_TOOL_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(15 * 60))
}

/// Read a child's pipe to the end on its own thread, so a chatty tool never blocks on a full pipe.
fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut p) = pipe {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    })
}

fn run(tool: &Path, args: &[&str]) -> WireResult<Value> {
    let mut child = Command::new(tool)
        .arg("kalyptra-wallet")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run {}: {e}", tool.display()))?;
    let out_t = drain(child.stdout.take());
    let err_t = drain(child.stderr.take());
    let deadline = Instant::now() + tool_timeout();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "the wallet tool did not finish within {} s and was stopped",
                    tool_timeout().as_secs()
                ));
            }
            Err(e) => return Err(format!("cannot wait for the wallet tool: {e}")),
        }
    };
    let out_stdout = out_t.join().unwrap_or_default();
    let out_stderr = err_t.join().unwrap_or_default();
    let stdout = String::from_utf8_lossy(&out_stdout);
    let line = stdout
        .lines()
        .rev()
        .find_map(|l| l.split_once("KALYPTRA_RESULT ").map(|(_, j)| j.to_string()))
        .ok_or_else(|| {
            let err = String::from_utf8_lossy(&out_stderr);
            let tail: Vec<&str> = err.lines().rev().take(6).collect();
            format!(
                "the wallet tool gave no result (exit {:?}): {}",
                status.code(),
                tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
            )
        })?;
    serde_json::from_str(&line).map_err(|e| format!("bad wallet tool output: {e}"))
}

fn ok(v: Value) -> WireResult<Value> {
    if v["ok"] == Value::Bool(true) {
        Ok(v)
    } else {
        Err(v["error"]
            .as_str()
            .unwrap_or("wallet tool failed")
            .to_string())
    }
}

fn dir_arg(p: &Path) -> String {
    p.display().to_string()
}

impl Wallet {
    pub fn new(tool: impl Into<PathBuf>, dir: impl Into<PathBuf>) -> Self {
        Wallet {
            tool: tool.into(),
            dir: dir.into(),
        }
    }

    /// Build this wallet's action group for one leg: send `send` of `send_asset`, receive
    /// `recv` of `recv_asset`. Returns the group's bytes.
    pub fn leg(
        &self,
        send_asset: &str,
        send: u64,
        recv_asset: &str,
        recv: u64,
        out: &Path,
    ) -> WireResult<Vec<u8>> {
        ok(run(
            &self.tool,
            &[
                "leg",
                "--wallet",
                &dir_arg(&self.dir),
                "--send-asset",
                send_asset,
                "--send",
                &send.to_string(),
                "--recv-asset",
                recv_asset,
                "--recv",
                &recv.to_string(),
                "--out",
                &dir_arg(out),
            ],
        )?)?;
        std::fs::read(out).map_err(|e| format!("{}: {e}", out.display()))
    }

    /// Check every part in `plan` against its leg, assemble one transaction and mine it.
    pub fn assemble(&self, plan: &Path) -> WireResult<Assembled> {
        let v = run(
            &self.tool,
            &[
                "assemble",
                "--wallet",
                &dir_arg(&self.dir),
                "--plan",
                &dir_arg(plan),
            ],
        )?;
        if let Some(bad) = v["bad"].as_array() {
            return Ok(Assembled::BadParts(
                bad.iter()
                    .filter_map(|b| b.as_str().map(String::from))
                    .collect(),
            ));
        }
        if v["ok"] != Value::Bool(true) {
            let error = v["error"]
                .as_str()
                .unwrap_or("wallet tool failed")
                .to_string();
            // Only the node's own "Block rejected" answer means "nothing moved". Any other
            // failure (a dropped connection, a timeout after submitting) leaves it unknown.
            if error.contains("Block rejected") {
                return Ok(Assembled::Rejected(error));
            }
            return Err(error);
        }
        Ok(Assembled::Settled {
            txid: v["txid"].as_str().unwrap_or_default().to_string(),
            height: v["height"].as_u64().unwrap_or(0) as u32,
        })
    }

    /// This wallet's full viewing key (hex): it shows holdings and history, and cannot spend.
    pub fn view_key(&self) -> WireResult<String> {
        let v = ok(run(
            &self.tool,
            &["view-key", "--wallet", &dir_arg(&self.dir)],
        )?)?;
        v["fvk"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| "no viewing key".into())
    }

    /// As an auditor (this wallet is the auditor's own): the balances of the wallet whose
    /// viewing key is `fvk`, for each asset id given.
    pub fn audit(&self, fvk: &str, assets: &[String]) -> WireResult<Vec<(String, u64)>> {
        let mut args = vec![
            "audit".to_string(),
            "--wallet".into(),
            dir_arg(&self.dir),
            "--fvk".into(),
            fvk.to_string(),
            "--asset".into(),
        ];
        args.extend(assets.iter().cloned());
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let v = ok(run(&self.tool, &args)?)?;
        assets
            .iter()
            .map(|a| {
                v["balances"][a]
                    .as_u64()
                    .map(|b| (a.clone(), b))
                    .ok_or_else(|| format!("no balance reported for {a}"))
            })
            .collect()
    }

    /// Issue a new asset to this wallet, optionally capping its supply for good. Returns the
    /// asset id (hex).
    pub fn issue(&self, name: &str, amount: u64, finalize: bool) -> WireResult<String> {
        let amount = amount.to_string();
        let dir = dir_arg(&self.dir);
        let mut args = vec![
            "issue", "--wallet", &dir, "--name", name, "--amount", &amount,
        ];
        if finalize {
            args.push("--finalize");
        }
        let v = ok(run(&self.tool, &args)?)?;
        v["asset"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| "no asset id".into())
    }

    /// This wallet's balance of each asset id given, from one scan of the chain.
    pub fn balances(&self, assets: &[String]) -> WireResult<Vec<(String, u64)>> {
        let dir = dir_arg(&self.dir);
        let mut args = vec!["balances", "--wallet", &dir, "--asset"];
        args.extend(assets.iter().map(String::as_str));
        let v = ok(run(&self.tool, &args)?)?;
        assets
            .iter()
            .map(|a| {
                v["balances"][a]
                    .as_u64()
                    .map(|b| (a.clone(), b))
                    .ok_or_else(|| format!("no balance reported for {a}"))
            })
            .collect()
    }

    pub fn balance(&self, asset: &str) -> WireResult<u64> {
        let v = ok(run(
            &self.tool,
            &["balance", "--wallet", &dir_arg(&self.dir), "--asset", asset],
        )?)?;
        v["balance"].as_u64().ok_or_else(|| "no balance".into())
    }
}
