import { BigNumber, ethers } from "ethers";

// -----------------------------------------------------------------------------
// Number / Amount helpers
// -----------------------------------------------------------------------------

export function formatTokenAmount(
  amount: string | number,
  decimals = 18,
  displayDecimals = 4
): string {
  if (!amount || amount === "0") return "0";

  try {
    const formattedAmount = ethers.utils.formatUnits(amount.toString(), decimals);
    const numAmount = parseFloat(formattedAmount);

    if (numAmount === 0) return "0";
    if (numAmount < 0.0001) return "< 0.0001";

    return numAmount.toFixed(displayDecimals);
  } catch (error) {
    console.error("Error formatting token amount:", error);
    return "0";
  }
}

export function parseTokenAmount(
  amount: string | number,
  decimals = 18
): BigNumber {
  if (!amount || amount === "") return BigNumber.from(0);

  try {
    return ethers.utils.parseUnits(amount.toString(), decimals);
  } catch (error) {
    console.error("Error parsing token amount:", error);
    return BigNumber.from(0);
  }
}

export function truncateAddress(
  address: string,
  startChars = 6,
  endChars = 4
): string {
  if (!address) return "";
  if (address.length <= startChars + endChars) return address;
  return `${address.slice(0, startChars)}...${address.slice(-endChars)}`;
}

export function isValidAddress(address: string): boolean {
  try {
    return ethers.utils.isAddress(address);
  } catch {
    return false;
  }
}

export function formatPrice(price: number, decimals = 6): string {
  if (!price || price === 0) return "0";
  if (price < 0.000001) return "< 0.000001";
  if (price >= 1_000_000) return `${(price / 1_000_000).toFixed(2)}M`;
  if (price >= 1_000) return `${(price / 1_000).toFixed(2)}K`;
  return price.toFixed(decimals);
}

export function calculateSlippage(
  amount: string | number,
  slippagePercent: number,
  decimals = 18
): BigNumber {
  try {
    const parsedAmount = parseTokenAmount(amount, decimals);
    const slippageMultiplier = (100 - slippagePercent) / 100;
    const slippageAmount = parsedAmount
      .mul(Math.floor(slippageMultiplier * 100))
      .div(100);
    return slippageAmount;
  } catch (error) {
    console.error("Error calculating slippage:", error);
    return BigNumber.from(0);
  }
}

export function getTokenSymbol(
  address: string,
  popularTokens: Record<string, { address: string; symbol: string }> = {}
): string {
  const token = Object.values(popularTokens).find(
    (t) => t.address.toLowerCase() === address.toLowerCase()
  );
  return token ? token.symbol : "TOKEN";
}

export function formatTxHash(hash: string): string {
  if (!hash) return "";
  return `${hash.slice(0, 10)}...${hash.slice(-8)}`;
}

export function getExplorerUrl(
  hash: string,
  network: "polygon" | "ethereum" | "bsc" = "polygon"
): string {
  const explorers: Record<string, string> = {
    polygon: "https://polygonscan.com/tx/",
    ethereum: "https://etherscan.io/tx/",
    bsc: "https://bscscan.com/tx/",
  };
  return `${explorers[network] ?? explorers.polygon}${hash}`;
}

// -----------------------------------------------------------------------------
// Async helpers
// -----------------------------------------------------------------------------

export function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function retryWithBackoff<T>(
  fn: () => Promise<T>,
  maxRetries = 3,
  baseDelay = 1000
): Promise<T> {
  let lastError: unknown;
  for (let i = 0; i <= maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error;
      if (i === maxRetries) {
        throw lastError;
      }
      const delayTime = baseDelay * Math.pow(2, i);
      await delay(delayTime);
    }
  }
  // This is just to satisfy TypeScript; we will always either return or throw.
  throw lastError as Error;
}

export function hasSufficientBalance(
  amount: string | number,
  balance: string | number,
  decimals = 18
): boolean {
  try {
    if (!amount || !balance) return false;
    const parsedAmount = parseTokenAmount(amount, decimals);
    const parsedBalance = parseTokenAmount(balance, decimals);
    return parsedBalance.gte(parsedAmount);
  } catch (error) {
    console.error("Error checking balance:", error);
    return false;
  }
}

// -----------------------------------------------------------------------------
// Formatting helpers
// -----------------------------------------------------------------------------

export function formatLargeNumber(num: number): string {
  if (!num || num === 0) return "0";
  const absNum = Math.abs(num);
  if (absNum >= 1e12) return `${(num / 1e12).toFixed(2)}T`;
  if (absNum >= 1e9) return `${(num / 1e9).toFixed(2)}B`;
  if (absNum >= 1e6) return `${(num / 1e6).toFixed(2)}M`;
  if (absNum >= 1e3) return `${(num / 1e3).toFixed(2)}K`;
  return num.toFixed(2);
}

export function getRelativeTime(date: Date | number): string {
  const now = Date.now();
  const diffMs = now - (date instanceof Date ? date.getTime() : date);
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 60) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;

  return new Date(date).toLocaleDateString();
}

export function calculatePercentageChange(
  oldValue: number,
  newValue: number
): number {
  if (!oldValue || oldValue === 0) return 0;
  return ((newValue - oldValue) / oldValue) * 100;
}

export function debounce<Args extends unknown[]>(fn: (...args: Args) => void, wait: number) {
  let timeout: NodeJS.Timeout;
  return (...args: Args): void => {
    const later = (): void => {
      clearTimeout(timeout);
      fn(...args);
    };
    clearTimeout(timeout);
    timeout = setTimeout(later, wait);
  };
}
