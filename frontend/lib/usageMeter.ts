// Simple in-memory usage metering. This will reset whenever the server restarts.
// For production consider persistent storage (e.g. database).

const usage: Record<string, number> = {};
const LIMIT = Number(process.env.API_CALL_LIMIT ?? 1000);

export function recordCall(apiKey: string) {
  usage[apiKey] = (usage[apiKey] || 0) + 1;
  return usage[apiKey];
}

export function remainingCalls(apiKey: string) {
  const used = usage[apiKey] || 0;
  return LIMIT - used;
}