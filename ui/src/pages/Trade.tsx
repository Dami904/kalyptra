import { useState } from "react";
import { Link } from "react-router-dom";
import { Button, Card, Chip, Empty, Field, Input, Notice, PageHead, Segmented, Select, Table, Td, cx } from "../components/ui";
import { api, errText, usePoll } from "../lib/api";
import type { OrderResult, Side, Ticket } from "../lib/types";
import { pair } from "./Rounds";

function ticketState(t: Ticket): { label: string; tone: "neutral" | "up" | "down" | "warn" | "ink" } {
  if (t.phase === "open") return { label: "Sealed", tone: "ink" };
  if (t.phase === "closed" || t.phase === "committed") return { label: "Opening", tone: "warn" };
  if (t.phase === "voided") return { label: "Voided", tone: "down" };
  if (t.phase === "cleared") {
    if (t.filled && t.filled > 0) return { label: `Filled ${t.filled} at ${t.price}`, tone: "up" };
    return { label: "Not filled", tone: "neutral" };
  }
  return { label: "Unknown", tone: "neutral" };
}

function MyOrders() {
  const { data, error, reload } = usePoll(() => api.tickets(), 3000);
  const [busy, setBusy] = useState<string | null>(null);
  const [msg, setMsg] = useState<{ tone: "up" | "down"; text: string } | null>(null);

  async function settle(t: Ticket) {
    setBusy(t.id);
    setMsg(null);
    try {
      const r = await api.settle(t.id);
      setMsg(
        r.settled
          ? { tone: "up", text: `Your part is built and handed to ${r.handed_to} nodes. The leader settles once every filled order has handed in its part.` }
          : { tone: "down", text: r.reason ?? "Nothing to settle." },
      );
      void reload();
    } catch (e) {
      setMsg({ tone: "down", text: errText(e) });
    } finally {
      setBusy(null);
    }
  }

  return (
    <Card className="mt-6 overflow-hidden">
      <div className="flex items-center justify-between border-b border-line px-5 py-3">
        <p className="eyebrow m-0">My orders</p>
        <span className="text-xs text-muted">Kept on this machine, with each order's one-time key</span>
      </div>
      {error && !data && <div className="p-4"><Notice tone="down">{error}</Notice></div>}
      {data && data.length === 0 && <Empty title="No orders yet">Place your first sealed order above.</Empty>}
      {data && data.length > 0 && (
        <Table head={["Round", "Market", "Side", "Size", "Limit", "Status", ""]}>
          {[...data].reverse().map((t) => {
            const s = ticketState(t);
            const canSettle = t.phase === "cleared" && (t.filled ?? 0) > 0;
            return (
              <tr key={t.id}>
                <Td left>{t.round_id}</Td>
                <Td>{pair(t.market)}</Td>
                <Td className={t.side === "buy" ? "text-up" : "text-down"}>{t.side}</Td>
                <Td>{t.size}</Td>
                <Td>{t.limit}</Td>
                <Td>
                  <Chip tone={s.tone}>{s.label}</Chip>
                </Td>
                <Td>
                  {canSettle && (
                    <Button size="sm" onClick={() => settle(t)} busy={busy === t.id} disabled={busy !== null && busy !== t.id}>
                      Settle
                    </Button>
                  )}
                </Td>
              </tr>
            );
          })}
        </Table>
      )}
      {msg && (
        <div className="p-4">
          <Notice tone={msg.tone}>{msg.text}</Notice>
        </div>
      )}
    </Card>
  );
}

