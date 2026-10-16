//! # kalyptra-node
//!
//! Runs Kalyptra matcher nodes as HTTP servers, and provides the client and checker used by
//! investors, operators and anyone verifying a round. The `kalyptra` binary wraps both.
//!
//! * [`server`]: a node: distributed key generation with its peers, the bulletin board, gossip,
//!   and automatic progression of rounds (release partials once committed, clear once opened).
//! * [`client`]: open and close rounds, submit sealed orders, wait for the result, check it,
//!   and settle a filled order.
//! * [`wire`]: the JSON forms of everything on the network and in files.
//! * [`gateway`]: the local web server the Kalyptra page talks to (keys and wallets stay local).
//! * [`bundle`]: a round saved to one file, and verified offline.
//! * [`files`]: node identity and key-share files.
//! * [`log`]: one-line JSON records for failures and outcomes.
//! * [`wallet`]: drives a Zcash wallet (QEDIT's tool with Kalyptra's commands) to build and
//!   assemble settlement swaps.
//!
//! Local demo only: plain HTTP, no TLS, and the operator endpoints are not access-controlled.

pub mod bundle;
pub mod client;
pub mod files;
pub mod gateway;
pub mod log;
pub mod server;
pub mod wallet;
pub mod wire;
