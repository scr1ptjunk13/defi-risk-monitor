-- Advanced Connection Pool Monitoring and Optimization
-- Migration: 20250801000007_advanced_connection_pool_monitoring.sql

-- Connection pool metrics table
CREATE TABLE IF NOT EXISTS connection_pool_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_name VARCHAR(100) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Pool configuration
    max_connections INTEGER NOT NULL,
    min_connections INTEGER NOT NULL,
    current_size INTEGER NOT NULL,
    idle_connections INTEGER NOT NULL,
    active_connections INTEGER NOT NULL,
    
    -- Performance metrics
    utilization_rate DECIMAL(5,4) NOT NULL, -- 0.0000 to 1.0000
    avg_acquire_time_ms BIGINT NOT NULL,
    pending_acquires INTEGER NOT NULL DEFAULT 0,
    total_acquires BIGINT NOT NULL DEFAULT 0,
    failed_acquires BIGINT NOT NULL DEFAULT 0,
    
    -- Connection lifecycle
    connections_created BIGINT NOT NULL DEFAULT 0,
    connections_closed BIGINT NOT NULL DEFAULT 0,
    connections_recycled BIGINT NOT NULL DEFAULT 0,
    
    -- Health metrics
    health_check_failures INTEGER NOT NULL DEFAULT 0,
    last_health_check TIMESTAMPTZ,
    avg_health_response_ms BIGINT,
    
    CONSTRAINT valid_utilization_rate CHECK (utilization_rate >= 0.0 AND utilization_rate <= 1.0),
    CONSTRAINT valid_connections CHECK (
        current_size >= 0 AND 
        idle_connections >= 0 AND 
        active_connections >= 0 AND
        idle_connections + active_connections <= current_size
    )
);

-- Connection health status table
CREATE TABLE IF NOT EXISTS connection_health_status (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_name VARCHAR(100) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Health status
    is_healthy BOOLEAN NOT NULL,
    health_score DECIMAL(3,2) NOT NULL DEFAULT 1.00, -- 0.00 to 1.00
    last_check TIMESTAMPTZ NOT NULL,
    response_time_ms BIGINT NOT NULL,
    
    -- Error tracking
    failed_checks INTEGER NOT NULL DEFAULT 0,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    total_queries BIGINT NOT NULL DEFAULT 0,
    error_rate DECIMAL(5,4) NOT NULL DEFAULT 0.0000,
    
    -- Connection validation
    validation_query_used VARCHAR(500),
    validation_success BOOLEAN NOT NULL DEFAULT true,
    
    CONSTRAINT valid_health_score CHECK (health_score >= 0.00 AND health_score <= 1.00),
    CONSTRAINT valid_error_rate CHECK (error_rate >= 0.0 AND error_rate <= 1.0)
);

-- Statement cache performance table
CREATE TABLE IF NOT EXISTS statement_cache_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_name VARCHAR(100) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Cache statistics
    cache_size INTEGER NOT NULL,
    cache_capacity INTEGER NOT NULL,
    hit_rate DECIMAL(5,4) NOT NULL DEFAULT 0.0000,
    
    -- Usage metrics
    total_hits BIGINT NOT NULL DEFAULT 0,
    total_misses BIGINT NOT NULL DEFAULT 0,
    total_evictions BIGINT NOT NULL DEFAULT 0,
    
    -- Performance impact
    avg_cache_lookup_ms DECIMAL(10,3) NOT NULL DEFAULT 0.000,
    cache_memory_usage_bytes BIGINT,
    
    CONSTRAINT valid_hit_rate CHECK (hit_rate >= 0.0 AND hit_rate <= 1.0),
    CONSTRAINT valid_cache_size CHECK (cache_size >= 0 AND cache_size <= cache_capacity)
);

-- Connection pool scaling events
CREATE TABLE IF NOT EXISTS pool_scaling_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_name VARCHAR(100) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Scaling details
    event_type VARCHAR(20) NOT NULL CHECK (event_type IN ('scale_up', 'scale_down', 'manual_resize')),
    old_max_connections INTEGER NOT NULL,
    new_max_connections INTEGER NOT NULL,
    old_min_connections INTEGER NOT NULL,
    new_min_connections INTEGER NOT NULL,
    
    -- Trigger information
    trigger_reason VARCHAR(200),
    utilization_before DECIMAL(5,4),
    utilization_after DECIMAL(5,4),
    load_threshold DECIMAL(5,4),
    
    -- Performance impact
    scaling_duration_ms BIGINT,
    connections_affected INTEGER NOT NULL DEFAULT 0,
    
    -- Metadata
    initiated_by VARCHAR(50) DEFAULT 'auto_scaler',
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT
);

