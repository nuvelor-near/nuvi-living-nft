import SandboxExecutorModule from "@hot-labs/near-connect/build/SandboxedWallet/executor.js";
import {
  emitWalletDiagnostic,
  isMeteorWebUrl,
  isMobileBrowser,
  routePrimedMeteorWebWindow,
  setMeteorExecutorPatchState,
} from "./diagnostics.js";

const SandboxExecutor =
  SandboxExecutorModule?.default || SandboxExecutorModule;

const MOBILE_GATE_START = "function usingBrowserThatRequiresUserAction(){";
const MOBILE_GATE_END = "}function immediateKnownExecutionTarget";

let installed = false;

export function patchMeteorMobileWebGate(code) {
  if (typeof code !== "string" || !code) {
    throw new Error("Meteor executor code is unavailable");
  }

  const start = code.indexOf(MOBILE_GATE_START);
  const end = code.indexOf(MOBILE_GATE_END, start + MOBILE_GATE_START.length);

  if (
    start < 0 ||
    end < 0 ||
    code.indexOf(MOBILE_GATE_START, start + MOBILE_GATE_START.length) >= 0
  ) {
    throw new Error("Meteor executor mobile gate signature changed");
  }

  return (
    code.slice(0, start) +
    "function usingBrowserThatRequiresUserAction(){return!1}function immediateKnownExecutionTarget" +
    code.slice(end + MOBILE_GATE_END.length)
  );
}

export function installMeteorMobileWebExecutorPatch() {
  if (installed || !isMobileBrowser()) return;

  const prototype = SandboxExecutor?.prototype;
  const originalLoadCode = prototype?.loadCode;

  if (typeof originalLoadCode !== "function") {
    setMeteorExecutorPatchState("error");
    throw new Error("Near Connect sandbox executor is incompatible");
  }

  prototype.loadCode = async function nuvelorMeteorMobileLoadCode(...args) {
    const code = await originalLoadCode.apply(this, args);

    if (this?.manifest?.id !== "meteor-wallet") return code;

    try {
      const patched = patchMeteorMobileWebGate(code);
      setMeteorExecutorPatchState("active");
      emitWalletDiagnostic("meteor-executor-mobile-web-gate-bypass", {
        walletVersion: this?.manifest?.version || "unknown",
      });
      return patched;
    } catch (cause) {
      setMeteorExecutorPatchState("error");
      emitWalletDiagnostic("meteor-executor-mobile-web-gate-error", {
        message: cause instanceof Error ? cause.message : String(cause),
      });
      throw new Error(
        "Meteor mobile signing compatibility check failed. Reload the demo before retrying.",
      );
    }
  };

  installed = true;
  setMeteorExecutorPatchState("installed");
}

const METEOR_PARENT_OPEN_BRIDGE_MARK = Symbol.for(
  "nuvelor.hackathon.meteor-parent-open-bridge",
);

export function bindMeteorMobileNearConnectOpenBridge(wallet) {
  if (
    !isMobileBrowser() ||
    wallet?.manifest?.id !== "meteor-wallet"
  ) {
    return false;
  }

  const executor = wallet?.executor;
  if (!executor || typeof executor._onMessage !== "function") {
    emitWalletDiagnostic("meteor-parent-open-bridge-error", {
      message: "near-connect executor unavailable",
    });
    throw new Error(
      "Meteor mobile signing compatibility check failed. Reload the demo before retrying.",
    );
  }

  if (executor[METEOR_PARENT_OPEN_BRIDGE_MARK]) return true;

  const originalOnMessage = executor._onMessage.bind(executor);

  executor._onMessage = async (iframe, event) => {
    const requestedUrl =
      event?.data?.method === "open" ? event?.data?.params?.url : "";

    if (!isMeteorWebUrl(requestedUrl)) {
      return originalOnMessage(iframe, event);
    }

    emitWalletDiagnostic("meteor-parent-open-request", {
      host: "meteorwallet.app",
    });

    const previousOpen = window.open;
    const forcedOpen = (url, target, features) => {
      if (String(url) === String(requestedUrl)) {
        const routed = routePrimedMeteorWebWindow(
          url,
          "near-connect-executor",
        );
        if (routed) {
          emitWalletDiagnostic("meteor-parent-open-bridge-routed", {
            host: "meteorwallet.app",
          });
          return routed;
        }
      }
      return previousOpen.call(window, url, target, features);
    };

    window.open = forcedOpen;
    try {
      return await originalOnMessage(iframe, event);
    } finally {
      if (window.open === forcedOpen) {
        window.open = previousOpen;
      }
    }
  };

  executor[METEOR_PARENT_OPEN_BRIDGE_MARK] = true;
  emitWalletDiagnostic("meteor-parent-open-bridge-ready", {
    walletVersion: wallet?.manifest?.version || "unknown",
  });
  return true;
}
