import { useState } from "react";
import { Button, Card, Field, Metric, Notice, PageHead } from "../components/ui";
import { api, errText } from "../lib/api";

export default function Audit() {
  const [fvk, setFvk] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [found, setFound] = useState<Record<string, number> | null>(null);
  const [mine, setMine] = useState<string | null>(null);
  const [mineBusy, setMineBusy] = useState(false);

  async function look() {
    setBusy(true);
    setErr(null);
    setFound(null);
    try {
      setFound(await api.audit(fvk));
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  }

  async function exportKey() {
    setMineBusy(true);
    try {
      setMine((await api.viewKey()).fvk);
    } catch (e) {
      setErr(errText(e));
    } finally {
      setMineBusy(false);
    }
  }

  return (
    <>
      <PageHead eyebrow="VIEW, DON'T TOUCH" title="Audit" />
      <div className="grid gap-6 lg:grid-cols-2">
        <Card className="self-start p-6">
          <h2 className="m-0 text-[20px] font-medium">See holdings with a viewing key</h2>
          <p className="m-0 mt-1 mb-5 text-sm text-muted">An auditor, given an investor's viewing key, sees that investor's holdings and history. A viewing key can't spend anything.</p>
          <div className="grid gap-4">
            <Field label="Viewing key">
              <textarea
                value={fvk}
                onChange={(e) => setFvk(e.target.value)}
                rows={3}
                placeholder="Paste a viewing key"
                className="mono w-full resize-none rounded-xl border border-line bg-card p-3.5 text-[13px] break-all placeholder:text-faint hover:border-faint focus:border-ink focus:outline-none"
              />
            </Field>
            <Button onClick={look} busy={busy} disabled={fvk.trim().length === 0}>
              Show holdings
            </Button>
            {err && <Notice tone="down">{err}</Notice>}
          </div>
          {found && (
            <div className="rise mt-6">
              <p className="eyebrow m-0 mb-2">Holdings seen through this key</p>
              <div className="grid grid-cols-2 gap-3">
                {Object.entries(found).map(([s, n]) => (
                  <Metric key={s} label={s} value={n} />
                ))}
              </div>
            </div>
          )}
        </Card>

        <Card className="self-start p-6">
          <h2 className="m-0 text-[20px] font-medium">Give an auditor access to yours</h2>
          <p className="m-0 mt-1 mb-5 text-sm text-muted">Export your wallet's viewing key and hand it over. You can't take it back, so share it only with someone you trust to see your history.</p>
          {mine ? (
            <div className="rise grid gap-3">
              <p className="eyebrow m-0">Your viewing key</p>
              <p className="mono m-0 rounded-xl bg-wash p-3.5 text-[12px] leading-relaxed break-all">{mine}</p>
              <Button variant="outline" onClick={() => void navigator.clipboard?.writeText(mine)}>
                Copy
              </Button>
            </div>
          ) : (
            <Button variant="outline" onClick={exportKey} busy={mineBusy}>
              Export my viewing key
            </Button>
          )}
        </Card>
      </div>
    </>
  );
}
