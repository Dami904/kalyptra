//! A small in-process network of matcher nodes for tests. Messages move by function call;
//! nothing here is trusted by the code under test.
#![allow(dead_code)]

use ed25519_dalek::SigningKey;
use kalyptra_matcher::{
    round_aad, CommitVote, MatcherNode, Order, PublicRound, Receipt, RecordVote, Roster,
    RoundConfig, Side,
};
use kalyptra_tse::{run_local_dkg, seal, GroupKey, PartialDecryption};
use rand_core::{OsRng, RngCore};

pub const MARKET: &str = "ACME/vUSD";

pub struct Net {
    pub nodes: Vec<MatcherNode>,
    pub group: GroupKey,
    pub roster: Roster,
    /// Copies of the nodes' signing keys, for simulating colluding nodes in tests.
    pub keys: Vec<SigningKey>,
}

pub fn cfg(round_id: u64, min_orders: usize) -> RoundConfig {
    RoundConfig {
        round_id,
        market: MARKET.into(),
        min_orders,
    }
}

impl Net {
    pub fn new(n: u16, t: u16) -> Net {
        let (shares, group) = run_local_dkg(n, t, &mut OsRng).unwrap();
        let keys: Vec<SigningKey> = (0..n).map(|_| SigningKey::generate(&mut OsRng)).collect();
        let roster = Roster {
            threshold: t as usize,
            nodes: keys
                .iter()
                .enumerate()
                .map(|(i, k)| (i as u16 + 1, k.verifying_key()))
                .collect(),
        };
        let nodes = shares
            .into_iter()
            .zip(keys.iter().cloned())
            .map(|(s, k)| MatcherNode::new(s.id(), k, s, group.clone(), roster.clone()).unwrap())
            .collect();
        Net {
            nodes,
            group,
            roster,
            keys,
        }
    }

    pub fn node(&mut self, id: u16) -> &mut MatcherNode {
        &mut self.nodes[id as usize - 1]
    }

    pub fn open_round(&mut self, c: RoundConfig) {
        for n in &mut self.nodes {
            n.open_round(c.clone()).unwrap();
        }
    }

    /// Sign an order with a FRESH investor key and seal it for `round_id`.
    pub fn sealed(&self, round_id: u64, side: Side, size: u64, limit: u64) -> Vec<u8> {
        self.sealed_with(round_id, round_id, side, size, limit)
    }

    /// Seal for `seal_round`, with `order_round` written inside the order.
    pub fn sealed_with(
        &self,
        seal_round: u64,
        order_round: u64,
        side: Side,
        size: u64,
        limit: u64,
    ) -> Vec<u8> {
        self.sealed_keyed(seal_round, order_round, side, size, limit)
            .0
    }

    /// Like [`Net::sealed_with`], also returning the order's one-time key.
    pub fn sealed_keyed(
        &self,
        seal_round: u64,
        order_round: u64,
        side: Side,
        size: u64,
        limit: u64,
    ) -> (Vec<u8>, SigningKey) {
        let key = SigningKey::generate(&mut OsRng);
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let order = Order {
            side,
            size,
            limit,
            round_id: order_round,
            market: MARKET.into(),
            salt,
            investor: key.verifying_key().to_bytes(),
        };
        let signed = order.sign(&key).unwrap();
        let env = seal(
            self.group.public_key(),
            &round_aad(seal_round, MARKET),
            &signed,
            &mut OsRng,
        )
        .to_bytes();
        (env, key)
    }

    /// Submit to the listed nodes, then gossip the envelope and every receipt to all nodes.
    pub fn submit_to(&mut self, round_id: u64, bytes: &[u8], to: &[u16]) -> Vec<Receipt> {
        let receipts: Vec<Receipt> = to
            .iter()
            .map(|id| self.node(*id).submit(round_id, bytes).unwrap())
            .collect();
        for n in &mut self.nodes {
            n.learn_envelope(round_id, bytes).unwrap();
            for r in &receipts {
                n.learn_receipt(r).unwrap();
            }
        }
        receipts
    }

    pub fn submit_all(&mut self, round_id: u64, bytes: &[u8]) -> Vec<Receipt> {
        let ids: Vec<u16> = self.nodes.iter().map(|n| n.id()).collect();
        self.submit_to(round_id, bytes, &ids)
    }

    /// Close on the listed nodes and gossip their votes to every node.
    pub fn close(&mut self, round_id: u64, on: &[u16]) -> Vec<CommitVote> {
        let votes: Vec<CommitVote> = on
            .iter()
            .map(|id| self.node(*id).close(round_id).unwrap())
            .collect();
        for n in &mut self.nodes {
            for v in &votes {
                n.learn_commit_vote(v).unwrap();
            }
        }
        votes
    }

    pub fn close_all(&mut self, round_id: u64) -> Vec<CommitVote> {
        let ids: Vec<u16> = self.nodes.iter().map(|n| n.id()).collect();
        self.close(round_id, &ids)
    }

    pub fn release(
        &mut self,
        round_id: u64,
        from: &[u16],
    ) -> Vec<(kalyptra_matcher::Hash, PartialDecryption)> {
        let mut all = Vec::new();
        for id in from {
            all.extend(
                self.node(*id)
                    .release_partials(round_id, &mut OsRng)
                    .unwrap(),
            );
        }
        self.gossip_partials(round_id, &all);
        all
    }

    pub fn gossip_partials(
        &mut self,
        round_id: u64,
        ps: &[(kalyptra_matcher::Hash, PartialDecryption)],
    ) {
        for n in &mut self.nodes {
            for (h, p) in ps {
                // a node rejects partials that do not verify; tests check that separately
                let _ = n.learn_partial(round_id, *h, p);
            }
        }
    }

    pub fn clear(&mut self, round_id: u64, on: &[u16]) -> Vec<RecordVote> {
        let votes: Vec<RecordVote> = on
            .iter()
            .map(|id| self.node(*id).clear(round_id).unwrap())
            .collect();
        for n in &mut self.nodes {
            for v in &votes {
                let _ = n.learn_record_vote(v);
            }
        }
        votes
    }

    /// Run a whole round on every node.
    pub fn run_all(&mut self, round_id: u64) {
        let ids: Vec<u16> = self.nodes.iter().map(|n| n.id()).collect();
        self.close(round_id, &ids);
        self.release(round_id, &ids);
        self.clear(round_id, &ids);
    }

    /// Merge what every node saw into one public view.
    pub fn public(&self, round_id: u64) -> PublicRound {
        let mut p = self.nodes[0].export(round_id).unwrap();
        for n in &self.nodes[1..] {
            let q = n.export(round_id).unwrap();
            p.receipts.extend(q.receipts);
            p.commit_votes.extend(q.commit_votes);
            p.partials.extend(q.partials);
            p.record_votes.extend(q.record_votes);
            p.envelopes.extend(q.envelopes);
        }
        p.receipts.sort_by_key(|r| (r.envelope, r.node));
        p.receipts.dedup();
        p.commit_votes.sort_by_key(|v| (v.epoch, v.node));
        p.commit_votes.dedup();
        p.record_votes.sort_by_key(|v| v.node);
        p.record_votes.dedup();
        p.envelopes.sort();
        p.envelopes.dedup();
        p
    }
}
