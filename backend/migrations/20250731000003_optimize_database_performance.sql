-- Database Performance Optimization Migration
-- Migration: 20250731000003_optimize_database_performance.sql

-- Add performance indexes for frequently queried tables

-- Positions table indexes
CREATE INDEX IF NOT EXISTS idx_positions_user_address ON positions(user_address);
CREATE INDEX IF NOT EXISTS idx_positions_pool_address ON positions(pool_address);
CREATE INDEX IF NOT EXISTS idx_positions_chain_id ON positions(chain_id);
CREATE INDEX IF NOT EXISTS idx_positions_protocol ON positions(protocol);
CREATE INDEX IF NOT EXISTS idx_positions_created_at ON positions(created_at);
CREATE INDEX IF NOT EXISTS idx_positions_user_protocol ON positions(user_address, protocol);
CREATE INDEX IF NOT EXISTS idx_positions_pool_chain ON positions(pool_address, chain_id);

-- Pool states table indexes
CREATE INDEX IF NOT EXISTS idx_pool_states_pool_address ON pool_states(pool_address);
CREATE INDEX IF NOT EXISTS idx_pool_states_chain_id ON pool_states(chain_id);
CREATE INDEX IF NOT EXISTS idx_pool_states_timestamp ON pool_states(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_pool_states_pool_chain_time ON pool_states(pool_address, chain_id, timestamp DESC);

-- Risk metrics table indexes (conditional creation removed due to PostgreSQL limitations)
-- CREATE INDEX IF NOT EXISTS idx_risk_metrics_position_id ON risk_metrics(position_id);
-- CREATE INDEX IF NOT EXISTS idx_risk_metrics_calculated_at ON risk_metrics(calculated_at DESC);

-- MEV risks table indexes
CREATE INDEX IF NOT EXISTS idx_mev_risks_pool_address ON mev_risks(pool_address);
CREATE INDEX IF NOT EXISTS idx_mev_risks_chain_id ON mev_risks(chain_id);
CREATE INDEX IF NOT EXISTS idx_mev_risks_created_at ON mev_risks(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_mev_risks_pool_chain ON mev_risks(pool_address, chain_id);
CREATE INDEX IF NOT EXISTS idx_mev_risks_overall_risk ON mev_risks(overall_mev_risk DESC);

-- MEV transactions table indexes (conditional - only if table exists)
-- CREATE INDEX IF NOT EXISTS idx_mev_transactions_hash ON mev_transactions(transaction_hash);
-- CREATE INDEX IF NOT EXISTS idx_mev_transactions_block ON mev_transactions(block_number);
-- CREATE INDEX IF NOT EXISTS idx_mev_transactions_chain ON mev_transactions(chain_id);
-- CREATE INDEX IF NOT EXISTS idx_mev_transactions_type ON mev_transactions(mev_type);
-- CREATE INDEX IF NOT EXISTS idx_mev_transactions_severity ON mev_transactions(severity);

-- Oracle deviations table indexes (conditional - only if table exists)
-- CREATE INDEX IF NOT EXISTS idx_oracle_deviations_oracle_address ON oracle_deviations(oracle_address);
-- CREATE INDEX IF NOT EXISTS idx_oracle_deviations_chain_id ON oracle_deviations(chain_id);
-- CREATE INDEX IF NOT EXISTS idx_oracle_deviations_severity ON oracle_deviations(severity);

-- Cross-chain risks table indexes
CREATE INDEX IF NOT EXISTS idx_cross_chain_risks_position_id ON cross_chain_risks(position_id);
CREATE INDEX IF NOT EXISTS idx_cross_chain_risks_primary_chain ON cross_chain_risks(primary_chain_id);
CREATE INDEX IF NOT EXISTS idx_cross_chain_risks_created_at ON cross_chain_risks(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_cross_chain_risks_overall_risk ON cross_chain_risks(overall_cross_chain_risk DESC);

-- Bridge risks table indexes
CREATE INDEX IF NOT EXISTS idx_bridge_risks_bridge_protocol ON bridge_risks(bridge_protocol);
CREATE INDEX IF NOT EXISTS idx_bridge_risks_source_chain ON bridge_risks(source_chain_id);
CREATE INDEX IF NOT EXISTS idx_bridge_risks_overall_risk ON bridge_risks(overall_bridge_risk DESC);

-- Chain risks table indexes
CREATE INDEX IF NOT EXISTS idx_chain_risks_chain_id ON chain_risks(chain_id);
CREATE INDEX IF NOT EXISTS idx_chain_risks_overall_risk ON chain_risks(overall_chain_risk DESC);

-- Protocol risks table indexes
CREATE INDEX IF NOT EXISTS idx_protocol_risks_protocol_name ON protocol_risks(protocol_name);
CREATE INDEX IF NOT EXISTS idx_protocol_risks_chain_id ON protocol_risks(chain_id);
CREATE INDEX IF NOT EXISTS idx_protocol_risks_overall_risk ON protocol_risks(overall_protocol_risk DESC);
CREATE INDEX IF NOT EXISTS idx_protocol_risks_last_updated ON protocol_risks(last_updated DESC);

-- Audit logs table indexes
CREATE INDEX IF NOT EXISTS idx_audit_logs_event_type ON audit_logs(event_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_severity ON audit_logs(severity);
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_type ON audit_logs(resource_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource_id ON audit_logs(resource_id);

-- User risk configs table indexes
CREATE INDEX IF NOT EXISTS idx_user_risk_configs_user_address ON user_risk_configs(user_address);
CREATE INDEX IF NOT EXISTS idx_user_risk_configs_is_active ON user_risk_configs(is_active);
CREATE INDEX IF NOT EXISTS idx_user_risk_configs_user_active ON user_risk_configs(user_address, is_active);

-- Webhooks table indexes (commented out - table doesn't exist yet)
-- CREATE INDEX IF NOT EXISTS idx_webhooks_user_address ON webhooks(user_address);
-- CREATE INDEX IF NOT EXISTS idx_webhooks_is_active ON webhooks(is_active);
-- CREATE INDEX IF NOT EXISTS idx_webhooks_user_active ON webhooks(user_address, is_active);

-- Alerts table indexes
CREATE INDEX IF NOT EXISTS idx_alerts_position_id ON alerts(position_id);
CREATE INDEX IF NOT EXISTS idx_alerts_is_resolved ON alerts(is_resolved);
CREATE INDEX IF NOT EXISTS idx_alerts_severity ON alerts(severity);
CREATE INDEX IF NOT EXISTS idx_alerts_created_at ON alerts(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_alerts_position_resolved ON alerts(position_id, is_resolved);

-- Price history table indexes
CREATE INDEX IF NOT EXISTS idx_price_history_token_address ON price_history(token_address);
CREATE INDEX IF NOT EXISTS idx_price_history_chain_id ON price_history(chain_id);
CREATE INDEX IF NOT EXISTS idx_price_history_timestamp ON price_history(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_price_history_token_chain_time ON price_history(token_address, chain_id, timestamp DESC);

-- Composite indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_positions_user_protocol_chain ON positions(user_address, protocol, chain_id);
CREATE INDEX IF NOT EXISTS idx_pool_states_comprehensive ON pool_states(pool_address, chain_id, timestamp DESC, tvl_usd DESC);
CREATE INDEX IF NOT EXISTS idx_mev_risks_comprehensive ON mev_risks(pool_address, chain_id, created_at DESC, overall_mev_risk DESC);

-- Partial indexes for active records only
CREATE INDEX IF NOT EXISTS idx_active_user_risk_configs ON user_risk_configs(user_address, updated_at DESC) WHERE is_active = true;
-- CREATE INDEX IF NOT EXISTS idx_active_webhooks ON webhooks(user_address, endpoint_url) WHERE is_active = true;
CREATE INDEX IF NOT EXISTS idx_unresolved_alerts ON alerts(position_id, created_at DESC) WHERE is_resolved = false;

-- GIN indexes for JSONB columns
CREATE INDEX IF NOT EXISTS idx_audit_logs_metadata_gin ON audit_logs USING GIN(metadata);
CREATE INDEX IF NOT EXISTS idx_audit_logs_compliance_tags_gin ON audit_logs USING GIN(compliance_tags);

-- Add database statistics and analyze tables for query optimization
ANALYZE positions;
ANALYZE pool_states;
ANALYZE mev_risks;
ANALYZE mev_transactions;
ANALYZE oracle_deviations;
ANALYZE cross_chain_risks;
ANALYZE protocol_risks;
ANALYZE audit_logs;
ANALYZE user_risk_configs;
-- ANALYZE webhooks;
ANALYZE alerts;
ANALYZE price_history;

-- Create materialized views for expensive aggregations
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_user_position_summary AS
SELECT 
    p.user_address,
    p.protocol,
    p.chain_id,
    COUNT(*) as position_count,
    SUM(p.token0_amount * COALESCE(ps.token0_price_usd, 0) + p.token1_amount * COALESCE(ps.token1_price_usd, 0)) as total_value_usd,
    AVG(COALESCE(ps.tvl_usd, 0)) as avg_pool_tvl,
    MAX(p.created_at) as last_position_created
FROM positions p
LEFT JOIN pool_states ps ON p.pool_address = ps.pool_address AND p.chain_id = ps.chain_id
WHERE ps.timestamp = (
    SELECT MAX(timestamp) 
    FROM pool_states ps2 
    WHERE ps2.pool_address = ps.pool_address 
    AND ps2.chain_id = ps.chain_id
)
GROUP BY p.user_address, p.protocol, p.chain_id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_user_position_summary_unique 
ON mv_user_position_summary(user_address, protocol, chain_id);

-- Create materialized view for MEV risk aggregations
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_mev_risk_summary AS
SELECT 
    pool_address,
    chain_id,
    AVG(sandwich_risk_score) as avg_sandwich_risk,
    AVG(frontrun_risk_score) as avg_frontrun_risk,
    AVG(oracle_manipulation_risk) as avg_oracle_risk,
    AVG(overall_mev_risk) as avg_overall_risk,
    COUNT(*) as assessment_count,
    MAX(created_at) as last_assessment
FROM mev_risks
WHERE created_at >= NOW() - INTERVAL '7 days'
GROUP BY pool_address, chain_id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_mev_risk_summary_unique 
ON mv_mev_risk_summary(pool_address, chain_id);

-- Set up automatic refresh for materialized views
-- Note: This would typically be done with a cron job or scheduled task
-- For now, we'll create a function to refresh them manually

CREATE OR REPLACE FUNCTION refresh_materialized_views()
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_user_position_summary;
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_mev_risk_summary;
    
    -- Update table statistics
    ANALYZE mv_user_position_summary;
    ANALYZE mv_mev_risk_summary;
END;
$$;

-- Add comments for documentation
COMMENT ON FUNCTION refresh_materialized_views() IS 'Refreshes all materialized views for performance optimization';
COMMENT ON MATERIALIZED VIEW mv_user_position_summary IS 'Aggregated user position data for dashboard queries';
COMMENT ON MATERIALIZED VIEW mv_mev_risk_summary IS 'Aggregated MEV risk data for the last 7 days';
