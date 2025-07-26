-- Audit and compliance database schema
-- Migration: 002_audit_schema.sql

-- Create custom types for audit events
CREATE TYPE audit_event_type AS ENUM (
    'risk_calculation',
    'alert_triggered',
    'alert_resolved',
    'position_created',
    'position_updated',
    'position_closed',
    'price_validation',
    'system_startup',
    'system_shutdown',
    'configuration_change',
    'user_action',
    'api_call',
    'database_query',
    'external_api_call',
    'cache_operation',
    'security_event'
);

CREATE TYPE audit_severity AS ENUM (
    'info',
    'warning',
    'error',
    'critical'
);

-- Main audit logs table
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type audit_event_type NOT NULL,
    severity audit_severity NOT NULL DEFAULT 'info',
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id TEXT,
    session_id TEXT,
    ip_address INET,
    user_agent TEXT,
    resource_type TEXT,
    resource_id TEXT,
    action TEXT NOT NULL,
    description TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    before_state JSONB,
    after_state JSONB,
    risk_impact DECIMAL(20,8),
    financial_impact DECIMAL(20,8),
    compliance_tags JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance and compliance queries
CREATE INDEX idx_audit_logs_timestamp ON audit_logs (timestamp DESC);
CREATE INDEX idx_audit_logs_event_type ON audit_logs (event_type);
CREATE INDEX idx_audit_logs_severity ON audit_logs (severity);
CREATE INDEX idx_audit_logs_user_id ON audit_logs (user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_audit_logs_session_id ON audit_logs (session_id) WHERE session_id IS NOT NULL;
CREATE INDEX idx_audit_logs_resource ON audit_logs (resource_type, resource_id) WHERE resource_type IS NOT NULL;
CREATE INDEX idx_audit_logs_compliance_tags ON audit_logs USING GIN (compliance_tags);
CREATE INDEX idx_audit_logs_metadata ON audit_logs USING GIN (metadata);
CREATE INDEX idx_audit_logs_financial_impact ON audit_logs (financial_impact) WHERE financial_impact IS NOT NULL;

-- Composite indexes for common compliance queries
CREATE INDEX idx_audit_logs_period_severity ON audit_logs (timestamp DESC, severity);
CREATE INDEX idx_audit_logs_user_period ON audit_logs (user_id, timestamp DESC) WHERE user_id IS NOT NULL;
CREATE INDEX idx_audit_logs_resource_period ON audit_logs (resource_type, resource_id, timestamp DESC) WHERE resource_type IS NOT NULL;

-- Compliance reports table for storing generated reports
CREATE TABLE compliance_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_type TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    total_events BIGINT NOT NULL DEFAULT 0,
    critical_events BIGINT NOT NULL DEFAULT 0,
    error_events BIGINT NOT NULL DEFAULT 0,
    warning_events BIGINT NOT NULL DEFAULT 0,
    info_events BIGINT NOT NULL DEFAULT 0,
    total_financial_impact DECIMAL(20,8) DEFAULT 0,
    total_risk_impact DECIMAL(20,8) DEFAULT 0,
    unique_users BIGINT NOT NULL DEFAULT 0,
    unique_sessions BIGINT NOT NULL DEFAULT 0,
    compliance_violations BIGINT NOT NULL DEFAULT 0,
    summary TEXT,
    recommendations JSONB DEFAULT '[]',
    report_data JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for compliance reports
CREATE INDEX idx_compliance_reports_generated_at ON compliance_reports (generated_at DESC);
CREATE INDEX idx_compliance_reports_period ON compliance_reports (period_start, period_end);
CREATE INDEX idx_compliance_reports_type ON compliance_reports (report_type);

-- Audit configuration table for retention policies and settings
CREATE TABLE audit_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    config_key TEXT UNIQUE NOT NULL,
    config_value JSONB NOT NULL,
    description TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by TEXT
);

-- Insert default audit configuration
INSERT INTO audit_config (config_key, config_value, description) VALUES
('retention_days', '2555', 'Audit log retention period in days (7 years for financial compliance)'),
('auto_cleanup_enabled', 'true', 'Enable automatic cleanup of old audit logs'),
('compliance_tags_required', '["risk_management", "trading_activity"]', 'Required compliance tags for financial operations'),
('critical_event_notification', 'true', 'Send notifications for critical audit events'),
('report_generation_schedule', '{"daily": true, "weekly": true, "monthly": true}', 'Automated compliance report generation schedule');

