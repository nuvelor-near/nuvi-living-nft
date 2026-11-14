import React, {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { NearConnector } from "@hot-labs/near-connect";
import { appConfig } from "../config";
import {
  emitWalletDiagnostic,
  installWalletOpenInterceptor,
  nearConnectLogger,
} from "./diagnostics";
import { installMeteorMobileWebExecutorPatch } from "./meteorExecutorPatch";

let globalConnectorPromise = null;

const NEAR_CONNECT_MANIFEST_URLS = [
  "https://raw.githubusercontent.com/hot-dao/near-selector/refs/heads/main/repository/manifest.json",
  "https://cdn.jsdelivr.net/gh/azbang/hot-connector/repository/manifest.json",
];

const LOCAL_HOT_WALLET_EXECUTOR_PATH = "wallet-executors/hotwallet.js";
const ALLOWED_WALLET_IDS = new Set(["hot-wallet", "meteor-wallet"]);

function localHotWalletExecutorUrl() {
  const baseUrl = import.meta.env.BASE_URL || "/";
  const normalizedBase = baseUrl.endsWith("/") ? baseUrl : `${baseUrl}/`;
  const relativePath = `${normalizedBase}${LOCAL_HOT_WALLET_EXECUTOR_PATH}`;
  return new URL(relativePath, window.location.origin).toString();
}

async function loadNearConnectorManifest() {
  const executor = localHotWalletExecutorUrl();

  for (const manifestUrl of NEAR_CONNECT_MANIFEST_URLS) {
    const response = await fetch(manifestUrl).catch(() => null);
    if (!response?.ok) continue;

    const manifest = await response.json().catch(() => null);
    if (!manifest || !Array.isArray(manifest.wallets)) continue;

    return {
      ...manifest,
      wallets: manifest.wallets
        .filter((wallet) => ALLOWED_WALLET_IDS.has(wallet?.id))
        .map((wallet) =>
          wallet?.id === "hot-wallet"
            ? {
                ...wallet,
                executor,
              }
            : wallet,
        ),
    };
  }

  return null;
}

async function createConnector() {
  installWalletOpenInterceptor();
  installMeteorMobileWebExecutorPatch();
  const manifest = await loadNearConnectorManifest();
  emitWalletDiagnostic("connector-create", {
    network: appConfig.networkId,
    manifest: manifest ? "remote-allowlist" : "default",
  });
  return new NearConnector({
    network: appConfig.networkId,
    logger: nearConnectLogger,
    footerBranding: null,
    ...(manifest ? { manifest } : {}),
  });
}

function getConnector() {
  if (!globalConnectorPromise) {
    globalConnectorPromise = createConnector().catch((error) => {
      globalConnectorPromise = null;
      throw error;
    });
  }
  return globalConnectorPromise;
}

async function readWalletConnection(connector) {
  if (typeof connector?.getConnectedWallet === "function") {
    const connected = await connector.getConnectedWallet().catch(() => null);
    const accountId = connected?.accounts?.[0]?.accountId;
    if (accountId) {
      return {
        accountId,
        walletId: connected?.wallet?.manifest?.id || null,
      };
    }
  }

  const wallet = await connector?.wallet?.().catch(() => null);
  const accounts = (await wallet?.getAccounts?.().catch(() => [])) || [];
  return {
    accountId: accounts[0]?.accountId || null,
    walletId: wallet?.manifest?.id || null,
  };
}

const WalletContext = createContext({
  connector: null,
  accountId: null,
  walletId: null,
  isLoading: true,
  error: null,
  connect: async () => false,
  disconnect: async () => {},
});

export function WalletConnectProvider({ children }) {
  const [connector, setConnector] = useState(null);
  const [accountId, setAccountId] = useState(null);
  const [walletId, setWalletId] = useState(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(null);
  const unsubscribeRef = useRef(null);

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      setIsLoading(true);
      setError(null);

      try {
        const nextConnector = await getConnector();
        if (cancelled) return;

        setConnector(nextConnector);
        const restored = await readWalletConnection(nextConnector);
        setAccountId(restored.accountId);
        setWalletId(restored.walletId);
        if (restored.accountId) {
          emitWalletDiagnostic("wallet-restored", {
            walletId: restored.walletId || "unknown",
          });
        }

        const handleSignIn = async (event) => {
          const restoredAfterSignIn = await readWalletConnection(nextConnector);
          const eventAccount = event?.accounts?.[0]?.accountId;
          setAccountId(eventAccount || restoredAfterSignIn.accountId);
          setWalletId(restoredAfterSignIn.walletId);
          emitWalletDiagnostic("wallet-sign-in-event", {
            walletId: restoredAfterSignIn.walletId || "unknown",
          });
        };
        const handleSignOut = () => {
          setAccountId(null);
          setWalletId(null);
          emitWalletDiagnostic("wallet-sign-out-event");
        };

        if (typeof nextConnector.on === "function") {
          nextConnector.on("wallet:signIn", handleSignIn);
          nextConnector.on("wallet:signOut", handleSignOut);
          unsubscribeRef.current = () => {
            nextConnector.off?.("wallet:signIn", handleSignIn);
            nextConnector.off?.("wallet:signOut", handleSignOut);
          };
        }
      } catch (cause) {
        if (!cancelled) {
          setError(cause instanceof Error ? cause.message : String(cause));
          setConnector(null);
          setAccountId(null);
          setWalletId(null);
        }
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    }

    bootstrap();

    return () => {
      cancelled = true;
      unsubscribeRef.current?.();
      unsubscribeRef.current = null;
    };
  }, []);

  async function connect() {
    if (!connector) return false;
    setError(null);

    try {
      emitWalletDiagnostic("wallet-connect-start");
      const connectedWallet = await connector.connect();
      const restored = await readWalletConnection(connector);
      setAccountId(restored.accountId);
      setWalletId(connectedWallet?.manifest?.id || restored.walletId);
      emitWalletDiagnostic("wallet-connect-success", {
        walletId: connectedWallet?.manifest?.id || restored.walletId || "unknown",
      });
      return true;
    } catch (cause) {
      emitWalletDiagnostic("wallet-connect-error", {
        message: cause instanceof Error ? cause.message : String(cause),
      });
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(message);
      throw cause;
    }
  }

  async function disconnect() {
    if (!connector) return;

    try {
      if (typeof connector.disconnect === "function") {
        await connector.disconnect();
      } else {
        const wallet = await connector.wallet();
        await wallet?.signOut?.();
      }
    } finally {
      setAccountId(null);
      setWalletId(null);
      emitWalletDiagnostic("wallet-disconnected");
    }
  }

  return (
    <WalletContext.Provider
      value={{
        connector,
        accountId,
        walletId,
        isLoading,
        error,
        connect,
        disconnect,
      }}
    >
      {children}
    </WalletContext.Provider>
  );
}

export function useWalletConnect() {
  return useContext(WalletContext);
}
