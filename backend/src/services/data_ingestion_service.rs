use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::interval;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use crate::models::{PoolState, PriceHistory};
use crate::services::{
    DexWebSocketClient, PoolUpdate, PriceFeedService, PriceStorageService,
    WebSocketService, RealTimeRiskService
};
use crate::error::AppError;

/// Configuration for data ingestion pipeline
#[derive(Debug, Clone)]
pub struct DataIngestionConfig {
    pub enable_websocket_feeds: bool,
    pub enable_price_polling: bool,
    pub price_update_interval_ms: u64,
    pub pool_state_update_interval_ms: u64,
    pub batch_size: usize,
    pub max_queue_size: usize,
}

impl Default for DataIngestionConfig {
    fn default() -> Self {
        Self {
            enable_websocket_feeds: true,
            enable_price_polling: true,
            price_update_interval_ms: 5000,  // 5 seconds
            pool_state_update_interval_ms: 1000,  // 1 second
            batch_size: 100,
            max_queue_size: 10000,
        }
    }
}

/// Real-time data ingestion service that coordinates multiple data sources
#[derive(Clone)]
pub struct DataIngestionService {
    config: DataIngestionConfig,
    dex_client: DexWebSocketClient,
    price_feed: PriceFeedService,
    price_storage: PriceStorageService,
    websocket_service: WebSocketService,
    real_time_risk_service: RealTimeRiskService,
    database_ops: crate::database::DatabaseOperationsService,
    
    // Internal channels for data flow
    pool_updates_tx: broadcast::Sender<PoolUpdate>,
    price_updates_tx: broadcast::Sender<PriceHistory>,
    
    // Processing queues
    pool_update_queue: Arc<RwLock<Vec<PoolUpdate>>>,
    price_update_queue: Arc<RwLock<Vec<PriceHistory>>>,
    
    // Statistics
    stats: Arc<RwLock<IngestionStats>>,
}

#[derive(Debug, Clone, Default)]
pub struct IngestionStats {
    pub total_pool_updates: u64,
    pub total_price_updates: u64,
    pub websocket_connections: u32,
    pub last_update_timestamp: Option<DateTime<Utc>>,
    pub processing_rate_per_second: f64,
    pub queue_sizes: HashMap<String, usize>,
    pub error_count: u64,
}