export default function Trade() {
  const { data: status } = usePoll(() => api.status(), 0);
  const { data: rounds } = usePoll(() => api.rounds(), 3000);
  const symbols = status ? Object.keys(status.assets).filter((s) => s !== "vUSD") : [];
  const [sym, setSym] = useState("");
  const [roundSel, setRoundSel] = useState("");
  const [side, setSide] = useState<Side>("buy");
  const [size, setSize] = useState("8");
  const [limit, setLimit] = useState("6");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [res, setRes] = useState<OrderResult | null>(null);

  // Until the user picks one, start on a market that has an open round.
  const withOpen = symbols.find((s) => (rounds ?? []).some((r) => r.market === `${s}/vUSD` && r.phase === "open"));
  const chosen = sym || withOpen || symbols[0] || "";
  const market = `${chosen}/vUSD`;
  const open = (rounds ?? []).filter((r) => r.market === market && r.phase === "open").sort((a, b) => b.round_id - a.round_id);
  const roundId = open.find((r) => String(r.round_id) === roundSel)?.round_id ?? open[0]?.round_id;
  const n = Number(size);
  const p = Number(limit);
  const valid = Number.isInteger(n) && n > 0 && Number.isInteger(p) && p > 0;

  async function submit() {
    if (roundId === undefined) return;
    setBusy(true);
    setErr(null);
    setRes(null);
    try {
      setRes(await api.placeOrder(roundId, { market, side, size: n, limit: p }));
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <PageHead eyebrow="SEALED ORDERS" title="Trade" />
      <div className="grid gap-6 lg:grid-cols-[300px_minmax(0,1fr)]">
        <Card className="overflow-hidden self-start">
          <p className="eyebrow m-0 border-b border-line px-4 py-3">Markets</p>
          <ul className="m-0 list-none p-0">
            {symbols.map((s) => {
              const o = (rounds ?? []).filter((r) => r.market === `${s}/vUSD` && r.phase === "open").length;
              return (
                <li key={s}>
                  <button
                    onClick={() => {
                      setSym(s);
                      setRes(null);
                    }}
                    className={cx(
                      "flex w-full cursor-pointer items-center justify-between border-b border-line px-4 py-3 text-left transition-colors last:border-0",
                      s === chosen ? "bg-wash" : "hover:bg-wash/60",
                    )}
                  >
                    <span>
                      <span className="block text-[15px] font-medium">{s} / vUSD</span>
                      <span className="mono text-xs text-muted">{o ? `${o} open ${o === 1 ? "round" : "rounds"}` : "no open round"}</span>
                    </span>
                    {o > 0 && <span className="h-2 w-2 rounded-full bg-up" aria-label="Open" />}
                  </button>
                </li>
              );
            })}
          </ul>
        </Card>

        <Card className="p-6">
          <div className="mb-5 flex flex-wrap items-center justify-between gap-2">
            <h2 className="m-0 text-[22px] font-medium tracking-tight">{pair(market)}</h2>
            <Chip>Uniform price · one per round</Chip>
          </div>

          {open.length === 0 ? (
            <Notice title="No open round for this market">
              Orders are sealed into a round. <Link to="/rounds" className="text-ink underline">Open one in Rounds</Link>, then come back.
            </Notice>
          ) : (
            <div className="grid gap-5">
              {open.length > 1 && (
                <Field label="Round">
                  <Select value={String(roundId)} onChange={(e) => setRoundSel(e.target.value)}>
                    {open.map((r) => (
                      <option key={r.round_id} value={r.round_id}>
                        Round {r.round_id} · {r.orders} sealed
                      </option>
                    ))}
                  </Select>
                </Field>
              )}
              <Segmented
                value={side}
                onChange={setSide}
                options={[
                  { value: "buy", label: "Buy" },
                  { value: "sell", label: "Sell" },
                ]}
                tone={(v) => (v === "buy" ? "up" : "down")}
              />
              <div className="grid gap-4 sm:grid-cols-2">
                <Field label={`Size (${chosen})`}>
                  <Input type="number" min={1} value={size} onChange={(e) => setSize(e.target.value)} className="num" />
                </Field>
                <Field label="Limit price (vUSD)" hint={side === "buy" ? "The most you'll pay per share." : "The least you'll accept per share."}>
                  <Input type="number" min={1} value={limit} onChange={(e) => setLimit(e.target.value)} className="num" />
                </Field>
              </div>

              <div className="rounded-xl bg-wash px-4 py-3 text-sm text-muted">
                {valid ? (
                  <>
                    You {side === "buy" ? "pay at most" : "receive at least"} <span className="num text-ink">{n * p} vUSD</span> if filled in full. Everyone who trades gets the
                    same clearing price, which is never worse than your limit.
                  </>
                ) : (
                  "Enter a whole number for size and for price."
                )}
              </div>

              <Button onClick={submit} busy={busy} disabled={!valid || roundId === undefined}>
                Seal and submit
              </Button>
              {err && <Notice tone="down">{err}</Notice>}
              {res && (
                <Notice
                  tone={res.counted ? "up" : "warn"}
                  title={
                    res.counted
                      ? `Sealed. Receipted by ${res.receipts} of ${status?.nodes.length ?? res.receipts} nodes`
                      : `Only ${res.receipts} nodes receipted this, and ${res.needed} are needed`
                  }
                >
                  {res.counted
                    ? "A fresh one-time key was made for this order and saved on this machine with your ticket. Nothing about it is readable until the round closes."
                    : res.ticket
                      ? "It is saved on this machine with its key, so it can still count if the other nodes catch up, and its funds stay reserved until the round ends."
                      : "Every node refused it (the round may have closed). Nothing was kept."}
                </Notice>
              )}
            </div>
          )}
        </Card>
      </div>
      <MyOrders />
    </>
  );
}
