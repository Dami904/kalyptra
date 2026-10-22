//! Key generation run node by node. Nodes exchange only bytes.

use std::collections::BTreeMap;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use kalyptra_tse::{open, partial_decrypt, seal, Error, GroupKey, NodeKey, Participant};
use rand_core::OsRng;

fn round1_all(n: u16, t: u16) -> (Vec<Participant>, Vec<Vec<u8>>) {
    let mut nodes: Vec<Participant> = (1..=n)
        .map(|i| Participant::new(i, n, t).unwrap())
        .collect();
    let r1 = nodes
        .iter_mut()
        .map(|p| p.round1(&mut OsRng).unwrap())
        .collect();
    (nodes, r1)
}

/// Deliver each node's round 2 output to its recipients.
fn route(nodes: &mut [Participant], r1: &[Vec<u8>]) -> BTreeMap<u16, Vec<Vec<u8>>> {
    let mut inbox: BTreeMap<u16, Vec<Vec<u8>>> = BTreeMap::new();
    for p in nodes.iter_mut() {
        for m in p.round2(r1).unwrap() {
            inbox.entry(m.to).or_default().push(m.bytes);
        }
    }
    inbox
}

#[test]
fn nodes_that_only_exchange_bytes_agree_on_one_group_key_and_a_working_set_of_shares() {
    let (mut nodes, r1) = round1_all(5, 3);
    let mut inbox = route(&mut nodes, &r1);
    let mut keys: Vec<NodeKey> = Vec::new();
    let mut groups: Vec<GroupKey> = Vec::new();
    for p in nodes.iter_mut() {
        let (k, g) = p.round3(&r1, &inbox.remove(&p.id()).unwrap()).unwrap();
        keys.push(k);
        groups.push(g);
    }
    assert!(
        groups.windows(2).all(|w| w[0] == w[1]),
        "all nodes see the same group key"
    );
    let group = &groups[0];
    let env = seal(group.public_key(), b"round-1", b"BUY 10 @ 5", &mut OsRng);
    let ps: Vec<_> = [2u16, 4, 5]
        .iter()
        .map(|id| {
            partial_decrypt(
                keys.iter().find(|k| k.id() == *id).unwrap(),
                group,
                &env,
                b"round-1",
                &mut OsRng,
            )
            .unwrap()
        })
        .collect();
    assert_eq!(
        open(group, &env, b"round-1", &ps).unwrap().plaintext,
        b"BUY 10 @ 5"
    );
}

#[test]
fn nodes_running_in_separate_threads_with_channels_between_them_complete_the_dkg() {
    let (n, t) = (3u16, 2u16);
    // one inbox channel per node; a node can only talk to others by sending bytes
    let mut txs = Vec::new();
    let mut rxs = Vec::new();
    for _ in 0..n {
        let (tx, rx) = mpsc::channel::<(bool, Vec<u8>)>(); // (is_round2, bytes)
        txs.push(tx);
        rxs.push(rx);
    }
    let handles: Vec<_> = rxs
        .into_iter()
        .enumerate()
        .map(|(idx, rx)| {
            let id = idx as u16 + 1;
            let txs = txs.clone();
            thread::spawn(move || {
                let mut me = Participant::new(id, n, t).unwrap();
                let r1 = me.round1(&mut OsRng).unwrap();
                for (j, tx) in txs.iter().enumerate() {
                    if j as u16 + 1 != id {
                        tx.send((false, r1.clone())).unwrap();
                    }
                }
                // A faster node may send its round 2 message before this one has all of
                // round 1, as on a real network: keep it for later instead of failing. A
                // timeout turns a lost message into a failure rather than a hang.
                let wait = Duration::from_secs(30);
                let mut got_r1 = vec![r1];
                let mut got_r2 = Vec::new();
                while got_r1.len() < n as usize {
                    let (is2, b) = rx.recv_timeout(wait).expect("a round 1 message");
                    if is2 {
                        got_r2.push(b);
                    } else {
                        got_r1.push(b);
                    }
                }
                for m in me.round2(&got_r1).unwrap() {
                    txs[m.to as usize - 1].send((true, m.bytes)).unwrap();
                }
                while got_r2.len() < n as usize - 1 {
                    let (is2, b) = rx.recv_timeout(wait).expect("a round 2 message");
                    assert!(is2, "a second round 1 message from the same node");
                    got_r2.push(b);
                }
                me.round3(&got_r1, &got_r2).unwrap()
            })
        })
        .collect();
    let results: Vec<(NodeKey, GroupKey)> =
        handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(results.windows(2).all(|w| w[0].1 == w[1].1));
    let group = &results[0].1;
    let env = seal(group.public_key(), b"r", b"hello", &mut OsRng);
    let ps: Vec<_> = results[1..]
        .iter()
        .map(|(k, _)| partial_decrypt(k, group, &env, b"r", &mut OsRng).unwrap())
        .collect();
    assert_eq!(open(group, &env, b"r", &ps).unwrap().plaintext, b"hello");
}

