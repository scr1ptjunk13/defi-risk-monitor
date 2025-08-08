use crate::error::AppError;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GraphService {
    client: Client,
    uniswap_v3_endpoint: String,
}

#[derive(Debug, Serialize)]
struct GraphQuery {
    query: String,
    variables: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct GraphResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphError>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GraphError {
    message: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PoolData {
    id: String,
    #[serde(rename = "volumeUSD")]
    volume_usd: String,
    #[serde(rename = "feesUSD")]
    fees_usd: String,
    #[serde(rename = "totalValueLockedUSD")]
    tvl_usd: String,
    #[serde(rename = "poolDayData")]
    pool_day_data: Vec<PoolDayData>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PoolDayData {
    date: i64,
    #[serde(rename = "volumeUSD")]
    volume_usd: String,
    #[serde(rename = "feesUSD")]
    fees_usd: String,
    #[serde(rename = "tvlUSD")]
    tvl_usd: String,
}

#[derive(Debug, Deserialize)]
struct PoolQueryResponse {
    pool: Option<PoolData>,
}

#[derive(Debug, Clone)]
pub struct PoolVolumeData {
    pub volume_24h_usd: BigDecimal,
    pub fees_24h_usd: BigDecimal,
    pub tvl_usd: BigDecimal,
    pub volume_7d_usd: BigDecimal,
    pub fees_7d_usd: BigDecimal,
}

impl GraphService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            // Uniswap V3 Subgraph endpoint
            uniswap_v3_endpoint: "https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3".to_string(),
        }
    }

    /// Get pool volume and fees data from The Graph
    pub async fn get_pool_volume_data(&self, pool_address: &str) -> Result<PoolVolumeData, AppError> {
        let query = format!(
            r#"
            query GetPoolData($poolId: String!) {{
                pool(id: $poolId) {{
                    id
                    volumeUSD
                    feesUSD
                    totalValueLockedUSD
                    poolDayData(first: 7, orderBy: date, orderDirection: desc) {{
                        date
                        volumeUSD
                        feesUSD
                        tvlUSD
                    }}
                }}
            }}
            "#
        );

        let mut variables = HashMap::new();
        variables.insert("poolId".to_string(), serde_json::Value::String(pool_address.to_lowercase()));

        let graph_query = GraphQuery {
            query,
            variables: Some(variables),
        };

        let response = self.client
            .post(&self.uniswap_v3_endpoint)
            .json(&graph_query)
            .send()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("Graph API request failed: {}", e)))?;

        let graph_response: GraphResponse<PoolQueryResponse> = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("Failed to parse Graph response: {}", e)))?;

        if let Some(errors) = graph_response.errors {
            return Err(AppError::ExternalApiError(format!(
                "Graph API errors: {:?}", 
                errors
            )));
        }

        let pool_data = graph_response
            .data
            .and_then(|d| d.pool)
            .ok_or_else(|| AppError::ExternalApiError("No pool data found".to_string()))?;

        // Parse current totals
        let _total_volume_usd = pool_data.volume_usd.parse::<f64>()
            .map_err(|_| AppError::ExternalApiError("Invalid volume data".to_string()))?;
        let _total_fees_usd = pool_data.fees_usd.parse::<f64>()
            .map_err(|_| AppError::ExternalApiError("Invalid fees data".to_string()))?;
        let tvl_usd = pool_data.tvl_usd.parse::<f64>()
            .map_err(|_| AppError::ExternalApiError("Invalid TVL data".to_string()))?;

        // Calculate 24h and 7d volumes from daily data
        let mut volume_24h = 0.0;
        let mut fees_24h = 0.0;
        let mut volume_7d = 0.0;
        let mut fees_7d = 0.0;

        let now = Utc::now().timestamp();
        let day_seconds = 86400;

        for day_data in &pool_data.pool_day_data {
            let day_volume = day_data.volume_usd.parse::<f64>().unwrap_or(0.0);
            let day_fees = day_data.fees_usd.parse::<f64>().unwrap_or(0.0);
            
            let days_ago = (now - day_data.date) / day_seconds;
            
            if days_ago <= 1 {
                volume_24h += day_volume;
                fees_24h += day_fees;
            }
            
            if days_ago <= 7 {
                volume_7d += day_volume;
                fees_7d += day_fees;
            }
        }

        Ok(PoolVolumeData {
            volume_24h_usd: BigDecimal::from(volume_24h as i64),
            fees_24h_usd: BigDecimal::from(fees_24h as i64),
            tvl_usd: BigDecimal::from(tvl_usd as i64),
            volume_7d_usd: BigDecimal::from(volume_7d as i64),
            fees_7d_usd: BigDecimal::from(fees_7d as i64),
        })
    }

    /// Get historical volume data for charts
    pub async fn get_historical_volume(&self, pool_address: &str, days: i32) -> Result<Vec<(DateTime<Utc>, BigDecimal)>, AppError> {
        let query = format!(
            r#"
            query GetHistoricalVolume($poolId: String!, $days: Int!) {{
                pool(id: $poolId) {{
                    poolDayData(first: $days, orderBy: date, orderDirection: desc) {{
                        date
                        volumeUSD
                    }}
                }}
            }}
            "#
        );

        let mut variables = HashMap::new();
        variables.insert("poolId".to_string(), serde_json::Value::String(pool_address.to_lowercase()));
        variables.insert("days".to_string(), serde_json::Value::Number(days.into()));

        let graph_query = GraphQuery {
            query,
            variables: Some(variables),
        };

        let response = self.client
            .post(&self.uniswap_v3_endpoint)
            .json(&graph_query)
            .send()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("Graph API request failed: {}", e)))?;

        let graph_response: GraphResponse<PoolQueryResponse> = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("Failed to parse Graph response: {}", e)))?;

        let pool_data = graph_response
            .data
            .and_then(|d| d.pool)
            .ok_or_else(|| AppError::ExternalApiError("No pool data found".to_string()))?;

        let mut historical_data = Vec::new();
        
        for day_data in pool_data.pool_day_data {
            let timestamp = DateTime::from_timestamp(day_data.date, 0)
                .unwrap_or_else(|| Utc::now());
            let volume = day_data.volume_usd.parse::<f64>()
                .map(|v| BigDecimal::from(v as i64))
                .unwrap_or_else(|_| BigDecimal::from(0));
            
            historical_data.push((timestamp, volume));
        }

        Ok(historical_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Ignore by default to avoid hitting The Graph in CI
    async fn test_real_graph_integration() {
        let graph_service = GraphService::new();
        
        // Test with USDC/WETH 0.05% pool
        let pool_address = "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640";
        
        let volume_data = graph_service.get_pool_volume_data(pool_address).await;
        
        match volume_data {
            Ok(data) => {
                println!("✅ Graph integration test passed!");
                println!("📊 24h Volume: ${}", data.volume_24h_usd);
                println!("💰 24h Fees: ${}", data.fees_24h_usd);
                println!("🏦 TVL: ${}", data.tvl_usd);
                assert!(data.volume_24h_usd > BigDecimal::from(0));
            }
            Err(e) => {
                println!("❌ Graph integration test failed: {}", e);
                panic!("Graph integration failed");
            }
        }
    }
}
