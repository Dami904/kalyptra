import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { NodeTracker } from "../components/NodeTracker";
import { Stepper } from "../components/Stepper";
import { CountUp, Decrypt } from "../components/motion";
import { Button, Card, Chip, Empty, Field, Input, Metric, Notice, PageHead, Select, Skeleton, Spinner, Table, Td, cx } from "../components/ui";
import { api, errText, usePoll } from "../lib/api";
import type { Phase, RoundDetail, RoundSummary, Settlement } from "../lib/types";

export const pair = (m: string) => m.replace("/", " / ");

const phaseTone = (p: Phase) => (p === "cleared" ? "up" : p === "voided" ? "down" : p === "open" ? "ink" : "warn");

function OpenRound({ onOpened }: { onOpened: (id: number) => void }) {
  const { data: status } = usePoll(() => api.status(), 0);
  const markets = status ? Object.keys(status.assets).filter((s) => s !== "vUSD").map((s) => `${s}/vUSD`) : [];
  const [market, setMarket] = useState("");
  const [min, setMin] = useState("2");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const chosen = market || markets[0] || "";
  async function go() {
    setBusy(true);
    setErr(null);
    try {
      const r = await api.openRound(chosen, Math.max(1, Number(min) || 2));
      onOpened(r.round_id);
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Card className="p-4">
      <p className="eyebrow m-0 mb-3">Open a round</p>
      <div className="grid gap-3">
        <Field label="Market">
          <Select value={chosen} onChange={(e) => setMarket(e.target.value)} disabled={!markets.length}>
            {markets.map((m) => (
              <option key={m} value={m}>
                {pair(m)}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="Minimum orders" hint="Below this the round is voided and nothing is ever opened.">
          <Input type="number" min={1} value={min} onChange={(e) => setMin(e.target.value)} />
        </Field>
        <Button onClick={go} busy={busy} disabled={!chosen}>
          Open round
        </Button>
        {err && <Notice tone="down">{err}</Notice>}
      </div>
    </Card>
  );
}

function RoundList({ rounds, selected }: { rounds: RoundSummary[]; selected?: number }) {
  const nav = useNavigate();
  const list = [...rounds].sort((a, b) => b.round_id - a.round_id);
  return (
    <Card className="overflow-hidden">
      <p className="eyebrow m-0 border-b border-line px-4 py-3">All rounds</p>
      {list.length === 0 ? (
        <Empty title="No rounds yet">Open the first one to start trading.</Empty>
      ) : (
        <ul className="m-0 list-none p-0">
          {list.map((r) => (
            <li key={r.round_id}>
              <button
                onClick={() => nav(`/rounds/${r.round_id}`)}
                className={cx(
                  "flex w-full cursor-pointer items-center justify-between gap-3 border-b border-line px-4 py-3 text-left transition-colors last:border-0",
                  selected === r.round_id ? "bg-wash" : "hover:bg-wash/60",
                )}
              >
                <span>
                  <span className="block text-[15px] font-medium">{pair(r.market)}</span>
                  <span className="mono text-xs text-muted">
                    Round {r.round_id} · {r.orders} sealed
                  </span>
                </span>
                <Chip tone={phaseTone(r.phase)}>{r.phase}</Chip>
              </button>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}

function SettlementBlock({ s }: { s: Settlement }) {
  const got = s.legs.filter((l) => l.received).length;
  return (
    <div>
      <div className="mb-3 flex items-center justify-between">
        <p className="eyebrow m-0">Settlement</p>
        <Chip tone={s.status === "settled" ? "up" : s.status === "failed" ? "down" : "warn"}>{s.status}</Chip>
      </div>
      {s.status === "collecting" && (
        <Notice>
          Waiting for each filled order's owner to hand in its part: <span className="num">{got}</span> of <span className="num">{s.legs.length}</span>. Owners do this from{" "}
          <span className="mono">Trade</span>, with their own wallet.
        </Notice>
      )}
      {s.status === "assembling" && (
        <Notice>
          <span className="inline-flex items-center gap-2">
            <Spinner /> Leader node {s.leader} is checking every part and building one transaction.
          </span>
        </Notice>
      )}
      {s.status === "settled" && (
        <Notice tone="up" title="Settled in one transaction">
          <span className="mono break-all text-[13px]">{s.txid}</span>
          {s.height != null && <span className="block text-[13px]">Block {s.height}. Either every order moved, or none did.</span>}
        </Notice>
      )}
      {s.status === "failed" && (
        <Notice tone="down" title="Settlement did not go through">
          {s.error}
          {s.bad.length > 0 && <span className="block">Parts that did not match: orders {s.bad.join(", ")}. Their owners can hand in a new one.</span>}
          {s.bad.length === 0 && <span className="block">Nothing moved. The leader can try again with the parts it already holds.</span>}
        </Notice>
      )}
      {s.status === "unknown" && (
        <Notice tone="warn" title="Outcome unknown">
          The wallet tool gave no answer, so the transaction may or may not have gone out: {s.error}. Check balances in Portfolio. Trying again is safe: the chain refuses to spend the
          same notes twice, so at most one attempt can land.
        </Notice>
      )}
      {(s.status === "unknown" || (s.status === "failed" && s.bad.length === 0)) && <RetryButton round={s.round_id} />}
    </div>
  );
}

function RetryButton({ round }: { round: number }) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  return (
    <div className="mt-3 grid gap-2">
      <Button
        variant="outline"
        busy={busy}
        onClick={async () => {
          setBusy(true);
          setErr(null);
          try {
            await api.retrySettlement(round);
          } catch (e) {
            setErr(errText(e));
          } finally {
            setBusy(false);
          }
        }}
      >
        Try the settlement again
      </Button>
      {err && <Notice tone="down">{err}</Notice>}
    </div>
  );
}

function Detail({ id }: { id: number }) {
  const [tick, setTick] = useState(0);
  const { data, error, loading } = usePoll<RoundDetail>(() => api.round(id), 2000, [id, tick]);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  if (loading && !data)
    return (
      <Card className="grid gap-5 p-6" aria-busy="true">
        <Skeleton className="h-8 w-48" />
        <Skeleton className="h-5 w-full" />
        <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
          {[0, 1, 2, 3].map((k) => (
            <Skeleton key={k} className="h-20" />
          ))}
        </div>
        <Skeleton className="h-40 w-full" />
      </Card>
    );
  if (!data) return <Notice tone="down">{error}</Notice>;
  const { round: r, record, check, settlement, nodes } = data;

  const voided = r.phase === "voided";
  const steps = voided ? ["Open", "Closed", "Voided"] : ["Open", "Closed", "Committed", "Cleared", "Settled"];
  let current = voided ? 2 : { open: 0, closed: 1, committed: 2, cleared: 3, voided: 2 }[r.phase];
  if (r.phase === "cleared" && (settlement?.status === "settled" || settlement?.status === "nothing" || (record && record.price === null))) current = 5;

  async function close() {
    setBusy(true);
    setErr(null);
    try {
      await api.closeRound(id);
      setTick((t) => t + 1);
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  }

  const filledOf = (i: number) => record?.fills.filter((f) => f.id === i).reduce((s, f) => s + f.quantity, 0) ?? 0;

  return (
    <Card className="rise p-6">
      <div className="mb-6 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="m-0 text-[26px] font-medium tracking-tight">{pair(r.market)}</h2>
          <p className="mono m-0 mt-1 text-xs text-muted">Round {r.round_id} · needs at least {r.min_orders} orders</p>
        </div>
        <Chip tone={phaseTone(r.phase)}>{r.phase}</Chip>
      </div>

      <div className="mb-7 px-2">
        <Stepper steps={steps} current={current} failed={voided} />
      </div>

      {nodes && nodes.orders > 0 && (
        <div className="mb-7">
          <p className="eyebrow m-0 mb-2">What each node has done</p>
          <NodeTracker progress={nodes} />
        </div>
      )}

      {r.phase === "open" && (
        <div className="grid gap-4">
          <Notice title={`${r.orders} sealed ${r.orders === 1 ? "order" : "orders"}`}>
            Nobody can read them, including the nodes. The board holds only ciphertexts and signed receipts until the round closes.
          </Notice>
          <div className="flex flex-wrap items-center gap-3">
            <Button onClick={close} busy={busy} variant="outline">
              Close round
            </Button>
            <span className="text-sm text-muted">Closing makes the nodes agree on which orders count. Then they open them together.</span>
          </div>
          {err && <Notice tone="down">{err}</Notice>}
        </div>
      )}

      {(r.phase === "closed" || r.phase === "committed") && (
        <Notice>
          <span className="inline-flex items-center gap-2">
            <Spinner /> {r.phase === "closed" ? "The nodes are agreeing on the set of orders." : "Two nodes are releasing their shares to open the orders and clear the price."}
          </span>
        </Notice>
      )}

      {voided && (
        <Notice tone="warn" title="Voided: too few orders">
          Fewer than {r.min_orders} orders came in, so nothing in this round was ever decrypted.
        </Notice>
      )}

      {r.phase === "cleared" && record && (
        <div className="grid gap-7">
          <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
            <Metric
              label="Clearing price"
              value={record.price === null ? "none" : <CountUp to={record.price} />}
              hint={record.price === null ? "Nothing crossed" : "One price for everyone"}
            />
            <Metric label="Volume" value={<CountUp to={record.volume} />} />
            <Metric label="Orders" value={<CountUp to={record.orders.length} duration={700} />} />
            <Metric label="Public check" value={check?.clean ? "Passed" : "Failed"} tone={check?.clean ? "up" : "down"} hint="Recomputed from public data" />
          </div>

          {check && !check.clean && (
            <Notice tone="down" title="The public check found a problem with this round">
              <ul className="m-0 mt-1 pl-5">
                {check.findings.map((f) => (
                  <li key={f} className="mono text-[13px]">
                    {f}
                  </li>
                ))}
              </ul>
            </Notice>
          )}

          <div>
            <p className="eyebrow m-0 mb-2">Opened orders</p>
            <Table head={["#", "Side", "Size", "Limit", "Filled"]}>
              {record.orders.map(([i, o]) => (
                <tr key={i}>
                  <Td left className="text-muted">{i}</Td>
                  <Td className={o.side === "buy" ? "text-up" : "text-down"}>
                    <Decrypt text={o.side} delay={i * 120} />
                  </Td>
                  <Td>
                    <Decrypt text={String(o.size)} delay={i * 120 + 60} />
                  </Td>
                  <Td>
                    <Decrypt text={String(o.limit)} delay={i * 120 + 120} />
                  </Td>
                  <Td>{filledOf(i)}</Td>
                </tr>
              ))}
            </Table>
            <p className="m-0 mt-2 text-xs text-muted">No names. Each order was signed with its own one-time key, so an opened order shows what was asked for, not who asked.</p>
          </div>

          {settlement && settlement.status !== "nothing" && <SettlementBlock s={settlement} />}

          <div className="flex flex-wrap items-center gap-3 border-t border-line pt-5">
            <a href={api.bundleUrl(r.round_id)} download={`kalyptra-round-${r.round_id}.json`} className="no-underline">
              <Button variant="outline" size="sm" type="button">
                Save round for offline check
              </Button>
            </a>
            <span className="text-xs text-muted">
              Then run <span className="mono">kalyptra verify</span> on the file: no network, and no node is trusted.
            </span>
          </div>
        </div>
      )}
    </Card>
  );
}

export default function Rounds() {
  const { id } = useParams();
  const nav = useNavigate();
  const { data: rounds, error } = usePoll(() => api.rounds(), 3000);
  const selected = id ? Number(id) : rounds && rounds.length ? Math.max(...rounds.map((r) => r.round_id)) : undefined;
  return (
    <>
      <PageHead eyebrow="THE BOARD" title="Rounds" />
      {error && !rounds && <Notice tone="down" title="Can't load rounds">{error}</Notice>}
      <div className="grid gap-6 lg:grid-cols-[340px_minmax(0,1fr)]">
        <div className="grid content-start gap-6">
          <RoundList rounds={rounds ?? []} selected={selected} />
          <OpenRound onOpened={(n) => nav(`/rounds/${n}`)} />
        </div>
        <div>{selected !== undefined ? <Detail id={selected} /> : <Card><Empty title="Pick a round">Its progress, price and check result show here.</Empty></Card>}</div>
      </div>
    </>
  );
}
