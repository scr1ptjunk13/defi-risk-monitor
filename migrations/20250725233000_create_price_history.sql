-- Migration: Create price_history table for persistent price storage
CREATE TABLE IF NOT EXISTS price_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_address VARCHAR(64) NOT NULL,
    chain_id INTEGER NOT NULL,
    price_usd NUMERIC(78, 18) NOT NULL,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(token_address, chain_id, timestamp)
);

-- Index for fast historical queries
CREATE INDEX IF NOT EXISTS idx_price_history_token_chain_time
    ON price_history (token_address, chain_id, timestamp DESC);
