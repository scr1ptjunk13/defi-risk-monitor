-- Add missing liquidity_depth column to chain_risks table

ALTER TABLE chain_risks 
ADD COLUMN IF NOT EXISTS liquidity_depth DECIMAL(10,8) NOT NULL DEFAULT 0.5 
CHECK (liquidity_depth >= 0 AND liquidity_depth <= 1);

COMMENT ON COLUMN chain_risks.liquidity_depth IS 'Liquidity depth score for the chain (0-1, higher is better)';
