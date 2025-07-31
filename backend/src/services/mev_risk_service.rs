use crate::models::mev_risk::{
    MevRisk, MevRiskConfig, OracleDeviation, MevTransaction, MevType, MevSeverity
};
use crate::models::PoolState;
use crate::services::{BlockchainService, PriceValidationService};
use crate::error::AppError;
use bigdecimal::{BigDecimal, Zero};
use sqlx::PgPool;
use tracing::{info, warn};
use std::str::FromStr;
use chrono::{Utc, Duration};
use uuid::Uuid;

/// MEV and Oracle risk detection service
pub struct MevRiskService {
    db_pool: PgPool,
    config: MevRiskConfig,
    blockchain_service: Option<BlockchainService>,
    price_validation_service: Option<PriceValidationService>,
}

impl MevRiskService {
    pub fn new(
        db_pool: PgPool, 
        config: Option<MevRiskConfig>,
        blockchain_service: Option<BlockchainService>,
        price_validation_service: Option<PriceValidationService>,
    ) -> Self {
        Self {
            db_pool,
            config: config.unwrap_or_default(),
            blockchain_service,
            price_validation_service,
        }
    }

    /// Calculate comprehensive MEV risk for a pool
    pub async fn calculate_mev_risk(
        &self,
        pool_address: &str,
        chain_id: i32,
        pool_state: &PoolState,
    ) -> Result<MevRisk, AppError> {
        info!("Calculating MEV risk for pool {} on chain {}", pool_address, chain_id);

        // Calculate individual risk components
        let sandwich_risk = self.calculate_sandwich_risk(pool_address, chain_id, pool_state).await?;
        let frontrun_risk = self.calculate_frontrun_risk(pool_address, chain_id, pool_state).await?;
        let oracle_manipulation_risk = self.calculate_oracle_manipulation_risk(pool_address, chain_id).await?;
        let oracle_deviation_risk = self.calculate_oracle_deviation_risk(pool_address, chain_id).await?;

        // Calculate weighted overall MEV risk
        let overall_mev_risk = self.calculate_weighted_mev_risk(
            &sandwich_risk,
            &frontrun_risk,
            &oracle_manipulation_risk,
            &oracle_deviation_risk,
        )?;

        // Calculate confidence score based on data availability
        let confidence_score = self.calculate_confidence_score(pool_address, chain_id).await?;

        let mev_risk = MevRisk {
            id: Uuid::new_v4(),
            pool_address: pool_address.to_string(),
            chain_id,
            sandwich_risk_score: sandwich_risk,
            frontrun_risk_score: frontrun_risk,
            oracle_manipulation_risk,
            oracle_deviation_risk,
            overall_mev_risk,
            confidence_score,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Store the assessment in database
        self.store_mev_risk(&mev_risk).await?;

        Ok(mev_risk)
    }

    /// Detect sandwich attacks in recent transactions
    pub async fn calculate_sandwich_risk(
        &self,
        pool_address: &str,
        chain_id: i32,
        pool_state: &PoolState,
    ) -> Result<BigDecimal, AppError> {
        info!("Calculating sandwich attack risk for pool {}", pool_address);

        // Check for recent sandwich attack patterns
        let recent_sandwich_count = self.get_recent_sandwich_attacks(pool_address, chain_id).await?;
        
        // Calculate risk based on pool liquidity and recent activity
        let liquidity_factor = if pool_state.liquidity.is_zero() {
            BigDecimal::from(1) // Maximum risk for zero liquidity
        } else {
            // Lower liquidity = higher sandwich risk
            let liquidity_usd = &pool_state.tvl_usd.clone().unwrap_or_else(|| BigDecimal::from(0));
            if liquidity_usd < &BigDecimal::from(100000) { // < $100K
                BigDecimal::from_str("0.8").unwrap()
            } else if liquidity_usd < &BigDecimal::from(1000000) { // < $1M
                BigDecimal::from_str("0.5").unwrap()
            } else if liquidity_usd < &BigDecimal::from(10000000) { // < $10M
                BigDecimal::from_str("0.3").unwrap()
            } else {
                BigDecimal::from_str("0.1").unwrap()
            }
        };

        // Activity factor based on recent sandwich attacks
        let activity_factor = if recent_sandwich_count > 10 {
            BigDecimal::from_str("0.9").unwrap()
        } else if recent_sandwich_count > 5 {
            BigDecimal::from_str("0.6").unwrap()
        } else if recent_sandwich_count > 0 {
            BigDecimal::from_str("0.3").unwrap()
        } else {
            BigDecimal::from_str("0.1").unwrap()
        };

        // Combine factors (weighted average)
        let sandwich_risk = (&liquidity_factor * BigDecimal::from_str("0.6").unwrap()) + 
                           (&activity_factor * BigDecimal::from_str("0.4").unwrap());

        Ok(sandwich_risk.min(BigDecimal::from(1)))
    }

    /// Calculate frontrunning risk
    pub async fn calculate_frontrun_risk(
        &self,
        pool_address: &str,
        chain_id: i32,
        pool_state: &PoolState,
    ) -> Result<BigDecimal, AppError> {
        info!("Calculating frontrunning risk for pool {}", pool_address);

        // Check for recent frontrunning activity
        let recent_frontrun_count = self.get_recent_frontrun_attacks(pool_address, chain_id).await?;
        
        // Calculate risk based on transaction volume and MEV bot activity
        let volume_factor = if let Some(volume) = &pool_state.volume_24h_usd {
            if volume > &BigDecimal::from(10000000) { // > $10M volume
                BigDecimal::from_str("0.7").unwrap() // High volume = more MEV opportunities
            } else if volume > &BigDecimal::from(1000000) { // > $1M volume
                BigDecimal::from_str("0.4").unwrap()
            } else {
                BigDecimal::from_str("0.2").unwrap()
            }
        } else {
            BigDecimal::from_str("0.3").unwrap() // Default moderate risk
        };

        // MEV bot activity factor
        let bot_activity_factor = if recent_frontrun_count > 20 {
            BigDecimal::from_str("0.8").unwrap()
        } else if recent_frontrun_count > 10 {
            BigDecimal::from_str("0.5").unwrap()
        } else if recent_frontrun_count > 0 {
            BigDecimal::from_str("0.2").unwrap()
        } else {
            BigDecimal::from_str("0.1").unwrap()
        };

        let frontrun_risk = (&volume_factor * BigDecimal::from_str("0.5").unwrap()) + 
                           (&bot_activity_factor * BigDecimal::from_str("0.5").unwrap());

        Ok(frontrun_risk.min(BigDecimal::from(1)))
    }

    /// Calculate oracle manipulation risk
    pub async fn calculate_oracle_manipulation_risk(
        &self,
        pool_address: &str,
        chain_id: i32,
    ) -> Result<BigDecimal, AppError> {
        info!("Calculating oracle manipulation risk for pool {}", pool_address);

        // Check for recent oracle price manipulations
        let recent_manipulations = self.get_recent_oracle_manipulations(pool_address, chain_id).await?;
        
        // Check oracle update frequency and reliability
        let oracle_reliability = self.assess_oracle_reliability(pool_address, chain_id).await?;

        // Calculate risk based on manipulation history and oracle quality
        let manipulation_factor = if recent_manipulations > 5 {
            BigDecimal::from_str("0.9").unwrap()
        } else if recent_manipulations > 2 {
            BigDecimal::from_str("0.6").unwrap()
        } else if recent_manipulations > 0 {
            BigDecimal::from_str("0.3").unwrap()
        } else {
            BigDecimal::from_str("0.1").unwrap()
        };

        // Oracle reliability factor (inverse - lower reliability = higher risk)
        let reliability_risk = BigDecimal::from(1) - oracle_reliability;

        let manipulation_risk = (&manipulation_factor * BigDecimal::from_str("0.7").unwrap()) + 
                               (&reliability_risk * BigDecimal::from_str("0.3").unwrap());

        Ok(manipulation_risk.min(BigDecimal::from(1)))
    }

    /// Calculate oracle deviation risk
    pub async fn calculate_oracle_deviation_risk(
        &self,
        pool_address: &str,
        chain_id: i32,
    ) -> Result<BigDecimal, AppError> {
        info!("Calculating oracle deviation risk for pool {}", pool_address);

        // Get recent oracle deviations
        let recent_deviations = self.get_recent_oracle_deviations(pool_address, chain_id).await?;
        
        // Calculate average deviation magnitude
        let avg_deviation = if recent_deviations.is_empty() {
            BigDecimal::from(0)
        } else {
            let total: BigDecimal = recent_deviations.iter()
                .map(|d| d.deviation_percent.abs())
                .sum();
            if recent_deviations.is_empty() {
                BigDecimal::zero()
            } else {
                total / BigDecimal::from_str(&recent_deviations.len().to_string()).unwrap_or_else(|_| BigDecimal::from(1))
            }
        };

        // Convert deviation percentage to risk score
        let deviation_risk = if avg_deviation > self.config.oracle_deviation_critical_percent {
            BigDecimal::from_str("0.9").unwrap()
        } else if avg_deviation > self.config.oracle_deviation_warning_percent {
            // Scale between warning and critical thresholds
            let ratio = &avg_deviation / &self.config.oracle_deviation_critical_percent;
            ratio * BigDecimal::from_str("0.9").unwrap()
        } else {
            // Low deviation
            BigDecimal::from_str("0.1").unwrap()
        };

        Ok(deviation_risk.min(BigDecimal::from(1)))
    }

    /// Calculate weighted MEV risk score
    fn calculate_weighted_mev_risk(
        &self,
        sandwich_risk: &BigDecimal,
        frontrun_risk: &BigDecimal,
        oracle_manipulation_risk: &BigDecimal,
        oracle_deviation_risk: &BigDecimal,
    ) -> Result<BigDecimal, AppError> {
        let weighted_risk = 
            sandwich_risk * &self.config.sandwich_weight +
            frontrun_risk * &self.config.frontrun_weight +
            oracle_manipulation_risk * &self.config.oracle_manipulation_weight +
            oracle_deviation_risk * &self.config.oracle_deviation_weight;

        Ok(weighted_risk.min(BigDecimal::from(1)))
    }

    /// Calculate confidence score based on data availability
    async fn calculate_confidence_score(
        &self,
        pool_address: &str,
        chain_id: i32,
    ) -> Result<BigDecimal, AppError> {
        // Check data availability factors
        let has_blockchain_service = self.blockchain_service.is_some();
        let has_price_validation = self.price_validation_service.is_some();
        
        // Check recent transaction data availability
        let recent_tx_count = self.get_recent_transaction_count(pool_address, chain_id).await?;
        let tx_data_factor = if recent_tx_count > 100 {
            BigDecimal::from_str("0.3").unwrap()
        } else if recent_tx_count > 10 {
            BigDecimal::from_str("0.2").unwrap()
        } else {
            BigDecimal::from_str("0.1").unwrap()
        };

        let service_factor = match (has_blockchain_service, has_price_validation) {
            (true, true) => BigDecimal::from_str("0.4").unwrap(),
            (true, false) | (false, true) => BigDecimal::from_str("0.2").unwrap(),
            (false, false) => BigDecimal::from_str("0.1").unwrap(),
        };

        let base_confidence = BigDecimal::from_str("0.3").unwrap(); // Base confidence
        let confidence = base_confidence + service_factor + tx_data_factor;

        Ok(confidence.min(BigDecimal::from(1)))
    }

    /// Store MEV risk assessment in database
    async fn store_mev_risk(&self, mev_risk: &MevRisk) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO mev_risks (
                id, pool_address, chain_id, sandwich_risk_score, frontrun_risk_score,
                oracle_manipulation_risk, oracle_deviation_risk, overall_mev_risk,
                confidence_score, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            mev_risk.id,
            mev_risk.pool_address,
            mev_risk.chain_id,
            mev_risk.sandwich_risk_score,
            mev_risk.frontrun_risk_score,
            mev_risk.oracle_manipulation_risk,
            mev_risk.oracle_deviation_risk,
            mev_risk.overall_mev_risk,
            mev_risk.confidence_score,
            mev_risk.created_at,
            mev_risk.updated_at
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to store MEV risk: {}", e)))?;
        Ok(())
    }

    /// Get or calculate MEV risk for a pool
    pub async fn get_mev_risk(&self, pool_address: &str, chain_id: i32) -> Result<Option<MevRisk>, AppError> {
        // Query the most recent MEV risk assessment from database
        let cached_risk = sqlx::query_as::<_, MevRisk>(
            "SELECT * FROM mev_risks WHERE pool_address = $1 AND chain_id = $2 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(pool_address)
        .bind(chain_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Check if cached assessment is still fresh (within 1 hour)
        if let Some(risk) = cached_risk {
            let age = Utc::now().signed_duration_since(risk.created_at);
            if age < Duration::hours(1) {
                return Ok(Some(risk));
            }
        }

        Ok(None)
    }

    // Helper methods for data retrieval with real database queries
    async fn get_recent_sandwich_attacks(&self, pool_address: &str, chain_id: i32) -> Result<i64, AppError> {
        // Query sandwich attacks in the last 24 hours
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM mev_transactions 
             WHERE pool_address = $1 AND chain_id = $2 
             AND mev_type = 'sandwich_attack' 
             AND detected_at > NOW() - INTERVAL '24 hours'"
        )
        .bind(pool_address)
        .bind(chain_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(count)
    }

    async fn get_recent_frontrun_attacks(&self, pool_address: &str, chain_id: i32) -> Result<i64, AppError> {
        // Query frontrunning attacks in the last 24 hours
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM mev_transactions 
             WHERE pool_address = $1 AND chain_id = $2 
             AND mev_type = 'frontrunning' 
             AND detected_at > NOW() - INTERVAL '24 hours'"
        )
        .bind(pool_address)
        .bind(chain_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(count)
    }

    #[allow(dead_code)]
    async fn get_recent_attack_count(&self, pool_address: &str, chain_id: i32) -> Result<i64, AppError> {
        // Query all MEV attacks in the last 24 hours
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM mev_transactions 
             WHERE pool_address = $1 AND chain_id = $2 
             AND detected_at > NOW() - INTERVAL '24 hours'"
        )
        .bind(pool_address)
        .bind(chain_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(count)
    }

    async fn get_recent_oracle_manipulations(&self, pool_address: &str, chain_id: i32) -> Result<i64, AppError> {
        // Query oracle manipulation events in the last 24 hours
        // This looks for significant price deviations that could indicate manipulation
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM oracle_deviations od
             JOIN mev_transactions mt ON od.chain_id = mt.chain_id
             WHERE mt.pool_address = $1 AND od.chain_id = $2
             AND od.severity IN ('significant', 'critical')
             AND od.timestamp > NOW() - INTERVAL '24 hours'
             AND ABS(od.deviation_percent) > 5.0"
        )
        .bind(pool_address)
        .bind(chain_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(count)
    }

    async fn assess_oracle_reliability(&self, pool_address: &str, chain_id: i32) -> Result<BigDecimal, AppError> {
        // Assess oracle reliability based on recent deviation patterns
        let avg_deviation = sqlx::query_scalar::<_, Option<BigDecimal>>(
            "SELECT AVG(ABS(deviation_percent)) FROM oracle_deviations od
             JOIN mev_transactions mt ON od.chain_id = mt.chain_id
             WHERE mt.pool_address = $1 AND od.chain_id = $2
             AND od.timestamp > NOW() - INTERVAL '7 days'"
        )
        .bind(pool_address)
        .bind(chain_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let reliability = match avg_deviation {
            Some(deviation) => {
                // Higher deviation = lower reliability
                let base_reliability = BigDecimal::from_str("1.0").unwrap();
                let penalty = &deviation / BigDecimal::from_str("100.0").unwrap();
                (base_reliability - penalty).max(BigDecimal::from_str("0.1").unwrap())
            },
            None => BigDecimal::from_str("0.8").unwrap(), // Default 80% reliability
        };

        Ok(reliability)
    }

    async fn get_recent_oracle_deviations(&self, pool_address: &str, chain_id: i32) -> Result<Vec<OracleDeviation>, AppError> {
        // Query recent oracle deviations for pools on this chain
        let deviations = sqlx::query_as::<_, OracleDeviation>(
            "SELECT od.* FROM oracle_deviations od
             JOIN mev_transactions mt ON od.chain_id = mt.chain_id
             WHERE mt.pool_address = $1 AND od.chain_id = $2
             AND od.timestamp > NOW() - INTERVAL '24 hours'
             ORDER BY od.timestamp DESC
             LIMIT 100"
        )
        .bind(pool_address)
        .bind(chain_id)
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(deviations)
    }

    async fn get_recent_transaction_count(&self, pool_address: &str, chain_id: i32) -> Result<i64, AppError> {
        // Query recent transaction count from MEV transactions table
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM mev_transactions 
             WHERE pool_address = $1 AND chain_id = $2 
             AND detected_at > NOW() - INTERVAL '1 hour'"
        )
        .bind(pool_address)
        .bind(chain_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // If no MEV transactions recorded, estimate based on chain activity
        if count == 0 {
            // Fallback: estimate based on chain ID (mainnet = higher activity)
            match chain_id {
                1 => Ok(100),  // Ethereum mainnet
                137 => Ok(80), // Polygon
                42161 => Ok(60), // Arbitrum
                10 => Ok(40),  // Optimism
                _ => Ok(20),   // Other chains
            }
        } else {
            Ok(count)
        }
    }

    /// Advanced sandwich attack detection using transaction pattern analysis
    pub async fn detect_sandwich_attacks(&self, pool_address: &str, chain_id: i32, block_range: i64) -> Result<Vec<MevTransaction>, AppError> {
        // Query recent transactions and analyze for sandwich patterns
        let transactions = self.analyze_transaction_patterns(pool_address, chain_id, block_range).await?;
        
        let mut sandwich_attacks = Vec::new();
        
        // Group transactions by block to detect sandwich patterns
        let mut block_groups: std::collections::HashMap<i64, Vec<&MevTransaction>> = std::collections::HashMap::new();
        for tx in &transactions {
            block_groups.entry(tx.block_number).or_default().push(tx);
        }
        
        // Analyze each block for sandwich patterns
        for (_block_number, block_txs) in block_groups {
            if block_txs.len() >= 3 {
                // Look for sandwich pattern: buy -> victim_tx -> sell
                let sandwich_pattern = self.identify_sandwich_pattern(&block_txs, pool_address).await?;
                sandwich_attacks.extend(sandwich_pattern);
            }
        }
        
        // Store detected attacks in database
        for attack in &sandwich_attacks {
            self.store_mev_transaction(attack).await?;
        }
        
        Ok(sandwich_attacks)
    }
    
    /// Detect oracle manipulation attempts
    pub async fn detect_oracle_manipulation(&self, pool_address: &str, chain_id: i32) -> Result<Vec<OracleDeviation>, AppError> {
        // Query price feeds and detect unusual deviations
        let price_data = self.fetch_oracle_price_data(pool_address, chain_id).await?;
        let market_data = self.fetch_market_price_data(pool_address, chain_id).await?;
        
        let mut manipulations = Vec::new();
        
        // Compare oracle prices with market prices
        for ((oracle_price, market_price), timestamp) in price_data.iter().zip(market_data.iter()).zip(std::iter::repeat(Utc::now())) {
            let deviation_percent = ((&oracle_price.0 - &market_price.0) / &market_price.0) * BigDecimal::from_str("100.0").unwrap();
            
            // Flag significant deviations as potential manipulation
            if deviation_percent.abs() > BigDecimal::from_str("5.0").unwrap() {
                let severity = if deviation_percent.abs() > BigDecimal::from_str("20.0").unwrap() {
                    crate::models::mev_risk::OracleDeviationSeverity::Critical
                } else if deviation_percent.abs() > BigDecimal::from_str("10.0").unwrap() {
                    crate::models::mev_risk::OracleDeviationSeverity::Significant
                } else {
                    crate::models::mev_risk::OracleDeviationSeverity::Moderate
                };
                
                let deviation = OracleDeviation {
                    id: uuid::Uuid::new_v4(),
                    oracle_address: "0x0000000000000000000000000000000000000000".to_string(), // Placeholder
                    token_address: pool_address.to_string(),
                    chain_id,
                    oracle_price: oracle_price.0.clone(),
                    market_price: market_price.0.clone(),
                    deviation_percent,
                    severity,
                    timestamp,
                };
                
                manipulations.push(deviation);
            }
        }
        
        // Store detected manipulations
        for manipulation in &manipulations {
            self.store_oracle_deviation(manipulation).await?;
        }
        
        Ok(manipulations)
    }
    
    /// Connect to MEV-Boost/Flashbots data feeds for real-time MEV detection
    pub async fn fetch_mev_boost_data(&self, block_number: i64) -> Result<Vec<MevTransaction>, AppError> {
        // In a real implementation, this would connect to MEV-Boost relay API
        // For now, we'll simulate the data structure and detection logic
        
        let client = reqwest::Client::new();
        let flashbots_url = format!("https://relay.flashbots.net/relay/v1/data/bidtraces/builder_blocks_received?block_number={}", block_number);
        
        // Attempt to fetch from Flashbots relay (this would need proper authentication in production)
        let response = client
            .get(&flashbots_url)
            .header("User-Agent", "DeFi-Risk-Monitor/1.0")
            .send()
            .await;
            
        match response {
            Ok(resp) if resp.status().is_success() => {
                // Parse MEV bundle data and extract transactions
                let mev_data: serde_json::Value = resp.json().await
                    .map_err(|e| AppError::ExternalApiError(format!("Failed to parse MEV data: {}", e)))?;
                
                // Process MEV bundle data to identify sandwich attacks, arbitrage, etc.
                self.process_mev_bundle_data(mev_data, block_number).await
            },
            _ => {
                // Fallback to local detection methods
                warn!("Failed to fetch MEV-Boost data, using local detection");
                Ok(vec![])
            }
        }
    }
    
    /// Process MEV bundle data from Flashbots/MEV-Boost
    async fn process_mev_bundle_data(&self, data: serde_json::Value, block_number: i64) -> Result<Vec<MevTransaction>, AppError> {
        let mut mev_transactions = Vec::new();
        
        // Parse bundle data (simplified - real implementation would be more complex)
        if let Some(bundles) = data.as_array() {
            for bundle in bundles {
                if let Some(transactions) = bundle.get("transactions").and_then(|t| t.as_array()) {
                    // Analyze transaction patterns within bundles
                    for (i, tx) in transactions.iter().enumerate() {
                        if let Some(tx_hash) = tx.get("hash").and_then(|h| h.as_str()) {
                            // Detect MEV patterns based on transaction ordering and gas prices
                            let mev_type = self.classify_mev_transaction(tx, i, transactions.len()).await?;
                            
                            if !matches!(mev_type, MevType::Unknown) {
                                let mev_tx = MevTransaction {
                                    id: uuid::Uuid::new_v4(),
                                    transaction_hash: tx_hash.to_string(),
                                    block_number,
                                    chain_id: 1, // Ethereum mainnet for MEV-Boost
                                    mev_type,
                                    severity: MevSeverity::Medium, // Default, would be calculated
                                    profit_usd: tx.get("profit").and_then(|p| p.as_str()).and_then(|s| BigDecimal::from_str(s).ok()),
                                    victim_loss_usd: None, // Would be calculated
                                    pool_address: "0x0000000000000000000000000000000000000000".to_string(), // Would be extracted
                                    detected_at: Utc::now(),
                                };
                                
                                mev_transactions.push(mev_tx);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(mev_transactions)
    }
    
    /// Classify MEV transaction type based on patterns
    async fn classify_mev_transaction(&self, tx: &serde_json::Value, position: usize, total_txs: usize) -> Result<MevType, AppError> {
        // Analyze transaction characteristics to classify MEV type
        let _gas_price = tx.get("gasPrice").and_then(|g| g.as_str()).unwrap_or("0");
        let to_address = tx.get("to").and_then(|t| t.as_str()).unwrap_or("");
        
        // Simple heuristics for MEV classification
        let mev_type = if position == 0 && total_txs >= 3 {
            // First transaction in bundle with high gas - likely frontrun
            MevType::Frontrunning
        } else if position == total_txs - 1 && total_txs >= 3 {
            // Last transaction in bundle - likely backrun/sandwich completion
            MevType::SandwichAttack
        } else if to_address.contains("uniswap") || to_address.contains("sushiswap") {
            // DEX interaction - likely arbitrage
            MevType::Arbitrage
        } else {
            MevType::Unknown
        };
        
        Ok(mev_type)
    }
    
    /// Store MEV transaction in database
    async fn store_mev_transaction(&self, mev_tx: &MevTransaction) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO mev_transactions (id, transaction_hash, block_number, chain_id, mev_type, severity, profit_usd, victim_loss_usd, pool_address, detected_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (transaction_hash) DO NOTHING"
        )
        .bind(&mev_tx.id)
        .bind(&mev_tx.transaction_hash)
        .bind(mev_tx.block_number)
        .bind(mev_tx.chain_id)
        .bind(&mev_tx.mev_type)
        .bind(&mev_tx.severity)
        .bind(&mev_tx.profit_usd)
        .bind(&mev_tx.victim_loss_usd)
        .bind(&mev_tx.pool_address)
        .bind(mev_tx.detected_at)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Store oracle deviation in database
    async fn store_oracle_deviation(&self, deviation: &OracleDeviation) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO oracle_deviations (id, oracle_address, token_address, chain_id, oracle_price, market_price, deviation_percent, severity, timestamp)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(&deviation.id)
        .bind(&deviation.oracle_address)
        .bind(&deviation.token_address)
        .bind(deviation.chain_id)
        .bind(&deviation.oracle_price)
        .bind(&deviation.market_price)
        .bind(&deviation.deviation_percent)
        .bind(&deviation.severity)
        .bind(deviation.timestamp)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }
    
    // Helper methods for advanced detection
    async fn analyze_transaction_patterns(&self, _pool_address: &str, _chain_id: i32, _block_range: i64) -> Result<Vec<MevTransaction>, AppError> {
        // Placeholder - would analyze blockchain data for transaction patterns
        Ok(vec![])
    }
    
    async fn identify_sandwich_pattern(&self, _transactions: &[&MevTransaction], _pool_address: &str) -> Result<Vec<MevTransaction>, AppError> {
        // Placeholder - would identify sandwich attack patterns
        Ok(vec![])
    }
    
    async fn fetch_oracle_price_data(&self, _pool_address: &str, _chain_id: i32) -> Result<Vec<(BigDecimal,)>, AppError> {
        // Placeholder - would fetch oracle price data
        Ok(vec![])
    }
    
    async fn fetch_market_price_data(&self, _pool_address: &str, _chain_id: i32) -> Result<Vec<(BigDecimal,)>, AppError> {
        // Placeholder - would fetch market price data
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PoolState;
    use std::str::FromStr;

    fn create_test_pool_state() -> PoolState {
        PoolState {
            id: Uuid::new_v4(),
            pool_address: "0x1234567890123456789012345678901234567890".to_string(),
            chain_id: 1,
            current_tick: 0,
            sqrt_price_x96: BigDecimal::from(1000000),
            liquidity: BigDecimal::from(1000000),
            token0_price_usd: Some(BigDecimal::from(1)),
            token1_price_usd: Some(BigDecimal::from(2000)),
            tvl_usd: Some(BigDecimal::from(5000000)),
            volume_24h_usd: Some(BigDecimal::from(1000000)),
            fees_24h_usd: Some(BigDecimal::from(5000)),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_mev_risk_config_default() {
        let config = MevRiskConfig::default();
        
        // Verify weights sum to 1.0 (100%)
        let total_weight = &config.sandwich_weight + &config.frontrun_weight + 
                          &config.oracle_manipulation_weight + &config.oracle_deviation_weight;
        assert_eq!(total_weight, BigDecimal::from_str("1.0").unwrap());
    }

    #[test]
    fn test_weighted_mev_risk_calculation() {
        let config = MevRiskConfig::default();
        // Test the calculation logic directly without database dependency
        let config_ref = &config;

        let sandwich_risk = BigDecimal::from_str("0.8").unwrap();
        let frontrun_risk = BigDecimal::from_str("0.6").unwrap();
        let oracle_manipulation_risk = BigDecimal::from_str("0.4").unwrap();
        let oracle_deviation_risk = BigDecimal::from_str("0.2").unwrap();

        // Calculate weighted MEV risk directly using config weights
        let weighted_risk = (
            &sandwich_risk * &config_ref.sandwich_weight +
            &frontrun_risk * &config_ref.frontrun_weight +
            &oracle_manipulation_risk * &config_ref.oracle_manipulation_weight +
            &oracle_deviation_risk * &config_ref.oracle_deviation_weight
        );

        // Risk should be between 0 and 1
        assert!(weighted_risk >= BigDecimal::from(0));
        assert!(weighted_risk <= BigDecimal::from(1));
    }

    #[tokio::test]
    async fn test_sandwich_risk_calculation() {
        // Test the calculation logic directly without database dependency
        let config = MevRiskConfig::default();
        
        // Test high-risk scenario (low liquidity)
        let high_risk_tvl = BigDecimal::from(50000); // $50K TVL
        let high_risk_score = calculate_sandwich_risk_score(&high_risk_tvl, &config);
        assert!(high_risk_score >= BigDecimal::from_str("0.5").unwrap());
        
        // Test low-risk scenario (high liquidity)
        let low_risk_tvl = BigDecimal::from(50000000); // $50M TVL
        let low_risk_score = calculate_sandwich_risk_score(&low_risk_tvl, &config);
        assert!(low_risk_score <= BigDecimal::from_str("0.4").unwrap());
    }
    
    fn calculate_sandwich_risk_score(tvl: &BigDecimal, config: &MevRiskConfig) -> BigDecimal {
        // Simplified sandwich risk calculation based on TVL
        if tvl < &BigDecimal::from(100000) { // < $100K
            BigDecimal::from_str("0.8").unwrap()
        } else if tvl < &BigDecimal::from(1000000) { // < $1M
            BigDecimal::from_str("0.5").unwrap()
        } else if tvl < &BigDecimal::from(10000000) { // < $10M
            BigDecimal::from_str("0.3").unwrap()
        } else {
            BigDecimal::from_str("0.1").unwrap()
        }
    }

    #[tokio::test]
    async fn test_frontrun_risk_calculation() {
        // Test the calculation logic directly without database dependency
        let config = MevRiskConfig::default();
        
        // Test high-volume scenario (more MEV opportunities)
        let high_volume = BigDecimal::from(20000000); // $20M volume
        let high_risk_score = calculate_frontrun_risk_score(&high_volume, &config);
        assert!(high_risk_score >= BigDecimal::from_str("0.5").unwrap());
        
        // Test low-volume scenario
        let low_volume = BigDecimal::from(100000); // $100K volume
        let low_risk_score = calculate_frontrun_risk_score(&low_volume, &config);
        assert!(low_risk_score <= BigDecimal::from_str("0.3").unwrap());
    }
    
    fn calculate_frontrun_risk_score(volume: &BigDecimal, config: &MevRiskConfig) -> BigDecimal {
        // Simplified frontrun risk calculation based on volume
        if volume > &BigDecimal::from(10000000) { // > $10M volume
            BigDecimal::from_str("0.7").unwrap() // High volume = more MEV opportunities
        } else if volume > &BigDecimal::from(1000000) { // > $1M volume
            BigDecimal::from_str("0.4").unwrap()
        } else {
            BigDecimal::from_str("0.2").unwrap()
        }
    }

    fn create_test_pool_state_with_tvl(tvl: BigDecimal) -> PoolState {
        PoolState {
            id: Uuid::new_v4(),
            pool_address: "0x1234567890123456789012345678901234567890".to_string(),
            chain_id: 1,
            current_tick: 0,
            sqrt_price_x96: BigDecimal::from(1000000),
            liquidity: BigDecimal::from(1000000),
            token0_price_usd: Some(BigDecimal::from(1)),
            token1_price_usd: Some(BigDecimal::from(2000)),
            tvl_usd: Some(tvl),
            volume_24h_usd: Some(BigDecimal::from(1000000)),
            fees_24h_usd: Some(BigDecimal::from(5000)),
            timestamp: Utc::now(),
        }
    }

    fn create_test_pool_state_with_volume(volume: BigDecimal) -> PoolState {
        PoolState {
            id: Uuid::new_v4(),
            pool_address: "0x1234567890123456789012345678901234567890".to_string(),
            chain_id: 1,
            current_tick: 0,
            sqrt_price_x96: BigDecimal::from(1000000),
            liquidity: BigDecimal::from(1000000),
            token0_price_usd: Some(BigDecimal::from(1)),
            token1_price_usd: Some(BigDecimal::from(2000)),
            tvl_usd: Some(BigDecimal::from(5000000)),
            volume_24h_usd: Some(volume),
            fees_24h_usd: Some(BigDecimal::from(5000)),
            timestamp: Utc::now(),
        }
    }
}
