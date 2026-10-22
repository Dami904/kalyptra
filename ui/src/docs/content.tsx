import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { B, C, Cards, Code, H3, Note, P, REPO, Steps, Table, Ul } from "./blocks";

/* ============================================================
   Docs content: every page is data, rendered by pages/Docs.tsx.
   Sections carry an `id` for the "On this page" list and #links.
   Written from the code: commands, routes and settings are real.
   ============================================================ */

export type DocSection = { id: string; title: string; body: ReactNode };
export type DocPage = { slug: string; group: string; title: string; description: string; sections: DocSection[] };

export const DOC_PAGES: DocPage[] = [
  /* ---------------- Overview ---------------- */
  {
    slug: "",
    group: "Overview",
    title: "Introduction",
    description: "Sealed-bid auctions for private company shares on Zcash Shielded Assets. The price is public; the participants are not.",
    sections: [
      {
        id: "the-problem",
        title: "The problem",
        body: (
          <>
            <P>
              Private company shares trade rarely and in the dark. If the orders are visible, everyone trades against what they can see. If one operator hides them, you have to
              trust that operator not to look, not to favour anyone, and not to quietly pick the price.
            </P>
            <P>
              Kalyptra removes that operator. Orders are encrypted to a key no single machine holds, cleared at <B>one price for everyone</B>, and that price can be recomputed by
              anyone from published data. A cleared round settles as <B>one atomic swap</B>: every filled order moves, or none does.
            </P>
          </>
        ),
      },
      {
        id: "choose-your-path",
        title: "Choose your path",
        body: (
          <Cards
            items={[
              { to: "/docs/quickstart", title: "Try it", text: "Run a whole round in your browser in two minutes, on sample data." },
              { to: "/docs/how-a-round-works", title: "Understand it", text: "Sealed orders, how the nodes agree, and how one price is found." },
              { to: "/docs/privacy", title: "Check the privacy", text: "Exactly what stays private, what is public by design, and who you trust." },
              { to: "/docs/run-the-real-system", title: "Run it for real", text: "Three nodes and a Zcash regtest chain on your own machine." },
            ]}
          />
        ),
      },
      {
        id: "the-rule",
        title: "The one rule",
        body: (
          <Note title="Never broken">
            No order is readable by anyone before two of the three nodes have signed the final set of orders, and no price is accepted that the public checker cannot reproduce
            from published data.
          </Note>
        ),
      },
      {
        id: "key-terms",
        title: "Key terms",
        body: (
          <Table
            head={["Term", "Meaning"]}
            rows={[
              ["Round", "One sealed-bid auction for one market (for example ACME/vUSD), from open to settled."],
              ["Node", "One of three matcher servers. Each holds one share of the decryption key."],
              ["Envelope", "A sealed order. Nobody can read it alone."],
              ["Receipt", "A node's signature saying it accepted an envelope."],
              ["Committed set", "The final list of orders in a round, signed by two nodes."],
              ["Record", "The round's result (price, volume, fills), signed by two nodes."],
              ["Checker", "A program anyone can run that recomputes the result from published data."],
              ["Leader", "The node that assembles the round's settlement transaction. It rotates each round."],
            ]}
          />
        ),
      },
      {
        id: "the-parts",
        title: "The parts",
        body: (
          <Table
            head={["Part", "What it does"]}
            rows={[
              [<C>kalyptra-tse</C>, "Threshold sealed envelopes: encrypt to a key split across the nodes; any 2 of 3 open, each with a proof."],
              [<C>kalyptra-matcher</C>, "The round state machine, the auction, the signed record, the public checker, settlement terms."],
              [<C>kalyptra-node</C>, "The kalyptra program: a node over HTTP, the command line, offline verification, the local gateway."],
              ["ui", "This site. It holds no keys; it talks to the gateway on your machine, or runs on sample data."],
              ["settlement", "Wallet commands for QEDIT's zcash_tx_tool, so each investor builds only their own part of the swap."],
            ]}
          />
        ),
      },
    ],
  },
  {
    slug: "quickstart",
    group: "Overview",
    title: "Quick start",
    description: "Run a whole round in your browser in about two minutes. Nothing to install.",
    sections: [
      {
        id: "the-tour",
        title: "Take the tour",
        body: (
          <>
            <P>
              This site runs on <B>sample data</B>: the round happens in your browser tab, with the same rules as the real system, including its refusals. Open the app and press{" "}
              <B>Show me</B> on the tour card; it highlights each control and moves on when you use it.
            </P>
            <P>
              <Link to="/rounds?tour=1">Start the tour →</Link>
            </P>
          </>
        ),
      },
      {
        id: "by-hand",
        title: "Or do it by hand",
        body: (
          <Steps
            items={[
              <>
                <Link to="/trade">Trade</Link>: sell 10 ACME at a limit of 4. Two other investors have already sealed their orders.
              </>,
              <>
                <Link to="/rounds/2">Rounds</Link>: close round 2 and watch each node receipt, vote, release its share and sign.
              </>,
              <>Read the result: clearing price 4, volume 6, and Public check: Passed.</>,
              <>Trade: settle your round 2 order. One transaction moves every filled order.</>,
              <>
                <Link to="/portfolio">Portfolio</Link>: ACME went down by your fill and vUSD went up by the same trade.
              </>,
            ]}
          />
        ),
      },
      {
        id: "break-it",
        title: "Try to break it",
        body: (
          <Ul>
            <li>Sell 500 ACME. It is refused before it is counted: you hold 90.</li>
            <li>After your first sell, place a second ACME sell in the same round. It is refused: both parts would spend the same note.</li>
          </Ul>
        ),
      },
    ],
  },

  /* ---------------- Concepts ---------------- */
  {
    slug: "how-a-round-works",
    group: "Concepts",
    title: "How a round works",
    description: "From a sealed order to one public price, and why no node can cheat along the way.",
    sections: [
      {
        id: "lifecycle",
        title: "The lifecycle",
        body: (
          <>
            <Code>{`Open ──close──> Closed ──2 matching votes──> Committed ──open together──> Cleared ──one swap──> Settled
                  └── too few orders ──> Voided (nothing is ever decrypted)`}</Code>
            <Steps
              items={[
                <><B>Open.</B> Every node opens the round with a market and a minimum number of orders.</>,
                <><B>Submit.</B> An investor seals an order and sends it to every node; each node that accepts it signs a receipt.</>,
                <><B>Close.</B> Each node votes on the set of orders that 2 nodes receipted. Two matching votes make the set final.</>,
                <><B>Open together.</B> Only now does each node release its share of the decryption, with a proof it used its real share.</>,
                <><B>Clear.</B> Every node computes the same price and signs the record. Two signatures make it final.</>,
                <><B>Check.</B> Anyone recomputes the price and verifies every proof and signature.</>,
                <><B>Settle.</B> Each investor builds their own part of the swap; the leader checks every part and assembles one transaction.</>,
              ]}
            />
          </>
        ),
      },
      {
        id: "sealed-orders",
        title: "Sealed orders",
        body: (
          <>
            <P>
              An order is <C>{"{side, size, limit, round, market, salt}"}</C>, signed with a <B>fresh one-time key</B> made for that order alone, so it names no one. It is sealed to
              the nodes' group key with an ephemeral key exchange on ristretto255, HKDF and ChaCha20-Poly1305. The round and market are inside the signed order and in the envelope's
              authenticated data, so an order cannot be replayed into another round or market.
            </P>
            <P>
              The group key comes from a <B>distributed key generation</B> the three nodes run among themselves on first start. No node ever holds the whole key; any 2 of 3 shares
              together open an envelope. The keys nodes sign with are separate from these shares.
            </P>
          </>
        ),
      },
      {
        id: "agreeing",
        title: "Agreeing on the orders",
        body: (
          <>
            <P>
              At close, each node signs a <B>commit vote</B>: the sorted set of envelopes that 2 nodes receipted, the round's minimum, and whether it was missed (a void). Two
              identical votes make the set final. Because 2 of 3 is more than half, two different sets can never both be final.
            </P>
            <H3>When nodes saw different receipts</H3>
            <P>
              Receipts travel between nodes, so at the moment of closing nodes can disagree. Each vote carries an <B>epoch</B>. A node votes again only when the earlier epoch provably
              cannot finish: the largest group of matching votes plus every vote not yet seen is still below 2. A dead epoch stays dead, and the first epoch in which 2 nodes agree
              decides. Epochs stop at 16.
            </P>
          </>
        ),
      },
      {
        id: "clearing",
        title: "The clearing price",
        body: (
          <>
            <P>
              Once the set is final, each node publishes its partial decryption of every envelope with a <B>DLEQ proof</B>. A wrong partial fails its proof and is dropped; any two
              valid ones open the order. Nothing is ever released for a voided round.
            </P>
            <Steps
              items={[
                "The candidate prices are the orders' limit prices.",
                "At a price p, demand is the total size of buys with limit ≥ p, and supply the total size of sells with limit ≤ p. The volume is the smaller.",
                "The clearing price gives the most volume; ties go to the smallest imbalance, then to the lowest price.",
                "Buyers fill highest limit first, sellers lowest limit first; equal limits by position in the committed set. The marginal order may fill partly.",
              ]}
            />
            <P>Every buyer pays the same price, never more than their limit; every seller receives the same price, never less than theirs.</P>
          </>
        ),
      },
    ],
  },
  {
    slug: "public-checker",
    group: "Concepts",
    title: "The public checker",
    description: "Don't trust the nodes: recompute the round yourself, even offline with every node stopped.",
    sections: [
      {
        id: "what-it-does",
        title: "What it does",
        body: (
          <P>
            The checker is a separate, pure function of the published round: the envelopes, receipts, commit votes, partials, the record and its signatures. It re-opens every
            committed order, recomputes the auction and compares. Run it against live nodes with <C>kalyptra check</C>, or against a saved file with <C>kalyptra verify</C>.
          </P>
        ),
      },
      {
        id: "findings",
        title: "What it can catch",
        body: (
          <Table
            head={["Finding", "Meaning"]}
            rows={[
              [<C>Censored</C>, "Receipts from 2 nodes prove an order was accepted, yet it is missing from the final set."],
              [<C>Unreceipted</C>, "An order in the final set without receipts from 2 nodes: injected after the fact."],
              [<C>RecordMismatch</C>, "The published price, volume, fills or orders differ from the recomputed ones."],
              [<C>RecordNotFinal</C>, "The record was not signed by 2 nodes."],
              [<C>WrongVoidDecision</C>, "A round voided with enough orders, or not voided with too few."],
              [<C>ConflictingCommits</C>, "Two different final sets, or a second one in a later epoch."],
              [<C>PrematureRevote</C>, "Nodes voted again while an earlier epoch could still finish, or skipped an epoch."],
              [<C>Equivocation</C>, "A node signed two different votes in one epoch, or two different records."],
              [<C>CannotOpen</C>, "Too few valid partials to open a committed order."],
              [<C>PartialsForVoidedRound</C>, "Someone published decryption shares for a voided round."],
            ]}
          />
        ),
      },
      {
        id: "offline",
        title: "Verify offline",
        body: (
          <>
            <Code>{`kalyptra export --round 1 --out round-1.json
kalyptra verify --bundle round-1.json --group-key <the announced key>`}</Code>
            <P>Change one number in the file, for example the price from 5 to 6, and verification fails with two findings: the record no longer matches, and nodes never signed it.</P>
          </>
        ),
      },
    ],
  },
  {
    slug: "settlement",
    group: "Concepts",
    title: "Settlement",
    description: "Every filled order in a round moves in one Zcash Shielded Assets transaction, or none does.",
    sections: [
      {
        id: "how",
        title: "How it works",
        body: (
          <P>
            The terms come only from the signed record: a seller sends its fill of the base asset and receives fill × price of vUSD; a buyer the reverse. Each investor builds{" "}
            <B>their own part</B> with their own wallet and signs it with the order's one-time key. The round's <B>leader</B> (node ids[round mod 3], or the next node in turn if the leader has not taken it up) checks every part's amounts and
            assembles one transaction; the chain rejects any transaction that does not balance.
          </P>
        ),
      },
      {
        id: "outcomes",
        title: "Outcomes and retry",
        body: (
          <>
            <Table
              head={["Outcome", "Meaning", "What to do"]}
              rows={[
                ["settled", "One transaction landed; every filled order moved.", "Nothing."],
                ["failed", 'The node answered "Block rejected": nothing moved.', "Retry, or fix the cause."],
                ["unknown", "The wallet tool gave no answer or ran past its time limit: it may have gone out.", "Check balances, then retry."],
              ]}
            />
            <P>
              A retry is safe: the leader reuses the parts it holds, and the chain refuses to spend the same notes twice, so at most one attempt can land. A rejection after an
              unknown attempt stays unknown, because the first may already have settled.
            </P>
          </>
        ),
      },
      {
        id: "one-per-asset",
        title: "One order per asset per round",
        body: (
          <P>
            One wallet can settle only one order per asset per round, because two parts from one wallet would spend the same note and the chain would reject the block. The gateway
            refuses the second order when it is placed. A buy and a sell in the same round are fine.
          </P>
        ),
      },
    ],
  },
  {
    slug: "listings",
    group: "Concepts",
    title: "Listing a company",
    description: "Issue a company's token, cap its supply, and have the nodes sign what it means.",
    sections: [
      {
        id: "listing",
        title: "How a listing works",
        body: (
          <>
            <P>
              List a company issues the token from your wallet (optionally capping its supply for good) and asks every node to sign that this symbol means this asset id. Two valid
              signatures make the listing count.
            </P>
            <Ul>
              <li>A node refuses a symbol already listed as another asset, or an asset already listed under another symbol.</li>
              <li>A listing that got fewer than 2 signatures is kept, and asking again retries it without issuing a second token.</li>
              <li>The signature binds the name to the asset id. Nothing checks the company is real, so compare asset ids, not names.</li>
            </Ul>
          </>
        ),
      },
    ],
  },
  {
    slug: "privacy",
    group: "Concepts",
    title: "Privacy and trust",
    description: "Exactly what stays private, what is public by design, and the one assumption you rely on.",
    sections: [
      {
        id: "who-sees-what",
        title: "Who sees what",
        body: (
          <Table
            head={["", "Who can see it"]}
            rows={[
              ["Who placed an order", "Nobody. Every order has its own one-time key; nothing links it to a person, a wallet or another order."],
              ["An order before the round closes", "Nobody, including the nodes. Opening needs 2 of 3, after they sign the same final set."],
              ["Holdings and transfers", "Shielded on chain. Only an auditor you give your viewing key to, and they cannot spend."],
              ["Each order after the close (side, size, limit)", "Everyone, anonymously. This is what lets anyone recheck the price."],
              ["The clearing price and volume", "Everyone."],
            ]}
          />
        ),
      },
      {
        id: "trust",
        title: "The trust assumption",
        body: (
          <Note>
            Privacy before the close holds while fewer than 2 of the 3 nodes collude. Two colluding nodes could open orders early, but still could not forge the price, drop a
            receipted order or wrongly void a round without the checker proving it.
          </Note>
        ),
      },
      {
        id: "keys",
        title: "Keys stay with you",
        body: (
          <P>
            The page holds no keys. Each order's one-time key is saved by your local gateway before the order is sent, and never sent to the page. The gateway only listens on a
            loopback address, because it has no login, and it refuses requests from other sites' pages in your browser, so a web page you visit cannot read your open orders.
          </P>
        ),
      },
    ],
  },

  /* ---------------- Guides ---------------- */
  {
    slug: "using-the-app",
    group: "Guides",
    title: "Using the app",
    description: "What each screen does.",
    sections: [
      {
        id: "screens",
        title: "Screens",
        body: (
          <Table
            head={["Screen", "What you do there"]}
            rows={[
              [<Link to="/rounds">Rounds</Link>, "Open and close rounds, follow each node, read the price and the check, save a round, retry a settlement."],
              [<Link to="/trade">Trade</Link>, "Seal an order into the open round; see your orders and settle the filled ones."],
              [<Link to="/list">List a company</Link>, "Issue a token, cap its supply, and have the nodes sign the listing."],
              [<Link to="/portfolio">Portfolio</Link>, "Your balances, read from your own wallet."],
              [<Link to="/audit">Audit</Link>, "Export a viewing key, or, as an auditor, read the holdings behind one."],
            ]}
          />
        ),
      },
      {
        id: "acting-as",
        title: "Acting as",
        body: (
          <P>
            When the gateway holds several investors' wallets, the header's <B>Acting as</B> switch picks which one the page uses. It is a demo convenience: in real use each investor
            runs their own gateway.
          </P>
        ),
      },
    ],
  },
  {
    slug: "run-the-real-system",
    group: "Guides",
    title: "Run the real system",
    description: "Three nodes, a Zcash regtest chain, wallets and the page, on your own machine.",
    sections: [
      {
        id: "requirements",
        title: "Requirements",
        body: <P>Linux or WSL, with Docker, Rust 1.85 or newer, and Node 20.19 or newer.</P>,
      },
      {
        id: "start",
        title: "Start it",
        body: (
          <Code>{`git clone ${REPO}
cd kalyptra
bash settlement/setup.sh   # once: QEDIT's Zebra image and the patched wallet tool (20-30 min)
bash scripts/run.sh        # chain, wallets, assets, three nodes, gateway (15-20 min to start)
# then open http://127.0.0.1:8787`}</Code>
        ),
      },
      {
        id: "scripts",
        title: "Lighter scripts",
        body: (
          <Table
            head={["Script", "What it runs", "Needs"]}
            rows={[
              [<C>scripts/demo.sh</C>, "Three nodes, a whole round, the check, offline verification, a forged price rejected, a voided round.", "Rust"],
              [<C>scripts/demo-ui.sh</C>, "Nodes and the gateway without a chain: rounds and orders work in the page.", "Rust, Node"],
              [<C>scripts/demo-settle.sh</C>, "A round settled on regtest, balances checked, an auditor reading a viewing key.", "Docker, wallet tool"],
              [<C>scripts/run.sh</C>, "The whole app behind one page.", "Docker, wallet tool"],
            ]}
          />
        ),
      },
      {
        id: "settings",
        title: "Settings",
        body: (
          <Table
            head={["Variable", "Default", "Used for"]}
            rows={[
              [<C>KALYPTRA_RUN_DIR</C>, "~/kalyptra-run", "run.sh's working folder (wallets, logs)."],
              [<C>KALYPTRA_WORK</C>, "~/kalyptra-spike", "Where setup.sh builds the wallet tool."],
              [<C>KALYPTRA_TOOL</C>, "the built tool", "A wallet tool somewhere else."],
              [<C>KALYPTRA_ZEBRA_IMAGE</C>, "qedit/zebra-regtest-swap", "The regtest node's Docker image."],
              [<C>KALYPTRA_DEMO_DIR</C>, "a temporary folder", "The demo scripts' working folder."],
              [<C>KALYPTRA_TOOL_TIMEOUT_SECS</C>, "900", "How long one wallet command may run."],
            ]}
          />
        ),
      },
      {
        id: "sample",
        title: "The page alone",
        body: <Code>{`cd ui && npm ci && VITE_USE_MOCK=true npm run dev`}</Code>,
      },
    ],
  },

  /* ---------------- Reference ---------------- */
  {
    slug: "cli",
    group: "Reference",
    title: "Command line",
    description: "Everything the page does, the kalyptra binary does too.",
    sections: [
      {
        id: "commands",
        title: "Commands",
        body: (
          <>
            <P>
              Most commands take <C>--roster roster.json</C> (the default).
            </P>
            <Table
              head={["Command", "What it does"]}
              rows={[
                [<C>init --id N --url URL</C>, "Create a node's identity keys (secret) and its public roster entry."],
                [<C>roster --threshold 2 node-*.pub.json</C>, "Combine the public entries into a roster."],
                [<C>node --id N</C>, "Run a node; key generation with its peers on first start. With --tool, --wallet, --assets it can lead settlement."],
                [<C>keys</C>, "Wait for key generation and print the group key."],
                [<C>open --round R --market ACME/vUSD</C>, "Open a round on every node (--min-orders, default 2)."],
                [<C>submit --round R --side sell --size 10 --limit 4 --ticket t.json</C>, "Seal an order with a fresh key and submit it. Keep the ticket secret."],
                [<C>close --round R</C>, "Close a round on every node."],
                [<C>wait --round R</C>, "Wait for the final result and print it."],
                [<C>check --round R</C>, "Verify a round from public data, trusting no node."],
                [<C>export --round R --out round.json</C>, "Save every node's view of a round to one file."],
                [<C>verify --bundle round.json --group-key HEX</C>, "Verify a saved round offline."],
                [<C>settle --ticket t.json --tool T --wallet W</C>, "Build, sign and hand in your part of the swap."],
                [<C>settlement --round R</C>, "Wait for a round's settlement and print it."],
                [<C>view-key --tool T --wallet W --out key.txt</C>, "Export a wallet's viewing key."],
                [<C>audit --tool T --wallet W --view-key key.txt</C>, "As an auditor, report the holdings behind a viewing key."],
                [<C>serve --wallet alice=DIR --tool T --ui ui/dist</C>, "Run the local gateway and the page (loopback only)."],
              ]}
            />
          </>
        ),
      },
    ],
  },
  {
    slug: "gateway-api",
    group: "Reference",
    title: "Gateway API",
    description: "The local JSON API behind the page, under /api.",
    sections: [
      {
        id: "basics",
        title: "Basics",
        body: (
          <P>
            The gateway (<C>kalyptra serve</C>, default <C>127.0.0.1:8787</C>) serves the page and a JSON API under <C>/api</C>. Add <C>?as=name</C> to act as one of its wallets.
            It answers only this machine: a request whose <C>Host</C> or <C>Origin</C> names another site gets 403, and it sends no CORS headers.
            Errors come back as <C>{'{"error": "..."}'}</C> with a status code and are logged as one-line JSON.
          </P>
        ),
      },
      {
        id: "routes",
        title: "Routes",
        body: (
          <Table
            head={["Request", "Body", "Answer and errors"]}
            rows={[
              [<C>GET /api/status</C>, "", "Nodes, threshold, group key, assets, wallets."],
              [<C>GET /api/rounds</C>, "", "Every round, merged from all nodes."],
              [<C>POST /api/rounds</C>, <C>{"{market, round_id?, min_orders?}"}</C>, "{round_id}"],
              [<C>GET /api/rounds/R</C>, "", "Round, record, check, settlement, node progress."],
              [<C>POST /api/rounds/R/close</C>, "", "Closes it on every node."],
              [<C>POST /api/rounds/R/orders</C>, <C>{"{market, side, size, limit}"}</C>, "{receipts, needed, counted, ticket}. 422 can't pay; 409 second order on the same asset; 503 a ticket can't be read."],
              [<C>GET /api/rounds/R/bundle</C>, "", "The saved round for offline verification."],
              [<C>GET /api/tickets</C>, "", "Your orders and fills (never their keys)."],
              [<C>POST /api/tickets/ID/settle</C>, "", "Builds and hands in your part."],
              [<C>POST /api/rounds/R/settle/retry</C>, "", "Leader assembles again. 409 if nothing failed."],
              [<C>GET /api/assets</C>, "", "Listed assets."],
              [<C>POST /api/assets</C>, <C>{"{symbol, amount, finalize?}"}</C>, "{asset_id, signed_by, needed}. 409 already listed."],
              [<C>GET /api/balances</C>, "", "Your balances."],
              [<C>GET /api/viewkey</C>, "", "Your viewing key."],
              [<C>POST /api/audit</C>, <C>{"{fvk}"}</C>, "Holdings behind a viewing key."],
            ]}
          />
        ),
      },
    ],
  },
  {
    slug: "node-api",
    group: "Reference",
    title: "Node API",
    description: "What nodes say to each other and to clients, under /v1. Every statement is signed.",
    sections: [
      {
        id: "routes",
        title: "Routes",
        body: (
          <Table
            head={["Request", "Purpose"]}
            rows={[
              [<C>GET /v1/status</C>, "Node id, key generation done, the group key."],
              [<C>POST /v1/dkg/round1</C>, "Key generation between nodes (also /round2)."],
              [<C>POST /v1/gossip</C>, "Envelopes, receipts, commit votes, partials and record votes between nodes."],
              [<C>POST /v1/rounds</C>, "Open a round."],
              [<C>POST /v1/rounds/R/submit</C>, "Submit a sealed envelope; answers with a signed receipt."],
              [<C>POST /v1/rounds/R/close</C>, "Close and vote."],
              [<C>GET /v1/rounds</C>, "This node's rounds."],
              [<C>GET /v1/rounds/R</C>, "Phase and final record."],
              [<C>GET /v1/rounds/R/public</C>, "Everything the checker needs, as this node saw it."],
              [<C>POST /v1/rounds/R/settle</C>, "Hand in a part, signed with the order's one-time key."],
              [<C>GET /v1/rounds/R/settlement</C>, "Settlement status, legs and outcome."],
              [<C>POST /v1/rounds/R/settle/retry</C>, "Leader only: assemble again."],
              [<C>POST /v1/listings</C>, "Ask the node to sign a listing (GET lists its signed listings)."],
            ]}
          />
        ),
      },
    ],
  },
  {
    slug: "evidence",
    group: "Reference",
    title: "Tests and evidence",
    description: "How the claims on this site are checked, and how to check them yourself.",
    sections: [
      {
        id: "numbers",
        title: "The numbers",
        body: (
          <Ul>
            <li>
              <B>153 automated tests</B>, run on every push in CI with lint, docs, a Rust 1.85 build and the page build.
            </li>
            <li>
              <B>50 defences proven needed:</B> each switched off in turn, and a test failed every time.
            </li>
            <li>
              <B>47 known-answer vectors</B> re-derived by a separate Python and libsodium implementation.
            </li>
            <li>
              <B>On a real Zcash regtest chain:</B> one-transaction settlement with exact balances, failed and unknown outcomes and safe retry, a node-signed listing.
            </li>
          </Ul>
        ),
      },
      {
        id: "run-them",
        title: "Run them",
        body: (
          <>
            <Code>{`cargo test --workspace
python3 tools/check_vectors.py
bash scripts/mutate-new.sh
bash scripts/guardrails.sh`}</Code>
            <P>
              Every attack and the test that proves it is stopped: <a href={`${REPO}/blob/main/ATTACKS.md`}>ATTACKS.md</a>.
            </P>
          </>
        ),
      },
      {
        id: "limits",
        title: "Limits",
        body: (
          <>
            <Ul>
              <li><B>Local regtest only.</B> ZSA swaps are not on a public network yet.</li>
              <li><B>One operator in the demo.</B> All three nodes run on one machine.</li>
              <li><B>Not audited.</B> AI review and attack tests, no human audit. Do not use real assets.</li>
              <li><B>Rounds are periodic,</B> not continuous: the trade-off for one fair price.</li>
            </Ul>
            <P>
              The full list: <a href={`${REPO}/blob/main/docs/LIMITATIONS.md`}>LIMITATIONS.md</a> and <a href={`${REPO}/blob/main/docs/THREAT_MODEL.md`}>THREAT_MODEL.md</a>.
            </P>
          </>
        ),
      },
    ],
  },
];

export function docHref(p: DocPage) {
  return p.slug ? `/docs/${p.slug}` : "/docs";
}
export function findDocPage(slug: string | undefined) {
  return DOC_PAGES.find((p) => p.slug === (slug ?? ""));
}
export function docGroups() {
  const groups: { label: string; pages: DocPage[] }[] = [];
  for (const p of DOC_PAGES) {
    const g = groups.find((x) => x.label === p.group);
    if (g) g.pages.push(p);
    else groups.push({ label: p.group, pages: [p] });
  }
  return groups;
}
