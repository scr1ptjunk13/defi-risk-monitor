use sqlx::PgPool;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Creating user risk configurations for position holders...");
    
    // Connect to database directly
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5434/defi_risk_monitor".to_string());
    
    let pool = PgPool::connect(&database_url).await?;
    
    // Get all unique user addresses from positions
    let user_addresses = sqlx::query!(
        r#"
        SELECT DISTINCT user_address 
        FROM positions
        "#
    )
    .fetch_all(&pool)
    .await?;
    
    println!("📊 Found {} unique user addresses in positions", user_addresses.len());
    
    for addr_record in &user_addresses {
        let user_address = &addr_record.user_address;
        println!("🔧 Processing user address: {}", user_address);
        
        // First, ensure the user exists in the users table
        let user_result = sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, password_hash, role, is_active)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (username) DO NOTHING
            RETURNING id
            "#,
            Uuid::new_v4(),
            format!("user_{}", &user_address[2..8]), // Use address prefix as username
            format!("{}@example.com", &user_address[2..8]),
            "placeholder_hash",
            "viewer",
            true
        )
        .fetch_optional(&pool)
        .await?;
        
        // Get the user ID (either newly created or existing)
        let user_id = if let Some(user) = user_result {
            user.id
        } else {
            // User already exists, get their ID
            let existing_user = sqlx::query!(
                r#"
                SELECT id FROM users WHERE username = $1
                "#,
                format!("user_{}", &user_address[2..8])
            )
            .fetch_one(&pool)
            .await?;
            existing_user.id
        };
        
        // Create user_addresses entry
        sqlx::query!(
            r#"
            INSERT INTO user_addresses (user_id, address, chain_id, address_type, is_primary)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id, address, chain_id) DO NOTHING
            "#,
            user_id,
            user_address,
            1, // Ethereum mainnet
            "ethereum",
            true
        )
        .execute(&pool)
        .await?;
        
        // Create user risk preferences
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
            serde_json::json!(["uniswap-v3", "aave", "compound"]),
            serde_json::json!([]),
            serde_json::json!(["ethereum", "polygon"]),
            BigDecimal::from_str("2.00").unwrap(),        // 2% max slippage
            false,
            false,
            BigDecimal::from_str("10.00").unwrap(),       // 10% stop loss
        )
        .execute(&pool)
        .await?;
        
        // Create user settings
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
        
        println!("✅ Created complete user configuration for address: {} (user_id: {})", user_address, user_id);
    }
    
    println!("🎉 Successfully created {} user configurations", user_addresses.len());
    println!("All users with positions now have proper risk configurations!");
    
    Ok(())
}
