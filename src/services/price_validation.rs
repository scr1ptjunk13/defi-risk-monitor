use std::collections::HashMap;
use std::time::Duration;
use bigdecimal::{BigDecimal, Zero, ToPrimitive};
use serde::{Serialize, Deserialize};
use tracing::{info, warn};
use crate::error::AppError;
use crate::utils::caching::{CacheManager, CachedPrice};
use crate::utils::fault_tolerance::{FaultTolerantService, RetryConfig};

/// Price source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceSource {
    pub name: String,
    pub url: String,
    pub weight: f64,        // Weight for price aggregation (0.0 to 1.0)
    pub timeout: Duration,
    pub enabled: bool,
}

impl Default for PriceSource {
    fn default() -> Self {
        Self {
            name: "unknown".to_string(),
            url: "".to_string(),
            weight: 1.0,
            timeout: Duration::from_secs(5),
            enabled: true,
        }
    }
}

/// Price validation configuration
#[derive(Debug, Clone)]
pub struct PriceValidationConfig {
    pub max_deviation_percent: f64,    // Maximum allowed deviation between sources
    pub min_sources_required: usize,   // Minimum number of sources for validation
    pub anomaly_threshold: f64,        // Threshold for anomaly detection
    pub price_staleness_seconds: u64,  // Maximum age for cached prices
}

impl Default for PriceValidationConfig {
    fn default() -> Self {
        Self {
            max_deviation_percent: 5.0,    // 5% maximum deviation
            min_sources_required: 2,       // At least 2 sources
            anomaly_threshold: 10.0,       // 10% anomaly threshold
            price_staleness_seconds: 300,  // 5 minutes staleness
        }
    }
}

/// Validated price result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedPrice {
    pub token_address: String,
    pub chain_id: i32,
    pub price_usd: BigDecimal,
    pub confidence_score: f64,         // 0.0 to 1.0
    pub sources_used: Vec<String>,
    pub deviation_percent: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub anomaly_detected: bool,
}

/// Price anomaly detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceAnomaly {
    pub token_address: String,
    pub chain_id: i32,
    pub current_price: BigDecimal,
    pub expected_price: BigDecimal,
    pub deviation_percent: f64,
    pub anomaly_type: AnomalyType,
    pub severity: AnomalySeverity,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    PriceSpike,
    PriceDrop,
    SourceDiscrepancy,
    StaleData,
    InvalidData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Multi-source price validation service
pub struct PriceValidationService {
    sources: HashMap<String, PriceSource>,
    config: PriceValidationConfig,
    cache_manager: CacheManager,
    #[allow(dead_code)]
    fault_tolerant_service: FaultTolerantService,
    price_history: HashMap<String, Vec<BigDecimal>>, // For anomaly detection
}

impl PriceValidationService {
    pub async fn new(
        sources: Vec<PriceSource>,
        config: PriceValidationConfig,
        cache_manager: CacheManager,
    ) -> Result<Self, AppError> {
        let sources_map: HashMap<String, PriceSource> = sources
            .into_iter()
            .map(|source| (source.name.clone(), source))
            .collect();

        info!("Initializing price validation service with {} sources", sources_map.len());

        let fault_tolerant_service = FaultTolerantService::new(
            "price_validation",
            RetryConfig::price_api(),
        );

        Ok(Self {
            sources: sources_map,
            config,
            cache_manager,
            fault_tolerant_service,
            price_history: HashMap::new(),
        })
    }

    /// Get validated price from multiple sources
    pub async fn get_validated_price(
        &mut self,
        token_address: &str,
        chain_id: i32,
    ) -> Result<ValidatedPrice, AppError> {
        let cache_key = format!("{}:{}", token_address, chain_id);

        // Check cache first
        if let Some(cached_price) = self.cache_manager.price_cache.get(&cache_key).await? {
            if self.is_price_fresh(&cached_price) {
                return Ok(ValidatedPrice {
                    token_address: cached_price.token_address,
                    chain_id: cached_price.chain_id,
                    price_usd: cached_price.price_usd,
                    confidence_score: 0.9, // High confidence for cached data
                    sources_used: vec!["cache".to_string()],
                    deviation_percent: 0.0,
                    timestamp: cached_price.timestamp,
                    anomaly_detected: false,
                });
            }
        }

        // Fetch prices from multiple sources
        let source_prices = self.fetch_from_multiple_sources(token_address, chain_id).await?;

        if source_prices.len() < self.config.min_sources_required {
            return Err(AppError::ValidationError(format!(
                "Insufficient price sources: got {}, required {}",
                source_prices.len(),
                self.config.min_sources_required
            )));
        }

        // Validate and aggregate prices
        let validated_price = self.validate_and_aggregate_prices(
            token_address,
            chain_id,
            source_prices,
        ).await?;

        // Check for anomalies
        let anomaly_detected = self.detect_price_anomaly(&validated_price).await?;

        let mut final_price = validated_price;
        final_price.anomaly_detected = anomaly_detected;

        // Cache the validated price
        let cached_price = CachedPrice {
            token_address: final_price.token_address.clone(),
            chain_id: final_price.chain_id,
            price_usd: final_price.price_usd.clone(),
            timestamp: final_price.timestamp,
        };
        self.cache_manager.price_cache.set(&cache_key, cached_price).await?;

        // Update price history for anomaly detection
        self.update_price_history(token_address, &final_price.price_usd);

        info!("Validated price for {}:{} = ${} (confidence: {:.2}%)", 
              token_address, chain_id, final_price.price_usd, final_price.confidence_score * 100.0);

        Ok(final_price)
    }

