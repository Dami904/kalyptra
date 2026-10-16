// A pretend backend for working on the page without any nodes or wallets. It follows the
// same rules as the real thing: orders are sealed until close, one uniform clearing price,
// then each filled order settles. Turn it on with VITE_USE_MOCK=true.
import type {
  Api,
  NodeProgress,
  Phase,
  PublicOrder,
  RoundDetail,
  RoundRecord,
  RoundSummary,
  Settlement,
  Side,
  Ticket,
} from "./types";

interface MockRound extends RoundSummary {
  orders_list: { ticket: string; o: PublicOrder }[];
  record: RoundRecord | null;
  settlement: Settlement | null;
  parts: Set<number>;
}

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
const hex = (n: number) =>
  Array.from({ length: n }, () => Math.floor(Math.random() * 16).toString(16)).join("");

const assets: Record<string, string> = {
  ACME: hex(64),
  BOLT: hex(64),
  CRUX: hex(64),
  vUSD: hex(64),
};
const holdings: Record<string, number> = { ACME: 90, BOLT: 0, CRUX: 0, vUSD: 50 };

/// Uniform-price call auction: most volume, then least imbalance, then the lowest price.
function clear(orders: { id: number; side: Side; size: number; limit: number }[]) {
  const prices = [...new Set(orders.map((o) => o.limit))].sort((a, b) => a - b);
  let best: { p: number; vol: number; imb: number } | null = null;
  for (const p of prices) {
    const demand = orders.filter((o) => o.side === "buy" && o.limit >= p).reduce((s, o) => s + o.size, 0);
    const supply = orders.filter((o) => o.side === "sell" && o.limit <= p).reduce((s, o) => s + o.size, 0);
    const vol = Math.min(demand, supply);
    const imb = Math.abs(demand - supply);
    if (vol > 0 && (!best || vol > best.vol || (vol === best.vol && imb < best.imb))) best = { p, vol, imb };
  }
  if (!best) return { price: null, volume: 0, fills: [] as RoundRecord["fills"] };
  const fill = (side: Side) => {
    let left = best!.vol;
    const eligible = orders
      .filter((o) => o.side === side && (side === "buy" ? o.limit >= best!.p : o.limit <= best!.p))
      .sort((a, b) => (side === "buy" ? b.limit - a.limit : a.limit - b.limit) || a.id - b.id);
    return eligible.map((o) => {
      const q = Math.min(o.size, left);
      left -= q;
      return { id: o.id, side, quantity: q };
    });
  };
  return { price: best.p, volume: best.vol, fills: [...fill("buy"), ...fill("sell")].filter((f) => f.quantity > 0) };
}

function build(id: number, market: string, list: { ticket: string; o: PublicOrder }[]): RoundRecord {
  const c = clear(list.map((x, i) => ({ id: i, ...x.o })));
  return {
    round_id: id,
    market,
    set: list.map(() => hex(64)),
    orders: list.map((x, i) => [i, x.o]),
    invalid: [],
    price: c.price,
    volume: c.volume,
    fills: c.fills,
  };
}

const seedOrders: { ticket: string; o: PublicOrder }[] = [
  { ticket: "r1-a1", o: { side: "sell", size: 10, limit: 4, owner: hex(64) } },
  { ticket: "r1-a2", o: { side: "sell", size: 4, limit: 5, owner: hex(64) } },
  { ticket: "r1-a3", o: { side: "buy", size: 8, limit: 6, owner: hex(64) } },
  { ticket: "r1-a4", o: { side: "buy", size: 6, limit: 5, owner: hex(64) } },
];
const seedRecord = build(1, "ACME/vUSD", seedOrders);
const rounds: MockRound[] = [
  {
    round_id: 1,
    market: "ACME/vUSD",
    min_orders: 2,
    phase: "cleared",
    orders: 4,
    orders_list: seedOrders,
    record: seedRecord,
    parts: new Set([0, 1, 2, 3]),
    settlement: {
      round_id: 1,
      leader: 2,
      legs: seedRecord.fills.map((f) => ({
        order: f.id,
        side: f.side,
        base: f.quantity,
        quote: f.quantity * 5,
        received: true,
      })),
      status: "settled",
      txid: hex(64),
      height: 8,
      bad: [],
    },
  },
  {
    round_id: 2,
    market: "BOLT/vUSD",
    min_orders: 2,
    phase: "open",
    orders: 0,
    orders_list: [],
    record: null,
    settlement: null,
    parts: new Set(),
  },
];
const myTickets = new Set<string>(["r1-a3"]);

