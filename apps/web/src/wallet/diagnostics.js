const MAX_EVENTS = 48;

const state = {
  events: [],
  listeners: new Set(),
  installed: false,
  primedWindow: null,
  capturedMeteorUrl: "",
  fallbackAvailable: false,
  executorPatchState: "idle",
  originalOpen:
    typeof window !== "undefined" && typeof window.open === "function"
      ? window.open.bind(window)
      : null,
};

function safeValue(value) {
  if (value == null) return "";
  if (typeof value === "string") return value.slice(0, 180);
  if (typeof value === "number" || typeof value === "boolean") return value;
  return String(value).slice(0, 180);
}

function safeDetail(detail = {}) {
  return Object.fromEntries(
    Object.entries(detail)
      .slice(0, 8)
      .map(([key, value]) => [key, safeValue(value)]),
  );
}

function safeUrl(raw) {
  try {
    const url = new URL(String(raw), window.location.origin);
    return `${url.origin}${url.pathname}`;
  } catch {
    return "unparseable-url";
  }
}

export function isMeteorWebUrl(raw) {
  try {
    const url = new URL(String(raw), window.location.origin);
    return (
      url.protocol === "https:" &&
      (url.hostname === "meteorwallet.app" ||
        url.hostname.endsWith(".meteorwallet.app"))
    );
  } catch {
    return false;
  }
}

export function routePrimedMeteorWebWindow(raw, mode = "preopened-user-window") {
  if (!isMeteorWebUrl(raw)) return null;

  state.capturedMeteorUrl = String(raw);
  emitWalletDiagnostic("meteor-web-request", {
    source: mode,
    url: safeUrl(raw),
  });

  if (!state.primedWindow || state.primedWindow.closed) {
    state.fallbackAvailable = true;
    notify();
    return null;
  }

  try {
    const panel = state.primedWindow;
    state.primedWindow = null;
    panel.location.href = String(raw);
    panel.focus?.();
    state.fallbackAvailable = false;
    emitWalletDiagnostic("meteor-web-routed", {
      mode,
      url: safeUrl(raw),
    });
    notify();
    return panel;
  } catch (cause) {
    emitWalletDiagnostic("meteor-web-route-error", {
      message: cause instanceof Error ? cause.message : String(cause),
    });
    state.fallbackAvailable = true;
    notify();
    return null;
  }
}

export function isMobileBrowser() {
  if (typeof navigator === "undefined") return false;
  return /Android|iPhone|iPad|iPod/i.test(navigator.userAgent);
}

export function setMeteorExecutorPatchState(value) {
  state.executorPatchState = safeValue(value) || "idle";
  notify();
}

function notify() {
  const snapshot = getWalletDiagnosticSnapshot();
  state.listeners.forEach((listener) => listener(snapshot));
}

export function emitWalletDiagnostic(event, detail = {}) {
  const entry = {
    at: new Date().toISOString(),
    event: String(event),
    detail: safeDetail(detail),
  };
  state.events = [...state.events, entry].slice(-MAX_EVENTS);
  notify();
  return entry;
}

export function getWalletDiagnosticSnapshot() {
  return {
    events: [...state.events],
    meteor: {
      primed: Boolean(state.primedWindow && !state.primedWindow.closed),
      fallbackAvailable: state.fallbackAvailable && Boolean(state.capturedMeteorUrl),
      capturedUrl: state.capturedMeteorUrl
        ? safeUrl(state.capturedMeteorUrl)
        : "",
      executorPatchState: state.executorPatchState,
    },
  };
}

export function subscribeWalletDiagnostics(listener) {
  state.listeners.add(listener);
  listener(getWalletDiagnosticSnapshot());
  return () => state.listeners.delete(listener);
}

export const nearConnectLogger = {
  log: (...logs) => {
    const message = typeof logs[0] === "string" ? logs[0] : "connector-event";
    const interesting =
      /Calling method|Add to queue|Iframe ready|Code loaded|postMessage|New version|cache/i.test(
        message,
      );
    if (interesting) {
      emitWalletDiagnostic("near-connect", { message });
    }
  },
};

export function installWalletOpenInterceptor() {
  if (
    state.installed ||
    typeof window === "undefined" ||
    typeof state.originalOpen !== "function"
  ) {
    return;
  }

  state.installed = true;

  window.open = (url, target, features) => {
    if (!isMeteorWebUrl(url)) {
      return state.originalOpen(url, target, features);
    }

    const routed = routePrimedMeteorWebWindow(url);
    if (routed) return routed;

    const panel = state.originalOpen(url, target, features);
    state.fallbackAvailable = !panel;
    emitWalletDiagnostic("meteor-web-open", {
      opened: Boolean(panel),
      mode: "async-window-open",
      url: safeUrl(url),
    });
    notify();
    return panel;
  };

  emitWalletDiagnostic("wallet-open-interceptor-ready", {
    mobile: isMobileBrowser(),
  });
}

export function primeMeteorWebWindow(walletId, label) {
  if (
    walletId !== "meteor-wallet" ||
    !isMobileBrowser() ||
    typeof state.originalOpen !== "function"
  ) {
    return false;
  }

  releasePrimedMeteorWebWindow();

  const panel = state.originalOpen("about:blank", "_blank");
  if (!panel) {
    state.fallbackAvailable = Boolean(state.capturedMeteorUrl);
    emitWalletDiagnostic("meteor-web-prime-blocked", { label });
    notify();
    return false;
  }

  state.primedWindow = panel;
  state.fallbackAvailable = false;

  try {
    panel.document.title = "NUVELOR · Testnet signature";
    panel.document.body.style.cssText =
      "margin:0;min-height:100vh;display:grid;place-items:center;background:#0e1623;color:#e8f0fb;font-family:system-ui,sans-serif;";
    panel.document.body.innerHTML =
      '<div style="padding:28px;text-align:center;max-width:320px"><div style="font-size:12px;letter-spacing:.14em;color:#5dd6f5;text-transform:uppercase">NUVELOR · NEAR Testnet</div><h2 style="margin:14px 0 8px">Preparing signature…</h2><p style="margin:0;color:#b6c4d8;line-height:1.5">Meteor Web will open here for transaction approval.</p></div>';
  } catch {
    // Navigation can still continue even when the placeholder cannot be styled.
  }

  emitWalletDiagnostic("meteor-web-primed", { label });
  notify();
  return true;
}

export function releasePrimedMeteorWebWindow() {
  const panel = state.primedWindow;
  state.primedWindow = null;
  if (panel && !panel.closed) {
    try {
      panel.close();
    } catch {
      // Best-effort cleanup only.
    }
  }
  notify();
}

export function openCapturedMeteorWeb() {
  if (!state.capturedMeteorUrl || typeof state.originalOpen !== "function") {
    return false;
  }

  const panel = state.originalOpen(state.capturedMeteorUrl, "_blank");
  const opened = Boolean(panel);
  state.fallbackAvailable = !opened;
  emitWalletDiagnostic("meteor-web-manual-open", {
    opened,
    url: safeUrl(state.capturedMeteorUrl),
  });
  notify();
  return opened;
}
