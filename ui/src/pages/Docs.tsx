import { useEffect, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import { cx } from "../components/ui";

const REPO = "https://github.com/Dami904/kalyptra";

/// Documentation, written from the code. Each section has an id so it can be linked to.
const toc: [string, string][] = [
  ["overview", "Overview"],
  ["quickstart", "Try it in two minutes"],
  ["lifecycle", "A round, step by step"],
  ["sealing", "Sealed orders"],
  ["commit", "Agreeing on the orders"],
  ["clearing", "Opening and the clearing price"],
  ["checker", "The public checker"],
  ["settlement", "Settlement"],
  ["listings", "Listing a company"],
  ["privacy", "What stays private"],
  ["app", "Using the app"],
  ["run", "Run the real system"],
  ["cli", "Command line"],
  ["gateway-api", "Gateway API"],
  ["node-api", "Node API"],
  ["testing", "Tests and evidence"],
  ["limits", "Limits"],
];

function H2({ id, children, top }: { id: string; children: ReactNode; top?: boolean }) {
  return (
    <h2
      id={id}
      className={cx(
        "m-0 scroll-mt-24 text-[28px] leading-tight font-medium tracking-tight",
        top ? "mt-2" : "mt-16 border-t border-line pt-10",
      )}
    >
      {children}
    </h2>
  );
}
function H3({ children }: { children: ReactNode }) {
  return <h3 className="m-0 mt-8 text-[18px] font-medium">{children}</h3>;
}
function P({ children }: { children: ReactNode }) {
  return <p className="m-0 mt-4 max-w-[720px] text-[15.5px] leading-relaxed text-muted">{children}</p>;
}
function C({ children }: { children: ReactNode }) {
  return <code className="mono rounded bg-wash px-1.5 py-0.5 text-[13px] text-ink">{children}</code>;
}
function Pre({ children }: { children: string }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(children);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* no clipboard access: the text can still be selected by hand */
    }
  };
  return (
    <div className="relative mt-4">
      <pre className="mono m-0 overflow-x-auto rounded-xl border border-line bg-[#111] p-4 pr-16 text-[13px] leading-relaxed text-[#e9e7e1]">{children}</pre>
      <button
        type="button"
        onClick={copy}
        className="mono absolute top-2.5 right-2.5 cursor-pointer rounded-md border border-[#333] bg-[#1b1b1b] px-2 py-1 text-[11px] text-[#bbb] hover:text-white"
      >
        {copied ? "Copied" : "Copy"}
      </button>
    </div>
  );
}
function Ul({ children }: { children: ReactNode }) {
  return <ul className="m-0 mt-4 grid max-w-[720px] gap-2 pl-5 text-[15.5px] leading-relaxed text-muted">{children}</ul>;
}
function Ol({ children }: { children: ReactNode }) {
  return <ol className="m-0 mt-4 grid max-w-[720px] gap-2 pl-5 text-[15.5px] leading-relaxed text-muted">{children}</ol>;
}
function B({ children }: { children: ReactNode }) {
  return <strong className="font-medium text-ink">{children}</strong>;
}
function T({ head, rows }: { head: string[]; rows: ReactNode[][] }) {
  return (
    <div className="mt-5 overflow-x-auto rounded-xl border border-line bg-card">
      <table className="w-full min-w-[560px] border-collapse text-[14px]">
        <thead>
          <tr className="bg-wash">
            {head.map((h) => (
              <th key={h} className="eyebrow px-4 py-2.5 text-left font-normal">
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i}>
              {r.map((c, j) => (
                <td key={j} className={cx("border-t border-line px-4 py-2.5 align-top", j === 0 ? "text-ink" : "text-muted")}>
                  {c}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
function Callout({ children }: { children: ReactNode }) {
  return <div className="mt-5 max-w-[720px] rounded-xl border border-line bg-card px-4 py-3 text-[14.5px] leading-relaxed">{children}</div>;
}

/// Which section is on screen, for the table of contents.
function useActive(ids: string[]) {
  const [active, setActive] = useState(ids[0]);
  useEffect(() => {
    const els = ids.map((id) => document.getElementById(id)).filter((e): e is HTMLElement => !!e);
    const io = new IntersectionObserver(
      (entries) => {
        const top = entries.filter((e) => e.isIntersecting).sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)[0];
        if (top) setActive(top.target.id);
      },
      { rootMargin: "-80px 0px -70% 0px" },
    );
    els.forEach((e) => io.observe(e));
    return () => io.disconnect();
  }, [ids]);
  return active;
}

const ids = toc.map(([id]) => id);

export default function Docs() {
  const active = useActive(ids);
  return (
    <div className="grid gap-10 lg:grid-cols-[220px_minmax(0,1fr)]">
      <nav aria-label="Documentation" className="hidden lg:block">
        <div className="sticky top-24">
          <p className="eyebrow m-0 mb-3">Documentation</p>
          <ul className="m-0 grid list-none gap-0.5 p-0">
            {toc.map(([id, label]) => (
              <li key={id}>
                <a
                  href={`#${id}`}
                  className={cx(
                    "block rounded-md px-2.5 py-1.5 text-[13.5px] no-underline transition-colors",
                    active === id ? "bg-wash font-medium text-ink" : "text-muted hover:text-ink",
                  )}
                >
                  {label}
                </a>
              </li>
            ))}
          </ul>
          <a href={REPO} className="mono mt-6 block text-[12.5px]">
            Source on GitHub ↗
          </a>
        </div>
      </nav>

      <article className="min-w-0 pb-24">
        {/* small screens: the contents as a wrap of links */}
        <div className="mb-8 flex flex-wrap gap-2 lg:hidden">
          {toc.map(([id, label]) => (
            <a key={id} href={`#${id}`} className="mono rounded-md border border-line px-2 py-1 text-[12px] text-muted no-underline">
              {label}
            </a>
          ))}
        </div>

        <p className="eyebrow m-0">DOCUMENTATION</p>
        <H2 id="overview" top>
          Overview
        </H2>
        <P>
          Kalyptra is a sealed-bid, uniform-price auction for private company shares on Zcash Shielded Assets. Investors place orders that nobody can read while a round is open, not even
          the nodes running it. When the round closes, the nodes open the orders together, clear them at <B>one price for everyone</B>, and publish enough for anyone to recompute that
          price. A cleared round then settles as <B>one atomic swap</B>: every filled order moves, or none does.
        </P>
        <Callout>
          <B>The one rule that must never break:</B> no order is readable by anyone before two of the three nodes have signed the final set of orders, and no price is accepted that
          the public checker cannot reproduce from published data.
        </Callout>
        <T
          head={["Part", "What it does"]}
          rows={[
            [<C>kalyptra-tse</C>, "Threshold sealed envelopes: orders are encrypted to a key split across the nodes, opened only by any 2 of 3 together, each with a proof."],
            [<C>kalyptra-matcher</C>, "The round state machine, the auction, the signed round record, the public checker, and each filled order's settlement terms."],
            [<C>kalyptra-node</C>, "The kalyptra program: a node over HTTP, the investor and operator command line, offline verification, and the local gateway behind this page."],
            ["ui", "This page. It holds no keys; it talks to the gateway on your own machine, or runs on sample data."],
            ["settlement", "Kalyptra's wallet commands for QEDIT's zcash_tx_tool, so each investor builds only their own part of the swap."],
          ]}
        />

        <H2 id="quickstart">Try it in two minutes</H2>
        <P>
          This site runs on <B>sample data</B>: the whole round happens in your browser, with the same rules as the real system, including its refusals. Open the app and follow the
          small <B>Guided demo</B> card in the corner; it ticks each step off as you do it.
        </P>
        <Ol>
          <li>
            <Link to="/trade">Trade</Link>: sell 10 ACME at a limit of 4. Two other investors have already sealed theirs.
          </li>
          <li>
            <Link to="/rounds/2">Rounds</Link>: close round 2 and watch the nodes receipt, vote, share and sign.
          </li>
          <li>Read the result: one clearing price (4), volume 6, and Public check: Passed.</li>
          <li>Trade: settle your round 2 order. One transaction moves every filled order.</li>
          <li>
            <Link to="/portfolio">Portfolio</Link>: ACME down by your fill, vUSD up by the same trade.
          </li>
        </Ol>
        <P>Reloading the page starts the sample over. To run the real system with three nodes and a Zcash chain, see Run the real system.</P>

        <H2 id="lifecycle">A round, step by step</H2>
        <Pre>{`Open ──close──> Closed ──2 matching votes──> Committed ──open together──> Cleared ──one swap──> Settled
                   └── too few orders ──> Voided (nothing is ever decrypted)`}</Pre>
        <Ol>
          <li>
            <B>Open.</B> Every node opens the round with a market (for example ACME/vUSD) and a minimum number of orders.
          </li>
          <li>
            <B>Submit.</B> An investor seals an order and sends it to every node. Each node that accepts it signs a receipt.
          </li>
          <li>
            <B>Close.</B> Each node votes on the set of orders that 2 nodes receipted. Two matching votes make the set final.
          </li>
          <li>
            <B>Open together.</B> Only now does each node release its share of the decryption, with a proof that it used its real share.
          </li>
          <li>
            <B>Clear.</B> Every node computes the same price and signs the round record. Two signatures make it final.
          </li>
          <li>
            <B>Check.</B> Anyone recomputes the price and verifies every proof and signature.
          </li>
          <li>
            <B>Settle.</B> Each investor builds their own part of the swap; the round's leader checks every part and assembles one transaction.
          </li>
        </Ol>

        <H2 id="sealing">Sealed orders</H2>
        <P>
          An order is <C>{"{side, size, limit, round, market, salt}"}</C>, signed with a <B>fresh one-time key</B> made for that order alone, so it names no one. It is then sealed to the
          nodes' group key: an ephemeral key exchange on ristretto255, HKDF, and ChaCha20-Poly1305. The round id and the market are both inside the signed order and in the envelope's
          authenticated data, so an order cannot be replayed into another round or market. The envelope also carries a proof that its sender knew the encryption secret, checked for this
          round.
        </P>
        <P>
          The group key comes from a <B>distributed key generation</B> the three nodes run among themselves on first start. No node ever holds the whole key; each holds one share, and
          any 2 of 3 shares together can open an envelope. The signing keys the nodes use for receipts and votes are separate from these shares.
        </P>
        <P>
          A receipt is a node's signature on <C>(round, envelope hash)</C>. An order with receipts from 2 nodes must be in the final set; if it is not, the receipts prove it was
          censored.
        </P>

        <H2 id="commit">Agreeing on the orders</H2>
        <P>
          At close, each node signs a <B>commit vote</B>: the sorted set of envelopes that 2 nodes receipted, the round's minimum, and whether that minimum was missed (a void). The vote
          is final when 2 nodes sign exactly the same thing. Because 2 of 3 is more than half, two different sets can never both be final.
        </P>
        <H3>When nodes saw different receipts</H3>
        <P>
          Receipts travel between nodes, so at the moment of closing the nodes can disagree. Each vote carries an <B>epoch</B> (attempt number). A node votes again, in the next
          epoch, only when the earlier epoch provably cannot finish: the largest group of matching votes plus every vote not yet seen is still below 2. Votes only add up, so a dead
          epoch stays dead, and the first epoch in which 2 nodes agree decides. Epochs stop at 16.
        </P>

        <H2 id="clearing">Opening and the clearing price</H2>
        <P>
          Once the set is final, each node publishes, for every envelope, its partial decryption with a <B>DLEQ proof</B> that it used its real share. A wrong partial fails its proof
          and is dropped. Any two valid partials open the order. Nodes never release partials for a voided round.
        </P>
        <H3>The auction rules</H3>
        <Ol>
          <li>The candidate prices are the orders' limit prices.</li>
          <li>
            At a price p, <B>demand</B> is the total size of buys with limit ≥ p, and <B>supply</B> the total size of sells with limit ≤ p. The volume is the smaller of the two.
          </li>
          <li>The clearing price gives the most volume. Ties go to the smallest imbalance between demand and supply, then to the lowest price.</li>
          <li>
            Buyers fill highest limit first, sellers lowest limit first; equal limits by position in the committed set (sorted by envelope hash). The marginal order may fill partly.
          </li>
        </Ol>
        <P>Every buyer pays the same price, never more than their limit; every seller receives the same price, never less than theirs.</P>

        <H2 id="checker">The public checker</H2>
        <P>
          The checker is a separate, pure function of the published round: the envelopes, receipts, commit votes, partials, the record and the record signatures. It trusts no node. It
          re-opens every committed order, recomputes the auction, and compares. Run it against live nodes (<C>kalyptra check</C>) or a saved file with every node stopped (
          <C>kalyptra verify</C>).
        </P>
        <T
          head={["Finding", "What it means"]}
          rows={[
            [<C>Censored</C>, "Receipts from 2 nodes prove an order was accepted, but it is not in the final set."],
            [<C>Unreceipted</C>, "An order in the final set without receipts from 2 nodes: injected after the fact."],
            [<C>RecordMismatch</C>, "The published price, volume, fills or orders differ from the recomputed ones."],
            [<C>RecordNotFinal</C>, "The record was not signed by 2 nodes."],
            [<C>WrongVoidDecision</C>, "A round voided with enough orders, or not voided with too few."],
            [<C>ConflictingCommits</C>, "Two different final sets, or a second one in a later epoch."],
            [<C>PrematureRevote</C>, "Nodes voted again while an earlier epoch could still have finished, or skipped an epoch."],
            [<C>Equivocation</C>, "A node signed two different votes in the same epoch, or two different records."],
            [<C>BadEnvelope</C>, "A committed envelope whose seal or proof is invalid."],
            [<C>CannotOpen</C>, "Too few valid partials to open a committed order."],
            [<C>PartialsForVoidedRound</C>, "Someone published decryption shares for a voided round."],
          ]}
        />

        <H2 id="settlement">Settlement</H2>
        <P>
          The settlement terms come only from the signed round record: a seller sends its fill of the base asset and receives fill × price of vUSD; a buyer the reverse. Each investor
          builds <B>their own part</B> of the swap with their own wallet and signs it with the order's one-time key, which proves the part is theirs. The round's <B>leader</B> (it rotates:
          node ids[round mod 3]) checks every part's amounts and assembles one transaction. The chain itself rejects any transaction that does not balance.
        </P>
        <T
          head={["Outcome", "Meaning", "What to do"]}
          rows={[
            ["settled", "One transaction landed; every filled order moved.", "Nothing."],
            ["failed", 'The node answered "Block rejected": nothing moved.', "Retry, or fix the cause (for example a part that does not match)."],
            ["unknown", "The wallet tool gave no answer, or ran past its time limit: it may have gone out.", "Check balances, then retry."],
          ]}
        />
        <P>
          A retry is safe: the leader reuses the parts it holds, and the chain refuses to spend the same notes twice, so at most one attempt can land. A rejection after an unknown attempt
          stays unknown, because the first attempt may already have settled. One wallet can settle only one order per asset per round, because two parts from one wallet would spend the
          same note; the gateway refuses the second order up front.
        </P>

        <H2 id="listings">Listing a company</H2>
        <P>
          List a company issues its token from your wallet (optionally capping the supply for good) and asks every node to sign that this symbol means this asset id. A node refuses a
          symbol already listed as another asset, or an asset already listed under another symbol. Two valid signatures make the listing count. The signature binds the name to the
          asset id; nothing checks that the company is real, so always compare asset ids, not names.
        </P>

        <H2 id="privacy">What stays private</H2>
        <T
          head={["", "Who can see it"]}
          rows={[
            ["Who placed an order", "Nobody. Every order uses its own one-time key; nothing links it to a person, a wallet or another order."],
            ["An order before the round closes", "Nobody, including the nodes. Opening needs 2 of 3 nodes, after they sign the same final set."],
            ["Holdings and transfers", "Shielded on chain. Only an auditor you give your viewing key to, and they cannot spend."],
            ["Each order after the close (side, size, limit)", "Everyone, anonymously. This is what lets anyone recheck the price."],
            ["The clearing price and volume", "Everyone."],
          ]}
        />
        <P>
          <B>Trust assumption:</B> privacy before the close holds while fewer than 2 of the 3 nodes collude. Two colluding nodes could open orders early, but still could not forge the
          price, drop a receipted order or wrongly void a round without the checker proving it.
        </P>

        <H2 id="app">Using the app</H2>
        <T
          head={["Screen", "What you do there"]}
          rows={[
            [<Link to="/rounds">Rounds</Link>, "Open and close rounds, follow each node, read the price and the check, save a round for offline verification, retry a settlement."],
            [<Link to="/trade">Trade</Link>, "Seal an order into the open round for a market; see your orders and settle the filled ones."],
            [<Link to="/list">List a company</Link>, "Issue a token, cap its supply, and have the nodes sign the listing."],
            [<Link to="/portfolio">Portfolio</Link>, "Your balances, read from your own wallet."],
            [<Link to="/audit">Audit</Link>, "Export a viewing key, or, as an auditor, read the holdings behind one."],
          ]}
        />
        <P>
          The header's <B>Acting as</B> switch picks which investor's wallet the page uses when the gateway holds several (a demo convenience; in real use each investor runs their own
          gateway). Order tickets, which hold each order's one-time key, stay on the gateway's machine and are never sent to the page.
        </P>

        <H2 id="run">Run the real system</H2>
        <P>Linux or WSL, with Docker, Rust 1.85 or newer, and Node 20.19 or newer.</P>
        <Pre>{`git clone ${REPO}
cd kalyptra
bash settlement/setup.sh   # once: QEDIT's Zebra image and the patched wallet tool (20-30 min)
bash scripts/run.sh        # chain, wallets, assets, three nodes, gateway (15-20 min to start)
# then open http://127.0.0.1:8787`}</Pre>
        <T
          head={["Script", "What it runs", "Needs"]}
          rows={[
            [<C>scripts/demo.sh</C>, "Three nodes, a whole round, the public check, offline verification with every node stopped, a forged price rejected, a voided round.", "Rust only"],
            [<C>scripts/demo-ui.sh</C>, "Nodes and the gateway without a chain: rounds and orders work in the page.", "Rust, Node"],
            [<C>scripts/demo-settle.sh</C>, "A round settled on regtest, balances checked, an auditor reading a viewing key.", "Docker, the wallet tool"],
            [<C>scripts/run.sh</C>, "The whole app behind one page, with an Acting as switch.", "Docker, the wallet tool"],
          ]}
        />
        <T
          head={["Variable", "Default", "Used for"]}
          rows={[
            [<C>KALYPTRA_RUN_DIR</C>, "~/kalyptra-run", "run.sh's working folder (wallets, logs)."],
            [<C>KALYPTRA_WORK</C>, "~/kalyptra-spike", "Where setup.sh builds the wallet tool."],
            [<C>KALYPTRA_TOOL</C>, "the built zcash_tx_tool", "A wallet tool somewhere else."],
            [<C>KALYPTRA_ZEBRA_IMAGE</C>, "qedit/zebra-regtest-swap", "The regtest node's Docker image."],
            [<C>KALYPTRA_DEMO_DIR</C>, "a temporary folder", "The demo scripts' working folder."],
            [<C>KALYPTRA_TOOL_TIMEOUT_SECS</C>, "900", "How long one wallet command may run before it is stopped."],
          ]}
        />
        <P>
          The page alone, on sample data: <C>cd ui && npm ci && VITE_USE_MOCK=true npm run dev</C>.
        </P>

        <H2 id="cli">Command line</H2>
        <P>
          Everything the page does, the <C>kalyptra</C> binary does too. Most commands take <C>--roster roster.json</C> (the default).
        </P>
        <T
          head={["Command", "What it does"]}
          rows={[
            [<C>init --id N --url URL</C>, "Create a node's identity keys (a secret file) and its public roster entry."],
            [<C>roster --threshold 2 node-*.pub.json</C>, "Combine the public entries into a roster."],
            [<C>node --id N</C>, "Run a matcher node; runs key generation with its peers on first start. With --tool, --wallet and --assets it can lead settlement."],
            [<C>keys</C>, "Wait for key generation and print the group key."],
            [<C>open --round R --market ACME/vUSD --min-orders 2</C>, "Open a round on every node."],
            [<C>submit --round R --side sell --size 10 --limit 4 --ticket t.json</C>, "Seal an order with a fresh key and submit it. The ticket holds the key: keep it secret."],
            [<C>close --round R</C>, "Close a round on every node."],
            [<C>wait --round R</C>, "Wait for the final result and print it."],
            [<C>check --round R</C>, "Verify a round from public data, trusting no node."],
            [<C>export --round R --out round.json</C>, "Save every node's view of a round, merged, to one file."],
            [<C>verify --bundle round.json --group-key HEX</C>, "Verify a saved round offline: no network, no node trusted."],
            [<C>settle --ticket t.json --tool T --wallet W</C>, "Build your part of the swap for exactly the record's terms, sign it and hand it to the nodes."],
            [<C>settlement --round R</C>, "Wait for a round's settlement and print it."],
            [<C>view-key --tool T --wallet W --out key.txt</C>, "Export a wallet's viewing key for an auditor."],
            [<C>audit --tool T --wallet W --view-key key.txt</C>, "As an auditor, report the holdings behind a viewing key."],
            [<C>serve --wallet alice=DIR --tool T --ui ui/dist</C>, "Run the local gateway and the page. It only listens on a loopback address, because it has no login."],
          ]}
        />

        <H2 id="gateway-api">Gateway API</H2>
        <P>
          The local gateway (<C>kalyptra serve</C>, default <C>127.0.0.1:8787</C>) serves the page and a JSON API under <C>/api</C>. Add <C>?as=name</C> to act as one of its wallets.
          Errors are <C>{'{"error": "..."}'}</C> with a status code, and are logged as one-line JSON.
        </P>
        <T
          head={["Request", "Body", "Answer and errors"]}
          rows={[
            [<C>GET /api/status</C>, "", "Nodes up and ready, threshold, group key, listed assets, wallets."],
            [<C>GET /api/rounds</C>, "", "Every round, merged from all nodes."],
            [<C>POST /api/rounds</C>, <C>{"{market, round_id?, min_orders?}"}</C>, "{round_id}. min_orders defaults to 2."],
            [<C>GET /api/rounds/R</C>, "", "The round, its record, the public check, settlement and what each node did."],
            [<C>POST /api/rounds/R/close</C>, "", "Closes it on every node."],
            [<C>POST /api/rounds/R/orders</C>, <C>{"{market, side, size, limit}"}</C>, "{receipts, needed, counted, ticket}. 422: the wallet cannot pay. 409: a second order spending the same asset this round. 503: a saved ticket cannot be read."],
            [<C>GET /api/rounds/R/bundle</C>, "", "The saved round, for offline verification."],
            [<C>GET /api/tickets</C>, "", "Your orders and their fills (never their keys)."],
            [<C>POST /api/tickets/ID/settle</C>, "", "Builds and hands in your part."],
            [<C>POST /api/rounds/R/settle/retry</C>, "", "Asks the leader to assemble again. 409 if nothing failed."],
            [<C>GET /api/assets</C>, "", "Listed assets: symbol to asset id."],
            [<C>POST /api/assets</C>, <C>{"{symbol, amount, finalize?}"}</C>, "{asset_id, signed_by, needed}. 409 if already listed."],
            [<C>GET /api/balances</C>, "", "Your balances, by symbol."],
            [<C>GET /api/viewkey</C>, "", "Your viewing key."],
            [<C>POST /api/audit</C>, <C>{"{fvk}"}</C>, "The holdings behind a viewing key, read by the auditor's wallet."],
          ]}
        />

        <H2 id="node-api">Node API</H2>
        <P>What nodes say to each other and to clients, under <C>/v1</C>. Every statement in it is signed and checked by the receiver.</P>
        <T
          head={["Request", "Purpose"]}
          rows={[
            [<C>GET /v1/status</C>, "Node id, whether key generation finished, the group key."],
            [<C>POST /v1/dkg/round1</C>, "Key generation between nodes, round 1 and 2 (also /round2)."],
            [<C>POST /v1/gossip</C>, "Envelopes, receipts, commit votes, partials and record votes passed between nodes."],
            [<C>POST /v1/rounds</C>, "Open a round."],
            [<C>POST /v1/rounds/R/submit</C>, "Submit a sealed envelope; answers with the node's signed receipt."],
            [<C>POST /v1/rounds/R/close</C>, "Close and vote."],
            [<C>GET /v1/rounds</C>, "This node's rounds."],
            [<C>GET /v1/rounds/R</C>, "Phase and final record."],
            [<C>GET /v1/rounds/R/public</C>, "Everything the checker needs, as this node saw it."],
            [<C>POST /v1/rounds/R/settle</C>, "Hand in a settlement part, signed with the order's one-time key."],
            [<C>GET /v1/rounds/R/settlement</C>, "Settlement status, legs and outcome."],
            [<C>POST /v1/rounds/R/settle/retry</C>, "Leader only: assemble again after a failed or unknown attempt."],
            [<C>POST /v1/listings</C>, "Ask the node to sign a listing (also GET for its signed listings)."],
          ]}
        />

        <H2 id="testing">Tests and evidence</H2>
        <Ul>
          <li>
            <B>149 automated tests</B> across the three crates, run on every push in CI, along with lint, docs, a Rust 1.85 build and the page build.
          </li>
          <li>
            <B>44 defences proven needed:</B> each was switched off in turn and a test failed every time (<C>scripts/mutate-new.sh</C> for the newest).
          </li>
          <li>
            <B>47 known-answer vectors</B> re-derived by a separate Python and libsodium implementation that shares no code with the Rust one (<C>tools/check_vectors.py</C>).
          </li>
          <li>
            <B>Repo rules checked as code:</B> no URL that is not this machine, no .env or keystore, no mainnet (<C>scripts/guardrails.sh</C>).
          </li>
          <li>
            <B>On a real Zcash regtest chain:</B> one-transaction settlement with exact balances, failed and unknown outcomes and safe retry, a node-signed listing.
          </li>
        </Ul>
        <Pre>{`cargo test --workspace
python3 tools/check_vectors.py
bash scripts/mutate-new.sh
bash scripts/guardrails.sh`}</Pre>
        <P>
          Every attack and the test that proves it is stopped:{" "}
          <a href={`${REPO}/blob/main/ATTACKS.md`}>ATTACKS.md</a>.
        </P>

        <H2 id="limits">Limits</H2>
        <Ul>
          <li>
            <B>Local regtest only.</B> ZSA swaps are not on any public network yet; settlement runs on QEDIT's prototype node locally.
          </li>
          <li>
            <B>One operator in the demo.</B> All three nodes run on one machine; real use puts each node with a different operator.
          </li>
          <li>
            <B>Not audited.</B> The threshold design is our own composition of standard parts; it has had an AI review and attack tests, not a human audit. Do not use real assets.
          </li>
          <li>
            <B>Rounds are periodic,</B> not continuous: the trade-off for one fair price.
          </li>
        </Ul>
        <P>
          The full list is in <a href={`${REPO}/blob/main/docs/LIMITATIONS.md`}>docs/LIMITATIONS.md</a>, and who is trusted with what in{" "}
          <a href={`${REPO}/blob/main/docs/THREAT_MODEL.md`}>docs/THREAT_MODEL.md</a>.
        </P>
      </article>
    </div>
  );
}
