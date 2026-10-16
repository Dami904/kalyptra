import { useCallback, useEffect, useRef, useState } from "react";
import { mockApi } from "./mock";
import { realApi } from "./real";
import type { Api } from "./types";

export const USE_MOCK = import.meta.env.VITE_USE_MOCK === "true";
export const api: Api = USE_MOCK ? mockApi : realApi;

/// Run an async loader now and then every `ms`. Keeps the last good value if a refresh fails.
export function usePoll<T>(load: () => Promise<T>, ms: number, deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const loadRef = useRef(load);
  loadRef.current = load;
  const run = useCallback(async () => {
    try {
      setData(await loadRef.current());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => {
    setLoading(true);
    void run();
    if (ms <= 0) return;
    const t = setInterval(() => void run(), ms);
    return () => clearInterval(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ms, run, ...deps]);
  return { data, error, loading, reload: run };
}

export function errText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