    /// Fetch prices from multiple sources concurrently
    async fn fetch_from_multiple_sources(
        &self,
        token_address: &str,
        chain_id: i32,
    ) -> Result<HashMap<String, BigDecimal>, AppError> {
        let mut source_prices = HashMap::new();
        let enabled_sources: Vec<_> = self.sources.values()
            .filter(|source| source.enabled)
            .collect();

        info!("Fetching prices from {} sources for {}:{}", 
              enabled_sources.len(), token_address, chain_id);

        // Fetch from each source concurrently
        let mut fetch_tasks = Vec::new();
        
        for source in enabled_sources {
            let source_name = source.name.clone();
            let _token_addr = token_address.to_lowercase();
            let fault_tolerant_service = FaultTolerantService::new(
                "price_validation",
                RetryConfig::price_api(),
            );
            
            let task = tokio::spawn(async move {
                let result = fault_tolerant_service.execute(|| async {
                    // Mock price fetching - in production, this would call actual APIs
                    // like CoinGecko, CoinMarketCap, Chainlink, etc.
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    
                    // Simulate different prices from different sources with small variations
                    let base_price = 1000.0; // Mock base price
                    let variation = match source_name.as_str() {
                        "coingecko" => 0.98,
                        "coinmarketcap" => 1.02,
                        "chainlink" => 1.00,
                        _ => 1.01,
                    };
                    
                    let final_price = (base_price * variation) as i64;
                    Ok::<BigDecimal, AppError>(BigDecimal::from(final_price))
                }).await;
                
                (source_name, result)
            });
            
            fetch_tasks.push(task);
        }

        // Collect results
        for task in fetch_tasks {
            match task.await {
                Ok((source_name, Ok(price))) => {
                    source_prices.insert(source_name, price);
                }
                Ok((source_name, Err(e))) => {
                    warn!("Failed to fetch price from {}: {}", source_name, e);
                }
                Err(e) => {
                    warn!("Task error: {}", e);
                }
            }
        }

        Ok(source_prices)
    }

    /// Validate and aggregate prices from multiple sources
    async fn validate_and_aggregate_prices(
        &self,
        token_address: &str,
        chain_id: i32,
        source_prices: HashMap<String, BigDecimal>,
    ) -> Result<ValidatedPrice, AppError> {
        if source_prices.is_empty() {
            return Err(AppError::ValidationError("No valid prices received".to_string()));
        }

        // Calculate weighted average
        let mut total_weighted_price = BigDecimal::zero();
        let mut total_weight = 0.0;
        let mut sources_used = Vec::new();

        for (source_name, price) in &source_prices {
            if let Some(source) = self.sources.get(source_name) {
                let weight_int = (source.weight * 1000.0) as i64;
                let weight = BigDecimal::from(weight_int) / BigDecimal::from(1000);
                total_weighted_price += price * &weight;
                total_weight += source.weight;
                sources_used.push(source_name.clone());
            }
        }

        if total_weight == 0.0 {
            return Err(AppError::ValidationError("Total weight is zero".to_string()));
        }

        let total_weight_int = (total_weight * 1000.0) as i64;
        let total_weight_bd = BigDecimal::from(total_weight_int) / BigDecimal::from(1000);
        let weighted_average = total_weighted_price / total_weight_bd;

        // Calculate deviation
        let max_deviation = self.calculate_max_deviation(&source_prices, &weighted_average)?;

        // Calculate confidence score
        let confidence_score = self.calculate_confidence_score(&source_prices, max_deviation);

        // Check if deviation exceeds threshold
        if max_deviation > self.config.max_deviation_percent {
            warn!("Price deviation ({:.2}%) exceeds threshold ({:.2}%) for {}:{}", 
                  max_deviation, self.config.max_deviation_percent, token_address, chain_id);
        }

        Ok(ValidatedPrice {
            token_address: token_address.to_string(),
            chain_id,
            price_usd: weighted_average,
            confidence_score,
            sources_used,
            deviation_percent: max_deviation,
            timestamp: chrono::Utc::now(),
            anomaly_detected: false, // Will be set later
        })
    }