-- Function to automatically tag financial operations
CREATE OR REPLACE FUNCTION auto_tag_financial_operations()
RETURNS TRIGGER AS $$
BEGIN
    -- Automatically add compliance tags for financial operations
    IF NEW.event_type IN ('risk_calculation', 'position_created', 'position_updated', 'position_closed', 'alert_triggered') THEN
        NEW.compliance_tags = NEW.compliance_tags || '["financial_operation", "regulatory_compliance"]'::jsonb;
    END IF;
    
    -- Add high-value transaction tags
    IF NEW.financial_impact IS NOT NULL AND NEW.financial_impact > 100000 THEN
        NEW.compliance_tags = NEW.compliance_tags || '["high_value_transaction"]'::jsonb;
    END IF;
    
    -- Add critical risk tags
    IF NEW.risk_impact IS NOT NULL AND NEW.risk_impact > 0.8 THEN
        NEW.compliance_tags = NEW.compliance_tags || '["high_risk_operation"]'::jsonb;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for automatic compliance tagging
CREATE TRIGGER trigger_auto_tag_financial_operations
    BEFORE INSERT ON audit_logs
    FOR EACH ROW
    EXECUTE FUNCTION auto_tag_financial_operations();

-- Function to clean up old audit logs
CREATE OR REPLACE FUNCTION cleanup_old_audit_logs(retention_days INTEGER DEFAULT 2555)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM audit_logs 
    WHERE timestamp < NOW() - INTERVAL '1 day' * retention_days;
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    -- Log the cleanup operation
    INSERT INTO audit_logs (event_type, severity, action, description, metadata)
    VALUES (
        'system_startup',
        'info',
        'cleanup_audit_logs',
        'Automated cleanup of old audit logs',
        jsonb_build_object('deleted_count', deleted_count, 'retention_days', retention_days)
    );
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- View for compliance dashboard
CREATE VIEW compliance_dashboard AS
SELECT 
    DATE_TRUNC('day', timestamp) as date,
    event_type,
    severity,
    COUNT(*) as event_count,
    COUNT(DISTINCT user_id) as unique_users,
    COUNT(DISTINCT session_id) as unique_sessions,
    SUM(CASE WHEN financial_impact IS NOT NULL THEN financial_impact ELSE 0 END) as total_financial_impact,
    AVG(CASE WHEN risk_impact IS NOT NULL THEN risk_impact ELSE 0 END) as avg_risk_impact
FROM audit_logs
WHERE timestamp >= NOW() - INTERVAL '30 days'
GROUP BY DATE_TRUNC('day', timestamp), event_type, severity
ORDER BY date DESC, event_count DESC;

-- View for high-risk operations
CREATE VIEW high_risk_operations AS
SELECT 
    id,
    event_type,
    severity,
    timestamp,
    user_id,
    resource_type,
    resource_id,
    action,
    description,
    risk_impact,
    financial_impact,
    compliance_tags
FROM audit_logs
WHERE 
    severity IN ('critical', 'error')
    OR risk_impact > 0.7
    OR financial_impact > 50000
    OR compliance_tags ? 'high_risk_operation'
ORDER BY timestamp DESC;

-- Comments for documentation
COMMENT ON TABLE audit_logs IS 'Comprehensive audit trail for all system operations and user actions';
COMMENT ON TABLE compliance_reports IS 'Generated compliance reports for regulatory and internal audit purposes';
COMMENT ON TABLE audit_config IS 'Configuration settings for audit and compliance system';
COMMENT ON VIEW compliance_dashboard IS 'Real-time compliance metrics and statistics';
COMMENT ON VIEW high_risk_operations IS 'High-risk operations requiring special attention';

-- Grant appropriate permissions (adjust as needed for your security model)
-- GRANT SELECT, INSERT ON audit_logs TO audit_service_role;
-- GRANT SELECT ON compliance_dashboard TO compliance_officer_role;
-- GRANT ALL ON compliance_reports TO compliance_officer_role;
