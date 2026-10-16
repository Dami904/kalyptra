import { useState } from "react";
import { Link } from "react-router-dom";
import { VStepper } from "../components/Stepper";
import { Button, Card, Chip, Field, Input, Notice, PageHead, cx, short } from "../components/ui";
import { api, errText, usePoll } from "../lib/api";
import type { Listed } from "../lib/types";

function Switch({ on, onChange, label }: { on: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={() => onChange(!on)}
      className={cx("relative h-6 w-11 shrink-0 cursor-pointer rounded-full transition-colors", on ? "bg-ink" : "bg-line")}
    >
      <span className={cx("absolute top-0.5 h-5 w-5 rounded-full bg-white transition-all", on ? "left-[22px]" : "left-0.5")} />
    </button>
  );
}

export default function List() {
  const { data: status, reload } = usePoll(() => api.status(), 0);
  const [name, setName] = useState("");
  const [symbol, setSymbol] = useState("");
  const [supply, setSupply] = useState("1000");
  const [cap, setCap] = useState(true);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [done, setDone] = useState<Listed | null>(null);

  const sym = symbol.trim().toUpperCase();
  const n = Number(supply);
  const symOk = /^[A-Z0-9]{1,12}$/.test(sym);
  const taken = !!status?.assets[sym];
  const valid = symOk && !taken && Number.isInteger(n) && n > 0;
  const current = done ? 3 : busy ? 1 : 0;

  async function issue() {
    setBusy(true);
    setErr(null);
    try {
      const r = await api.listCompany({ symbol: sym, amount: n, finalize: cap });
      setDone(r);
      void reload();
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <PageHead eyebrow="NEW MARKET" title="List a company" />
      <div className="grid gap-6 lg:grid-cols-[320px_minmax(0,1fr)]">
        <Card className="self-start p-6">
          <VStepper
            current={current}
            steps={[
              { title: "Describe it", text: "A name, a symbol and a supply." },
              { title: "Issue the token", text: "Created from your own wallet." },
              { title: "Cap the supply", text: "Optional, and permanent." },
              { title: "Trade it", text: "A market against vUSD appears." },
            ]}
          />
        </Card>

        <Card className="p-6">
          {done ? (
            <div className="rise grid gap-5">
              <Notice tone="up" title={`${done.symbol} is listed`}>
                {done.finalized ? "Its supply is capped for good: no more can ever be issued." : "Its supply is not capped, so more can be issued later."}
              </Notice>
              <div className="rounded-xl bg-wash px-4 py-3">
                <p className="eyebrow m-0">Asset id</p>
                <p className="mono m-0 mt-1 text-[13px] break-all">{done.asset_id}</p>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Chip tone="up">
                  Signed by {done.signed_by.length} of {status?.nodes.length ?? done.signed_by.length} nodes
                </Chip>
                <span className="text-sm text-muted">{done.needed} signatures are needed for a listing to count.</span>
              </div>
              <p className="m-0 text-sm text-muted">
                The asset id, not the name, is what makes this token this one. Anyone can issue another token called {done.symbol}; it would get a different id.
              </p>
              <div className="flex gap-3">
                <Link to="/rounds" className="no-underline">
                  <Button>Open a round</Button>
                </Link>
                <Button
                  variant="outline"
                  onClick={() => {
                    setDone(null);
                    setName("");
                    setSymbol("");
                  }}
                >
                  List another
                </Button>
              </div>
            </div>
          ) : (
            <div className="grid gap-5">
              <div className="grid gap-4 sm:grid-cols-[1fr_160px]">
                <Field label="Company name" hint="For you. The chain only knows the symbol.">
                  <Input placeholder="Nova Robotics" value={name} onChange={(e) => setName(e.target.value)} />
                </Field>
                <Field label="Symbol">
                  <Input placeholder="NOVA" value={symbol} onChange={(e) => setSymbol(e.target.value)} className="mono uppercase" maxLength={12} />
                </Field>
              </div>
              {symbol && !symOk && <Notice tone="down">A symbol is 1 to 12 letters or digits.</Notice>}
              {taken && <Notice tone="down">{sym} is already listed.</Notice>}
              <Field label="Total supply (shares)">
                <Input type="number" min={1} value={supply} onChange={(e) => setSupply(e.target.value)} className="num" />
              </Field>

              <div className="flex items-start justify-between gap-4 rounded-xl border border-line px-4 py-3.5">
                <div>
                  <p className="m-0 text-[15px] font-medium">Cap the supply forever</p>
                  <p className="m-0 mt-0.5 text-sm text-muted">Investors can rely on it: after this, the network itself refuses any new shares. On by default.</p>
                </div>
                <Switch on={cap} onChange={setCap} label="Cap the supply forever" />
              </div>

              <Button onClick={issue} busy={busy} disabled={!valid}>
                {busy ? "Issuing. This takes a moment" : `Issue ${sym || "token"}`}
              </Button>
              {err && <Notice tone="down">{err}</Notice>}

              <Notice tone="warn" title="What listing means here">
                The token is issued from your wallet, and the matcher nodes each sign that this symbol means this asset id. At least two signatures make the listing count, and a node will never list the same
                symbol as a different asset. Nothing checks that the company is real, and the nodes cannot see inside a shielded issuance. Anyone can list anything on a local test chain, so always compare
                asset ids, not names.
              </Notice>
            </div>
          )}
        </Card>
      </div>

      {status && (
        <Card className="mt-6 overflow-hidden">
          <p className="eyebrow m-0 border-b border-line px-5 py-3">Listed assets</p>
          <ul className="m-0 list-none p-0">
            {Object.entries(status.assets).map(([s, id]) => (
              <li key={s} className="flex flex-wrap items-center justify-between gap-2 border-b border-line px-5 py-3 last:border-0">
                <span className="flex items-center gap-2 text-[15px] font-medium">
                  {s}
                  {s === "vUSD" && <Chip>play-money quote, no peg</Chip>}
                </span>
                <span className="mono text-xs text-muted">{short(id, 10)}</span>
              </li>
            ))}
          </ul>
        </Card>
      )}
    </>
  );
}
