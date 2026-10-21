import { useEffect, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { demoTour, runTour, tourRunning } from "../lib/tour";

const KEY = "kalyptra-tour-dismissed";

function dismissed() {
  try {
    return window.localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}

/// A small card offering the guided tour of the sample-data build. "Show me" starts it; it
/// also starts on its own when the address carries ?tour=1 (the landing page and the docs link
/// there).
export function TourNudge() {
  const navigate = useNavigate();
  const { search } = useLocation();
  const [hidden, setHidden] = useState(dismissed);
  const [running, setRunning] = useState(false);

  const dismiss = () => {
    setHidden(true);
    try {
      window.localStorage.setItem(KEY, "1");
    } catch {
      /* not remembered */
    }
  };

  const start = () => {
    setRunning(true);
    runTour(demoTour, (to) => navigate(to), () => {
      setRunning(false);
      // seen it: from now on only the small "Take the tour" pill
      dismiss();
    });
  };

  useEffect(() => {
    if (new URLSearchParams(search).get("tour") === "1" && !tourRunning()) {
      navigate({ search: "" }, { replace: true });
      start();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search]);

  if (running) return null;
  if (hidden) {
    return (
      <button
        type="button"
        onClick={start}
        className="fixed right-5 bottom-5 z-30 cursor-pointer rounded-full border border-line bg-card px-4 py-2 text-[13px] shadow-lg hover:border-ink"
      >
        Take the tour
      </button>
    );
  }

  return (
    <div className="fixed right-4 bottom-4 left-4 z-30 flex items-center gap-3 rounded-2xl border border-line bg-card px-4 py-3.5 shadow-[0_12px_32px_rgba(17,17,17,0.12)] sm:left-auto sm:max-w-sm">
      <span className="grid h-8 w-8 shrink-0 place-items-center rounded-xl bg-wash text-[15px]" aria-hidden="true">
        ◎
      </span>
      <span className="flex-1 text-sm leading-snug">
        <span className="font-medium">New here?</span> <span className="text-muted">Run a whole round in two minutes, on sample data.</span>
      </span>
      <button type="button" onClick={start} className="shrink-0 cursor-pointer rounded-full bg-ink px-3.5 py-2 text-[12.5px] font-medium text-paper hover:opacity-90">
        Show me
      </button>
      <button type="button" onClick={dismiss} aria-label="Dismiss" className="shrink-0 cursor-pointer px-1 text-muted hover:text-ink">
        ×
      </button>
    </div>
  );
}
