# All 109 rounds of the 2026-10-23 run

From `scripts/soak_chain.py --repeat 4 --random 98` against a fresh regtest chain. "Matches reference"
compares the nodes' price and volume with a separate computation from the orders placed;
"Balances reconciled" compares every trader's balances after the round with what the signed record says moved.

| Round | Situation | Market | Orders counted | Refused | Price / volume | Matches reference | Public check | Settlement | Balances reconciled | Result | Seconds |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | plain round, four investors | ACME/vUSD | 4 | 0 | 5 / 14 | yes | clean | settled at block 14, tx dbdd6596a4e233e9… | yes | as expected | 61 |
| 2 | orders the wallet cannot cover, or a second order in one asset | BOLT/vUSD | 3 | 2 | 3 / 20 | yes | clean | settled at block 15, tx d4b8e3e4198df7cf… | yes | as expected | 50 |
| 3 | too few orders: voided | CRUX/vUSD | 1 | 0 | voided | yes | clean | nothing opened | yes | as expected | 13 |
| 4 | nothing crosses | ACME/vUSD | 2 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 12 |
| 5 | node 3 (this round's settlement leader) down, back after the parts | ACME/vUSD | 2 | 0 | 5 / 6 | yes | clean | settled at block 16, tx 3758922809ce2c63… | yes | as expected | 106 |
| 6 | node 2 restarted between orders and close | CRUX/vUSD | 2 | 0 | 7 / 10 | yes | clean | settled at block 17, tx c54a45c4999232a2… | yes | as expected | 43 |
| 7 | partial fill of the marginal order | BOLT/vUSD | 3 | 0 | 2 / 9 | yes | clean | settled at block 18, tx e5d7cb0d16ef1cb8… | yes | as expected | 58 |
| 8 | plain round, repeated | ACME/vUSD | 3 | 0 | 4 / 3 | yes | clean | settled at block 19, tx 0ee9670349b56ef8… | yes | as expected | 52 |
| 9 | plain round, repeated | BOLT/vUSD | 3 | 0 | 4 / 4 | yes | clean | settled at block 20, tx 6cd4d0d352361577… | yes | as expected | 54 |
| 10 | plain round, repeated | CRUX/vUSD | 3 | 0 | 4 / 5 | yes | clean | settled at block 21, tx e4e88a646a873074… | yes | as expected | 55 |
| 11 | plain round, repeated | ACME/vUSD | 3 | 0 | 4 / 6 | yes | clean | settled at block 22, tx 1ff15c5c3aa79087… | yes | as expected | 58 |
| 12 | sell everything | ACME/vUSD | 3 | 0 | 7 / 3 | yes | clean | settled at block 23, tx 22bc109aec86fc3c… | yes | as expected | 46 |
| 13 | all at one limit | BOLT/vUSD | 3 | 0 | 7 / 2 | yes | clean | settled at block 24, tx baa77740c340adc7… | yes | as expected | 60 |
| 14 | all at one limit | CRUX/vUSD | 3 | 0 | 3 / 4 | yes | clean | settled at block 25, tx e9346d097b1da826… | yes | as expected | 60 |
| 15 | all at one limit | CRUX/vUSD | 2 | 0 | 6 / 1 | yes | clean | settled at block 26, tx 6c33b63a5af47a11… | yes | as expected | 43 |
| 16 | refused | CRUX/vUSD | 2 | 1 | 5 / 2 | yes | clean | settled at block 27, tx 61a4ee0f6822dd3c… | yes | as expected | 51 |
| 17 | refused | ACME/vUSD | 2 | 1 | 4 / 3 | yes | clean | settled at block 28, tx 7aa0d677ac83e4af… | yes | as expected | 48 |
| 18 | node 2 down (not the leader) | CRUX/vUSD | 3 | 0 | 7 / 1 | yes | clean | settled at block 29, tx aa4d6b95585af7b7… | yes | as expected | 54 |
| 19 | sell everything | ACME/vUSD | 3 | 0 | 4 / 9 | yes | clean | settled at block 30, tx e639c0923d782397… | yes | as expected | 64 |
| 20 | voided | ACME/vUSD | 1 | 0 | voided | yes | clean | nothing opened | yes | as expected | 24 |
| 21 | leader down (node 1) | ACME/vUSD | 4 | 0 | 6 / 6 | yes | clean | settled at block 31, tx 390ec5b801025bb6… | yes | as expected | 132 |
| 22 | refused | BOLT/vUSD | 3 | 1 | 2 / 5 | yes | clean | settled at block 32, tx 7e8f699ced36678f… | yes | as expected | 75 |
| 23 | node restarted | BOLT/vUSD | 3 | 0 | 8 / 6 | yes | clean | settled at block 33, tx cd95ab918c0beafa… | yes | as expected | 65 |
| 24 | sell everything | CRUX/vUSD | 2 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 23 |
| 25 | refused | CRUX/vUSD | 4 | 1 | 2 / 4 | yes | clean | settled at block 34, tx 4ff66517dd1d6df3… | yes | as expected | 73 |
| 26 | plain | CRUX/vUSD | 3 | 0 | 2 / 6 | yes | clean | settled at block 35, tx ccc0fec750effe0b… | yes | as expected | 79 |
| 27 | refused | BOLT/vUSD | 3 | 1 | 8 / 5 | yes | clean | settled at block 36, tx 9803e1dc3dac5057… | yes | as expected | 66 |
| 28 | plain | BOLT/vUSD | 4 | 0 | 6 / 8 | yes | clean | settled at block 37, tx 4027d8c59fc42936… | yes | as expected | 95 |
| 29 | leader down (node 3) | CRUX/vUSD | 3 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 42 |
| 30 | nothing crosses | BOLT/vUSD | 2 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 32 |
| 31 | node restarted | BOLT/vUSD | 3 | 0 | 3 / 5 | yes | clean | settled at block 38, tx cd34d862837c0713… | yes | as expected | 92 |
| 32 | plain | CRUX/vUSD | 4 | 0 | 3 / 6 | yes | clean | settled at block 39, tx 3b50d31fd52858af… | yes | as expected | 95 |
| 33 | plain | BOLT/vUSD | 3 | 0 | 7 / 1 | yes | clean | settled at block 40, tx fd6378757de4bf83… | yes | as expected | 70 |
| 34 | all at one limit | CRUX/vUSD | 4 | 0 | 5 / 4 | yes | clean | settled at block 41, tx cd7a2da81e376185… | yes | as expected | 91 |
| 35 | node 2 down (not the leader) | BOLT/vUSD | 4 | 0 | 7 / 4 | yes | clean | settled at block 42, tx 3cb0fdc2c2bad344… | yes | as expected | 101 |
| 36 | refused | BOLT/vUSD | 2 | 1 | None / 0 | yes | clean | nothing to settle | yes | as expected | 50 |
| 37 | sell everything | BOLT/vUSD | 3 | 0 | 3 / 14 | yes | clean | settled at block 43, tx 133c4b7432548064… | yes | as expected | 111 |
| 38 | all at one limit | BOLT/vUSD | 2 | 0 | 6 / 3 | yes | clean | settled at block 44, tx 7e3c66818c6fdd3d… | yes | as expected | 82 |
| 39 | node 2 down (not the leader) | CRUX/vUSD | 3 | 0 | 4 / 4 | yes | clean | settled at block 45, tx 583a985c6a0a51ab… | yes | as expected | 109 |
| 40 | refused | CRUX/vUSD | 4 | 1 | 4 / 4 | yes | clean | settled at block 46, tx 24d07abb1913f968… | yes | as expected | 98 |
| 41 | plain | BOLT/vUSD | 4 | 0 | 4 / 6 | yes | clean | settled at block 47, tx b8612b82f70a656a… | yes | as expected | 106 |
| 42 | voided | ACME/vUSD | 1 | 0 | voided | yes | clean | nothing opened | yes | as expected | 43 |
| 43 | leader down (node 2) | CRUX/vUSD | 3 | 0 | 5 / 2 | yes | clean | settled at block 48, tx 4e4e056f87e53ad7… | yes | as expected | 239 |
| 44 | sell everything | CRUX/vUSD | 3 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 48 |
| 45 | all at one limit | CRUX/vUSD | 5 | 0 | 4 / 5 | yes | clean | settled at block 49, tx 82d4db1f49cc0b12… | yes | as expected | 108 |
| 46 | node 1 down (not the leader) | CRUX/vUSD | 3 | 0 | 8 / 2 | yes | clean | settled at block 50, tx 91b5848d4b567499… | yes | as expected | 103 |
| 47 | sell everything | CRUX/vUSD | 3 | 0 | 3 / 8 | yes | clean | settled at block 51, tx f45d58a98c1c25fc… | yes | as expected | 120 |
| 48 | leader down (node 1) | ACME/vUSD | 3 | 0 | 2 / 3 | yes | clean | settled at block 52, tx 468114d90d9d5a22… | yes | as expected | 3362 |
| 49 | node restarted | ACME/vUSD | 4 | 0 | 8 / 8 | yes | clean | settled at block 53, tx 9657dfdc430081fe… | yes | as expected | 115 |
| 50 | voided | BOLT/vUSD | 1 | 0 | voided | yes | clean | nothing opened | yes | as expected | 43 |
| 51 | node restarted | CRUX/vUSD | 2 | 0 | 3 / 5 | yes | clean | settled at block 54, tx e0a1602563e7d5b0… | yes | as expected | 118 |
| 52 | new company NOVA, first trade | NOVA/vUSD | 3 | 0 | 7 / 8 | yes | clean | settled at block 57, tx 4f7cec14176ce56b… | yes | as expected | 102 |
| 53 | node restarted | CRUX/vUSD | 3 | 0 | 6 / 1 | yes | clean | settled at block 58, tx caa284bc06c7b50b… | yes | as expected | 129 |
| 54 | node restarted | BOLT/vUSD | 2 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 59 |
| 55 | node 1 down (not the leader) | NOVA/vUSD | 4 | 0 | 8 / 1 | yes | clean | settled at block 59, tx 779a7f0887af9282… | yes | as expected | 172 |
| 56 | refused | NOVA/vUSD | 3 | 1 | None / 0 | yes | clean | nothing to settle | yes | as expected | 40 |
| 57 | plain | CRUX/vUSD | 3 | 0 | 7 / 3 | yes | clean | settled at block 60, tx b03cb8efe8d18f29… | yes | as expected | 268 |
| 58 | plain | BOLT/vUSD | 2 | 0 | 4 / 2 | yes | clean | settled at block 61, tx d8009c57e5dc8f01… | yes | as expected | 114 |
| 59 | voided | NOVA/vUSD | 1 | 0 | voided | yes | clean | nothing opened | yes | as expected | 32 |
| 60 | voided | BOLT/vUSD | 1 | 0 | voided | yes | clean | nothing opened | yes | as expected | 33 |
| 61 | node 3 down (not the leader) | NOVA/vUSD | 5 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 57 |
| 62 | plain | ACME/vUSD | 3 | 0 | 7 / 3 | yes | clean | settled at block 62, tx 1e979bf658fddf67… | yes | as expected | 69 |
| 63 | plain | ACME/vUSD | 3 | 0 | 6 / 8 | yes | clean | settled at block 63, tx 543a9fca4a048169… | yes | as expected | 67 |
| 64 | plain | CRUX/vUSD | 3 | 0 | 4 / 2 | yes | clean | settled at block 64, tx bea591fd08931b37… | yes | as expected | 58 |
| 65 | leader down (node 3) | CRUX/vUSD | 3 | 0 | 5 / 7 | yes | clean | settled at block 65, tx ff452ff96ebffa12… | yes | as expected | 165 |
| 66 | sell everything | BOLT/vUSD | 3 | 0 | 5 / 5 | yes | clean | settled at block 66, tx 1e10adb8203daea4… | yes | as expected | 81 |
| 67 | all at one limit | NOVA/vUSD | 4 | 0 | 4 / 9 | yes | clean | settled at block 67, tx 60e1bf1708238db8… | yes | as expected | 119 |
| 68 | nothing crosses | NOVA/vUSD | 3 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 40 |
| 69 | plain | BOLT/vUSD | 3 | 0 | 7 / 1 | yes | clean | settled at block 68, tx f507ba552054742d… | yes | as expected | 119 |
| 70 | node restarted | CRUX/vUSD | 2 | 0 | 2 / 3 | yes | clean | settled at block 69, tx dbdca4b279fa7444… | yes | as expected | 138 |
| 71 | refused | CRUX/vUSD | 4 | 1 | 7 / 4 | yes | clean | settled at block 70, tx 9f1295bcbea69998… | yes | as expected | 85 |
| 72 | leader down (node 1) | ACME/vUSD | 2 | 0 | 2 / 6 | yes | clean | settled at block 71, tx 5a868d1d76171696… | yes | as expected | 263 |
| 73 | voided | NOVA/vUSD | 1 | 0 | voided | yes | clean | nothing opened | yes | as expected | 36 |
| 74 | plain | BOLT/vUSD | 3 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 50 |
| 75 | refused | ACME/vUSD | 3 | 1 | None / 0 | yes | clean | nothing to settle | yes | as expected | 92 |
| 76 | leader down (node 2) | NOVA/vUSD | 2 | 0 | 2 / 2 | yes | clean | settled at block 72, tx f1f38f1b922d45fe… | yes | as expected | 234 |
| 77 | nothing crosses | BOLT/vUSD | 3 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 77 |
| 78 | plain | NOVA/vUSD | 3 | 0 | 5 / 1 | yes | clean | settled at block 73, tx 0eef802bee12cd9c… | yes | as expected | 86 |
| 79 | plain | ACME/vUSD | 2 | 0 | 2 / 2 | yes | clean | settled at block 74, tx 5b570f7e76465413… | yes | as expected | 63 |
| 80 | all at one limit | ACME/vUSD | 2 | 0 | 6 / 1 | yes | clean | settled at block 75, tx 410f5ae1e73c10c7… | yes | as expected | 50 |
| 81 | refused | NOVA/vUSD | 3 | 1 | 8 / 2 | yes | clean | settled at block 76, tx 37c6162e67d4921a… | yes | as expected | 60 |
| 82 | leader down (node 2) | BOLT/vUSD | 2 | 0 | 4 / 4 | yes | clean | settled at block 77, tx 3464a3abdd196092… | yes | as expected | 116 |
| 83 | leader down (node 3) | ACME/vUSD | 3 | 0 | 5 / 2 | yes | clean | settled at block 78, tx 93a86b49f416f7b0… | yes | as expected | 111 |
| 84 | voided | NOVA/vUSD | 1 | 0 | voided | yes | clean | nothing opened | yes | as expected | 23 |
| 85 | plain | ACME/vUSD | 4 | 0 | 5 / 4 | yes | clean | settled at block 79, tx 11a5b7145b76a706… | yes | as expected | 77 |
| 86 | nothing crosses | NOVA/vUSD | 4 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 57 |
| 87 | plain | NOVA/vUSD | 3 | 0 | 4 / 2 | yes | clean | settled at block 80, tx 8ea17a7e788e20f4… | yes | as expected | 71 |
| 88 | leader down (node 2) | CRUX/vUSD | 5 | 0 | 7 / 9 | yes | clean | settled at block 81, tx 1f3885780674b5f2… | yes | as expected | 138 |
| 89 | node 2 down (not the leader) | CRUX/vUSD | 3 | 0 | 7 / 1 | yes | clean | settled at block 82, tx ed7d11f26bf2f461… | yes | as expected | 53 |
| 90 | leader down (node 1) | ACME/vUSD | 3 | 0 | 2 / 5 | yes | clean | settled at block 83, tx 94acaef1691eb3df… | yes | as expected | 113 |
| 91 | voided | NOVA/vUSD | 1 | 0 | voided | yes | clean | nothing opened | yes | as expected | 25 |
| 92 | new company ORBT, first trade | ORBT/vUSD | 2 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 28 |
| 93 | node restarted | ACME/vUSD | 5 | 0 | 5 / 7 | yes | clean | settled at block 86, tx 575f8c9244312621… | yes | as expected | 102 |
| 94 | plain | ACME/vUSD | 3 | 0 | 3 / 3 | yes | clean | settled at block 87, tx 1c42fcf1597a25fc… | yes | as expected | 53 |
| 95 | plain | NOVA/vUSD | 4 | 0 | 6 / 4 | yes | clean | settled at block 88, tx 55f3e9726f18dd7e… | yes | as expected | 54 |
| 96 | leader down (node 1) | CRUX/vUSD | 4 | 0 | 2 / 6 | yes | clean | settled at block 89, tx b6e2d8c21f3ab56f… | yes | as expected | 126 |
| 97 | plain | CRUX/vUSD | 4 | 0 | 7 / 5 | yes | clean | settled at block 90, tx d0328e23f358b014… | yes | as expected | 55 |
| 98 | plain | BOLT/vUSD | 3 | 0 | None / 0 | yes | clean | nothing to settle | yes | as expected | 32 |
| 99 | plain | BOLT/vUSD | 4 | 0 | 7 / 3 | yes | clean | settled at block 91, tx 7817ac022a487540… | yes | as expected | 57 |
| 100 | all at one limit | BOLT/vUSD | 3 | 0 | 7 / 3 | yes | clean | settled at block 92, tx e03473aaba6a9677… | yes | as expected | 56 |
| 101 | refused | BOLT/vUSD | 3 | 1 | 7 / 4 | yes | clean | settled at block 93, tx 1bc58536a7f7bd36… | yes | as expected | 72 |
| 102 | refused | ACME/vUSD | 3 | 1 | 6 / 2 | yes | clean | settled at block 94, tx 7730aad7e6c3f3e2… | yes | as expected | 58 |
| 103 | plain | CRUX/vUSD | 5 | 0 | 6 / 3 | yes | clean | settled at block 95, tx e2c64fbf6cc1d2ac… | yes | as expected | 67 |
| 104 | voided | ACME/vUSD | 1 | 0 | voided | yes | clean | nothing opened | yes | as expected | 31 |
| 105 | plain | ACME/vUSD | 5 | 0 | 4 / 3 | yes | clean | settled at block 96, tx d7ab50a55ac1dbfe… | yes | as expected | 82 |
| 106 | plain | NOVA/vUSD | 4 | 0 | 7 / 5 | yes | clean | settled at block 97, tx c010fd0c54f7de39… | yes | as expected | 89 |
| 107 | plain | ORBT/vUSD | 4 | 0 | 3 / 8 | yes | clean | settled at block 98, tx 41f7fcdc0a27d117… | yes | as expected | 69 |
| 108 | refused | ACME/vUSD | 4 | 1 | 4 / 5 | yes | clean | settled at block 99, tx 9a4ec1d2f3417534… | yes | as expected | 71 |
| 109 | plain | CRUX/vUSD | 5 | 0 | 2 / 10 | yes | clean | settled at block 100, tx 1c9b1f28216929c3… | yes | as expected | 97 |

- round 5: one minute after every part was in, with the leader down, settlement was 'settled'
- round 21: one minute after every part was in, with the leader down, settlement was 'settled'
- round 43: one minute after every part was in, with the leader down, settlement was 'settled'
- round 48: one minute after every part was in, with the leader down, settlement was 'settled'
- round 65: one minute after every part was in, with the leader down, settlement was 'settled'
- round 72: one minute after every part was in, with the leader down, settlement was 'settled'
- round 76: one minute after every part was in, with the leader down, settlement was 'settled'
- round 82: one minute after every part was in, with the leader down, settlement was 'settled'
- round 83: one minute after every part was in, with the leader down, settlement was 'settled'
- round 88: one minute after every part was in, with the leader down, settlement was 'settled'
- round 90: one minute after every part was in, with the leader down, settlement was 'settled'
- round 96: one minute after every part was in, with the leader down, settlement was 'settled'
