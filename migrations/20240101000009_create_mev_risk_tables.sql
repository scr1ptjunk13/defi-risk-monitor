-- Create MEV risk assessment tables
-- Migration: 20240101000009_create_mev_risk_tables

-- Create enum types for MEV risk classification
CREATE TYPE mev_type AS ENUM (
    'sandwich_attack',
    'frontrunning', 
    'backrunning',
    'arbitrage',
    'liquidation',
    'unknown'
);

CREATE TYPE mev_severity AS ENUM (
    'low',
    'medium', 
    'high',
    'critical'
);

CREATE TYPE oracle_deviation_severity AS ENUM (
    'minor',
    'moderate',
    'significant', 
    'critical'
);

-- MEV risk assessments table
CREATE TABLE mev_risks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_address VARCHAR(42) NOT NULL,
    chain_id INTEGER NOT NULL,
    sandwich_risk_score DECIMAL(5,4) NOT NULL CHECK (sandwich_risk_score >= 0 AND sandwich_risk_score <= 1),
    frontrun_risk_score DECIMAL(5,4) NOT NULL CHECK (frontrun_risk_score >= 0 AND frontrun_risk_score <= 1),
    oracle_manipulation_risk DECIMAL(5,4) NOT NULL CHECK (oracle_manipulation_risk >= 0 AND oracle_manipulation_risk <= 1),
    oracle_deviation_risk DECIMAL(5,4) NOT NULL CHECK (oracle_deviation_risk >= 0 AND oracle_deviation_risk <= 1),
    overall_mev_risk DECIMAL(5,4) NOT NULL CHECK (overall_mev_risk >= 0 AND overall_mev_risk <= 1),
    confidence_score DECIMAL(5,4) NOT NULL CHECK (confidence_score >= 0 AND confidence_score <= 1),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- MEV transaction detection results
CREATE TABLE mev_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_hash VARCHAR(66) NOT NULL UNIQUE,
    block_number BIGINT NOT NULL,
    chain_id INTEGER NOT NULL,
    mev_type mev_type NOT NULL,
    severity mev_severity NOT NULL,
    profit_usd DECIMAL(20,8),
    victim_loss_usd DECIMAL(20,8),
    pool_address VARCHAR(42) NOT NULL,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Oracle price deviation events
CREATE TABLE oracle_deviations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    oracle_address VARCHAR(42) NOT NULL,
    token_address VARCHAR(42) NOT NULL,
    chain_id INTEGER NOT NULL,
    oracle_price DECIMAL(30,18) NOT NULL,
    market_price DECIMAL(30,18) NOT NULL,
    deviation_percent DECIMAL(8,4) NOT NULL,
    severity oracle_deviation_severity NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX idx_mev_risks_pool_chain ON mev_risks(pool_address, chain_id);
CREATE INDEX idx_mev_risks_created_at ON mev_risks(created_at);
CREATE INDEX idx_mev_risks_overall_risk ON mev_risks(overall_mev_risk);

CREATE INDEX idx_mev_transactions_pool_chain ON mev_transactions(pool_address, chain_id);
CREATE INDEX idx_mev_transactions_block ON mev_transactions(block_number);
CREATE INDEX idx_mev_transactions_detected_at ON mev_transactions(detected_at);
CREATE INDEX idx_mev_transactions_type_severity ON mev_transactions(mev_type, severity);

CREATE INDEX idx_oracle_deviations_token_chain ON oracle_deviations(token_address, chain_id);
CREATE INDEX idx_oracle_deviations_oracle ON oracle_deviations(oracle_address);
CREATE INDEX idx_oracle_deviations_timestamp ON oracle_deviations(timestamp);
CREATE INDEX idx_oracle_deviations_severity ON oracle_deviations(severity);

-- Create trigger for updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_mev_risks_updated_at 
    BEFORE UPDATE ON mev_risks 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

-- Add comments for documentation
COMMENT ON TABLE mev_risks IS 'MEV risk assessments for liquidity pools';
COMMENT ON TABLE mev_transactions IS 'Detected MEV transactions and their impact';
COMMENT ON TABLE oracle_deviations IS 'Oracle price deviation events';

COMMENT ON COLUMN mev_risks.sandwich_risk_score IS 'Risk score for sandwich attacks (0-1)';
COMMENT ON COLUMN mev_risks.frontrun_risk_score IS 'Risk score for frontrunning attacks (0-1)';
COMMENT ON COLUMN mev_risks.oracle_manipulation_risk IS 'Risk score for oracle manipulation (0-1)';
COMMENT ON COLUMN mev_risks.oracle_deviation_risk IS 'Risk score for oracle price deviations (0-1)';
COMMENT ON COLUMN mev_risks.overall_mev_risk IS 'Overall MEV risk score (0-1)';
COMMENT ON COLUMN mev_risks.confidence_score IS 'Confidence in the risk assessment (0-1)';

COMMENT ON COLUMN mev_transactions.profit_usd IS 'Estimated MEV profit in USD';
COMMENT ON COLUMN mev_transactions.victim_loss_usd IS 'Estimated victim loss in USD';

COMMENT ON COLUMN oracle_deviations.deviation_percent IS 'Percentage deviation from market price';
