-- User settings table
CREATE TABLE IF NOT EXISTS user_settings (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    email_notifications BOOLEAN NOT NULL DEFAULT true,
    sms_notifications BOOLEAN NOT NULL DEFAULT false,
    webhook_notifications BOOLEAN NOT NULL DEFAULT false,
    risk_tolerance VARCHAR(50) NOT NULL DEFAULT 'moderate' CHECK (risk_tolerance IN ('conservative', 'moderate', 'aggressive')),
    preferred_currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    dashboard_layout JSONB NOT NULL DEFAULT '{}',
    alert_frequency VARCHAR(20) NOT NULL DEFAULT 'immediate' CHECK (alert_frequency IN ('immediate', 'hourly', 'daily')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- User addresses table for wallet address mapping
CREATE TABLE IF NOT EXISTS user_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    address VARCHAR(255) NOT NULL,
    chain_id INTEGER NOT NULL DEFAULT 1,
    address_type VARCHAR(50) NOT NULL DEFAULT 'ethereum' CHECK (address_type IN ('ethereum', 'bitcoin', 'polygon', 'arbitrum', 'optimism', 'bsc', 'avalanche')),
    is_primary BOOLEAN NOT NULL DEFAULT false,
    label VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, address, chain_id)
);

-- User risk preferences table
CREATE TABLE IF NOT EXISTS user_risk_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    max_position_size_usd DECIMAL(20,2),
    max_protocol_allocation_percent DECIMAL(5,2) CHECK (max_protocol_allocation_percent >= 0 AND max_protocol_allocation_percent <= 100),
    max_single_pool_percent DECIMAL(5,2) CHECK (max_single_pool_percent >= 0 AND max_single_pool_percent <= 100),
    min_liquidity_threshold_usd DECIMAL(20,2),
    max_risk_score DECIMAL(3,2) CHECK (max_risk_score >= 0 AND max_risk_score <= 1),
    allowed_protocols JSONB NOT NULL DEFAULT '[]',
    blocked_protocols JSONB NOT NULL DEFAULT '[]',
    preferred_chains JSONB NOT NULL DEFAULT '["ethereum"]',
    max_slippage_percent DECIMAL(5,2) CHECK (max_slippage_percent >= 0 AND max_slippage_percent <= 100),
    auto_rebalance_enabled BOOLEAN NOT NULL DEFAULT false,
    stop_loss_enabled BOOLEAN NOT NULL DEFAULT false,
    stop_loss_threshold_percent DECIMAL(5,2) CHECK (stop_loss_threshold_percent >= 0 AND stop_loss_threshold_percent <= 100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_user_addresses_user_id ON user_addresses(user_id);
CREATE INDEX IF NOT EXISTS idx_user_addresses_address ON user_addresses(LOWER(address));
CREATE INDEX IF NOT EXISTS idx_user_addresses_chain_id ON user_addresses(chain_id);
CREATE INDEX IF NOT EXISTS idx_user_addresses_primary ON user_addresses(user_id, is_primary) WHERE is_primary = true;

CREATE INDEX IF NOT EXISTS idx_user_settings_user_id ON user_settings(user_id);
CREATE INDEX IF NOT EXISTS idx_user_settings_risk_tolerance ON user_settings(risk_tolerance);
CREATE INDEX IF NOT EXISTS idx_user_settings_preferred_currency ON user_settings(preferred_currency);

CREATE INDEX IF NOT EXISTS idx_user_risk_preferences_user_id ON user_risk_preferences(user_id);

-- Create updated_at triggers
CREATE TRIGGER update_user_settings_updated_at 
    BEFORE UPDATE ON user_settings 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_addresses_updated_at 
    BEFORE UPDATE ON user_addresses 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_risk_preferences_updated_at 
    BEFORE UPDATE ON user_risk_preferences 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

-- Insert some test data
INSERT INTO user_addresses (user_id, address, chain_id, address_type, is_primary, label) 
SELECT id, '0x742d35Cc6634C0532925a3b8D1c9c5E3C8C5C5C5', 1, 'ethereum', true, 'Primary Wallet'
FROM users WHERE username = 'admin'
ON CONFLICT (user_id, address, chain_id) DO NOTHING;

INSERT INTO user_addresses (user_id, address, chain_id, address_type, is_primary, label) 
SELECT id, '0x8ba1f109551bD432803012645Hac136c22C5c5C5', 137, 'polygon', false, 'Polygon Wallet'
FROM users WHERE username = 'admin'
ON CONFLICT (user_id, address, chain_id) DO NOTHING;
