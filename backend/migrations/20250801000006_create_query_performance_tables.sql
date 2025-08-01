-- Query Performance Monitoring Migration
-- Migration: 20250801000006_create_query_performance_tables.sql

-- Create query performance tracking table
CREATE TABLE IF NOT EXISTS query_performance_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_hash VARCHAR(64) NOT NULL,
    query_type VARCHAR(50) NOT NULL,
    execution_time_ms BIGINT NOT NULL,
    rows_examined BIGINT DEFAULT 0,
    rows_returned BIGINT DEFAULT 0,
    index_scans INTEGER DEFAULT 0,
    seq_scans INTEGER DEFAULT 0,
    nested_loops INTEGER DEFAULT 0,
    hash_joins INTEGER DEFAULT 0,
    sort_operations INTEGER DEFAULT 0,
    total_cost DECIMAL(12,4) DEFAULT 0,
    startup_cost DECIMAL(12,4) DEFAULT 0,
    execution_plan JSONB,
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for query performance log
CREATE INDEX IF NOT EXISTS idx_query_performance_log_hash ON query_performance_log(query_hash);
CREATE INDEX IF NOT EXISTS idx_query_performance_log_type ON query_performance_log(query_type);
CREATE INDEX IF NOT EXISTS idx_query_performance_log_time ON query_performance_log(execution_time_ms DESC);
CREATE INDEX IF NOT EXISTS idx_query_performance_log_created_at ON query_performance_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_query_performance_log_slow_queries ON query_performance_log(execution_time_ms DESC) 
    WHERE execution_time_ms > 1000; -- Index for queries slower than 1 second

