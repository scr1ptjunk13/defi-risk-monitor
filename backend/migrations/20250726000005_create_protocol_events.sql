-- Create enum types for protocol events
CREATE TYPE event_type AS ENUM (
    'exploit',
    'governance', 
    'audit',
    'upgrade',
    'emergency',
    'vulnerability',
    'regulatory',
    'partnership',
    'token_listing',
    'liquidity_change'
);

CREATE TYPE event_severity AS ENUM (
    'critical',
    'high',
    'medium',
    'low',
    'info'
);

-- Main protocol events table
CREATE TABLE protocol_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_name VARCHAR(100) NOT NULL,
    event_type event_type NOT NULL,
    severity event_severity NOT NULL,
    title VARCHAR(500) NOT NULL,
    description TEXT NOT NULL,
    source VARCHAR(100) NOT NULL,
    source_url TEXT,
    impact_score DECIMAL(10,2) NOT NULL DEFAULT 0,
    affected_chains INTEGER[] DEFAULT '{}',
    affected_tokens TEXT[] DEFAULT '{}',
    event_timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    detected_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    processed BOOLEAN NOT NULL DEFAULT FALSE,
    alert_sent BOOLEAN NOT NULL DEFAULT FALSE,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Exploit-specific events table
CREATE TABLE exploit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_event_id UUID NOT NULL REFERENCES protocol_events(id) ON DELETE CASCADE,
    exploit_type VARCHAR(100) NOT NULL,
    funds_lost_usd DECIMAL(20,2),
    attack_vector TEXT NOT NULL,
    root_cause TEXT NOT NULL,
    affected_contracts TEXT[] DEFAULT '{}',
    exploit_tx_hash VARCHAR(66),
    attacker_address VARCHAR(42),
    recovery_status VARCHAR(50) NOT NULL DEFAULT 'Unknown',
    post_mortem_url TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Governance events table
CREATE TABLE governance_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_event_id UUID NOT NULL REFERENCES protocol_events(id) ON DELETE CASCADE,
    proposal_id VARCHAR(100) NOT NULL,
    proposal_type VARCHAR(100) NOT NULL,
    voting_status VARCHAR(50) NOT NULL,
    voting_deadline TIMESTAMP WITH TIME ZONE,
    quorum_required DECIMAL(20,2),
    current_votes DECIMAL(20,2),
    proposal_url TEXT,
    risk_impact VARCHAR(100) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Audit events table
CREATE TABLE audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_event_id UUID NOT NULL REFERENCES protocol_events(id) ON DELETE CASCADE,
    auditor_name VARCHAR(100) NOT NULL,
    audit_type VARCHAR(50) NOT NULL,
    audit_status VARCHAR(50) NOT NULL,
    findings_count INTEGER NOT NULL DEFAULT 0,
    critical_findings INTEGER NOT NULL DEFAULT 0,
    high_findings INTEGER NOT NULL DEFAULT 0,
    medium_findings INTEGER NOT NULL DEFAULT 0,
    low_findings INTEGER NOT NULL DEFAULT 0,
    audit_report_url TEXT,
    completion_date TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Event alerts configuration table
CREATE TABLE event_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_address VARCHAR(42) NOT NULL,
    protocol_name VARCHAR(100) NOT NULL,
    event_types event_type[] NOT NULL,
    min_severity event_severity NOT NULL DEFAULT 'medium',
    notification_channels TEXT[] NOT NULL DEFAULT '{"email"}',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX idx_protocol_events_protocol_name ON protocol_events(protocol_name);
CREATE INDEX idx_protocol_events_event_type ON protocol_events(event_type);
CREATE INDEX idx_protocol_events_severity ON protocol_events(severity);
CREATE INDEX idx_protocol_events_timestamp ON protocol_events(event_timestamp DESC);
CREATE INDEX idx_protocol_events_detected_at ON protocol_events(detected_at DESC);
CREATE INDEX idx_protocol_events_processed ON protocol_events(processed) WHERE NOT processed;
CREATE INDEX idx_protocol_events_alert_sent ON protocol_events(alert_sent) WHERE NOT alert_sent;
CREATE INDEX idx_protocol_events_chains ON protocol_events USING GIN(affected_chains);
CREATE INDEX idx_protocol_events_tokens ON protocol_events USING GIN(affected_tokens);
CREATE INDEX idx_protocol_events_metadata ON protocol_events USING GIN(metadata);

