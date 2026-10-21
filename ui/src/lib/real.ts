import type { Api } from "./types";

class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}

const ACTOR_KEY = "kalyptra.actingAs";

/// Which of the gateway's wallets this page acts for. Remembered per browser.
export function getActor(): string | null {
  try {
    return localStorage.getItem(ACTOR_KEY);
  } catch {
    return null;
  }
}

export function clearActor() {
  try {
    localStorage.removeItem(ACTOR_KEY);
  } catch {
    /* nothing stored */
  }
}

export function setActor(name: string) {
  try {
    localStorage.setItem(ACTOR_KEY, name);
  } catch {
    /* private window: the choice lasts until reload */
  }
}

async function req<T>(method: string, path: string, body?: unknown): Promise<T> {
  let res: Response;
  const actor = getActor();
  const url = actor ? `${path}${path.includes("?") ? "&" : "?"}as=${encodeURIComponent(actor)}` : path;
  try {
    res = await fetch(url, {
      method,
      headers: body === undefined ? undefined : { "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new ApiError(0, "Can't reach the gateway. Start it with `kalyptra serve`.");
  }
  const text = await res.text();
  let data: unknown = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    /* not JSON */
  }
  if (!res.ok) {
    const msg = (data as { error?: string } | null)?.error ?? (text || res.statusText);
    throw new ApiError(res.status, msg);
  }
  return data as T;
}

export const realApi: Api = {
  status: () => req("GET", "/api/status"),
  rounds: () => req("GET", "/api/rounds"),
  round: (id) => req("GET", `/api/rounds/${id}`),
  openRound: (market, minOrders) => req("POST", "/api/rounds", { market, min_orders: minOrders }),
  closeRound: async (id) => {
    await req("POST", `/api/rounds/${id}/close`, {});
  },
  placeOrder: (round, o) => req("POST", `/api/rounds/${round}/orders`, o),
  tickets: () => req("GET", "/api/tickets"),
  settle: (t) => req("POST", `/api/tickets/${t}/settle`, {}),
  retrySettlement: (r) => req("POST", `/api/rounds/${r}/settle/retry`, {}),
  listCompany: (o) => req("POST", "/api/assets", o),
  balances: () => req("GET", "/api/balances"),
  viewKey: () => req("GET", "/api/viewkey"),
  audit: (fvk) => req("POST", "/api/audit", { fvk }),
  bundleUrl: (round) => `/api/rounds/${round}/bundle`,
};
