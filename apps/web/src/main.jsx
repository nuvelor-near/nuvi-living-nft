import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { WalletConnectProvider } from "./wallet/WalletConnectContext";
import "./styles.css";

createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <WalletConnectProvider>
      <App />
    </WalletConnectProvider>
  </React.StrictMode>,
);
