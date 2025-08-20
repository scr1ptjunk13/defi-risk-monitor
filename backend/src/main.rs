use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;

// Import ALL available working DeFi protocol adapters
use defi_risk_monitor::{
    health,
    adapters::{
        DeFiAdapter,
        UniswapV3Adapter,
        UniswapV2Adapter,
        LidoAdapter,
        RocketPoolAdapter,
        EtherFiAdapter,
        YearnAdapter,
        MorphoBlueAdapter,
        uniswap_v3::EthereumClient as V3EthereumClient,
        uniswap_v2::EthereumClient as V2EthereumClient,
        lido::EthereumClient as LidoEthereumClient,
        rocketpool::EthereumClient as RocketPoolEthereumClient,
        etherfi::EthereumClient as EtherFiEthereumClient,
        yearnfinance::EthereumClient as YearnEthereumClient,
        morphoblue::EthereumClient as MorphoBlueEthereumClient,
    },
};
use axum::{response::Json, extract::Path, http::StatusCode};
use alloy::primitives::Address;
use std::str::FromStr;
use chrono;
// For now, we'll implement a basic ENS resolution fallback
// In production, you'd want to use a proper ENS resolver

// Initialize ALL working DeFi protocol adapters
async fn initialize_adapters(rpc_url: &str, coingecko_api_key: Option<String>) -> Vec<Box<dyn DeFiAdapter>> {
    let mut adapters: Vec<Box<dyn DeFiAdapter>> = Vec::new();
    
    tracing::info!("🚀 Initializing ALL DeFi protocol adapters with RPC: {}", rpc_url);
    
    // Uniswap V3 Adapter
    let v3_client = V3EthereumClient { rpc_url: rpc_url.to_string() };
    match UniswapV3Adapter::new(v3_client) {
        Ok(adapter) => {
            adapters.push(Box::new(adapter));
            tracing::info!("✅ Initialized Uniswap V3 adapter");
        }
        Err(e) => {
            tracing::warn!("❌ Failed to initialize Uniswap V3 adapter: {}", e);
        }
    }
    
    // Uniswap V2 Adapter
    let v2_client = V2EthereumClient { rpc_url: rpc_url.to_string() };
    match UniswapV2Adapter::new(v2_client) {
        Ok(adapter) => {
            adapters.push(Box::new(adapter));
            tracing::info!("✅ Initialized Uniswap V2 adapter");
        }
        Err(e) => {
            tracing::warn!("❌ Failed to initialize Uniswap V2 adapter: {}", e);
        }
    }
    
    // Lido Adapter (Liquid Staking)
    let lido_client = LidoEthereumClient { rpc_url: rpc_url.to_string() };
    match LidoAdapter::new(lido_client) {
        Ok(adapter) => {
            adapters.push(Box::new(adapter));
            tracing::info!("✅ Initialized Lido adapter");
        }
        Err(e) => {
            tracing::warn!("❌ Failed to initialize Lido adapter: {}", e);
        }
    }
    
    // Rocket Pool Adapter (Decentralized Liquid Staking)
    let rocketpool_client = RocketPoolEthereumClient { rpc_url: rpc_url.to_string() };
    match RocketPoolAdapter::new(rocketpool_client) {
        Ok(adapter) => {
            adapters.push(Box::new(adapter));
            tracing::info!("✅ Initialized Rocket Pool adapter");
        }
        Err(e) => {
            tracing::warn!("❌ Failed to initialize Rocket Pool adapter: {}", e);
        }
    }
    
    // EtherFi Adapter (Liquid Staking + EigenLayer Restaking)
    let etherfi_client = EtherFiEthereumClient { rpc_url: rpc_url.to_string() };
    match EtherFiAdapter::new(etherfi_client) {
        Ok(adapter) => {
            adapters.push(Box::new(adapter));
            tracing::info!("✅ Initialized EtherFi adapter");
        }
        Err(e) => {
            tracing::warn!("❌ Failed to initialize EtherFi adapter: {}", e);
        }
    }
    
    // Yearn Finance Adapter (Yield Farming)
    let yearn_client = YearnEthereumClient { rpc_url: rpc_url.to_string() };
    match YearnAdapter::new(yearn_client, None) { // Expects Option<u64> for chain_id
        Ok(adapter) => {
            adapters.push(Box::new(adapter));
            tracing::info!("✅ Initialized Yearn Finance adapter");
        }
        Err(e) => {
            tracing::warn!("❌ Failed to initialize Yearn Finance adapter: {}", e);
        }
    }
    
    // MorphoBlue Adapter (Lending Protocol)
    let morphoblue_client = MorphoBlueEthereumClient { rpc_url: rpc_url.to_string() };
    match MorphoBlueAdapter::new(morphoblue_client, 1) { // Expects u64 for chain_id
        Ok(adapter) => {
            adapters.push(Box::new(adapter));
            tracing::info!("✅ Initialized MorphoBlue adapter");
        }
        Err(e) => {
            tracing::warn!("❌ Failed to initialize MorphoBlue adapter: {}", e);
        }
    }
    
    tracing::info!("🚀 Successfully initialized {} DeFi protocol adapters", adapters.len());
    tracing::info!("📊 Supported protocols: {}", 
        adapters.iter().map(|a| a.protocol_name()).collect::<Vec<_>>().join(", "));
    
    adapters
}

