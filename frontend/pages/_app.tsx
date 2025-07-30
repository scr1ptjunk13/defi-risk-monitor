import "../styles/globals.css";
import React from "react";
import { useEffect, useState } from "react";
import type { AppProps } from "next/app";
import { WagmiConfig } from "wagmi";
import { RainbowKitProvider, darkTheme } from "@rainbow-me/rainbowkit";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Toaster } from "react-hot-toast";
import { config } from "../provider/wagmi";

import "@rainbow-me/rainbowkit/styles.css";

const queryClient = new QueryClient();

export default function App({ Component, pageProps }: AppProps): React.ReactElement | null {
  const [mounted, setMounted] = useState(false);

  // Fix hydration error
  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) {
    return null;
  }

  return (
    <WagmiConfig config={config}>
      <QueryClientProvider client={queryClient}>
        <RainbowKitProvider
          theme={darkTheme({
            accentColor: "#3B82F6",
            accentColorForeground: "white",
            borderRadius: "medium",
            fontStack: "system",
            overlayBlur: "small",
          })}
          showRecentTransactions={true}
        >
          <Component {...pageProps} />
          <Toaster
            position="top-right"
            toastOptions={{
              duration: 4000,
              style: {
                background: "#1F2937",
                color: "#F9FAFB",
                border: "1px solid #374151",
              },
              success: {
                iconTheme: {
                  primary: "#10B981",
                  secondary: "#F9FAFB",
                },
              },
              error: {
                iconTheme: {
                  primary: "#EF4444",
                  secondary: "#F9FAFB",
                },
              },
            }}
          />
        </RainbowKitProvider>
      </QueryClientProvider>
    </WagmiConfig>
  );
}
