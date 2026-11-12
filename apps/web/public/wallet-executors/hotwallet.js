(async () => {
  const REMOTE_EXECUTOR_URLS = [
    "https://raw.githubusercontent.com/hot-dao/near-selector/refs/heads/main/repository/hotwallet.js",
    "https://cdn.jsdelivr.net/gh/hot-dao/near-selector@main/repository/hotwallet.js",
  ];
  const OPEN_TARGET_KEY = "hot-open-target";

  const rememberTargetPatch =
    `window.openTelegram=()=>window.selector.storage.set("${OPEN_TARGET_KEY}","telegram").catch(()=>{}).then(()=>window.selector.open(\`https://t.me/hot_wallet/app?startapp=\${o}\`)),` +
    `window.openExtension=()=>window.selector.open("https://download.hot-labs.org?hotconnector"),` +
    `window.openMobile=()=>window.selector.storage.set("${OPEN_TARGET_KEY}","mobile").catch(()=>{}).then(()=>window.selector.open(\`hotwallet://\${o}\`)),` +
    `isMobile()&&window.selector.storage.get("${OPEN_TARGET_KEY}").then(t=>{t==="telegram"?window.openTelegram():t==="mobile"&&window.openMobile()}).catch(()=>{});`;

  const originalOpenHandlers =
    'window.openTelegram=()=>window.selector.open(`https://t.me/hot_wallet/app?startapp=${o}`),window.openExtension=()=>window.selector.open("https://download.hot-labs.org?hotconnector"),window.openMobile=()=>window.selector.open(`hotwallet://${o}`);';

  const requestFailedBridgePrelude = `
function __nuvelorHotSensitiveKey(key) {
  const normalized = String(key || "").trim().toLowerCase().replaceAll("-", "_");
  const compact = normalized.replaceAll("_", "");
  const sensitive = [
    "authorization",
    "cookie",
    "initdata",
    "init_data",
    "privatekey",
    "private_key",
    "seed",
    "secret",
    "signature",
    "signedmessage",
    "signed_message",
    "proof",
    "bot_token",
    "token",
  ];
  return sensitive.some((part) =>
    normalized.includes(part) || compact.includes(part.replaceAll("_", ""))
  );
}

function __nuvelorHotSanitize(value, depth = 0, seen = new WeakSet()) {
  const MAX_DEPTH = 5;
  const MAX_STRING = 1200;
  const MAX_ITEMS = 30;

  if (depth > MAX_DEPTH) return "[max-depth]";
  if (value == null || typeof value === "boolean" || typeof value === "number") return value;
  if (typeof value === "bigint") return value.toString();
  if (typeof value === "string") {
    return value.length <= MAX_STRING ? value : value.slice(0, MAX_STRING) + "…";
  }
  if (typeof value === "function" || typeof value === "symbol") return "[unsupported]";
  if (typeof value !== "object") return String(value);
  if (seen.has(value)) return "[circular]";
  seen.add(value);

  if (Array.isArray(value)) {
    const result = value
      .slice(0, MAX_ITEMS)
      .map((item) => __nuvelorHotSanitize(item, depth + 1, seen));
    if (value.length > MAX_ITEMS) result.push("[truncated]");
    return result;
  }

  const result = {};
  const keys = Array.from(new Set([
    ...Object.keys(value),
    ...Object.getOwnPropertyNames(value),
  ])).slice(0, MAX_ITEMS);

  for (const key of keys) {
    if (__nuvelorHotSensitiveKey(key)) {
      result[key] = "[redacted]";
      continue;
    }
    try {
      result[key] = __nuvelorHotSanitize(value[key], depth + 1, seen);
    } catch {
      result[key] = "[unreadable]";
    }
  }

  return result;
}

function __nuvelorRequestFailed(payload) {
  return {
    name: "RequestFailed",
    message: "",
    stack: "RequestFailed",
    payload: __nuvelorHotSanitize(payload),
    bridge: "nuvelor-hot-requestfailed-v1",
  };
}
`;

  const requestFailedThrowPattern = /throw new RequestFailed\(([^)]+)\)/g;

  function patchHotWalletExecutor(source) {
    let patched = source;

    if (!patched.includes(originalOpenHandlers)) {
      console.warn("[wallet] HOT executor open-target patch target not found");
    } else {
      patched = patched.replace(originalOpenHandlers, rememberTargetPatch);
    }

    let requestFailedPatchCount = 0;
    patched = patched.replace(requestFailedThrowPattern, (_match, payloadExpression) => {
      requestFailedPatchCount += 1;
      return `throw __nuvelorRequestFailed(${payloadExpression})`;
    });

    if (requestFailedPatchCount === 0) {
      console.warn("[wallet] HOT RequestFailed bridge patch target not found");
      return patched;
    }

    return `${requestFailedBridgePrelude}\n${patched}`;
  }

  async function loadExecutorSource() {
    let lastError = null;

    for (const url of REMOTE_EXECUTOR_URLS) {
      try {
        const response = await fetch(url, { cache: "force-cache" });
        if (!response.ok) {
          throw new Error(`HOT wallet executor load failed: ${response.status}`);
        }

        const source = await response.text();
        return patchHotWalletExecutor(source);
      } catch (error) {
        lastError = error;
      }
    }

    throw lastError || new Error("HOT wallet executor unavailable");
  }

  try {
    const source = await loadExecutorSource();
    const script = document.createElement("script");
    script.type = "module";
    script.textContent = `${source}\n//# sourceURL=hotwallet-patched-executor.js`;
    document.documentElement.appendChild(script);
  } catch (error) {
    console.error("[wallet] HOT executor override failed:", error);
    throw error;
  }
})();