// Helper function to resolve ENS names to addresses
async fn resolve_address(input: &str, _rpc_url: &str) -> Result<Address, String> {
    // First try to parse as a direct address
    if let Ok(addr) = Address::from_str(input) {
        return Ok(addr);
    }
    
    // If it looks like an ENS name, provide common known addresses for testing
    if input.ends_with(".eth") || input.ends_with(".ens") {
        tracing::info!("🔍 Resolving ENS name: {}", input);
        
        // For now, provide some known ENS mappings for testing
        // In production, you'd want to implement proper ENS resolution
        let known_ens = match input.to_lowercase().as_str() {
            "vitalik.eth" => "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            "hayden.eth" => "0x50EC05ADe8280758E2077fcBC08D878D4aef79C3", 
            "uniswap.eth" => "0x1a9C8182C09F50C8318d769245beA52c32BE35BC",
            "aave.eth" => "0x25F2226B597E8F9514B3F68F00f494cF4f286491",
            "compound.eth" => "0x3d9819210A31b4961b30EF54bE2aeD79B9c9Cd3B",
            _ => {
                tracing::warn!("❌ ENS name {} not in known mappings", input);
                return Err(format!("ENS name '{}' not found in known mappings. For testing, try: vitalik.eth, hayden.eth, uniswap.eth", input));
            }
        };
        
        match Address::from_str(known_ens) {
            Ok(addr) => {
                tracing::info!("✅ Resolved {} to {}", input, addr);
                return Ok(addr);
            }
            Err(e) => {
                return Err(format!("Invalid resolved address: {}", e));
            }
        }
    }
    
    Err(format!("Invalid address format: '{}'. Please provide a valid Ethereum address or ENS name (vitalik.eth, hayden.eth, etc.)", input))
}

// API endpoint handlers - Real position fetching from all adapters
async fn get_portfolio_positions(Path(address_str): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("🔍 Fetching portfolio positions for address: {}", address_str);
    
    // Get configuration from environment first (needed for ENS resolution)
    let rpc_url = std::env::var("ETHEREUM_RPC_URL")
        .unwrap_or_else(|_| "https://eth-mainnet.alchemyapi.io/v2/demo".to_string());
    
    // Resolve the address (handles both direct addresses and ENS names)
    let address = match resolve_address(&address_str, &rpc_url).await {
        Ok(addr) => addr,
        Err(error_msg) => {
            tracing::warn!("❌ Address resolution failed: {}", error_msg);
            return Ok(Json(serde_json::json!({
                "success": false,
                "error": "Address resolution failed",
                "message": error_msg
            })));
        }
    };

    // Get coingecko API key (RPC URL already obtained for ENS resolution)
    let coingecko_api_key = std::env::var("COINGECKO_API_KEY").ok();

    // Initialize all adapters
    let adapters = initialize_adapters(&rpc_url, coingecko_api_key).await;
    let mut all_positions = Vec::new();
    let mut errors = Vec::new();
    let mut protocol_stats = std::collections::HashMap::new();

    tracing::info!("📡 Querying {} protocol adapters for positions", adapters.len());

    // Store adapter count before consuming the vector
    let total_adapters = adapters.len();
    
    // Fetch positions from all adapters
    for adapter in adapters {
        let protocol_name = adapter.protocol_name();
        tracing::debug!("🔄 Querying {} for positions...", protocol_name);
        
        match adapter.fetch_positions(address).await {
            Ok(mut positions) => {
                let count = positions.len();
                if count > 0 {
                    tracing::info!("✅ Found {} positions in {}", count, protocol_name);
                    protocol_stats.insert(protocol_name.to_string(), count);
                    all_positions.append(&mut positions);
                } else {
                    tracing::debug!("ℹ️ No positions found in {}", protocol_name);
                }
            }
            Err(e) => {
                tracing::warn!("⚠️ Failed to fetch positions from {}: {}", protocol_name, e);
                errors.push(format!("{}: {}", protocol_name, e));
            }
        }
    }

    // Calculate portfolio summary before converting positions
    let total_value_usd: f64 = all_positions.iter().map(|p| p.value_usd).sum();
    let total_pnl_usd: f64 = all_positions.iter().map(|p| p.pnl_usd).sum();
    let total_positions = all_positions.len();

    // Convert positions to frontend format
    let frontend_positions: Vec<serde_json::Value> = all_positions
        .into_iter()
        .map(|pos| {
            serde_json::json!({
                "id": pos.id,
                "user_id": address_str,
                "protocol": pos.protocol,
                "pool_address": "", // Will be in metadata
                "chain_id": 1, // Default to mainnet
                "token0_address": "", // Will be in metadata
                "token1_address": "", // Will be in metadata
                "position_type": pos.position_type,
                "value_usd": pos.value_usd.to_string(),
                "liquidity": "0", // Will be calculated
                "tick_lower": 0, // Will be in metadata
                "tick_upper": 0, // Will be in metadata
                "pnl_usd": pos.pnl_usd.to_string(),
                "fees_earned_usd": "0.0", // Not tracked in current model
                "impermanent_loss_usd": "0.0", // Not tracked in current model
                "risk_score": 0.5, // Default risk score - will be calculated later
                "is_active": true,
                "created_at": chrono::DateTime::from_timestamp(pos.last_updated as i64, 0)
                    .unwrap_or_default()
                    .to_rfc3339(),
                "updated_at": chrono::DateTime::from_timestamp(pos.last_updated as i64, 0)
                    .unwrap_or_default()
                    .to_rfc3339(),
                "pair": pos.pair,
                "metadata": pos.metadata
            })
        })
        .collect();
    
    tracing::info!("📊 Portfolio Summary: {} positions, ${:.2} total value, ${:.2} PnL", 
        total_positions, total_value_usd, total_pnl_usd);

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "positions": frontend_positions,
            "summary": {
                "total_positions": total_positions,
                "total_value_usd": total_value_usd,
                "total_pnl_usd": total_pnl_usd,
                "protocol_breakdown": protocol_stats,
                "last_updated": chrono::Utc::now().to_rfc3339()
            }
        },
        "errors": if errors.is_empty() { None } else { Some(errors) },
        "meta": {
            "address": address_str,
            "protocols_queried": total_adapters,
            "protocols_with_positions": protocol_stats.len()
        }
    })))
}

