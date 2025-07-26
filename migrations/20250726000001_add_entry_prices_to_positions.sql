-- Add entry price tracking fields to positions table for accurate IL calculations
-- Migration: Add entry price fields to positions

ALTER TABLE positions 
ADD COLUMN entry_token0_price_usd DECIMAL(36, 18),
ADD COLUMN entry_token1_price_usd DECIMAL(36, 18),
ADD COLUMN entry_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Create index for efficient querying by entry timestamp
CREATE INDEX idx_positions_entry_timestamp ON positions(entry_timestamp);

-- Create index for positions with entry prices (for IL calculations)
CREATE INDEX idx_positions_with_entry_prices ON positions(id) 
WHERE entry_token0_price_usd IS NOT NULL AND entry_token1_price_usd IS NOT NULL;

-- Add comment explaining the new fields
COMMENT ON COLUMN positions.entry_token0_price_usd IS 'USD price of token0 when position was created, for accurate IL calculations';
COMMENT ON COLUMN positions.entry_token1_price_usd IS 'USD price of token1 when position was created, for accurate IL calculations';
COMMENT ON COLUMN positions.entry_timestamp IS 'Timestamp when position was created and entry prices were recorded';
