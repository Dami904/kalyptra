# Evidence from a run on the regtest chain

Saved by `scripts/soak_chain.py --evidence` while the chain was running. Public data only.

Check any round offline, trusting nobody (no node, no chain, no network):

```bash
kalyptra verify --bundle round-1.bundle.json --roster roster.json \
  --group-key 985750072683aea0c8e11c633dd964803882bd860e18a553d527d5ada74f511e
```

It recomputes the price from the published orders and checks every receipt, vote, decryption share and signature. Change one number in a bundle and it fails.

Each `round-N.tx.json` is the settlement as the chain's own node reported it (`getrawtransaction`, `getblock`), and `round-N.tx.hex` is the transaction's raw bytes: the swap is on chain, and its amounts and parties are shielded.

| Round | Bundle checked offline | Transaction id | Block | Block hash | In that block |
|---|---|---|---|---|---|
| 1 | verified offline | `dbdd6596a4e233e9d6b8e913a3e34edd3fe4886cebc2a4f88a7f02e62eb2e87a` | 14 | `b5d49b7f80a49998176231a94345939aa8ce32ae8145b3d2291323bc6ad69bda` | yes |
| 2 | verified offline | `d4b8e3e4198df7cfab64b3dbf88c58d6aea66f5de147c96a107ab06a962106a2` | 15 | `ea19a4932546eecca235fc3e21169e1d885f4d3c17999f313a712119272a4c01` | yes |
| 3 | verified offline | none (nothing moved) |  |  |  |
| 4 | verified offline | none (nothing moved) |  |  |  |
| 5 | verified offline | `3758922809ce2c6397626cbf335c81327a5fec4cf7b41177b31e031e6756770b` | 16 | `c85f86b21133168b8fbf932ba847c830d5025fbccc78fad2e859e0c7f805692e` | yes |
| 6 | verified offline | `c54a45c4999232a23dd8caf043499e1bde772b38c599fda91e4e3d2782352d92` | 17 | `8532c552b1939db0322ff9a8fd3fbb0aba633b7e8a769026f0f306b7eae6c040` | yes |
| 7 | verified offline | `e5d7cb0d16ef1cb8bf17bc1813d7117d6049cfa494f8589a797ccb6b5953d79e` | 18 | `8520a56e57e5cae05c3307df39e6e81425fc5043a461536ec627a2536bfdecd9` | yes |
| 8 | verified offline | `0ee9670349b56ef84fa51ba1251b172b9423ad11d368bfeadcae8e322cd61f6e` | 19 | `65a23ab5f5b7f427bec0763e1c9d245c0610790be7a010a66ce9fa09ab488d69` | yes |
| 9 | verified offline | `6cd4d0d352361577165e7f21997e38b340202bcf02dfb74abf96a6aa0f37d0ff` | 20 | `5e7904590385b6533da2a00d8b8efb59308450e26fc2b44f592aab674765d09b` | yes |
| 10 | verified offline | `e4e88a646a873074e0ad37a23b0bfdd322495de00d5d78291d7f22a11ff80a8d` | 21 | `6b15f18153eb84f7cd95c2a5fba8961868e1413c21df3529b4b0573c86c0c642` | yes |
| 11 | verified offline | `1ff15c5c3aa7908729662f41dcd63f85d100026caa8f1b310d4372f8e6927b40` | 22 | `6fdcde1894f7811c629c65748570534f37b86635b265b056f57e852b63a1b3ff` | yes |
| 12 | verified offline | `22bc109aec86fc3c29c27bebc2815541ea1a1cfa31597efb894d8871aae62e90` | 23 | `8869f5ce580efa86d674cec8de1a8e774d2b0c4f23bed7dfe6fd9aacfb94dd39` | yes |
| 13 | verified offline | `baa77740c340adc74ab61e1a1c387cd7719c05683b00b1dff9f56dc94a406f9f` | 24 | `cc211e6d9b6f245fba463ee9257f4d6f0c93ed417393710e3d5b5646bb52c062` | yes |
| 14 | verified offline | `e9346d097b1da82654ce1fd7bcbeabcef36ef4b37f96281f53d2a19946e9ea08` | 25 | `a9fe0cc2ef3cf338c30a6ef656de700da4f70ecb22b05054804b0fd842d47789` | yes |
| 15 | verified offline | `6c33b63a5af47a113b13bafc91049fcca3988d8605808130bdaf620cce0f81d0` | 26 | `1a4e15c69b90b7936555aa6ac6b787a597add0fd806233b4f729c92868913cec` | yes |
| 16 | verified offline | `61a4ee0f6822dd3cf3145eacf7461378fb53b19bf1a5893bf1dfaf390680e89b` | 27 | `c9a672aa4a0fb1a3a842c595a0517909a15c2e01c6dfa780b8ae889f20e39c78` | yes |
| 17 | verified offline | `7aa0d677ac83e4af0787307b75951f6244594341c2421f7f89f2b3cd00aefd8b` | 28 | `615f7395b1efabe1541f01170efc87dfff03fca37dc30b515b7e2c6da4566cdd` | yes |
| 18 | verified offline | `aa4d6b95585af7b7a31d9b9d32dc30ebdbb8c842ea62fa199e6cd6cf2c797ddc` | 29 | `f391f46d297fbed97471632b6e0e15b8d422507c8abc45e4ac55fa9bf56d10cf` | yes |
| 19 | verified offline | `e639c0923d7823973b0120e9f0316fa8e5204ac974242fe8164c3074e9053b93` | 30 | `08e4b8e15bb1dac3fa7dbe1bcf8e59f6b816ed5f58a10e7638cbf459d5b1ca1e` | yes |
| 20 | verified offline | none (nothing moved) |  |  |  |
| 21 | verified offline | `390ec5b801025bb67456270b966e567cf9622727cf01aad6fa8d0ed1507fc369` | 31 | `9d76ce70e771b2c59d340285b36d76b5f7553b99b5f346a5cc4d1d5f8bd3ce59` | yes |
| 22 | verified offline | `7e8f699ced36678f2f5ef693a8326190710e4a167f84410e6fa473b3da97c310` | 32 | `b2ed25e3a4d600b5773cd67ad30e269b30a4402641e6fcefe34c584c17a46e5d` | yes |
| 23 | verified offline | `cd95ab918c0beafa1e055bda378dd1188a4e5cf9d77e049cce8a3c2b0689d847` | 33 | `17f29768c71dc6fbcaeb1110f9410aa45d6154f7ec074e3d46986bff04124949` | yes |
| 24 | verified offline | none (nothing moved) |  |  |  |
| 25 | verified offline | `4ff66517dd1d6df3e6e3d699ee30717801dd638e4291122ead2e61b38785f75a` | 34 | `b59bf6898c40d4e3951fe65a4d352b4b6a08909d0645d8f0150e21482bdb7c98` | yes |
| 26 | verified offline | `ccc0fec750effe0b9e3f4db9264b21bd9783b04e5ff4a523a9896ad9d03c2f9f` | 35 | `9edc53f7f805f61158501943da2d66c852177aa49f17024e85dc79af4e4d7ce9` | yes |
| 27 | verified offline | `9803e1dc3dac5057ef63401daff2216ee375ef13381a892deb3d88803039c77a` | 36 | `497ba6c9586545f130e98fc517329f4003aadc38a81efe0d11d9fa5ffa68e721` | yes |
| 28 | verified offline | `4027d8c59fc429365889012400971334edeaa65b10fcffdbcb07ab00a204ff3a` | 37 | `bed1abd11258e637acbf0da2f148ff608633124f9b2fa88f38e0388cf2ef4ecf` | yes |
| 29 | verified offline | none (nothing moved) |  |  |  |
| 30 | verified offline | none (nothing moved) |  |  |  |
| 31 | verified offline | `cd34d862837c0713402348f081167bdbd4e76f5e4a9dfdd694011f21e9d9cc78` | 38 | `6d9a786682a37cdb4de536dd77707236fad0e5b507893084227fa7baaaf47760` | yes |
| 32 | verified offline | `3b50d31fd52858af66fcaf9bb199317037e18b234862e867bfc10015a2dc2fe2` | 39 | `8b5da279f7a0923933aa2f9bb30e19aad6edd9960895fe6a353c02195a2c789f` | yes |
| 33 | verified offline | `fd6378757de4bf8365a0c5cbf4220ddfc9a408ccf757020772ec991a4d867eb0` | 40 | `f350ced0ed0e03127dc3796d1dfd44fe2a9c597b919fb4d4bfb22b722d746083` | yes |
| 34 | verified offline | `cd7a2da81e376185fa684b03bd05276ea766b969c560b2a9da0f290e5a059151` | 41 | `9803c7c7fdd2cb280203fcf369190cf8146976d04f919884067f5af21b185312` | yes |
| 35 | verified offline | `3cb0fdc2c2bad3444ef55de139ca10c4bb8877149237720d85a40ba37a2be693` | 42 | `4bcc976c9343a664af0b1e3a9a566eb11e4310951a0e50c887c8d68b10953be8` | yes |
| 36 | verified offline | none (nothing moved) |  |  |  |
| 37 | verified offline | `133c4b743254806444ba42ae9f0cc66943de0a6c27f492cf714c7e7447450736` | 43 | `c806cab73052a9c1413393e6f834156a936b216f723bf179c33d2b87f2115eff` | yes |
| 38 | verified offline | `7e3c66818c6fdd3dc4efd85af6f499d115be543d8d0d633a628a7de79d7903ae` | 44 | `207333134448ebdf10113e64879fe6576ec206d79e114e2589bde490b4d3c19d` | yes |
| 39 | verified offline | `583a985c6a0a51ab54eb2ba15d3e669eb1f1816d5f26d4ae2a1a4df926f99378` | 45 | `d608d83f4f850c2203cb1686d279fe1bba1e491b3f3d59bb1d7b379735ea8ebc` | yes |
| 40 | verified offline | `24d07abb1913f968bcef49ba1aa2f9c24f30fba4bc1970043e7877a0b84459cb` | 46 | `54777fbeab04e4d8acfa6dc1fa35314170c9350d72d18f758e04425d71b3b52a` | yes |
| 41 | verified offline | `b8612b82f70a656a11b46481a570cb0b804ad7c6b22d4a51301af93f9ed68519` | 47 | `e2e46dd9789eb999b73ff1be1f42da277a6974dbf426169530e3767bcfa2c768` | yes |
| 42 | verified offline | none (nothing moved) |  |  |  |
| 43 | verified offline | `4e4e056f87e53ad74df138edafef383405aef1b41d714c4766cd8c0f578f5314` | 48 | `cf5ab5acd4cd61f47c4d111fe15c0b3ef15d3ba5c624669fb4087ca74242d873` | yes |
| 44 | verified offline | none (nothing moved) |  |  |  |
| 45 | verified offline | `82d4db1f49cc0b1262101628039db03c67d4952bdb751bb0886594918e8cb71f` | 49 | `57df9e09760cbd857f662b4d495d9d2267a7e52b2db3491091dea8402f77a95f` | yes |
| 46 | verified offline | `91b5848d4b567499ebb3b0fc6d1887277330f7971f4d09b1473e5aecfccdb1cf` | 50 | `9aa04a0cffb45ecd3202da262e0265375d4bf73abe777ccbe9e869badafd3797` | yes |
| 47 | verified offline | `f45d58a98c1c25fc4acec70e8ffd71f71125e919c55b1e534b7df4eb904b0101` | 51 | `f8a9fff54394db8d09654533c3839c121f1e640db211a13f8f7112ecb1b85f4b` | yes |
| 48 | verified offline | `468114d90d9d5a222fba291f9fd7aba75eb7582307315e22de324c9c0161df61` | 52 | `996d1fe4c08bba3ce041898a3d5c2260487efea0c0ea0558293c75e87960f3df` | yes |
| 49 | verified offline | `9657dfdc430081fea9aa200bbe0e4a2918061b79bbb121e56cd5a356542e797f` | 53 | `dfb4258e6bbc0c96d3d9736ad864862e9e1c7c0e362e87357f16bd3693289f41` | yes |
| 50 | verified offline | none (nothing moved) |  |  |  |
| 51 | verified offline | `e0a1602563e7d5b09efcf485f23b06752725bf144d8ce92d13fe72ccffcd7986` | 54 | `b84eafd574451c8bb1d4815bf588dd4eb5733a5ac52b0ea9da74c6780a746b69` | yes |
| 52 | verified offline | `4f7cec14176ce56bf8386d7e8e9fac146b9f7232d54874a6a9414ff04a30ddf5` | 57 | `61cee7d7ed7dc8637d7b517598884af064a83ef2a5e8bda4e5e11a8e035630f7` | yes |
| 53 | verified offline | `caa284bc06c7b50b15385c733cd73f85ddfd9fda1bd4e7487a486a5def3cc59c` | 58 | `55449c179b89876d503cf7885c29adbef4131c3d6a850d4e37ae2d8c7d820b0c` | yes |
| 54 | verified offline | none (nothing moved) |  |  |  |
| 55 | verified offline | `779a7f0887af92825cea9b8bfffa33f3a5dbfdb58eb23784242f2d6b60287531` | 59 | `a169bc340c9f8784c033b173f3eba3adecbdfdb8ef8f282c9f6612a36a0bf80f` | yes |
| 56 | verified offline | none (nothing moved) |  |  |  |
| 57 | verified offline | `b03cb8efe8d18f290e50c6d519136907110167eb7048605e1ef0060527988ec5` | 60 | `d61fe3609583246d29c060bcbbd2f5c95340079598cf2a283b33fdb6258ed1f9` | yes |
| 58 | verified offline | `d8009c57e5dc8f0144ff962dedc965664973c1f121682979fd59194fd77a4b0e` | 61 | `28c16f9b13586cea034f3b4215ad9f3c36f3a30bedc7bdceede92af6d3cd62e5` | yes |
| 59 | verified offline | none (nothing moved) |  |  |  |
| 60 | verified offline | none (nothing moved) |  |  |  |
| 61 | verified offline | none (nothing moved) |  |  |  |
| 62 | verified offline | `1e979bf658fddf672d6d039ad87a8af6c4eccdc7d1b7d75086b41870b3e0e541` | 62 | `3381e4d19845dd24ecf2b51188fe53414d5d73213153f5814e4bff0bcd0e2c97` | yes |
| 63 | verified offline | `543a9fca4a0481694e274834512f8594fccaa616f6d4e5bee2f2c971fc815b72` | 63 | `460800abf2b13e94d2cdd90e5c48b0d695ed16cbca5f45548b230cb74b01ed67` | yes |
| 64 | verified offline | `bea591fd08931b375c4181eebe9f64d5f4d87b4eecbbba3b0bb12db8c8759db8` | 64 | `861117682c5f0f20bba4c5bb62793ed3d0ff056dfb39ddd89315200e59b17995` | yes |
| 65 | verified offline | `ff452ff96ebffa125899b26ed3f80495ce18c2ef7d206b0306ec7f36e5595e4b` | 65 | `362005084d43d6fc0910211a9d3f994860770aef2156467a3095431ca7bbcb14` | yes |
| 66 | verified offline | `1e10adb8203daea46532d0f326219aa4b5c3b5719c49325f03664b8ce5846f39` | 66 | `679d03709a909a6a534da068ae0eb63a18d2f4328457c4f2fee934acc8ba6675` | yes |
| 67 | verified offline | `60e1bf1708238db80f780bb23e4bbf9353ff26d5671d506544aef02d808aa365` | 67 | `bcf5e01503b842085d39811a25d69cc5867032af66e3a0b18f73275819b3a02b` | yes |
| 68 | verified offline | none (nothing moved) |  |  |  |
| 69 | verified offline | `f507ba552054742d22b71d5682ff78fcc975044a50676fb2b157820f4f607f3a` | 68 | `1e2caf4eeb265e691de581696d7917868e0d2006e5d3d8d2325eb4140e7cb0f5` | yes |
| 70 | verified offline | `dbdca4b279fa7444c38ff2311697f84d6eeef2f248e0f3a788d9413ca0d0b05f` | 69 | `0269e3669639f8696bc36732b5325bad4a0139c62f02f450e1d1693a1a5cdde9` | yes |
| 71 | verified offline | `9f1295bcbea699986ac13a6747a7e54e669314ba5d52fa62187589a806730979` | 70 | `b048206dadaffbe751bca611292887e03d2ff9be36bb68d5391ac5dd74835a7c` | yes |
| 72 | verified offline | `5a868d1d761716969cfea8eead2632a95870a7b6a8eee41fdae2dfd2ce41adcc` | 71 | `52ca406e319c65c4b86542b769af3e780c4c727df19b5c9a6b08ff976225015e` | yes |
| 73 | verified offline | none (nothing moved) |  |  |  |
| 74 | verified offline | none (nothing moved) |  |  |  |
| 75 | verified offline | none (nothing moved) |  |  |  |
| 76 | verified offline | `f1f38f1b922d45fe0fea2d7c7221d9f1a7a93a2c8697c74a2f8be500a2700dc9` | 72 | `3194d2df8f4da65a4fc93d8617013dd69530914af9298cdb45399e932171aa6a` | yes |
| 77 | verified offline | none (nothing moved) |  |  |  |
| 78 | verified offline | `0eef802bee12cd9c9b9f123214f504a6503483ca1254065d41d2c166d6e16158` | 73 | `117a216028ed9f23126bb31106403f69ef19f1dd55e1bb53b326ef6337535cf3` | yes |
| 79 | verified offline | `5b570f7e764654137f6c492249995d5f7c260799e7386128a232aba89edb8da8` | 74 | `a17316ab85e2c50cc17ae35693808846951576d3607ba3667fbf031990c1f1af` | yes |
| 80 | verified offline | `410f5ae1e73c10c7304ee8f8c531241cfaa379d9c098ae564ba5b0e736206e9d` | 75 | `ebf9e06357b5da8ceb670002e193fc264701d6d9df34cce98ead7bb02c2bf59d` | yes |
| 81 | verified offline | `37c6162e67d4921a2ce6a3bdb0999302d3cdbf89ab5a2d2fbe0f26891a4c0c5d` | 76 | `416aeffdfb0dc471274ec015a8bee777c38b131afb3579bd73f6de3820208d57` | yes |
| 82 | verified offline | `3464a3abdd1960920c167afd59693081ff7a865a16cc299f12b9945809529d1b` | 77 | `f3dbd15f7426a5cfab56233a4cbfba9e8e45e3c19640047453d1ccb6ca941e86` | yes |
| 83 | verified offline | `93a86b49f416f7b0987978f6c935d644db827e0ff5c75e7c542154d66a0e1b1c` | 78 | `003c1040fcf0f56e9b81d960195fb930164f8fb0b863875ec7213b6565029435` | yes |
| 84 | verified offline | none (nothing moved) |  |  |  |
| 85 | verified offline | `11a5b7145b76a7061cc5794f93bd4546976639618bb6ca55a7c12ed02030a6c7` | 79 | `f44d395a1e6b6dd3242435f0262d2a17855b96af555b3f6f2e738e11e1907432` | yes |
| 86 | verified offline | none (nothing moved) |  |  |  |
| 87 | verified offline | `8ea17a7e788e20f4b05c5e428ca69813c07e14a462a511edc92ad58f4c9bb380` | 80 | `8b367b55e683da75decf64c20901485f1574a52cfc9283c29b264d572a857b87` | yes |
| 88 | verified offline | `1f3885780674b5f2437f58408a41452eeeb7623c622b1cf7b652456a020ec8f0` | 81 | `e4233c5c290ab86be2ca11218708caf5263489ca3495ba83bdffadfdc904c1f5` | yes |
| 89 | verified offline | `ed7d11f26bf2f4612bba0227f5f491831a9d3aea4608561b318fbfa7a2e62662` | 82 | `603ffe3944d5a40a46cb86b9c0a48ab25521ba7f6bb3f6e60af0ece9721114cb` | yes |
| 90 | verified offline | `94acaef1691eb3dfea486f08a9f3f6f51e3efc0da868af4942006616729183a1` | 83 | `5d3d50966433342eed0365690f04edc61db2ce2e420c3898a22f004fa536a136` | yes |
| 91 | verified offline | none (nothing moved) |  |  |  |
| 92 | verified offline | none (nothing moved) |  |  |  |
| 93 | verified offline | `575f8c9244312621c38f030d124ce85faa8647a102559c0c9ab8e03c0ebbb1ec` | 86 | `73d135f297fa751ecb8048f135f2e9b700cdbaa51849c241721fab833cdb8f74` | yes |
| 94 | verified offline | `1c42fcf1597a25fc7f3b653e2b1b391f32351b421b647051d144e503559cf810` | 87 | `53dc7aa99975263c03e6720e7ba76b2fdd936ef6c62f14d62078d558f60487c7` | yes |
| 95 | verified offline | `55f3e9726f18dd7e77bc4ef42fa06962f3275261f1be5931b39f1b2df3bbaa56` | 88 | `57398954404d4091ca04bfe26f35ccec294c0404c69ec5cb2fcd68c036b633f6` | yes |
| 96 | verified offline | `b6e2d8c21f3ab56f2651da0ad7672072f4377dd1cb0415fe98a2e1ef46f26cb1` | 89 | `cb59eb2f402e7242a9238c3b0951d7bcf6ae0d8d9e283a7b2d416fca6cc094ff` | yes |
| 97 | verified offline | `d0328e23f358b01462d54a07b5b9401c8b342af8a7b10afaf66773976f1b7d18` | 90 | `00f2eea3041009f7e73bed8db75c905ba9e8d076054f837b4400a81828f72b1f` | yes |
| 98 | verified offline | none (nothing moved) |  |  |  |
| 99 | verified offline | `7817ac022a487540c10b2b65540687ca9cc7d685b1e161c06a0f6dafa9712461` | 91 | `5be0266570d6572f1a15705c166355cf8a5f89fafc9e4a0c985c6e97caf68c05` | yes |
| 100 | verified offline | `e03473aaba6a9677968e3e72d4f20adf0d1c0ba8d3241014500e2ce26d88368f` | 92 | `7edf659908f914f40d9e4f413b05de024fe87dc26b7ce0e64c06781d6d92e0e3` | yes |
| 101 | verified offline | `1bc58536a7f7bd367caf2fef1d91ed6da7d1b0cc871f330df21fc7c8efe93d7a` | 93 | `cd0d8a255a29ae70710a74408a9fa05c99741cc2b5216c25e43a713c626cd6e5` | yes |
| 102 | verified offline | `7730aad7e6c3f3e25d40a79aa5c7ffaeaec10ec44da485c932d2f61a890a7600` | 94 | `d1f5b90f2ed71ebf655575bfe8c420488e1d33ee0269827a37948bfda270173b` | yes |
| 103 | verified offline | `e2c64fbf6cc1d2acac8d315fed7849d6a6e607d0fc713c017392a8e453dd3d38` | 95 | `725d0c96f2f1227d144d8fa7de9c31b0353fc4e7fb84d4c83bbb7b33323e2987` | yes |
| 104 | verified offline | none (nothing moved) |  |  |  |
| 105 | verified offline | `d7ab50a55ac1dbfe4bb94531b66c9fc4f2390ff8b26dbab7d5a3902e47b8c6f6` | 96 | `d8c5e50fdebfb447e56801fb7c09d8fbadf88e8433032bc6d5caf4dd55093681` | yes |
| 106 | verified offline | `c010fd0c54f7de39fb567c826583c13a8f4792d9699d492b69c683f950d58aa5` | 97 | `69060b0d897d59c55ba089a3ceb4e6c7d180fc6407864c6f6cf71a2a24f2833b` | yes |
| 107 | verified offline | `41f7fcdc0a27d11758d068725b757688146f26d5841037b96dfb9ea8a63b61d8` | 98 | `c37408daefdbdbb8971ec621c3f425ae1bae8cbd5197c197440d1756b2ae897b` | yes |
| 108 | verified offline | `9a4ec1d2f34175344b0067127e5757c9b37cc582f1f1ef96da288f0a9d640a25` | 99 | `97be6794f4577d0226a2c0792e6977532441bddfedcca6b9ed073ed93a88c245` | yes |
| 109 | verified offline | `1c9b1f28216929c3cdfa5562de39cc6716b994fd0beeb91fe1ec6749e2ea0f86` | 100 | `d0cb3df0989a02a79bb84d929662d024db4939f32c435cedec4971f69d114c01` | yes |
