-- Add missing database indexes for query optimization
-- Migration: 20250801000005_add_missing_indexes.sql

-- Add missing composite index for alerts (severity, created_at)
-- This improves queries that filter by severity and order by creation time
CREATE INDEX IF NOT EXISTS idx_alerts_severity_created ON alerts(severity, created_at DESC);

-- Add missing index for cross_chain_risks by user_address
-- Note: cross_chain_risks table doesn't have user_address directly, but we can create
-- an index that helps with joins through position_id to get user positions
CREATE INDEX IF NOT EXISTS idx_cross_chain_risks_position_created ON cross_chain_risks(position_id, created_at DESC);

-- Add additional performance indexes for common query patterns

-- Improve MEV risk queries by adding timestamp ordering to existing composite index
CREATE INDEX IF NOT EXISTS idx_mev_risks_pool_chain_time ON mev_risks(pool_address, chain_id, created_at DESC);

-- Add index for bridge risks by destination chain (useful for cross-chain analysis)
CREATE INDEX IF NOT EXISTS idx_bridge_risks_destination_chain ON bridge_risks(destination_chain_id);

-- Add composite index for positions with timestamp ordering (useful for recent position queries)
CREATE INDEX IF NOT EXISTS idx_positions_user_created ON positions(user_address, created_at DESC);

-- Add index for pool states by TVL for top pools queries
CREATE INDEX IF NOT EXISTS idx_pool_states_tvl ON pool_states(tvl_usd DESC NULLS LAST);

-- Add index for alerts by alert_type and severity (useful for alert dashboard filtering)
CREATE INDEX IF NOT EXISTS idx_alerts_type_severity ON alerts(alert_type, severity);

-- Add partial index for unresolved alerts only (more efficient for active alerts)
CREATE INDEX IF NOT EXISTS idx_alerts_unresolved_created ON alerts(created_at DESC) WHERE is_resolved = false;

-- Add index for risk_configs by user for faster user-specific risk configuration lookups
CREATE INDEX IF NOT EXISTS idx_risk_configs_user_updated ON risk_configs(user_address, updated_at DESC);

-- Comments for documentation
COMMENT ON INDEX idx_alerts_severity_created IS 'Composite index for filtering alerts by severity and ordering by creation time';
COMMENT ON INDEX idx_cross_chain_risks_position_created IS 'Index for cross-chain risks with position and time ordering';
COMMENT ON INDEX idx_mev_risks_pool_chain_time IS 'Composite index for MEV risks with pool, chain, and time ordering';
COMMENT ON INDEX idx_bridge_risks_destination_chain IS 'Index for bridge risks by destination chain';
COMMENT ON INDEX idx_positions_user_created IS 'Composite index for user positions ordered by creation time';
COMMENT ON INDEX idx_pool_states_tvl IS 'Index for pool states ordered by TVL (descending, nulls last)';
COMMENT ON INDEX idx_alerts_type_severity IS 'Composite index for alerts by type and severity';
COMMENT ON INDEX idx_alerts_unresolved_created IS 'Partial index for unresolved alerts ordered by creation time';
COMMENT ON INDEX idx_risk_configs_user_updated IS 'Composite index for risk configs by user and update time';