-- Pool load test results
CREATE TABLE IF NOT EXISTS pool_load_test_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_name VARCHAR(100) NOT NULL,
    test_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Test configuration
    concurrent_requests INTEGER NOT NULL,
    test_duration_secs INTEGER NOT NULL,
    test_type VARCHAR(50) NOT NULL DEFAULT 'standard_load',
    
    -- Results
    total_requests BIGINT NOT NULL,
    total_errors BIGINT NOT NULL,
    error_rate DECIMAL(5,4) NOT NULL,
    avg_response_time_ms BIGINT NOT NULL,
    requests_per_second DECIMAL(10,2) NOT NULL,
    
    -- Pool state during test
    pool_max_connections INTEGER NOT NULL,
    pool_min_connections INTEGER NOT NULL,
    peak_utilization DECIMAL(5,4) NOT NULL,
    avg_utilization DECIMAL(5,4) NOT NULL,
    
    -- Recommendations
    recommended_max_connections INTEGER,
    recommended_min_connections INTEGER,
    performance_grade CHAR(1) CHECK (performance_grade IN ('A', 'B', 'C', 'D', 'F')),
    notes TEXT
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_connection_pool_metrics_timestamp ON connection_pool_metrics(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_connection_pool_metrics_pool_name ON connection_pool_metrics(pool_name);
CREATE INDEX IF NOT EXISTS idx_connection_pool_metrics_utilization ON connection_pool_metrics(utilization_rate DESC);

CREATE INDEX IF NOT EXISTS idx_connection_health_timestamp ON connection_health_status(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_connection_health_pool_name ON connection_health_status(pool_name);
CREATE INDEX IF NOT EXISTS idx_connection_health_status ON connection_health_status(is_healthy, health_score DESC);

CREATE INDEX IF NOT EXISTS idx_statement_cache_timestamp ON statement_cache_metrics(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_statement_cache_pool_name ON statement_cache_metrics(pool_name);
CREATE INDEX IF NOT EXISTS idx_statement_cache_hit_rate ON statement_cache_metrics(hit_rate DESC);

CREATE INDEX IF NOT EXISTS idx_pool_scaling_timestamp ON pool_scaling_events(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_pool_scaling_pool_name ON pool_scaling_events(pool_name);
CREATE INDEX IF NOT EXISTS idx_pool_scaling_event_type ON pool_scaling_events(event_type);

CREATE INDEX IF NOT EXISTS idx_load_test_timestamp ON pool_load_test_results(test_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_load_test_pool_name ON pool_load_test_results(pool_name);
CREATE INDEX IF NOT EXISTS idx_load_test_performance ON pool_load_test_results(performance_grade, requests_per_second DESC);

-- Views for monitoring dashboards
CREATE OR REPLACE VIEW pool_health_summary AS
SELECT 
    pool_name,
    is_healthy,
    health_score,
    response_time_ms,
    error_rate,
    last_check,
    ROW_NUMBER() OVER (PARTITION BY pool_name ORDER BY timestamp DESC) as rn
FROM connection_health_status
WHERE timestamp >= NOW() - INTERVAL '1 hour';

CREATE OR REPLACE VIEW pool_performance_summary AS
SELECT 
    pool_name,
    utilization_rate,
    avg_acquire_time_ms,
    failed_acquires,
    total_acquires,
    CASE 
        WHEN utilization_rate > 0.9 THEN 'HIGH'
        WHEN utilization_rate > 0.7 THEN 'MEDIUM'
        ELSE 'LOW'
    END as load_level,
    timestamp,
    ROW_NUMBER() OVER (PARTITION BY pool_name ORDER BY timestamp DESC) as rn
FROM connection_pool_metrics
WHERE timestamp >= NOW() - INTERVAL '1 hour';

CREATE OR REPLACE VIEW statement_cache_performance AS
SELECT 
    pool_name,
    hit_rate,
    cache_size,
    cache_capacity,
    ROUND((cache_size::DECIMAL / cache_capacity::DECIMAL) * 100, 2) as cache_utilization_pct,
    total_hits + total_misses as total_requests,
    timestamp,
    ROW_NUMBER() OVER (PARTITION BY pool_name ORDER BY timestamp DESC) as rn
FROM statement_cache_metrics
WHERE timestamp >= NOW() - INTERVAL '1 hour';

COMMENT ON TABLE connection_pool_metrics IS 'Real-time connection pool performance metrics';
COMMENT ON TABLE connection_health_status IS 'Connection pool health monitoring and status tracking';
COMMENT ON TABLE statement_cache_metrics IS 'Statement cache performance and hit rate metrics';
COMMENT ON TABLE pool_scaling_events IS 'Connection pool scaling events and decisions';
COMMENT ON TABLE pool_load_test_results IS 'Load testing results for pool optimization';
