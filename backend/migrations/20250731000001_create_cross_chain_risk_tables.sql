-- Create cross-chain risk assessment tables

-- Cross-chain risk assessments
CREATE TABLE IF NOT EXISTS cross_chain_risks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    position_id UUID REFERENCES positions(id) ON DELETE CASCADE,
    primary_chain_id INTEGER NOT NULL,
    secondary_chain_ids INTEGER[] NOT NULL,
    bridge_risk_score DECIMAL(10,8) NOT NULL CHECK (bridge_risk_score >= 0 AND bridge_risk_score <= 1),
    liquidity_fragmentation_risk DECIMAL(10,8) NOT NULL CHECK (liquidity_fragmentation_risk >= 0 AND liquidity_fragmentation_risk <= 1),
    governance_divergence_risk DECIMAL(10,8) NOT NULL CHECK (governance_divergence_risk >= 0 AND governance_divergence_risk <= 1),
    technical_risk_score DECIMAL(10,8) NOT NULL CHECK (technical_risk_score >= 0 AND technical_risk_score <= 1),
    correlation_risk_score DECIMAL(10,8) NOT NULL CHECK (correlation_risk_score >= 0 AND correlation_risk_score <= 1),
    overall_cross_chain_risk DECIMAL(10,8) NOT NULL CHECK (overall_cross_chain_risk >= 0 AND overall_cross_chain_risk <= 1),
    confidence_score DECIMAL(10,8) NOT NULL CHECK (confidence_score >= 0 AND confidence_score <= 1),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Bridge risk assessments
CREATE TABLE IF NOT EXISTS bridge_risks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bridge_protocol VARCHAR(100) NOT NULL,
    source_chain_id INTEGER NOT NULL,
    destination_chain_id INTEGER NOT NULL,
    security_score DECIMAL(10,8) NOT NULL CHECK (security_score >= 0 AND security_score <= 1),
    tvl_locked DECIMAL(30,18),
    exploit_history_count INTEGER NOT NULL DEFAULT 0,
    audit_score DECIMAL(10,8) NOT NULL CHECK (audit_score >= 0 AND audit_score <= 1),
    decentralization_score DECIMAL(10,8) NOT NULL CHECK (decentralization_score >= 0 AND decentralization_score <= 1),
    overall_bridge_risk DECIMAL(10,8) NOT NULL CHECK (overall_bridge_risk >= 0 AND overall_bridge_risk <= 1),
    last_assessment TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Chain-specific risk metrics
CREATE TABLE IF NOT EXISTS chain_risks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain_id INTEGER NOT NULL UNIQUE,
    chain_name VARCHAR(100) NOT NULL,
    network_security_score DECIMAL(10,8) NOT NULL CHECK (network_security_score >= 0 AND network_security_score <= 1),
    validator_decentralization DECIMAL(10,8) NOT NULL CHECK (validator_decentralization >= 0 AND validator_decentralization <= 1),
    governance_risk DECIMAL(10,8) NOT NULL CHECK (governance_risk >= 0 AND governance_risk <= 1),
    technical_maturity DECIMAL(10,8) NOT NULL CHECK (technical_maturity >= 0 AND technical_maturity <= 1),
    ecosystem_health DECIMAL(10,8) NOT NULL CHECK (ecosystem_health >= 0 AND ecosystem_health <= 1),
    liquidity_depth DECIMAL(10,8) NOT NULL CHECK (liquidity_depth >= 0 AND liquidity_depth <= 1),
    overall_chain_risk DECIMAL(10,8) NOT NULL CHECK (overall_chain_risk >= 0 AND overall_chain_risk <= 1),
    last_updated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Cross-chain correlation metrics
CREATE TABLE IF NOT EXISTS chain_correlations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain_id_1 INTEGER NOT NULL,
    chain_id_2 INTEGER NOT NULL,
    price_correlation DECIMAL(10,8) NOT NULL CHECK (price_correlation >= -1 AND price_correlation <= 1),
    volume_correlation DECIMAL(10,8) NOT NULL CHECK (volume_correlation >= -1 AND volume_correlation <= 1),
    governance_correlation DECIMAL(10,8) NOT NULL CHECK (governance_correlation >= -1 AND governance_correlation <= 1),
    technical_correlation DECIMAL(10,8) NOT NULL CHECK (technical_correlation >= -1 AND technical_correlation <= 1),
    overall_correlation DECIMAL(10,8) NOT NULL CHECK (overall_correlation >= -1 AND overall_correlation <= 1),
    calculation_period_days INTEGER NOT NULL DEFAULT 30,
    last_calculated TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(chain_id_1, chain_id_2, calculation_period_days),
    CHECK (chain_id_1 != chain_id_2)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_cross_chain_risks_position_id ON cross_chain_risks(position_id);
CREATE INDEX IF NOT EXISTS idx_cross_chain_risks_primary_chain ON cross_chain_risks(primary_chain_id);
CREATE INDEX IF NOT EXISTS idx_cross_chain_risks_created_at ON cross_chain_risks(created_at);

CREATE INDEX IF NOT EXISTS idx_bridge_risks_protocol ON bridge_risks(bridge_protocol);
CREATE INDEX IF NOT EXISTS idx_bridge_risks_chains ON bridge_risks(source_chain_id, destination_chain_id);
CREATE INDEX IF NOT EXISTS idx_bridge_risks_last_assessment ON bridge_risks(last_assessment);

CREATE INDEX IF NOT EXISTS idx_chain_risks_chain_id ON chain_risks(chain_id);
CREATE INDEX IF NOT EXISTS idx_chain_risks_last_updated ON chain_risks(last_updated);

CREATE INDEX IF NOT EXISTS idx_chain_correlations_chains ON chain_correlations(chain_id_1, chain_id_2);
CREATE INDEX IF NOT EXISTS idx_chain_correlations_last_calculated ON chain_correlations(last_calculated);

-- Comments for documentation
COMMENT ON TABLE cross_chain_risks IS 'Cross-chain risk assessments for multi-chain DeFi positions';
COMMENT ON TABLE bridge_risks IS 'Bridge security assessments for cross-chain protocols';
COMMENT ON TABLE chain_risks IS 'Chain-specific risk metrics and scores';
COMMENT ON TABLE chain_correlations IS 'Cross-chain correlation metrics between different chains';

COMMENT ON COLUMN cross_chain_risks.secondary_chain_ids IS 'Array of secondary chain IDs involved in the cross-chain position';
COMMENT ON COLUMN bridge_risks.tvl_locked IS 'Total value locked in the bridge protocol';
COMMENT ON COLUMN chain_correlations.calculation_period_days IS 'Number of days used for correlation calculation';
