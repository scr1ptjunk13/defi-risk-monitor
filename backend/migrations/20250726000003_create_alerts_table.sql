-- Create alerts table for storing triggered alerts
CREATE TABLE IF NOT EXISTS alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    position_id UUID REFERENCES positions(id) ON DELETE CASCADE,
    alert_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    risk_score DECIMAL(10,6),
    current_value DECIMAL(10,6),
    threshold_value DECIMAL(10,6),
    is_resolved BOOLEAN NOT NULL DEFAULT false,
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_alerts_position_id ON alerts(position_id);
CREATE INDEX IF NOT EXISTS idx_alerts_severity ON alerts(severity);
CREATE INDEX IF NOT EXISTS idx_alerts_alert_type ON alerts(alert_type);
CREATE INDEX IF NOT EXISTS idx_alerts_is_resolved ON alerts(is_resolved);
CREATE INDEX IF NOT EXISTS idx_alerts_created_at ON alerts(created_at);

-- Add comments for documentation
COMMENT ON TABLE alerts IS 'Triggered alerts for risk threshold violations';
COMMENT ON COLUMN alerts.position_id IS 'Position that triggered the alert (NULL for system-wide alerts)';
COMMENT ON COLUMN alerts.alert_type IS 'Type of alert (ImpermanentLoss, TvlDrop, etc.)';
COMMENT ON COLUMN alerts.severity IS 'Alert severity level';
COMMENT ON COLUMN alerts.risk_score IS 'Overall risk score when alert was triggered';
COMMENT ON COLUMN alerts.current_value IS 'Current value that exceeded threshold';
COMMENT ON COLUMN alerts.threshold_value IS 'Threshold value that was exceeded';
COMMENT ON COLUMN alerts.is_resolved IS 'Whether the alert has been acknowledged/resolved';
COMMENT ON COLUMN alerts.resolved_at IS 'Timestamp when alert was resolved';
