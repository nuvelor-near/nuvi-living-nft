import React, {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { NearConnector } from "@hot-labs/near-connect";
import { appConfig } from "../config";

let globalConnectorPromise = null;

const NEAR_CONNECT_MANIFEST_URLS = [
  "https://raw.githubusercontent.com/hot-dao/near-selector/refs/heads/main/repository/manifest.json",
  "https://cdn.jsdelivr.net/gh/azbang/hot-connector/repository/manifest.json",
];

const LOCAL_HOT_WALLET_EXECUTOR_PATH = "wallet-executors/hotwallet.js";

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
      wallets: manifest.wallets.map((wallet) =>
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
  const manifest = await loadNearConnectorManifest();
  return new NearConnector({
    network: appConfig.networkId,
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

async function readAccountId(connector) {
  if (typeof connector?.getConnectedWallet === "function") {
    const connected = await connector.getConnectedWallet().catch(() => null);
    const accountId = connected?.accounts?.[0]?.accountId;
    if (accountId) return accountId;
  }

  const wallet = await connector?.wallet?.().catch(() => null);
  const accounts = (await wallet?.getAccounts?.().catch(() => [])) || [];
  return accounts[0]?.accountId || null;
}

const WalletContext = createContext({
  connector: null,
  accountId: null,
  isLoading: true,
  error: null,
  connect: async () => false,
  disconnect: async () => {},
});

export function WalletConnectProvider({ children }) {
  const [connector, setConnector] = useState(null);
  const [accountId, setAccountId] = useState(null);
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
        setAccountId(await readAccountId(nextConnector));

        const handleSignIn = async (event) => {
          const eventAccount = event?.accounts?.[0]?.accountId;
          setAccountId(eventAccount || (await readAccountId(nextConnector)));
        };
        const handleSignOut = () => setAccountId(null);

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
      await connector.connect();
      setAccountId(await readAccountId(connector));
      return true;
    } catch (cause) {
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
    }
  }

  return (
    <WalletContext.Provider
      value={{
        connector,
        accountId,
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
