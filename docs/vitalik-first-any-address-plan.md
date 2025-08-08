# DeFi Risk Dashboard: Vitalik-First, Any-Address-Ready Plan

Last updated: 2025-08-08

## 1) Goal and Scope
- Immediate demo: Fetch and display Vitalik’s live DeFi positions on the dashboard (top positions, protocol, chain, USD value, actions).
- Foundational design: Same pipeline must work for any public address (input field or connected wallet), not hardcoded.

## 2) Data Sources and Coverage
- Phase 1 (fastest path for demo, multi-chain, zero mocks):
  - Uniswap V3 positions across major chains for a given address: Ethereum (1), Polygon (137), Arbitrum (42161), Optimism (10), Base (8453).
  - Sources:
    - The Graph Uniswap V3 subgraphs per chain (primary)
    - Fallback aggregator APIs if needed (e.g., Covalent, Alchemy, Subgrounds) — still real data, no mock data
    - Prices via existing price services or a pricing API (Coingecko/DEX spot/TWAPs)
- Phase 2 (incremental coverage):
  - Aave v2/v3, Curve, PancakeSwap, Balancer, Lido, Compound across major chains.
  - Optional: Use a portfolio aggregator API for breadth, then enrich with our risk analytics.

Principles:
- Zero mock data in the live pipeline. All responses must be sourced from real protocols/APIs; if data is missing, return partial data with warnings and confidence flags.

## 3) Backend API Design (modular, any address)
- New endpoints:
  - `GET /api/v1/positions`
    - Query params: `address` (required), `protocols` (optional CSV), `chains` (optional CSV), `include_metrics=true|false`
    - Response:
      ```json
      {
        "total_count": 0,
        "last_updated": "2025-08-08T00:00:00Z",
        "data_source": "Real Multi-Chain Uniswap V3 Positions",
        "positions": [ /* Position[] */ ],
        "portfolio_stats": { /* optional */ }
      }
      ```
  - `GET /api/v1/positions/summary`
    - Query params: `address`, `period`, `include_risk=true|false`
    - Aggregated stats (value, pnl, risk summary, protocol exposure)
- Behavior:
  - If no address provided, fallback to demo address (Vitalik) only in demo mode (config flag).
  - Authentication: public read allowed; JWT optional for advanced views/rate limits.
  - Caching: short-lived cache (30–120s) keyed by `(address, protocols, chains)` to avoid API rate exhaustion.
- Position model alignment:
  - Use existing `Position` struct fields: `protocol`, `chain_id`, token addresses, amounts, liquidity, fee tier, `user_address`, and computed `usd_value`.
  - Map `chain_id ->` name on frontend (e.g., `1 -> Ethereum`, `137 -> Polygon`).
- Expandability:
  - Protocol adapters: Trait-based `PositionFetcher` per protocol/chain (`uniswap_v3`, `aave_v3`, `curve`, etc.) returning `Vec<Position>`.
  - Aggregator: orchestrates multiple fetchers based on requested protocols and chains.

## 4) Backend Implementation Steps (no migrations required)
- Step 1: Multi-chain Uniswap V3 Adapter
  - Fetch NFTs/positions by owner address via Uniswap V3 subgraphs per chain (ETH, Polygon, Arbitrum, Optimism, Base).
  - Pull tick ranges, liquidity, token0/1, pool fee tier; compute token balances and USD value.
  - Normalize to `Position` with correct `chain_id` and protocol metadata.
- Step 2: Price Enrichment
  - Reuse `PriceFeed`/`PriceStorage` services (already compiling per memory).
  - For tokens missing prices, fallback to pool spot price or ignore with degraded confidence flag.
- Step 3: Risk Enrichment (optional toggle)
  - If `include_metrics=true`: run existing risk and portfolio analytics services for summary endpoints.
- Step 4: Cache + Rate Limits
  - In-memory cache + simple per-IP per-address rate limiting; cache key includes chain list and protocol list.
- Step 5: Errors and Edge Cases
  - Empty positions: return 200 with `total_count=0`.
  - Invalid address: `400` with message.
  - Partial data: include `warnings: string[]`.

## 5) Frontend Integration Plan
- Data flow:
  - UI captures address (default prefilled Vitalik for demo).
  - Calls `GET /api/v1/positions?address=<addr>&protocols=uniswap_v3&chains=1,137,42161,10,8453&include_metrics=true`.
  - Renders:
    - Top positions: token pair, protocol badge, chain badge, USD value, fee tier, actions.
    - Portfolio summary metrics if requested (value, pnl, exposures).
- Components to adjust:
  - Replace “liquidity creator” UI with “Portfolio Overview” and “Positions” tables.
  - Chain/Protocol badges: lightweight mapping table in TS.
  - Loading/empty/error states.
- Config:
  - Backend base URL configurable via env.
  - Demo flag: if no wallet/address supplied, default to Vitalik’s address: `0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B`.
- UX:
  - Address input with sample buttons (Vitalik, Random Whale).
  - “Refresh” with timestamp of `last_updated`.
  - Minimalist, informative; emphasize protocol and chain differentiation.

