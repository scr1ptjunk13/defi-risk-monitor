-- Initial schema for DeFi Risk Monitor

-- Positions table
CREATE TABLE positions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_address VARCHAR(42) NOT NULL,
    protocol VARCHAR(50) NOT NULL,
    pool_address VARCHAR(42) NOT NULL,
    token0_address VARCHAR(42) NOT NULL,
    token1_address VARCHAR(42) NOT NULL,
    token0_amount DECIMAL(78, 18) NOT NULL,
    token1_amount DECIMAL(78, 18) NOT NULL,
    liquidity DECIMAL(78, 18) NOT NULL,
    tick_lower INTEGER NOT NULL,
    tick_upper INTEGER NOT NULL,
    fee_tier INTEGER NOT NULL,
    chain_id INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Pool states table
CREATE TABLE pool_states (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_address VARCHAR(42) NOT NULL,
    chain_id INTEGER NOT NULL,
    current_tick INTEGER NOT NULL,
    sqrt_price_x96 DECIMAL(78, 0) NOT NULL,
    liquidity DECIMAL(78, 18) NOT NULL,
    token0_price_usd DECIMAL(18, 8),
    token1_price_usd DECIMAL(18, 8),
    tvl_usd DECIMAL(18, 2),
    volume_24h_usd DECIMAL(18, 2),
    fees_24h_usd DECIMAL(18, 2),
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(pool_address, chain_id, timestamp)
);

-- Risk configurations table
CREATE TABLE risk_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_address VARCHAR(42) NOT NULL,
    max_position_size_usd DECIMAL(18, 2) NOT NULL DEFAULT 1000000,
    liquidation_threshold DECIMAL(5, 4) NOT NULL DEFAULT 0.85,
    price_impact_threshold DECIMAL(5, 4) NOT NULL DEFAULT 0.05,
    impermanent_loss_threshold DECIMAL(5, 4) NOT NULL DEFAULT 0.10,
    volatility_threshold DECIMAL(5, 4) NOT NULL DEFAULT 0.20,
    correlation_threshold DECIMAL(5, 4) NOT NULL DEFAULT 0.80,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(user_address)
);

-- Alerts table
CREATE TABLE alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    position_id UUID REFERENCES positions(id) ON DELETE CASCADE,
    alert_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    title VARCHAR(200) NOT NULL,
    message TEXT NOT NULL,
    risk_score DECIMAL(5, 4),
    current_value DECIMAL(18, 8),
    threshold_value DECIMAL(18, 8),
    is_resolved BOOLEAN DEFAULT FALSE,
    resolved_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Risk metrics table
CREATE TABLE risk_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    position_id UUID REFERENCES positions(id) ON DELETE CASCADE,
    impermanent_loss DECIMAL(5, 4),
    price_impact DECIMAL(5, 4),
    volatility_score DECIMAL(5, 4),
    correlation_score DECIMAL(5, 4),
    liquidity_score DECIMAL(5, 4),
    overall_risk_score DECIMAL(5, 4) NOT NULL,
    value_at_risk_1d DECIMAL(18, 8),
    value_at_risk_7d DECIMAL(18, 8),
    calculated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for better performance
CREATE INDEX idx_positions_user_address ON positions(user_address);
CREATE INDEX idx_positions_pool_address ON positions(pool_address);
CREATE INDEX idx_positions_chain_id ON positions(chain_id);
CREATE INDEX idx_pool_states_pool_address ON pool_states(pool_address);
CREATE INDEX idx_pool_states_timestamp ON pool_states(timestamp);
CREATE INDEX idx_alerts_position_id ON alerts(position_id);
CREATE INDEX idx_alerts_created_at ON alerts(created_at);
CREATE INDEX idx_alerts_severity ON alerts(severity);
CREATE INDEX idx_risk_metrics_position_id ON risk_metrics(position_id);
CREATE INDEX idx_risk_metrics_calculated_at ON risk_metrics(calculated_at);