CREATE INDEX idx_exploit_events_protocol_event ON exploit_events(protocol_event_id);
CREATE INDEX idx_exploit_events_funds_lost ON exploit_events(funds_lost_usd DESC) WHERE funds_lost_usd IS NOT NULL;
CREATE INDEX idx_exploit_events_exploit_type ON exploit_events(exploit_type);

CREATE INDEX idx_governance_events_protocol_event ON governance_events(protocol_event_id);
CREATE INDEX idx_governance_events_proposal_id ON governance_events(proposal_id);
CREATE INDEX idx_governance_events_voting_status ON governance_events(voting_status);
CREATE INDEX idx_governance_events_deadline ON governance_events(voting_deadline) WHERE voting_deadline IS NOT NULL;

CREATE INDEX idx_audit_events_protocol_event ON audit_events(protocol_event_id);
CREATE INDEX idx_audit_events_auditor ON audit_events(auditor_name);
CREATE INDEX idx_audit_events_findings ON audit_events(critical_findings DESC, high_findings DESC);

CREATE INDEX idx_event_alerts_user_address ON event_alerts(user_address);
CREATE INDEX idx_event_alerts_protocol ON event_alerts(protocol_name);
CREATE INDEX idx_event_alerts_enabled ON event_alerts(enabled) WHERE enabled;
CREATE INDEX idx_event_alerts_event_types ON event_alerts USING GIN(event_types);

-- Create trigger for updating updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_protocol_events_updated_at 
    BEFORE UPDATE ON protocol_events 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_alerts_updated_at 
    BEFORE UPDATE ON event_alerts 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default event alert configurations for common protocols
INSERT INTO event_alerts (user_address, protocol_name, event_types, min_severity, notification_channels) VALUES
('0x0000000000000000000000000000000000000000', 'uniswap_v3', '{"exploit","emergency","vulnerability"}', 'high', '{"email","slack"}'),
('0x0000000000000000000000000000000000000000', 'curve', '{"exploit","emergency","vulnerability"}', 'high', '{"email","slack"}'),
('0x0000000000000000000000000000000000000000', 'aave', '{"exploit","emergency","vulnerability"}', 'high', '{"email","slack"}');

-- Add comments for documentation
COMMENT ON TABLE protocol_events IS 'Main table for storing protocol events from external sources';
COMMENT ON TABLE exploit_events IS 'Detailed information about security exploits and hacks';
COMMENT ON TABLE governance_events IS 'Governance proposals and voting information';
COMMENT ON TABLE audit_events IS 'Security audit reports and findings';
COMMENT ON TABLE event_alerts IS 'User configuration for protocol event notifications';

COMMENT ON COLUMN protocol_events.impact_score IS 'Calculated impact score (0-200) based on severity and type';
COMMENT ON COLUMN protocol_events.affected_chains IS 'Array of chain IDs affected by this event';
COMMENT ON COLUMN protocol_events.affected_tokens IS 'Array of token addresses/symbols affected';
COMMENT ON COLUMN protocol_events.metadata IS 'Additional structured data specific to event type';

COMMENT ON COLUMN exploit_events.funds_lost_usd IS 'Estimated funds lost in USD (if applicable)';
COMMENT ON COLUMN exploit_events.recovery_status IS 'Status of fund recovery efforts';

COMMENT ON COLUMN governance_events.quorum_required IS 'Minimum votes required for proposal to pass';
COMMENT ON COLUMN governance_events.current_votes IS 'Current vote count for active proposals';

COMMENT ON COLUMN audit_events.findings_count IS 'Total number of findings in audit report';
COMMENT ON COLUMN audit_events.critical_findings IS 'Number of critical severity findings';
