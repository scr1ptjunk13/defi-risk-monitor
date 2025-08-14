use sqlx::PgPool;
use bigdecimal::BigDecimal;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Starting fix for missing user configurations...");
    
    // Connect to database directly
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5434/defi_risk_monitor".to_string());
    
    let pool = PgPool::connect(&database_url).await?;
    
    // Find users with positions but no risk configurations
    // Note: positions table uses user_address, but user_risk_preferences uses user_id
    // We need to find users by address and then get their user_id from users table
    let users_without_configs = sqlx::query!(
        r#"
        SELECT DISTINCT p.user_address, u.id as user_id
        FROM positions p 
        JOIN user_addresses ua ON LOWER(p.user_address) = LOWER(ua.address)
        JOIN users u ON u.id = ua.user_id
        LEFT JOIN user_risk_preferences urc ON u.id = urc.user_id 
        WHERE urc.user_id IS NULL
        "#
    )
    .fetch_all(&pool)
    .await?;
    
    println!("📊 Found {} users with positions but no risk configurations", users_without_configs.len());
    
    for user in &users_without_configs {
        let user_id = user.user_id;
        let user_address = &user.user_address;
        println!("🔧 Creating default risk configuration for user: {} (address: {})", user_id, user_address);
        
        // Insert default user risk preferences
        sqlx::query!(
            r#"
            INSERT INTO user_risk_preferences (
                user_id,
                max_position_size_usd,
                max_protocol_allocation_percent,
                max_single_pool_percent,
                min_liquidity_threshold_usd,
                max_risk_score,
                allowed_protocols,
                blocked_protocols,
                preferred_chains,
                max_slippage_percent,
                auto_rebalance_enabled,
                stop_loss_enabled,
                stop_loss_threshold_percent
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
            ) ON CONFLICT (user_id) DO NOTHING
            "#,
            user_id,
            BigDecimal::from_str("100000.00").unwrap(),   // $100k max position
            BigDecimal::from_str("25.00").unwrap(),       // 25% max protocol allocation
            BigDecimal::from_str("10.00").unwrap(),       // 10% max single pool
            BigDecimal::from_str("10000.00").unwrap(),    // $10k min liquidity
            BigDecimal::from_str("0.70").unwrap(),        // 0.7 max risk score
            serde_json::json!(["uniswap-v3", "aave"]),
            serde_json::json!([]),
            serde_json::json!(["ethereum", "polygon"]),
            BigDecimal::from_str("2.00").unwrap(),        // 2% max slippage
            false,
            false,
            BigDecimal::from_str("10.00").unwrap(),       // 10% stop loss
        )
        .execute(&pool)
        .await?;
        
        // Insert default user settings if they don't exist
        sqlx::query!(
            r#"
            INSERT INTO user_settings (
                user_id,
                email_notifications,
                sms_notifications,
                webhook_notifications,
                risk_tolerance,
                preferred_currency,
                dashboard_layout,
                alert_frequency
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8
            ) ON CONFLICT (user_id) DO NOTHING
            "#,
            user_id,
            true,                        // Email notifications enabled
            false,                       // SMS notifications disabled
            false,                       // Webhook notifications disabled
            "moderate",                  // Moderate risk tolerance
            "USD",                       // USD currency
            serde_json::json!({}),       // Empty dashboard layout
            "immediate",                 // Immediate alerts
        )
        .execute(&pool)
        .await?;
        
        println!("✅ Created default configurations for user: {}", user_id);
    }
    
    println!("🎉 Successfully fixed {} user configurations", users_without_configs.len());
    println!("Monitoring service should now work without errors!");
    
    Ok(())
}
