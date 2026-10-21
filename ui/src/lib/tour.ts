import { driver, type Driver } from "driver.js";
import "driver.js/dist/driver.css";

/* A spotlight tour: the page dims, one real control is highlighted, and a small card explains
   it. Steps can live on different pages; the tour navigates, waits for the control to appear,
   and can move on by itself when you actually use the control. */

export type TourStep = {
  route: string;
  /** CSS selector of the element to highlight. */
  element: string;
  title: string;
  description: string;
  /** Move on when this element is clicked (default: the highlighted one), after a short pause. */
  advanceOnClick?: { target?: string; delayMs?: number };
  /** How long to wait for the element to appear (it may need the round to clear first). */
  waitMs?: number;
};

function waitFor(selector: string, timeoutMs: number): Promise<Element | null> {
  return new Promise((resolve) => {
    const found = document.querySelector(selector);
    if (found) return resolve(found);
    const start = Date.now();
    const iv = setInterval(() => {
      const el = document.querySelector(selector);
      if (el || Date.now() - start > timeoutMs) {
        clearInterval(iv);
        resolve(el);
      }
    }, 120);
  });
}

let current: { destroy: () => void } | null = null;
export function tourRunning() {
  return current !== null;
}

export function runTour(steps: TourStep[], navigate: (to: string) => void, onEnd?: (finished: boolean) => void) {
  current?.destroy();
  let i = 0;
  let moving = false;
  let cleanup: (() => void) | null = null;
  let d: Driver;

  const clearListener = () => {
    cleanup?.();
    cleanup = null;
  };
  const hide = (hidden: boolean) => {
    const pop = document.querySelector<HTMLElement>(".driver-popover");
    if (pop) pop.style.visibility = hidden ? "hidden" : "";
    const overlay = document.querySelector<HTMLElement>(".driver-overlay");
    if (overlay) overlay.style.opacity = hidden ? "0" : "";
  };

  const end = (finished: boolean) => {
    clearListener();
    d.destroy();
    current = null;
    onEnd?.(finished);
  };

  const show = async () => {
    clearListener();
    const step = steps[i];
    if (!step) return end(true);
    // Never show the previous card over the next page while it loads.
    hide(true);
    if (window.location.pathname !== step.route) navigate(step.route);
    const el = await waitFor(step.element, step.waitMs ?? 4000);
    if (!el) {
      // The control is not there (for example, already done): skip this step.
      i += 1;
      return show();
    }
    // Bring it into view BEFORE the card is placed: a scroll after placing would leave the card
    // on top of the control (seen on phones).
    el.scrollIntoView({ block: "center" });
    // (two frames, or 60 ms if frames are paused, as in a background tab)
    await new Promise<void>((r) => {
      setTimeout(r, 60);
      requestAnimationFrame(() => requestAnimationFrame(() => r()));
    });
    const last = i === steps.length - 1;
    d.highlight({
      element: el,
      popover: {
        title: step.title,
        description: step.description,
        showButtons: i === 0 ? ["next", "close"] : ["next", "previous", "close"],
        nextBtnText: last ? "Done" : "Next",
        prevBtnText: "Back",
        progressText: `${i + 1} of ${steps.length}`,
      },
    });
    requestAnimationFrame(() => requestAnimationFrame(() => hide(false)));
    if (step.advanceOnClick) {
      const target = document.querySelector(step.advanceOnClick.target ?? step.element) ?? el;
      const onClick = () => setTimeout(() => go(1), step.advanceOnClick?.delayMs ?? 900);
      target.addEventListener("click", onClick, { once: true });
      cleanup = () => target.removeEventListener("click", onClick);
    }
  };

  const go = async (dir: 1 | -1) => {
    if (moving) return;
    moving = true;
    i = Math.max(0, i + dir);
    if (i >= steps.length) end(true);
    else await show();
    moving = false;
  };

  d = driver({
    showProgress: true,
    allowClose: true,
    overlayClickBehavior: "close",
    overlayColor: "rgba(17,17,17,0.55)",
    stagePadding: 8,
    stageRadius: 14,
    popoverClass: "kal-tour",
    onNextClick: () => void go(1),
    onPrevClick: () => void go(-1),
    onCloseClick: () => end(false),
  });
  current = { destroy: () => end(false) };
  void show();
}

/// The tour of the sample-data build: one round, from a sealed order to moved balances.
export const demoTour: TourStep[] = [
  {
    route: "/trade",
    element: '[data-tour="order-fields"]',
    title: "Write your order",
    description: "Choose Sell, a size of 10 and a limit of 4. (Try a size of 500 first if you like: it will be refused, because you only hold 90 ACME.) Then press Next.",
  },
  {
    route: "/trade",
    element: '[data-tour="order-submit"]',
    title: "Seal it",
    description: "Press Seal and submit. Your order is encrypted to a key split across three nodes: nobody, not even the nodes, can read it until the round closes.",
    advanceOnClick: { delayMs: 1600 },
  },
  {
    route: "/rounds/2",
    element: '[data-tour="round-close"]',
    title: "Close the round",
    description: "Two other investors have sealed orders too. Close the round: the nodes now agree on exactly which orders count.",
    advanceOnClick: { delayMs: 600 },
  },
  {
    route: "/rounds/2",
    element: '[data-tour="node-tracker"]',
    title: "The nodes open it together",
    description: "Each node receipts, votes, releases its share and signs. Nothing is opened until two nodes have signed the same final set of orders.",
  },
  {
    route: "/rounds/2",
    element: '[data-tour="round-result"]',
    title: "One price, checked by anyone",
    description: "Everyone trades at the same clearing price. Public check: Passed means it was recomputed from published data, not taken from the nodes.",
    waitMs: 10000,
  },
  {
    route: "/trade",
    element: '[data-tour="my-orders"]',
    title: "Settle in one transaction",
    description: "Press Settle on your round 2 order. Every filled order moves in one transaction, or none does. Then press Next.",
  },
  {
    route: "/portfolio",
    element: '[data-tour="balances"]',
    title: "Your balances moved",
    description: "ACME down and vUSD up by exactly your fill. Holdings stay shielded; only an auditor you give your viewing key to can see them.",
    waitMs: 6000,
  },
];
