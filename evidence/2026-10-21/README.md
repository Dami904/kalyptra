# Evidence from a run on the regtest chain

Saved by `scripts/soak_chain.py --evidence` while the chain was running. Public data only.

Check any round offline, trusting nobody (no node, no chain, no network):

```bash
kalyptra verify --bundle round-1.bundle.json --roster roster.json \
  --group-key 6a4fe8cab7c848407fefe52af3e05266d12d43c42ddf43b54d7aa9f7f11e3e66
```

It recomputes the price from the published orders and checks every receipt, vote, decryption share and signature. Change one number in a bundle and it fails.

Each `round-N.tx.json` is the settlement as the chain's own node reported it (`getrawtransaction`, `getblock`), and `round-N.tx.hex` is the transaction's raw bytes: the swap is on chain, and its amounts and parties are shielded.

| Round | Bundle checked offline | Transaction id | Block | Block hash | In that block |
|---|---|---|---|---|---|
| 1 | verified offline | `a082c0f3abc7d263fc13a19babce70ce458deaba23a3d2b6fa41738a6ae3eae8` | 14 | `2934a5a45fad0c574cb171e93e36a9e7aedc27b8cf7872c06dea174245ff4258` | yes |
| 2 | verified offline | `78af9b478b062623b9f591d82fa2ed3221e431507010a7218358464a25f0e64a` | 15 | `dc85ae0d6cda849731653e6e8b5f5340bf346efd5499bf07540dc8e2508d81d0` | yes |
| 3 | verified offline | none (nothing moved) |  |  |  |
| 4 | verified offline | none (nothing moved) |  |  |  |
| 5 | verified offline | `e7af9be2185757e5985415626a1d05a6a0ff34bad6f89ed373fa4bb8a3398bd5` | 16 | `c9167573f5751b9122b009ecc10e8e0c383e2b23d286a86eb6a4f6f041810475` | yes |
| 6 | verified offline | `5852bf79f7b376a4f1d4fdec5be52699c29183c91232d7f4ad9bb73e9fa96431` | 17 | `627ad8cfd189a21d4a86bee40d6c67dcb579dfe4a2232158870aa6f3ee3235e8` | yes |
| 7 | verified offline | `d1519e30b815366fce051c0ed866adbfb2fcdb9009ea90ba1826de0295e6940c` | 18 | `01c56c52a630503a9c0935003cd1e40d4c41e2e4f4c58333b2946c697b70f0e3` | yes |
| 8 | verified offline | `25dbf84e5c4bb863f6400edbe2307d4fa933cdd6e216c295275a7616ab6aed58` | 19 | `89290aa9319b569e1784b78788668601b5109744c74e05ec8e9cd4245b3f8eda` | yes |
| 9 | verified offline | `de42146e761c82633d27f46650d537a6d549b321843331db999007b60486d7a3` | 20 | `4cb598d70d827b7031d4473c9a30a26212a14cd9179a95664794ccc666a45af1` | yes |
| 10 | verified offline | `278084ba5c5482279529b748053651d3ecb683121569c04ebfabd33c953dc86e` | 21 | `dfb98aaff135b9a25855356fe7f9f20fe797b5e498750a56b60120dff8067199` | yes |
| 11 | verified offline | `e832d2abe6274e738c646f7a340e4e64101a2c7da2dd76f4ad4de78075ad521f` | 22 | `9b71fca84debf9e08b09d1fb6d84361922aee817a2b6f5b3424bdc156029e77c` | yes |