impl DataIngestionService {
    /// Create a new data ingestion service
    pub fn new(
        config: DataIngestionConfig,
        price_feed: PriceFeedService,
        price_storage: PriceStorageService,
        websocket_service: WebSocketService,
        real_time_risk_service: RealTimeRiskService,
        database_ops: crate::database::DatabaseOperationsService,
    ) -> Self {
        let dex_client = DexWebSocketClient::new();
        let (pool_updates_tx, _) = broadcast::channel(config.max_queue_size);
        let (price_updates_tx, _) = broadcast::channel(config.max_queue_size);

        Self {
            config,
            dex_client,
            price_feed,
            price_storage,
            websocket_service,
            real_time_risk_service,
            database_ops,
            pool_updates_tx,
            price_updates_tx,
            pool_update_queue: Arc::new(RwLock::new(Vec::new())),
            price_update_queue: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(IngestionStats::default())),
        }
    }

    /// Start the complete data ingestion pipeline
    pub async fn start_pipeline(&self) -> Result<(), AppError> {
        info!("Starting real-time data ingestion pipeline");

        // Initialize DEX WebSocket connections
        if self.config.enable_websocket_feeds {
            self.start_websocket_feeds().await?;
        }

        // Start price polling for tokens not covered by WebSocket feeds
        if self.config.enable_price_polling {
            self.start_price_polling().await?;
        }

        // Start data processing tasks
        self.start_data_processors().await?;

        // Start statistics and monitoring
        self.start_monitoring().await?;

        info!("Data ingestion pipeline started successfully");
        Ok(())
    }

    /// Initialize and start DEX WebSocket feeds
    async fn start_websocket_feeds(&self) -> Result<(), AppError> {
        info!("Initializing DEX WebSocket feeds");

        // Initialize default DEX configurations
        self.dex_client.initialize_default_configs().await?;

        // Start all DEX connections
        self.dex_client.start_all_connections().await?;

        // Subscribe to pool updates and forward them to our processing pipeline
        let mut pool_receiver = self.dex_client.subscribe_to_updates();
        let pool_updates_tx = self.pool_updates_tx.clone();
        let stats = self.stats.clone();

        tokio::spawn(async move {
            while let Ok(pool_update) = pool_receiver.recv().await {
                debug!("Received pool update: {:?}", pool_update);

                // Forward to internal pipeline
                if let Err(e) = pool_updates_tx.send(pool_update) {
                    warn!("Failed to forward pool update: {}", e);
                }

                // Update statistics
                let mut stats_guard = stats.write().await;
                stats_guard.total_pool_updates += 1;
                stats_guard.last_update_timestamp = Some(Utc::now());
            }
        });

        info!("DEX WebSocket feeds initialized");
        Ok(())
    }

    /// Start price polling for additional tokens
    async fn start_price_polling(&self) -> Result<(), AppError> {
        info!("Starting price polling service");

        let price_feed = self.price_feed.clone();
        let price_updates_tx = self.price_updates_tx.clone();
        let stats = self.stats.clone();
        let interval_ms = self.config.price_update_interval_ms;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(interval_ms));

            loop {
                interval.tick().await;

                // Get list of tokens to poll (major tokens not covered by WebSocket)
                let tokens_to_poll = vec![
                    "ethereum".to_string(),
                    "bitcoin".to_string(),
                    "usd-coin".to_string(),
                    "tether".to_string(),
                    "binancecoin".to_string(),
                    "cardano".to_string(),
                    "solana".to_string(),
                    "polkadot".to_string(),
                ];

                for token_id in tokens_to_poll {
                    // Note: Using fetch_prices method from PriceFeedService
                    match price_feed.fetch_prices(&token_id, 1).await {
                        Ok(prices) => {
                            if let Some(price) = prices.values().next() {
                                let price_history = PriceHistory {
                                    id: Uuid::new_v4(),
                                    token_address: token_id.clone(),
                                    price_usd: price.clone(),
                                    timestamp: Utc::now(),
                                    chain_id: 1, // Ethereum mainnet
                                };

                                if let Err(e) = price_updates_tx.send(price_history) {
                                    warn!("Failed to send price update for {}: {}", token_id, e);
                                }

                                // Update statistics
                                let mut stats_guard = stats.write().await;
                                stats_guard.total_price_updates += 1;
                            } else {
                                warn!("No price data returned for {}", token_id);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to fetch price for {}: {}", token_id, e);
                            let mut stats_guard = stats.write().await;
                            stats_guard.error_count += 1;
                        }
                    }
                }
            }
        });

        info!("Price polling service started");
        Ok(())
    }

    /// Start data processing tasks
    async fn start_data_processors(&self) -> Result<(), AppError> {
        info!("Starting data processors");

        // Start pool update processor
        self.start_pool_update_processor().await?;

        // Start price update processor
        self.start_price_update_processor().await?;

        // Start batch database writer
        self.start_batch_writer().await?;

        info!("Data processors started");
        Ok(())
    }

    /// Process pool updates and trigger risk calculations
    async fn start_pool_update_processor(&self) -> Result<(), AppError> {
        let mut pool_receiver = self.pool_updates_tx.subscribe();
        let pool_queue = self.pool_update_queue.clone();
        let _real_time_risk_service = self.real_time_risk_service.clone();
        let websocket_service = self.websocket_service.clone();
        let database_ops = self.database_ops.clone();
        let batch_size = self.config.batch_size;

        tokio::spawn(async move {
            while let Ok(pool_update) = pool_receiver.recv().await {
                // Add to processing queue
                {
                    let mut queue = pool_queue.write().await;
                    queue.push(pool_update.clone());
                }

                // Convert to PoolState and trigger risk calculation
                if let Ok(pool_state) = Self::convert_pool_update_to_state(&pool_update) {
                    // Note: Real-time risk assessment would be triggered here
                    // For now, we'll just log the pool state update
                    debug!("Pool state updated: {:?}", pool_state);

                    // Broadcast to WebSocket clients
                    let market_update_msg = crate::services::websocket_service::StreamMessage::MarketUpdate {
                        token_address: pool_update.pool_address.clone(),
                        price_usd: pool_update.token0_price_usd.unwrap_or_default(),
                        price_change_24h: BigDecimal::from(0), // Calculate from historical data
                        volatility: BigDecimal::from(0), // Calculate from price history
                        timestamp: pool_update.timestamp,
                    };

                    if let Err(e) = websocket_service.broadcast(market_update_msg).await {
                        debug!("Failed to broadcast market update: {}", e);
                    }
                }

                // Process batch if queue is full
                let queue_size = {
                    let queue = pool_queue.read().await;
                    queue.len()
                };

                if queue_size >= batch_size {
                    Self::process_pool_batch(&pool_queue, &database_ops).await;
                }
            }
        });

        Ok(())
    }

    /// Process price updates and store them
    async fn start_price_update_processor(&self) -> Result<(), AppError> {
        let mut price_receiver = self.price_updates_tx.subscribe();
        let price_queue = self.price_update_queue.clone();
        let price_storage = self.price_storage.clone();
        let database_ops = self.database_ops.clone();
        let batch_size = self.config.batch_size;

        tokio::spawn(async move {
            while let Ok(price_update) = price_receiver.recv().await {
                // Store price update (convert PriceHistory to CreatePriceHistory)
                let create_price = crate::models::CreatePriceHistory {
                    token_address: price_update.token_address.clone(),
                    chain_id: price_update.chain_id,
                    price_usd: price_update.price_usd.clone(),
                    timestamp: price_update.timestamp,
                };
                if let Err(e) = price_storage.store_price(&create_price).await {
                    warn!("Failed to store price update: {}", e);
                }

                // Add to batch processing queue
                {
                    let mut queue = price_queue.write().await;
                    queue.push(price_update);
                }

                // Process batch if queue is full
                let queue_size = {
                    let queue = price_queue.read().await;
                    queue.len()
                };

                if queue_size >= batch_size {
                    Self::process_price_batch(&price_queue, &database_ops).await;
                }
            }
        });

        Ok(())
    }

    /// Start batch database writer for efficient bulk operations
    async fn start_batch_writer(&self) -> Result<(), AppError> {
        let pool_queue = self.pool_update_queue.clone();
        let _price_queue = self.price_update_queue.clone();
        let database_ops = self.database_ops.clone();
        let batch_interval = Duration::from_millis(self.config.pool_state_update_interval_ms);

        tokio::spawn(async move {
            let mut interval = interval(batch_interval);

            loop {
                interval.tick().await;

                // Process any remaining items in queues
                Self::process_pool_batch(&pool_queue, &database_ops).await;
                // Price batch processing is handled in the price processor
            }
        });

        Ok(())
    }

    /// Start monitoring and statistics collection
    async fn start_monitoring(&self) -> Result<(), AppError> {
        let stats = self.stats.clone();
        let dex_client = self.dex_client.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Update stats every 30 seconds

            loop {
                interval.tick().await;

                let mut stats_guard = stats.write().await;
                
                // Update connection status
                let connection_status = dex_client.get_connection_status().await;
                stats_guard.websocket_connections = connection_status.values().filter(|&&active| active).count() as u32;

                // Calculate processing rate (simplified)
                stats_guard.processing_rate_per_second = 
                    (stats_guard.total_pool_updates + stats_guard.total_price_updates) as f64 / 30.0;

                info!("Data ingestion stats: {:?}", *stats_guard);
            }
        });

        Ok(())
    }

    /// Convert PoolUpdate to PoolState
    fn convert_pool_update_to_state(update: &PoolUpdate) -> Result<PoolState, AppError> {
        Ok(PoolState {
            id: Uuid::new_v4(),
            pool_address: update.pool_address.clone(),
            chain_id: update.chain_id as i32,
            current_tick: update.current_tick.unwrap_or(0),
            sqrt_price_x96: update.sqrt_price_x96.clone().unwrap_or_default(),
            liquidity: update.liquidity.clone().unwrap_or_default(),
            token0_price_usd: update.token0_price_usd.clone(),
            token1_price_usd: update.token1_price_usd.clone(),
            tvl_usd: update.tvl_usd.clone(),
            fees_24h_usd: update.fees_24h_usd.clone(),
            volume_24h_usd: update.volume_24h_usd.clone(),
            timestamp: update.timestamp,
        })
    }

    /// Process a batch of pool updates with actual database writes
    async fn process_pool_batch(
        pool_queue: &Arc<RwLock<Vec<PoolUpdate>>>,
        database_ops: &crate::database::DatabaseOperationsService,
    ) {
        let batch = {
            let mut queue = pool_queue.write().await;
            let batch = queue.drain(..).collect::<Vec<_>>();
            batch
        };

        if !batch.is_empty() {
            info!("Processing batch of {} pool updates", batch.len());
            
            // Convert PoolUpdates to PoolStates for database storage
            let pool_states: Vec<crate::models::PoolState> = batch
                .into_iter()
                .map(|update| crate::models::PoolState {
                    id: uuid::Uuid::new_v4(),
                    pool_address: update.pool_address,
                    chain_id: update.chain_id as i32, // Convert u64 to i32
                    current_tick: update.current_tick.unwrap_or(0), // Handle Option<i32>
                    sqrt_price_x96: update.sqrt_price_x96.unwrap_or_else(|| BigDecimal::from(0)), // Handle Option<BigDecimal>
                    liquidity: update.liquidity.unwrap_or_else(|| BigDecimal::from(0)), // Handle Option<BigDecimal>
                    token0_price_usd: update.token0_price_usd,
                    token1_price_usd: update.token1_price_usd,
                    tvl_usd: update.tvl_usd,
                    fees_24h_usd: update.fees_24h_usd,
                    volume_24h_usd: update.volume_24h_usd,
                    timestamp: update.timestamp,
                })
                .collect();

            // Bulk insert pool states
            match Self::bulk_insert_pool_states(database_ops, &pool_states).await {
                Ok(inserted_count) => {
                    info!("✅ Successfully inserted {} pool states to database", inserted_count);
                }
                Err(e) => {
                    warn!("❌ Failed to bulk insert pool states: {}", e);
                }
            }
        }
    }

    /// Process a batch of price updates with real bulk database operations
    async fn process_price_batch(
        price_queue: &Arc<RwLock<Vec<PriceHistory>>>,
        database_ops: &crate::database::DatabaseOperationsService,
    ) {
        let batch = {
            let mut queue = price_queue.write().await;
            let batch = queue.drain(..).collect::<Vec<_>>();
            batch
        };

        if !batch.is_empty() {
            info!("Processing batch of {} price updates", batch.len());
            match Self::bulk_insert_price_histories(database_ops, &batch).await {
                Ok(inserted_count) => {
                    info!("✅ Successfully inserted {} price histories to database", inserted_count);
                }
                Err(e) => {
                    warn!("❌ Failed to bulk insert price histories: {}", e);
                }
            }
        }
    }

    /// Bulk insert pool states into database with real SQL queries
    async fn bulk_insert_pool_states(
        database_ops: &crate::database::DatabaseOperationsService,
        pool_states: &[crate::models::PoolState],
    ) -> Result<u64, AppError> {
        if pool_states.is_empty() {
            return Ok(0);
        }

        info!("Bulk inserting {} pool states to database", pool_states.len());
        
        // Build the bulk insert SQL with ON CONFLICT handling
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO pool_states (
                id, pool_address, chain_id, current_tick, sqrt_price_x96, 
                liquidity, token0_price_usd, token1_price_usd, tvl_usd, 
                volume_24h_usd, fees_24h_usd, timestamp
            ) "
        );
        
        query_builder.push_values(pool_states, |mut b, pool_state| {
            b.push_bind(pool_state.id)
                .push_bind(&pool_state.pool_address)
                .push_bind(pool_state.chain_id)
                .push_bind(pool_state.current_tick)
                .push_bind(&pool_state.sqrt_price_x96)
                .push_bind(&pool_state.liquidity)
                .push_bind(&pool_state.token0_price_usd)
                .push_bind(&pool_state.token1_price_usd)
                .push_bind(&pool_state.tvl_usd)
                .push_bind(&pool_state.volume_24h_usd)
                .push_bind(&pool_state.fees_24h_usd)
                .push_bind(pool_state.timestamp);
        });
        
        // Add ON CONFLICT clause for upsert behavior
        query_builder.push(
            " ON CONFLICT (pool_address, chain_id, timestamp) 
             DO UPDATE SET 
                current_tick = EXCLUDED.current_tick,
                sqrt_price_x96 = EXCLUDED.sqrt_price_x96,
                liquidity = EXCLUDED.liquidity,
                token0_price_usd = EXCLUDED.token0_price_usd,
                token1_price_usd = EXCLUDED.token1_price_usd,
                tvl_usd = EXCLUDED.tvl_usd,
                volume_24h_usd = EXCLUDED.volume_24h_usd,
                fees_24h_usd = EXCLUDED.fees_24h_usd"
        );
        
        let query = query_builder.build();
        
        // Execute the bulk insert using the database operations service pool
        let pool = database_ops.get_pool();
        let result = query.execute(pool).await
            .map_err(|e| {
                error!("Failed to bulk insert pool states: {}", e);
                AppError::DatabaseError(format!("Bulk insert failed: {}", e))
            })?;
        
        let inserted_count = result.rows_affected();
        info!("✅ Successfully bulk inserted {} pool states to database", inserted_count);
        
        Ok(inserted_count)
    }

    /// Bulk insert price histories into database with real SQL queries
    async fn bulk_insert_price_histories(
        database_ops: &crate::database::DatabaseOperationsService,
        price_histories: &[PriceHistory],
    ) -> Result<u64, AppError> {
        if price_histories.is_empty() {
            return Ok(0);
        }

        info!("Bulk inserting {} price histories to database", price_histories.len());
        
        // Build the bulk insert SQL with ON CONFLICT handling
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO price_history (
                id, token_address, chain_id, price_usd, timestamp
            ) "
        );
        
        query_builder.push_values(price_histories, |mut b, price_history| {
            b.push_bind(price_history.id)
                .push_bind(&price_history.token_address)
                .push_bind(price_history.chain_id)
                .push_bind(&price_history.price_usd)
                .push_bind(price_history.timestamp);
        });
        
        // Add ON CONFLICT clause for upsert behavior
        query_builder.push(
            " ON CONFLICT (token_address, chain_id, timestamp) 
             DO UPDATE SET 
                price_usd = EXCLUDED.price_usd"
        );
        
        let query = query_builder.build();
        
        // Execute the bulk insert using the database operations service pool
        let pool = database_ops.get_pool();
        let result = query.execute(pool).await
            .map_err(|e| {
                error!("Failed to bulk insert price histories: {}", e);
                AppError::DatabaseError(format!("Bulk insert failed: {}", e))
            })?;
        
        let inserted_count = result.rows_affected();
        info!("✅ Successfully bulk inserted {} price histories to database", inserted_count);
        
        Ok(inserted_count)
    }

    /// Get current ingestion statistics
    pub async fn get_stats(&self) -> IngestionStats {
        self.stats.read().await.clone()
    }

    /// Stop the data ingestion pipeline
    pub async fn stop_pipeline(&self) -> Result<(), AppError> {
        info!("Stopping data ingestion pipeline");

        // Stop DEX WebSocket connections
        self.dex_client.stop_all_connections().await?;

        info!("Data ingestion pipeline stopped");
        Ok(())
    }

    /// Health check for the data ingestion service
    pub async fn health_check(&self) -> Result<HashMap<String, String>, AppError> {
        let mut health = HashMap::new();
        
        // Check DEX connections
        let connection_status = self.dex_client.get_connection_status().await;
        let active_connections = connection_status.values().filter(|&&active| active).count();
        health.insert("dex_connections".to_string(), format!("{}/{}", active_connections, connection_status.len()));

        // Check queue sizes
        let pool_queue_size = self.pool_update_queue.read().await.len();
        let price_queue_size = self.price_update_queue.read().await.len();
        health.insert("pool_queue_size".to_string(), pool_queue_size.to_string());
        health.insert("price_queue_size".to_string(), price_queue_size.to_string());

        // Check statistics
        let stats = self.get_stats().await;
        health.insert("total_updates".to_string(), (stats.total_pool_updates + stats.total_price_updates).to_string());
        health.insert("error_count".to_string(), stats.error_count.to_string());
        
        if let Some(last_update) = stats.last_update_timestamp {
            let seconds_since_update = (Utc::now() - last_update).num_seconds();
            health.insert("seconds_since_last_update".to_string(), seconds_since_update.to_string());
        }

        Ok(health)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabaseService;

    #[tokio::test]
    async fn test_data_ingestion_service_creation() {
        // This would require proper mocking of dependencies
        // For now, just test that the config works
        let config = DataIngestionConfig::default();
        assert!(config.enable_websocket_feeds);
        assert!(config.enable_price_polling);
    }

    #[test]
    fn test_pool_update_conversion() {
        let pool_update = PoolUpdate {
            pool_address: "0x123".to_string(),
            chain_id: 1,
            current_tick: Some(100),
            sqrt_price_x96: Some(BigDecimal::from(1000)),
            liquidity: Some(BigDecimal::from(50000)),
            token0_price_usd: Some(BigDecimal::from(1)),
            token1_price_usd: Some(BigDecimal::from(2000)),
            tvl_usd: Some(BigDecimal::from(1000000)),
            fees_24h_usd: Some(BigDecimal::from(5000)),
            volume_24h_usd: Some(BigDecimal::from(100000)),
            timestamp: Utc::now(),
            source: "test".to_string(),
        };

        let pool_state = DataIngestionService::convert_pool_update_to_state(&pool_update);
        assert!(pool_state.is_ok());
        
        let state = pool_state.unwrap();
        assert_eq!(state.pool_address, "0x123");
        assert_eq!(state.chain_id, 1);
    }
}
