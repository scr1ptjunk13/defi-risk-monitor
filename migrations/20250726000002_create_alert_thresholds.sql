-- Create alert_thresholds table for user-configurable alert thresholds
CREATE TABLE alert_thresholds (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_address VARCHAR(42) NOT NULL,
    position_id UUID REFERENCES positions(id) ON DELETE CASCADE,
    threshold_type VARCHAR(50) NOT NULL CHECK (threshold_type IN (
        'impermanent_loss',
        'tvl_drop', 
        'liquidity_risk',
        'volatility_risk',
        'protocol_risk',
        'mev_risk',
        'cross_chain_risk',
        'overall_risk'
    )),
    operator VARCHAR(30) NOT NULL CHECK (operator IN (
        'greater_than',
        'less_than',
        'greater_than_or_equal',
        'less_than_or_equal'
    )),
    threshold_value DECIMAL(10,6) NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX idx_alert_thresholds_user_address ON alert_thresholds(user_address);
CREATE INDEX idx_alert_thresholds_position_id ON alert_thresholds(position_id);
CREATE INDEX idx_alert_thresholds_type ON alert_thresholds(threshold_type);
CREATE INDEX idx_alert_thresholds_enabled ON alert_thresholds(is_enabled);

-- Create unique constraint to prevent duplicate thresholds
CREATE UNIQUE INDEX idx_alert_thresholds_unique ON alert_thresholds(
    user_address, 
    COALESCE(position_id, '00000000-0000-0000-0000-000000000000'::UUID), 
    threshold_type
);

-- Add comments for documentation
COMMENT ON TABLE alert_thresholds IS 'User-configurable alert thresholds for risk monitoring';
COMMENT ON COLUMN alert_thresholds.user_address IS 'Ethereum address of the user who owns this threshold';
COMMENT ON COLUMN alert_thresholds.position_id IS 'Specific position ID (NULL means applies to all positions)';
COMMENT ON COLUMN alert_thresholds.threshold_type IS 'Type of risk metric to monitor';
COMMENT ON COLUMN alert_thresholds.operator IS 'Comparison operator for threshold check';
COMMENT ON COLUMN alert_thresholds.threshold_value IS 'Threshold value (typically as decimal percentage)';
COMMENT ON COLUMN alert_thresholds.is_enabled IS 'Whether this threshold is currently active';

-- Create trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_alert_thresholds_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_alert_thresholds_updated_at
    BEFORE UPDATE ON alert_thresholds
    FOR EACH ROW
    EXECUTE FUNCTION update_alert_thresholds_updated_at();