async fn get_portfolio_summary() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "total_value_usd": 10000.0,
            "total_pnl_usd": 500.0,
            "pnl_percentage": 5.0,
            "risk_score": 0.65,
            "positions": []
        }
    })))
}

async fn get_portfolio_risk_metrics() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "overall_risk": 0.65,
            "liquidity_risk": 0.4,
            "volatility_risk": 0.7,
            "mev_risk": 0.3,
            "protocol_risk": 0.2,
            "timestamp": "2024-01-01T12:00:00Z"
        }
    })))
}

async fn get_live_risk_alerts() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "success": true,
        "data": []
    })))
}

async fn get_position_risk_heatmap() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "success": true,
        "data": []
    })))
}

async fn get_portfolio_analytics() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "total_return_usd": "500.0",
            "total_return_percentage": "5.0",
            "volatility": "0.25",
            "sharpe_ratio": "1.2",
            "max_drawdown": "0.15"
        }
    })))
}

async fn get_correlation_matrix() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "success": true,
        "data": {}
    })))
}

async fn get_risk_decomposition() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "systematic_risk": 0.4,
            "idiosyncratic_risk": 0.3,
            "concentration_risk": 0.2,
            "liquidity_risk": 0.1
        }
    })))
}

async fn get_stress_test_results() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "success": true,
        "data": []
    })))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize simple logging
    tracing_subscriber::fmt::init();

    info!("🚀 Starting DeFi Risk Monitor - Direct Adapter Integration!");
    
    // Load configuration from environment
    let rpc_url = std::env::var("ETHEREUM_RPC_URL")
        .unwrap_or_else(|_| "https://eth-mainnet.alchemyapi.io/v2/demo".to_string());
    let coingecko_api_key = std::env::var("COINGECKO_API_KEY").ok();
    
    info!("🔗 Using RPC URL: {}", rpc_url);
    info!("🪙 CoinGecko API: {}", if coingecko_api_key.is_some() { "Configured" } else { "Using free tier" });
    
    // Test adapter initialization
    let test_adapters = initialize_adapters(&rpc_url, coingecko_api_key.clone()).await;
    info!("✅ Successfully initialized {} DeFi protocol adapters", test_adapters.len());
    
    // Create AppState for future use
    let _app_state = defi_risk_monitor::AppState {
        rpc_url: rpc_url.clone(),
        coingecko_api_key: coingecko_api_key.clone(),
    };

    // Create lean web server with only working routes (no state)
    let app = Router::new()
        // Health check
        .route("/health", get(health::health_check))
        // Portfolio API endpoints (matching frontend expectations)
        .route("/api/v1/positions/wallet/:address", get(get_portfolio_positions))
        .route("/api/v1/portfolio/summary", get(get_portfolio_summary))
        // Risk Monitor API endpoints
        .route("/api/v1/portfolio-risk-metrics", get(get_portfolio_risk_metrics))
        .route("/api/v1/live-alerts", get(get_live_risk_alerts))
        .route("/api/v1/position-risk-heatmap", get(get_position_risk_heatmap))
        // Advanced Analytics API endpoints
        .route("/api/v1/analytics/portfolio-performance", get(get_portfolio_analytics))
        .route("/api/v1/analytics/correlation-matrix", get(get_correlation_matrix))
        .route("/api/v1/analytics/risk-decomposition", get(get_risk_decomposition))
        .route("/api/v1/analytics/stress-test", get(get_stress_test_results))
        // CORS for frontend
        .layer(CorsLayer::permissive());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("🌐 DeFi Risk Monitor running on http://{}", addr);
    info!("📊 Ready to track positions across all DeFi protocols!");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    // Use axum 0.7 compatible serving - simple approach
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

