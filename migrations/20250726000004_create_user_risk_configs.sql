-- Create enum for risk tolerance levels
CREATE TYPE risk_tolerance_level AS ENUM ('conservative', 'moderate', 'aggressive', 'custom');

-- Create user_risk_configs table for user-configurable risk settings
CREATE TABLE user_risk_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_address VARCHAR(42) NOT NULL,
    profile_name VARCHAR(100) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    
    -- Risk calculation weights (must sum to 1.0)
    liquidity_risk_weight DECIMAL(5,4) NOT NULL CHECK (liquidity_risk_weight >= 0 AND liquidity_risk_weight <= 1),
    volatility_risk_weight DECIMAL(5,4) NOT NULL CHECK (volatility_risk_weight >= 0 AND volatility_risk_weight <= 1),
    protocol_risk_weight DECIMAL(5,4) NOT NULL CHECK (protocol_risk_weight >= 0 AND protocol_risk_weight <= 1),
    mev_risk_weight DECIMAL(5,4) NOT NULL CHECK (mev_risk_weight >= 0 AND mev_risk_weight <= 1),
    cross_chain_risk_weight DECIMAL(5,4) NOT NULL CHECK (cross_chain_risk_weight >= 0 AND cross_chain_risk_weight <= 1),
    
    -- Liquidity risk parameters
    min_tvl_threshold DECIMAL(20,2) NOT NULL CHECK (min_tvl_threshold >= 0),
    max_slippage_tolerance DECIMAL(5,4) NOT NULL CHECK (max_slippage_tolerance >= 0 AND max_slippage_tolerance <= 1),
    thin_pool_threshold DECIMAL(5,4) NOT NULL CHECK (thin_pool_threshold >= 0 AND thin_pool_threshold <= 1),
    tvl_drop_threshold DECIMAL(5,4) NOT NULL CHECK (tvl_drop_threshold >= 0 AND tvl_drop_threshold <= 1),
    
    -- Volatility risk parameters
    volatility_lookback_days INTEGER NOT NULL CHECK (volatility_lookback_days > 0 AND volatility_lookback_days <= 365),
    high_volatility_threshold DECIMAL(5,4) NOT NULL CHECK (high_volatility_threshold >= 0),
    correlation_threshold DECIMAL(5,4) NOT NULL CHECK (correlation_threshold >= 0 AND correlation_threshold <= 1),
    
    -- Protocol risk parameters
    min_audit_score DECIMAL(5,4) NOT NULL CHECK (min_audit_score >= 0 AND min_audit_score <= 1),
    max_exploit_tolerance INTEGER NOT NULL CHECK (max_exploit_tolerance >= 0),
    governance_risk_weight DECIMAL(5,4) NOT NULL CHECK (governance_risk_weight >= 0 AND governance_risk_weight <= 1),
    
    -- MEV risk parameters
    sandwich_attack_threshold DECIMAL(5,4) NOT NULL CHECK (sandwich_attack_threshold >= 0),
    frontrun_threshold DECIMAL(5,4) NOT NULL CHECK (frontrun_threshold >= 0),
    oracle_deviation_threshold DECIMAL(5,4) NOT NULL CHECK (oracle_deviation_threshold >= 0),
    
    -- Cross-chain risk parameters
    bridge_risk_tolerance DECIMAL(5,4) NOT NULL CHECK (bridge_risk_tolerance >= 0 AND bridge_risk_tolerance <= 1),
    liquidity_fragmentation_threshold DECIMAL(5,4) NOT NULL CHECK (liquidity_fragmentation_threshold >= 0 AND liquidity_fragmentation_threshold <= 1),
    governance_divergence_threshold DECIMAL(5,4) NOT NULL CHECK (governance_divergence_threshold >= 0 AND governance_divergence_threshold <= 1),
    
    -- Overall risk calculation
    overall_risk_threshold DECIMAL(5,4) NOT NULL CHECK (overall_risk_threshold >= 0 AND overall_risk_threshold <= 1),
    risk_tolerance_level risk_tolerance_level NOT NULL,
    
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    
    -- Constraints
    CONSTRAINT unique_user_profile UNIQUE (user_address, profile_name),
    CONSTRAINT valid_risk_weights CHECK (
        ABS((liquidity_risk_weight + volatility_risk_weight + protocol_risk_weight + mev_risk_weight + cross_chain_risk_weight) - 1.0) < 0.01
    )
);

-- Create indexes for efficient queries
CREATE INDEX idx_user_risk_configs_user_address ON user_risk_configs(user_address);
CREATE INDEX idx_user_risk_configs_active ON user_risk_configs(user_address, is_active);
CREATE INDEX idx_user_risk_configs_tolerance_level ON user_risk_configs(risk_tolerance_level);
CREATE INDEX idx_user_risk_configs_created_at ON user_risk_configs(created_at);

-- Create trigger to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_user_risk_configs_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_user_risk_configs_updated_at
    BEFORE UPDATE ON user_risk_configs
    FOR EACH ROW
    EXECUTE FUNCTION update_user_risk_configs_updated_at();

