-- Create protocol audits table
CREATE TABLE protocol_audits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_name VARCHAR NOT NULL,
    auditor_name VARCHAR NOT NULL,
    audit_date TIMESTAMPTZ NOT NULL,
    audit_score DECIMAL(5,2) NOT NULL CHECK (audit_score >= 0 AND audit_score <= 100),
    critical_issues INTEGER NOT NULL DEFAULT 0,
    high_issues INTEGER NOT NULL DEFAULT 0,
    medium_issues INTEGER NOT NULL DEFAULT 0,
    low_issues INTEGER NOT NULL DEFAULT 0,
    audit_report_url VARCHAR,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create protocol exploits table
CREATE TABLE protocol_exploits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_name VARCHAR NOT NULL,
    exploit_date TIMESTAMPTZ NOT NULL,
    exploit_type VARCHAR NOT NULL CHECK (exploit_type IN ('FlashLoan', 'Reentrancy', 'Oracle', 'Governance', 'Bridge', 'Other')),
    amount_lost_usd DECIMAL(20,2) NOT NULL DEFAULT 0,
    severity VARCHAR NOT NULL CHECK (severity IN ('Critical', 'High', 'Medium', 'Low')),
    description TEXT,
    was_recovered BOOLEAN NOT NULL DEFAULT false,
    recovery_amount_usd DECIMAL(20,2) DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create protocol metrics table
CREATE TABLE protocol_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_name VARCHAR NOT NULL,
    total_tvl_usd DECIMAL(20,2) NOT NULL DEFAULT 0,
    tvl_change_24h DECIMAL(10,4) NOT NULL DEFAULT 0, -- Percentage change
    tvl_change_7d DECIMAL(10,4) NOT NULL DEFAULT 0,  -- Percentage change
    multisig_threshold INTEGER,
    timelock_delay_hours INTEGER,
    governance_participation_rate DECIMAL(5,2), -- Percentage
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create protocol risks table (stores calculated risk assessments)
CREATE TABLE protocol_risks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    protocol_name VARCHAR NOT NULL,
    protocol_address VARCHAR NOT NULL,
    chain_id INTEGER NOT NULL,
    audit_score DECIMAL(10,8) NOT NULL DEFAULT 0,
    exploit_history_score DECIMAL(10,8) NOT NULL DEFAULT 0,
    tvl_score DECIMAL(10,8) NOT NULL DEFAULT 0,
    governance_score DECIMAL(10,8) NOT NULL DEFAULT 0,
    code_quality_score DECIMAL(10,8) NOT NULL DEFAULT 0,
    overall_protocol_risk DECIMAL(10,8) NOT NULL DEFAULT 0,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(protocol_name, chain_id)
);

-- Create indexes for better query performance
CREATE INDEX idx_protocol_audits_protocol_date ON protocol_audits(protocol_name, audit_date DESC);
CREATE INDEX idx_protocol_audits_active ON protocol_audits(protocol_name, is_active);
CREATE INDEX idx_protocol_exploits_protocol_date ON protocol_exploits(protocol_name, exploit_date DESC);
CREATE INDEX idx_protocol_exploits_severity ON protocol_exploits(protocol_name, severity);
CREATE INDEX idx_protocol_metrics_protocol_timestamp ON protocol_metrics(protocol_name, timestamp DESC);
CREATE INDEX idx_protocol_risks_protocol_chain ON protocol_risks(protocol_name, chain_id);
CREATE INDEX idx_protocol_risks_updated ON protocol_risks(last_updated DESC);

-- Add updated_at trigger for protocol_audits
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_protocol_audits_updated_at BEFORE UPDATE ON protocol_audits
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_protocol_exploits_updated_at BEFORE UPDATE ON protocol_exploits
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_protocol_metrics_updated_at BEFORE UPDATE ON protocol_metrics
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