/// What each node would have done by this phase. Node 3 commits on seeing two matching votes
/// without voting itself, as happens for real.
function progress(r: MockRound): NodeProgress {
  const n = r.orders_list.length;
  const closed = r.phase !== "open";
  const committed = r.phase === "committed" || r.phase === "cleared";
  const cleared = r.phase === "cleared";
  return {
    orders: n,
    nodes: [1, 2, 3].map((id) => ({
      id,
      receipts: n,
      committed: closed && id !== 3,
      shares: committed && r.phase !== "voided" ? n : 0,
      signed: cleared && (id !== 3 || r.settlement?.status === "settled"),
    })),
  };
}

const find = (id: number) => {
  const r = rounds.find((x) => x.round_id === id);
  if (!r) throw new Error(`no round ${id}`);
  return r;
};
const summary = (r: MockRound): RoundSummary => ({
  round_id: r.round_id,
  market: r.market,
  min_orders: r.min_orders,
  phase: r.phase,
  orders: r.orders_list.length,
});

export const mockApi: Api = {
  async status() {
    await wait(150);
    return {
      threshold: 2,
      nodes: [1, 2, 3].map((id) => ({ id, url: `http://127.0.0.1:710${id}`, up: true, keys_ready: true })),
      group_key: "84f17a5fc42e3ba9bb8487e53d0d4bb348b12fa21dac83d8f08b84fb53c97756",
      assets,
      wallet: true,
      wallets: ["alice", "bob", "carol", "dave", "issuer"],
      default: "alice",
      auditor: true,
    };
  },
  async rounds() {
    await wait(120);
    return rounds.map(summary);
  },
  async round(id): Promise<RoundDetail> {
    await wait(120);
    const r = find(id);
    const done = r.phase === "cleared" || r.phase === "voided";
    return {
      round: summary(r),
      record: r.record,
      check: done ? { clean: true, voided: r.phase === "voided", findings: [] } : null,
      settlement: r.settlement,
      nodes: progress(r),
    };
  },
  async openRound(market, minOrders) {
    await wait(300);
    const id = Math.max(0, ...rounds.map((r) => r.round_id)) + 1;
    rounds.push({
      round_id: id,
      market,
      min_orders: minOrders,
      phase: "open",
      orders: 0,
      orders_list: [],
      record: null,
      settlement: null,
      parts: new Set(),
    });
    return { round_id: id };
  },
  async closeRound(id) {
    await wait(300);
    const r = find(id);
    if (r.phase !== "open") return;
    r.phase = "closed";
    const next = (p: Phase, ms: number, then?: () => void) =>
      setTimeout(() => {
        r.phase = p;
        then?.();
      }, ms);
    if (r.orders_list.length < r.min_orders) {
      next("voided", 1200);
      return;
    }
    next("committed", 1200);
    next("cleared", 2600, () => {
      r.record = build(r.round_id, r.market, r.orders_list);
      if (r.record.price !== null) {
        const p = r.record.price;
        r.settlement = {
          round_id: r.round_id,
          leader: (r.round_id % 3) + 1,
          legs: r.record.fills.map((f) => ({ order: f.id, side: f.side, base: f.quantity, quote: f.quantity * p, received: false })),
          status: "collecting",
          bad: [],
        };
      }
    });
  },
  async placeOrder(round, o) {
    await wait(500);
    const r = find(round);
    if (r.phase !== "open") throw new Error(`round ${round} is ${r.phase}, which does not allow this`);
    // Like the gateway: an order the wallet cannot pay for is refused, counting what the
    // investor's other open orders already commit.
    const cost = (market: string, side: Side, size: number, limit: number): [string, number] => {
      const [base, quote] = market.split("/");
      return side === "sell" ? [base!, size] : [quote!, size * limit];
    };
    const [sym, needed] = cost(o.market, o.side, o.size, o.limit);
    // Two orders spending the same asset in one round would spend the same note twice.
    for (const t of r.orders_list) {
      if (myTickets.has(t.ticket) && cost(r.market, t.o.side, t.o.size, t.o.limit)[0] === sym)
        throw new Error(
          `you already have an order in round ${round} that spends ${sym}; a wallet can settle only one such order per round, because both would spend the same note`,
        );
    }
    let reserved = 0;
    for (const x of rounds) {
      if (x.phase === "cleared" || x.phase === "voided") continue;
      for (const t of x.orders_list) {
        if (!myTickets.has(t.ticket)) continue;
        const [s2, c2] = cost(x.market, t.o.side, t.o.size, t.o.limit);
        if (s2 === sym) reserved += c2;
      }
    }
    const held = holdings[sym] ?? 0;
    if (held < reserved + needed)
      throw new Error(`this order can cost up to ${needed} ${sym}; you hold ${held} and have ${reserved} already committed to open orders`);
    const ticket = `r${round}-${hex(8)}`;
    r.orders_list.push({ ticket, o: { side: o.side, size: o.size, limit: o.limit, owner: hex(64) } });
    myTickets.add(ticket);
    return { receipts: 3, needed: 2, counted: true, ticket };
  },
  async tickets() {
    await wait(120);
    const out: Ticket[] = [];
    for (const r of rounds)
      r.orders_list.forEach((x, i) => {
        if (!myTickets.has(x.ticket)) return;
        const filled = r.record ? r.record.fills.filter((f) => f.id === i).reduce((s, f) => s + f.quantity, 0) : null;
        out.push({
          id: x.ticket,
          round_id: r.round_id,
          market: r.market,
          side: x.o.side,
          size: x.o.size,
          limit: x.o.limit,
          phase: r.phase,
          filled,
          price: r.record?.price ?? null,
        });
      });
    return out;
  },
  async settle(ticket) {
    await wait(1800);
    const r = rounds.find((x) => x.orders_list.some((o) => o.ticket === ticket));
    if (!r || !r.settlement) return { settled: false, reason: "not filled" };
    const idx = r.orders_list.findIndex((o) => o.ticket === ticket);
    const leg = r.settlement.legs.find((l) => l.order === idx);
    if (!leg) return { settled: false, reason: "not filled" };
    leg.received = true;
    if (r.settlement.legs.every((l) => l.received)) {
      r.settlement.status = "assembling";
      setTimeout(() => {
        r.settlement!.status = "settled";
        r.settlement!.txid = hex(64);
        r.settlement!.height = 9;
      }, 2500);
    }
    return { settled: true, handed_to: 3, order: idx, base: leg.base, quote: leg.quote, sells: leg.side === "sell" };
  },
  async retrySettlement(round) {
    await wait(300);
    const r = find(round);
    if (!r.settlement || (r.settlement.status !== "failed" && r.settlement.status !== "unknown"))
      throw new Error(`round ${round} settlement has not failed; nothing to retry`);
    r.settlement.status = "assembling";
    setTimeout(() => {
      r.settlement!.status = "settled";
      r.settlement!.txid = hex(64);
      r.settlement!.height = 9;
    }, 2500);
    return {};
  },
  async listCompany(o) {
    await wait(1500);
    if (assets[o.symbol]) throw new Error(`${o.symbol} is already listed`);
    assets[o.symbol] = hex(64);
    holdings[o.symbol] = o.amount;
    return { symbol: o.symbol, asset_id: assets[o.symbol], finalized: o.finalize, signed_by: [1, 2, 3], needed: 2 };
  },
  async balances() {
    await wait(600);
    return { ...holdings };
  },
  async viewKey() {
    await wait(300);
    return { fvk: hex(192) };
  },
  async audit(fvk) {
    await wait(900);
    if (fvk.trim().length < 8) throw new Error("a viewing key is 96 bytes of hex");
    return { ...holdings };
  },
  bundleUrl: () => "#",
};
