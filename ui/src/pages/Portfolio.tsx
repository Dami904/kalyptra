import { Link } from "react-router-dom";
import { Button, Card, Metric, Notice, PageHead, Spinner } from "../components/ui";
import { api, usePoll } from "../lib/api";

export default function Portfolio() {
  const { data, error, loading, reload } = usePoll(() => api.balances(), 0);
  return (
    <>
      <PageHead eyebrow="YOUR WALLET" title="Portfolio">
        <Button variant="outline" size="sm" onClick={() => void reload()} busy={loading}>
          Refresh
        </Button>
      </PageHead>
      {error && (
        <div className="mb-6">
          <Notice tone="down" title="Can't read your wallet">
            {error}
          </Notice>
        </div>
      )}
      {loading && !data && (
        <Card className="grid h-40 place-items-center">
          <div className="grid justify-items-center gap-2 text-sm text-muted">
            <Spinner className="h-6 w-6" />
            Scanning your wallet. This can take a few seconds.
          </div>
        </Card>
      )}
      {data && (
        <div data-tour="balances" className="rise grid grid-cols-2 gap-3 md:grid-cols-4">
          {Object.entries(data).map(([sym, n]) => (
            <Metric key={sym} label={sym} value={n} hint={sym === "vUSD" ? "play money" : "shares"} />
          ))}
        </div>
      )}
      <p className="mt-6 max-w-[620px] text-sm text-muted">
        These balances are read from your own wallet, on this machine. Nothing here is public: on the chain, your holdings are shielded. Your orders and their status are under{" "}
        <Link to="/trade" className="text-ink underline">
          Trade
        </Link>
        .
      </p>
    </>
  );
}