    /// Calculate maximum deviation between sources
    fn calculate_max_deviation(
        &self,
        source_prices: &HashMap<String, BigDecimal>,
        average: &BigDecimal,
    ) -> Result<f64, AppError> {
        let mut max_deviation = 0.0;

        for price in source_prices.values() {
            let deviation = if average.is_zero() {
                0.0
            } else {
                let diff = (price - average).abs();
                let percent = (&diff / average) * BigDecimal::from(100);
                percent.to_f64().unwrap_or(0.0)
            };
            
            if deviation > max_deviation {
                max_deviation = deviation;
            }
        }

        Ok(max_deviation)
    }

    /// Calculate confidence score based on source agreement
    fn calculate_confidence_score(&self, source_prices: &HashMap<String, BigDecimal>, deviation: f64) -> f64 {
        let source_count_factor = (source_prices.len() as f64 / 5.0).min(1.0); // Max factor at 5 sources
        let deviation_factor = (1.0 - (deviation / 100.0)).max(0.0); // Lower deviation = higher confidence
        
        (source_count_factor * 0.5 + deviation_factor * 0.5).max(0.1).min(1.0)
    }

    /// Detect price anomalies using historical data
    async fn detect_price_anomaly(&self, validated_price: &ValidatedPrice) -> Result<bool, AppError> {
        let history_key = format!("{}:{}", validated_price.token_address, validated_price.chain_id);
        
        if let Some(price_history) = self.price_history.get(&history_key) {
            if price_history.len() >= 3 {
                // Calculate moving average of last 3 prices
                let recent_avg = price_history.iter()
                    .rev()
                    .take(3)
                    .fold(BigDecimal::zero(), |acc, price| acc + price) / BigDecimal::from(3);

                // Check if current price deviates significantly from recent average
                let deviation = if recent_avg.is_zero() {
                    0.0
                } else {
                    let diff = (&validated_price.price_usd - &recent_avg).abs();
                    let percent = (&diff / &recent_avg) * BigDecimal::from(100);
                    percent.to_f64().unwrap_or(0.0)
                };

                if deviation > self.config.anomaly_threshold {
                    warn!("Price anomaly detected for {}:{} - deviation: {:.2}%", 
                          validated_price.token_address, validated_price.chain_id, deviation);
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Update price history for anomaly detection
    fn update_price_history(&mut self, token_address: &str, price: &BigDecimal) {
        let history_key = format!("{}:{}", token_address, 1); // Simplified for demo
        let history = self.price_history.entry(history_key).or_insert_with(Vec::new);
        
        history.push(price.clone());
        
        // Keep only last 10 prices
        if history.len() > 10 {
            history.remove(0);
        }
    }

    /// Check if cached price is still fresh
    fn is_price_fresh(&self, cached_price: &CachedPrice) -> bool {
        let age = chrono::Utc::now().signed_duration_since(cached_price.timestamp);
        age.num_seconds() < self.config.price_staleness_seconds as i64
    }

    /// Get price validation statistics
    pub async fn get_validation_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        stats.insert("total_sources".to_string(), serde_json::Value::from(self.sources.len()));
        stats.insert("enabled_sources".to_string(), serde_json::Value::from(
            self.sources.values().filter(|s| s.enabled).count()
        ));
        stats.insert("config".to_string(), serde_json::json!({
            "max_deviation_percent": self.config.max_deviation_percent,
            "min_sources_required": self.config.min_sources_required,
            "anomaly_threshold": self.config.anomaly_threshold,
            "price_staleness_seconds": self.config.price_staleness_seconds,
        }));
        
        stats
    }
}

/// Create default price sources for production
pub fn create_default_price_sources() -> Vec<PriceSource> {
    vec![
        PriceSource {
            name: "coingecko".to_string(),
            url: "https://api.coingecko.com/api/v3".to_string(),
            weight: 0.4,
            timeout: Duration::from_secs(5),
            enabled: true,
        },
        PriceSource {
            name: "coinmarketcap".to_string(),
            url: "https://pro-api.coinmarketcap.com/v1".to_string(),
            weight: 0.3,
            timeout: Duration::from_secs(5),
            enabled: true,
        },
        PriceSource {
            name: "chainlink".to_string(),
            url: "https://api.chain.link".to_string(),
            weight: 0.3,
            timeout: Duration::from_secs(10),
            enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_price_validation_config() {
        let config = PriceValidationConfig::default();
        assert_eq!(config.max_deviation_percent, 5.0);
        assert_eq!(config.min_sources_required, 2);
    }

    #[tokio::test]
    async fn test_default_price_sources() {
        let sources = create_default_price_sources();
        assert_eq!(sources.len(), 3);
        assert!(sources.iter().any(|s| s.name == "coingecko"));
        assert!(sources.iter().any(|s| s.name == "coinmarketcap"));
        assert!(sources.iter().any(|s| s.name == "chainlink"));
    }

    #[tokio::test]
    async fn test_price_source_weights() {
        let sources = create_default_price_sources();
        let total_weight: f64 = sources.iter().map(|s| s.weight).sum();
        assert!((total_weight - 1.0).abs() < 0.01); // Should sum to approximately 1.0
    }
}
