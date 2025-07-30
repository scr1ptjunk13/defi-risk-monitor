import { type Chain } from "wagmi/chains";
import { getDefaultConfig } from "@rainbow-me/rainbowkit";

// -----------------------------------------------------------------------------
// Environment variables
// -----------------------------------------------------------------------------
// NEXT_PUBLIC_* variables are always available in the browser in Next.js.
// We cast them to string because they can technically be undefined at runtime
// during the type-checking phase, but we know they will be supplied (otherwise
// RainbowKit will complain at runtime).
// -----------------------------------------------------------------------------

const PROJECT_ID = process.env.NEXT_PUBLIC_WALLET_CONNECT_PROJECT_ID as string;
const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL as string;

// -----------------------------------------------------------------------------
// Custom Polygon chain configuration
// -----------------------------------------------------------------------------
// We could import the default `polygon` object from `wagmi/chains` but we want
// to use the new POL denomination instead of MATIC. Therefore we spread the
// default properties and override the few that are different.
// -----------------------------------------------------------------------------

const polygonChain: Chain = {
  id: 137,
  name: "Polygon",
  nativeCurrency: {
    name: "POL",
    symbol: "POL",
    decimals: 18,
  },
  rpcUrls: {
    default: { http: [RPC_URL] },
    public: { http: [RPC_URL] },
  },
  blockExplorers: {
    default: {
      name: "PolygonScan",
      url: "https://polygonscan.com",
    },
  },
  testnet: false,
};

// -----------------------------------------------------------------------------
// Wagmi / RainbowKit client configuration
// -----------------------------------------------------------------------------

export const config = getDefaultConfig({
  appName: "Liquidity",
  projectId: PROJECT_ID,
  chains: [polygonChain],
  ssr: true,
});