-- Create query plan cache table
CREATE TABLE IF NOT EXISTS query_plan_cache (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_hash VARCHAR(64) UNIQUE NOT NULL,
    plan_hash VARCHAR(64) NOT NULL,
    execution_plan JSONB NOT NULL,
    estimated_cost DECIMAL(12,4) NOT NULL,
    estimated_rows BIGINT NOT NULL,
    usage_count BIGINT DEFAULT 1,
    avg_actual_time DECIMAL(10,4) DEFAULT 0,
    last_used TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for query plan cache
CREATE INDEX IF NOT EXISTS idx_query_plan_cache_hash ON query_plan_cache(query_hash);
CREATE INDEX IF NOT EXISTS idx_query_plan_cache_usage ON query_plan_cache(usage_count DESC);
CREATE INDEX IF NOT EXISTS idx_query_plan_cache_last_used ON query_plan_cache(last_used DESC);

-- Create materialized view refresh log
CREATE TABLE IF NOT EXISTS mv_refresh_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    view_name VARCHAR(100) NOT NULL,
    refresh_type VARCHAR(20) NOT NULL, -- 'full', 'incremental', 'concurrent'
    duration_ms BIGINT NOT NULL,
    rows_affected BIGINT DEFAULT 0,
    success BOOLEAN NOT NULL DEFAULT true,
    error_message TEXT,
    triggered_by VARCHAR(50) DEFAULT 'system', -- 'system', 'manual', 'schedule'
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for materialized view refresh log
CREATE INDEX IF NOT EXISTS idx_mv_refresh_log_view_name ON mv_refresh_log(view_name);
CREATE INDEX IF NOT EXISTS idx_mv_refresh_log_started_at ON mv_refresh_log(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_mv_refresh_log_duration ON mv_refresh_log(duration_ms DESC);
CREATE INDEX IF NOT EXISTS idx_mv_refresh_log_success ON mv_refresh_log(success, started_at DESC);

-- Create slow query alerts table
CREATE TABLE IF NOT EXISTS slow_query_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_hash VARCHAR(64) NOT NULL,
    query_type VARCHAR(50) NOT NULL,
    execution_time_ms BIGINT NOT NULL,
    threshold_ms BIGINT NOT NULL,
    alert_level VARCHAR(20) NOT NULL, -- 'warning', 'critical'
    recommendations TEXT[],
    index_recommendations JSONB,
    acknowledged BOOLEAN DEFAULT false,
    acknowledged_by VARCHAR(100),
    acknowledged_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for slow query alerts
CREATE INDEX IF NOT EXISTS idx_slow_query_alerts_hash ON slow_query_alerts(query_hash);
CREATE INDEX IF NOT EXISTS idx_slow_query_alerts_level ON slow_query_alerts(alert_level);
CREATE INDEX IF NOT EXISTS idx_slow_query_alerts_acknowledged ON slow_query_alerts(acknowledged, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_slow_query_alerts_created_at ON slow_query_alerts(created_at DESC);

-- Create database performance metrics table
CREATE TABLE IF NOT EXISTS db_performance_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_name VARCHAR(100) NOT NULL,
    metric_value DECIMAL(15,4) NOT NULL,
    metric_unit VARCHAR(20),
    tags JSONB DEFAULT '{}',
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for database performance metrics
CREATE INDEX IF NOT EXISTS idx_db_performance_metrics_name ON db_performance_metrics(metric_name);
CREATE INDEX IF NOT EXISTS idx_db_performance_metrics_recorded_at ON db_performance_metrics(recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_db_performance_metrics_name_time ON db_performance_metrics(metric_name, recorded_at DESC);

-- Create function to automatically log slow queries
CREATE OR REPLACE FUNCTION log_slow_query()
RETURNS event_trigger AS $$
BEGIN
    -- This would be implemented to automatically capture slow queries
    -- For now, it's a placeholder for future enhancement
    NULL;
END;
$$ LANGUAGE plpgsql;

-- Create function to clean up old performance data
CREATE OR REPLACE FUNCTION cleanup_performance_data(retention_days INTEGER DEFAULT 30)
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER := 0;
BEGIN
    -- Clean up old query performance logs
    DELETE FROM query_performance_log 
    WHERE created_at < NOW() - INTERVAL '1 day' * retention_days;
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    
    -- Clean up old materialized view refresh logs
    DELETE FROM mv_refresh_log 
    WHERE started_at < NOW() - INTERVAL '1 day' * retention_days;
    
    -- Clean up acknowledged slow query alerts older than retention period
    DELETE FROM slow_query_alerts 
    WHERE acknowledged = true AND acknowledged_at < NOW() - INTERVAL '1 day' * retention_days;
    
    -- Clean up old database performance metrics
    DELETE FROM db_performance_metrics 
    WHERE recorded_at < NOW() - INTERVAL '1 day' * retention_days;
    
    -- Clean up unused query plans (not used in last 7 days)
    DELETE FROM query_plan_cache 
    WHERE last_used < NOW() - INTERVAL '7 days';
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Create materialized view for query performance analytics
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_query_performance_analytics AS
SELECT 
    query_type,
    COUNT(*) as total_queries,
    AVG(execution_time_ms) as avg_execution_time_ms,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY execution_time_ms) as median_execution_time_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY execution_time_ms) as p95_execution_time_ms,
    MAX(execution_time_ms) as max_execution_time_ms,
    COUNT(CASE WHEN execution_time_ms > 1000 THEN 1 END) as slow_queries_count,
    COUNT(CASE WHEN success = false THEN 1 END) as failed_queries_count,
    AVG(rows_examined) as avg_rows_examined,
    AVG(rows_returned) as avg_rows_returned,
    SUM(seq_scans) as total_seq_scans,
    SUM(index_scans) as total_index_scans,
    DATE_TRUNC('hour', created_at) as hour_bucket
FROM query_performance_log
WHERE created_at >= NOW() - INTERVAL '24 hours'
GROUP BY query_type, DATE_TRUNC('hour', created_at)
WITH DATA;

-- Create index for query performance analytics
CREATE INDEX IF NOT EXISTS idx_mv_query_performance_analytics_type_hour 
ON mv_query_performance_analytics(query_type, hour_bucket DESC);

-- Create materialized view for materialized view performance
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_materialized_view_performance AS
SELECT 
    view_name,
    COUNT(*) as total_refreshes,
    AVG(duration_ms) as avg_refresh_time_ms,
    MAX(duration_ms) as max_refresh_time_ms,
    MIN(duration_ms) as min_refresh_time_ms,
    COUNT(CASE WHEN success = true THEN 1 END) as successful_refreshes,
    COUNT(CASE WHEN success = false THEN 1 END) as failed_refreshes,
    AVG(rows_affected) as avg_rows_affected,
    MAX(started_at) as last_refresh_time
FROM mv_refresh_log
WHERE started_at >= NOW() - INTERVAL '7 days'
GROUP BY view_name
WITH DATA;

-- Create index for materialized view performance
CREATE INDEX IF NOT EXISTS idx_mv_materialized_view_performance_view_name 
ON mv_materialized_view_performance(view_name);

-- Add comments for documentation
COMMENT ON TABLE query_performance_log IS 'Logs all database query performance metrics including execution plans';
COMMENT ON TABLE query_plan_cache IS 'Caches query execution plans for performance analysis and reuse';
COMMENT ON TABLE mv_refresh_log IS 'Tracks materialized view refresh operations and their performance';
COMMENT ON TABLE slow_query_alerts IS 'Stores alerts for queries exceeding performance thresholds';
COMMENT ON TABLE db_performance_metrics IS 'General database performance metrics storage';

COMMENT ON FUNCTION cleanup_performance_data IS 'Cleans up old performance monitoring data based on retention policy';

COMMENT ON MATERIALIZED VIEW mv_query_performance_analytics IS 'Aggregated query performance analytics for monitoring dashboards';
COMMENT ON MATERIALIZED VIEW mv_materialized_view_performance IS 'Performance metrics for materialized view refresh operations';
