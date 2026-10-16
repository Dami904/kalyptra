export type Phase = "open" | "closed" | "committed" | "voided" | "cleared";
export type Side = "buy" | "sell";

export interface RoundSummary {
  round_id: number;
  market: string;
  min_orders: number;
  phase: Phase;
  orders: number;
}
export interface PublicOrder {
  side: Side;
  size: number;
  limit: number;
  owner: string;
}
export interface RoundRecord {
  round_id: number;
  market: string;
  set: string[];
  orders: [number, PublicOrder][];
  invalid: number[];
  price: number | null;
  volume: number;
  fills: { id: number; side: Side; quantity: number }[];
}
export interface Check {
  clean: boolean;
  voided: boolean;
  findings: string[];
}
export interface Settlement {
  round_id: number;
  leader: number;
  legs: { order: number; side: Side; base: number; quote: number; received: boolean }[];
  status: "collecting" | "assembling" | "settled" | "failed" | "unknown" | "nothing";
  txid?: string | null;
  height?: number | null;
  bad: number[];
  error?: string | null;
}
export interface NodeProgress {
  orders: number;
  nodes: { id: number; receipts: number; committed: boolean; shares: number; signed: boolean }[];
}
export interface RoundDetail {
  round: RoundSummary;
  record: RoundRecord | null;
  check: Check | null;
  settlement: Settlement | null;
  nodes: NodeProgress | null;
}
export interface Ticket {
  id: string;
  round_id: number;
  market: string;
  side: Side;
  size: number;
  limit: number;
  phase: Phase | null;
  filled: number | null;
  price: number | null;
}
export interface NodeStatus {
  id: number;
  url: string;
  up: boolean;
  keys_ready: boolean;
}
export interface Status {
  threshold: number;
  nodes: NodeStatus[];
  group_key: string | null;
  assets: Record<string, string>;
  wallet: boolean;
  /// The investors' wallets this gateway holds, by name, and which one is the default.
  wallets?: string[];
  default?: string;
  auditor: boolean;
}
export interface OrderResult {
  receipts: number;
  needed: number;
  counted: boolean;
  ticket: string | null;
}
export interface SettleResult {
  settled: boolean;
  reason?: string;
  handed_to?: number;
  order?: number;
  base?: number;
  quote?: number;
  sells?: boolean;
}
export interface Listed {
  symbol: string;
  asset_id: string;
  finalized: boolean;
  /// Ids of the nodes that signed the listing, and how many signatures it needs.
  signed_by: number[];
  needed: number;
}

export interface Api {
  status(): Promise<Status>;
  rounds(): Promise<RoundSummary[]>;
  round(id: number): Promise<RoundDetail>;
  openRound(market: string, minOrders: number): Promise<{ round_id: number }>;
  closeRound(id: number): Promise<void>;
  placeOrder(
    round: number,
    o: { market: string; side: Side; size: number; limit: number },
  ): Promise<OrderResult>;
  tickets(): Promise<Ticket[]>;
  settle(ticket: string): Promise<SettleResult>;
  /// Ask the round's leader to assemble again after a failed or unknown attempt.
  retrySettlement(round: number): Promise<unknown>;
  listCompany(o: { symbol: string; amount: number; finalize: boolean }): Promise<Listed>;
  balances(): Promise<Record<string, number>>;
  viewKey(): Promise<{ fvk: string }>;
  audit(fvk: string): Promise<Record<string, number>>;
  bundleUrl(round: number): string;
}