-- Insert default configurations for each tolerance level
INSERT INTO user_risk_configs (
    user_address, profile_name, risk_tolerance_level,
    liquidity_risk_weight, volatility_risk_weight, protocol_risk_weight, mev_risk_weight, cross_chain_risk_weight,
    min_tvl_threshold, max_slippage_tolerance, thin_pool_threshold, tvl_drop_threshold,
    volatility_lookback_days, high_volatility_threshold, correlation_threshold,
    min_audit_score, max_exploit_tolerance, governance_risk_weight,
    sandwich_attack_threshold, frontrun_threshold, oracle_deviation_threshold,
    bridge_risk_tolerance, liquidity_fragmentation_threshold, governance_divergence_threshold,
    overall_risk_threshold
) VALUES 
-- Conservative template
('template_conservative', 'Conservative Template', 'conservative',
 0.30, 0.25, 0.20, 0.15, 0.10,
 10000000.00, 0.01, 0.8, 0.20,
 30, 0.15, 0.7,
 0.8, 0, 0.4,
 0.005, 0.01, 0.02,
 0.1, 0.3, 0.2,
 0.3),

-- Moderate template
('template_moderate', 'Moderate Template', 'moderate',
 0.25, 0.20, 0.20, 0.20, 0.15,
 1000000.00, 0.03, 0.6, 0.40,
 14, 0.30, 0.5,
 0.6, 1, 0.3,
 0.02, 0.03, 0.05,
 0.3, 0.5, 0.4,
 0.6),

-- Aggressive template
('template_aggressive', 'Aggressive Template', 'aggressive',
 0.15, 0.15, 0.15, 0.25, 0.30,
 100000.00, 0.10, 0.3, 0.70,
 7, 0.60, 0.2,
 0.3, 3, 0.1,
 0.10, 0.15, 0.20,
 0.7, 0.8, 0.7,
 0.9);

-- Add comments for documentation
COMMENT ON TABLE user_risk_configs IS 'User-configurable risk calculation parameters and thresholds';
COMMENT ON COLUMN user_risk_configs.user_address IS 'Ethereum address of the user';
COMMENT ON COLUMN user_risk_configs.profile_name IS 'User-defined name for this risk configuration profile';
COMMENT ON COLUMN user_risk_configs.is_active IS 'Whether this configuration is currently active for the user';
COMMENT ON COLUMN user_risk_configs.liquidity_risk_weight IS 'Weight for liquidity risk in overall calculation (0-1)';
COMMENT ON COLUMN user_risk_configs.volatility_risk_weight IS 'Weight for volatility risk in overall calculation (0-1)';
COMMENT ON COLUMN user_risk_configs.protocol_risk_weight IS 'Weight for protocol risk in overall calculation (0-1)';
COMMENT ON COLUMN user_risk_configs.mev_risk_weight IS 'Weight for MEV risk in overall calculation (0-1)';
COMMENT ON COLUMN user_risk_configs.cross_chain_risk_weight IS 'Weight for cross-chain risk in overall calculation (0-1)';
COMMENT ON COLUMN user_risk_configs.min_tvl_threshold IS 'Minimum TVL required for position (USD)';
COMMENT ON COLUMN user_risk_configs.max_slippage_tolerance IS 'Maximum acceptable slippage percentage (0-1)';
COMMENT ON COLUMN user_risk_configs.thin_pool_threshold IS 'Threshold for detecting thin liquidity pools (0-1)';
COMMENT ON COLUMN user_risk_configs.tvl_drop_threshold IS 'Threshold for detecting significant TVL drops (0-1)';
COMMENT ON COLUMN user_risk_configs.volatility_lookback_days IS 'Number of days to look back for volatility calculation';
COMMENT ON COLUMN user_risk_configs.high_volatility_threshold IS 'Threshold for high volatility detection';
COMMENT ON COLUMN user_risk_configs.correlation_threshold IS 'Threshold for correlation risk detection (0-1)';
COMMENT ON COLUMN user_risk_configs.min_audit_score IS 'Minimum required audit score for protocols (0-1)';
COMMENT ON COLUMN user_risk_configs.max_exploit_tolerance IS 'Maximum number of exploits tolerated for protocols';
COMMENT ON COLUMN user_risk_configs.governance_risk_weight IS 'Weight for governance risk in protocol risk calculation (0-1)';
COMMENT ON COLUMN user_risk_configs.sandwich_attack_threshold IS 'Threshold for sandwich attack detection';
COMMENT ON COLUMN user_risk_configs.frontrun_threshold IS 'Threshold for frontrunning detection';
COMMENT ON COLUMN user_risk_configs.oracle_deviation_threshold IS 'Threshold for oracle price deviation detection';
COMMENT ON COLUMN user_risk_configs.bridge_risk_tolerance IS 'Tolerance for bridge-related risks (0-1)';
COMMENT ON COLUMN user_risk_configs.liquidity_fragmentation_threshold IS 'Threshold for liquidity fragmentation detection (0-1)';
COMMENT ON COLUMN user_risk_configs.governance_divergence_threshold IS 'Threshold for governance divergence detection (0-1)';
COMMENT ON COLUMN user_risk_configs.overall_risk_threshold IS 'Overall risk threshold for position evaluation (0-1)';
COMMENT ON COLUMN user_risk_configs.risk_tolerance_level IS 'Predefined risk tolerance level';
