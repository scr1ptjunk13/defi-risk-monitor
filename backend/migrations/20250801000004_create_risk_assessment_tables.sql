-- Create risk assessment enums
CREATE TYPE risk_entity_type AS ENUM (
    'position',
    'protocol', 
    'user',
    'portfolio',
    'pool',
    'token'
);

CREATE TYPE risk_type AS ENUM (
    'impermanent_loss',
    'liquidity',
    'protocol',
    'mev',
    'cross_chain',
    'market',
    'slippage',
    'correlation',
    'volatility',
    'overall'
);

CREATE TYPE risk_severity AS ENUM (
    'critical',
    'high',
    'medium',
    'low',
    'minimal'
);

-- Create risk assessments table
CREATE TABLE risk_assessments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type risk_entity_type NOT NULL,
    entity_id TEXT NOT NULL,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    risk_type risk_type NOT NULL,
    risk_score DECIMAL(5,4) NOT NULL CHECK (risk_score >= 0 AND risk_score <= 1),
    severity risk_severity NOT NULL,
    confidence DECIMAL(5,4) NOT NULL DEFAULT 1.0 CHECK (confidence >= 0 AND confidence <= 1),
    description TEXT,
    metadata JSONB,
    expires_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create risk assessment history table
CREATE TABLE risk_assessment_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    risk_assessment_id UUID NOT NULL REFERENCES risk_assessments(id) ON DELETE CASCADE,
    previous_risk_score DECIMAL(5,4) NOT NULL,
    new_risk_score DECIMAL(5,4) NOT NULL,
    previous_severity risk_severity NOT NULL,
    new_severity risk_severity NOT NULL,
    change_reason TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX idx_risk_assessments_entity ON risk_assessments(entity_type, entity_id);
CREATE INDEX idx_risk_assessments_user_id ON risk_assessments(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_risk_assessments_risk_type ON risk_assessments(risk_type);
CREATE INDEX idx_risk_assessments_severity ON risk_assessments(severity);
CREATE INDEX idx_risk_assessments_active ON risk_assessments(is_active) WHERE is_active = true;
CREATE INDEX idx_risk_assessments_expires_at ON risk_assessments(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_risk_assessments_created_at ON risk_assessments(created_at);
CREATE INDEX idx_risk_assessments_risk_score ON risk_assessments(risk_score);

-- Composite indexes for common queries
CREATE INDEX idx_risk_assessments_entity_user ON risk_assessments(entity_type, entity_id, user_id);
CREATE INDEX idx_risk_assessments_user_type ON risk_assessments(user_id, risk_type) WHERE user_id IS NOT NULL;
CREATE INDEX idx_risk_assessments_severity_active ON risk_assessments(severity, is_active);

-- Index for history table
CREATE INDEX idx_risk_assessment_history_assessment_id ON risk_assessment_history(risk_assessment_id);
CREATE INDEX idx_risk_assessment_history_created_at ON risk_assessment_history(created_at);

-- Create trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_risk_assessments_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_risk_assessments_updated_at
    BEFORE UPDATE ON risk_assessments
    FOR EACH ROW
    EXECUTE FUNCTION update_risk_assessments_updated_at();