#[test]
fn a_node_with_a_forged_round1_proof_is_named_and_the_key_generation_stops() {
    let (mut nodes, mut r1) = round1_all(3, 2);
    // node 3 publishes node 2's commitment with its own id: the proof of knowledge no longer
    // matches the sender
    let stolen: Vec<u8> = r1[1][2..].to_vec();
    r1[2] = [r1[2][..2].to_vec(), stolen].concat();
    let err = nodes[0].round2(&r1).unwrap_err();
    match err {
        Error::DkgBlame { node, .. } => assert_eq!(node, 3),
        other => panic!("expected blame on node 3, got {other:?}"),
    }
}

#[test]
fn a_corrupted_round1_message_is_rejected() {
    let (mut nodes, mut r1) = round1_all(3, 2);
    let last = r1[1].len() - 1;
    r1[1][last] ^= 0x01;
    assert!(nodes[0].round2(&r1).is_err());
}

#[test]
fn a_node_that_sends_a_wrong_share_in_round2_is_named() {
    let (mut nodes, r1) = round1_all(3, 2);
    let mut inbox = route(&mut nodes, &r1);
    // node 3 sends node 1 a share that does not match its commitments (swap in node 2's)
    let from2_to1 = inbox[&1].iter().find(|m| m[0] == 2).unwrap()[2..].to_vec();
    for m in inbox.get_mut(&1).unwrap().iter_mut() {
        if m[0] == 3 {
            *m = [m[..2].to_vec(), from2_to1.clone()].concat();
        }
    }
    let err = nodes[0].round3(&r1, &inbox[&1]).unwrap_err();
    match err {
        Error::DkgBlame { node, .. } => assert_eq!(node, 3),
        other => panic!("expected blame on node 3, got {other:?}"),
    }
}

#[test]
fn missing_duplicate_or_misaddressed_messages_are_rejected() {
    let (mut nodes, r1) = round1_all(3, 2);
    // a missing round 1 message
    assert!(nodes[0].round2(&r1[..2]).is_err());
    // a duplicate round 1 message
    let (mut nodes, r1) = round1_all(3, 2);
    let dup = vec![r1[0].clone(), r1[1].clone(), r1[1].clone()];
    assert!(nodes[0].round2(&dup).is_err());
    // a message claiming an impossible sender
    let (mut nodes, mut r1) = round1_all(3, 2);
    r1[1][0] = 9;
    assert!(nodes[0].round2(&r1).is_err());
    // round 3 with a missing round 2 message
    let (mut nodes, r1) = round1_all(3, 2);
    let inbox = route(&mut nodes, &r1);
    assert!(nodes[0].round3(&r1, &inbox[&1][..1]).is_err());
}

#[test]
fn a_round1_message_impersonating_this_node_is_rejected_not_skipped() {
    // Found in review: a message claiming this node's id used to be skipped silently.
    let (mut nodes, mut r1) = round1_all(3, 2);
    // replace node 1's own broadcast (as node 1 sees the list) with a forgery under its id
    r1[0] = [r1[0][..2].to_vec(), r1[1][2..].to_vec()].concat();
    assert!(nodes[0].round2(&r1).is_err());
}

#[test]
fn a_bad_input_list_does_not_consume_the_participant() {
    let (mut nodes, r1) = round1_all(3, 2);
    // first call with a missing message fails...
    assert!(nodes[0].round2(&r1[..2]).is_err());
    // ...and the same participant can still complete round 2 with the full list
    assert!(nodes[0].round2(&r1).is_ok());
}

#[test]
fn rounds_must_be_run_in_order_and_only_once() {
    let mut p = Participant::new(1, 3, 2).unwrap();
    assert!(p.round2(&[]).is_err(), "round 2 before round 1");
    p.round1(&mut OsRng).unwrap();
    assert!(p.round1(&mut OsRng).is_err(), "round 1 twice");
}

#[test]
fn bad_participant_parameters_are_rejected() {
    assert!(Participant::new(0, 3, 2).is_err());
    assert!(Participant::new(4, 3, 2).is_err());
    assert!(Participant::new(1, 3, 0).is_err());
    assert!(Participant::new(1, 3, 4).is_err());
}