## 6) Testing and Validation
- Backend:
  - Unit tests: Uniswap adapter mapping, price enrichment, USD value computation.
  - Integration tests: `/positions` endpoint for Vitalik address, cache behavior, empty address, invalid address.
- Frontend:
  - Render tests for positions table and badges.
  - E2E: load dashboard, fetch Vitalik, verify at least N positions and total value > 0.
- Performance:
  - Expect initial fetch to complete < 1.5s uncached; cached < 300ms.
  - Ensure connection pooling remains optimal (already implemented).
- Observability:
  - Use EnhancedLogger and RequestLoggingMiddleware for correlation IDs.
  - Export Prometheus metrics for endpoint latencies.

## 7) Security and Operations
- Public read endpoints; apply low-rate limits.
- JWT-protected advanced analytics routes if needed.
- Avoid new migrations for demo; reuse existing schema and dynamic fetch pipeline.
- Configurable API keys via env (pricing/indexer providers).
- Respect migration hygiene rules if future schema changes arise (enum casing, timestamped migrations).

## 8) Timeline and Milestones
- Day 0–1:
  - Backend multi-chain Uniswap V3 adapters + `/positions` endpoint (`address`, `protocols`, `chains`, `include_metrics`).
  - Price enrichment and caching.
- Day 2:
  - Frontend integration replacing current UI with Positions + Summary.
  - Chain/protocol badges and basic UX polish.
- Day 3:
  - Validation, performance tuning, observability dashboards.
  - Optional: add Aave v3 adapter for basic deposit/borrow positions.

## 9) Future Extensions
- Multi-chain: Polygon, Arbitrum, Optimism via chain list param and per-chain adapters.
- Additional protocols: Aave/Compound (lending), Curve/Balancer (LP), Lido/RocketPool (staking).
- Historical analytics: connect to portfolio analytics service to populate PnL charts and trends.
- MEV and cross-chain risk overlays on positions using existing services.

### Position payload (per item)
- position_id (UUID or protocol-native ID)
- user_address
- protocol (e.g., uniswap_v3, aave_v3)
- chain_id, chain_name (e.g., 1, Ethereum)
- pair/pool symbol (e.g., WETH/USDC), token0/1 symbols and addresses
- fee_tier (AMMs), health/ltv (lending), strategy type (staking), as applicable
- token0_amount, token1_amount (or collateral/borrow amounts for lending)
- usd_value (current position value)
- pnl_unrealized_usd, pnl_realized_usd, pnl_pct
- fees_accrued_usd (AMMs), rewards_accrued_usd (staking/lending), optional risk_score
- actions: links (view on protocol, view tx, manage)

### Top positions display rules
- Sort: `usd_value` descending
- Columns: Position (pair/symbol), Value (USD), P&L (USD and %), Chain (badge), Protocol (badge)
- Badges: protocol and chain badges with color coding
- Tooltips: show fee tier, health factor, rewards as applicable

### Protocol and Chain roadmap
- Protocols (priority): Uniswap V3, Aave v2/v3, Curve, Balancer, PancakeSwap, Lido, Compound
- Chains (priority): Ethereum, Arbitrum, Optimism, Polygon, BSC, Avalanche, Base

### P&L methodology notes
- AMMs (Uniswap V3):
  - Unrealized P&L = current USD value − entry USD value; entry approximated from initial deposit prices and subsequent adds/removes.
  - Fees shown separately as `fees_accrued_usd`; realized P&L realized on full/partial withdrawal.
- Lending (Aave/Compound):
  - Unrealized P&L from collateral/reserve price changes minus borrow costs; realized via repayments/liquidations/harvests.
- Staking:
  - Rewards accrued valued at current price; realized when claimed/sold.

## Deliverables
- API contract (example request/response)
- Frontend data shape alignment document (field mappings and rendering rules)

so currently for the first phase we will focus on the unrealized P&L 
first focus on phase 1 

Phase 1 (demo, fast):
Show Top Positions with: Position, Value (USD), Chain, Protocol.
Add Unrealized P&L (estimate) for Uniswap V3 by inferring entry from adds/removes and valuing at current prices; include a confidence flag.
Use real-time price feed; poll/subscribe pool ticks.

Phase 2 (accuracy uplift):
Full historical reconstruction per protocol (deposits/withdrawals/borrows/repays/claims) for cost basis and realized P&L.
Protocol-specific adapters for Aave, Curve, Balancer, PancakeSwap, Lido, Compound.
Replace subgraph reliance with RPC+indexer for sharper updates.

Phase 3 (portfolio analytics):
Continuous P&L time series, risk overlays (MEV, cross-chain), fees/rewards attribution, and confidence scoring per metric.

UI/UX guidance

Always show Value and Chain.
Show P&L with a small “confidence” indicator:
High: complete history parsed and priced
Medium: partial cost basis inferred
Low: mark-to-market only (no reliable cost basis)
Tooltip explains methodology and timestamp of last refresh.

