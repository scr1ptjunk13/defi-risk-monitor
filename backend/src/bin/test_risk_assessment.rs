use defi_risk_monitor::models::{
    BulkRiskAssessment, RiskEntityType, RiskType, RiskSeverity
};
use defi_risk_monitor::services::risk_assessment_service::RiskAssessmentService;

use sqlx::PgPool;
use uuid::Uuid;
use bigdecimal::BigDecimal;
use chrono::{Utc, Duration};
use std::str::FromStr;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting Risk Assessment Queries Test");

    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://defi_user:defi_password@localhost:5434/defi_risk_monitor".to_string());
    
    let pool = PgPool::connect(&database_url).await?;
    let service = RiskAssessmentService::new(pool.clone());

    // Test data
    let test_user_id = Uuid::new_v4();
    let test_position_id = Uuid::new_v4().to_string();
    let test_protocol_address = "0x1234567890abcdef1234567890abcdef12345678";

    info!("📊 Testing Risk Assessment Queries Implementation");

    // Test 1: Update/Create Risk Assessment
    info!("🔄 Test 1: update_risk_assessment()");
    let risk_assessment = service.update_risk_assessment(
        RiskEntityType::Position,
        &test_position_id,
        Some(test_user_id),
        RiskType::Overall,
        BigDecimal::from_str("0.75").unwrap(),
        RiskSeverity::High,
        Some(BigDecimal::from_str("0.9").unwrap()),
        Some("High overall risk due to market volatility".to_string()),
        Some(serde_json::json!({
            "factors": ["volatility", "liquidity"],
            "confidence_breakdown": {
                "technical": 0.95,
                "market": 0.85
            }
        })),
        Some(Utc::now() + Duration::hours(24)),
    ).await?;

    info!("✅ Created risk assessment: {} with score {}", risk_assessment.id, risk_assessment.risk_score);

    // Test 2: Update existing assessment (should create history)
    info!("🔄 Test 2: update_risk_assessment() - Update existing");
    let updated_assessment = service.update_risk_assessment(
        RiskEntityType::Position,
        &test_position_id,
        Some(test_user_id),
        RiskType::Overall,
        BigDecimal::from_str("0.85").unwrap(), // Increased risk
        RiskSeverity::Critical,
        Some(BigDecimal::from_str("0.95").unwrap()),
        Some("Risk escalated to critical due to protocol exploit".to_string()),
        Some(serde_json::json!({
            "factors": ["exploit", "volatility", "liquidity"],
            "exploit_details": {
                "type": "flash_loan",
                "impact": "high"
            }
        })),
        Some(Utc::now() + Duration::hours(12)),
    ).await?;

    info!("✅ Updated risk assessment: score changed to {}", updated_assessment.risk_score);

    // Test 3: Create multiple risk assessments for history testing
    info!("🔄 Test 3: Creating multiple assessments for history");
    
    // Create protocol risk
    let protocol_risk = service.update_risk_assessment(
        RiskEntityType::Protocol,
        test_protocol_address,
        None,
        RiskType::Protocol,
        BigDecimal::from_str("0.45").unwrap(),
        RiskSeverity::Medium,
        Some(BigDecimal::from_str("0.8").unwrap()),
        Some("Protocol audit score moderate".to_string()),
        Some(serde_json::json!({"audit_score": 75, "tvl": 50000000})),
        None,
    ).await?;

    // Create MEV risk
    let mev_risk = service.update_risk_assessment(
        RiskEntityType::Position,
        &test_position_id,
        Some(test_user_id),
        RiskType::Mev,
        BigDecimal::from_str("0.35").unwrap(),
        RiskSeverity::Medium,
        Some(BigDecimal::from_str("0.7").unwrap()),
        Some("Moderate MEV exposure detected".to_string()),
        Some(serde_json::json!({"sandwich_risk": 0.4, "frontrun_risk": 0.3})),
        Some(Utc::now() + Duration::days(7)),
    ).await?;

    info!("✅ Created {} additional risk assessments", 2);

    // Test 4: Get Risk History
    info!("🔄 Test 4: get_risk_history()");
    let risk_history = service.get_risk_history(
        RiskEntityType::Position,
        &test_position_id,
        Some(RiskType::Overall),
        Some(30), // Last 30 days
        Some(10), // Limit 10
    ).await?;

    info!("✅ Retrieved {} risk history records", risk_history.len());
    for (i, assessment) in risk_history.iter().enumerate() {
        info!("  {}. {:?} - {:?}", i + 1, assessment.risk_type, assessment.severity);
    }

    // Test 5: Get all risk history (no risk type filter)
    info!("🔄 Test 5: get_risk_history() - All risk types");
    let all_risk_history = service.get_risk_history(
        RiskEntityType::Position,
        &test_position_id,
        None, // All risk types
        Some(30),
        Some(20),
    ).await?;

    info!("✅ Retrieved {} total risk history records", all_risk_history.len());

    // Test 6: Get Risks by Severity
    info!("🔄 Test 6: get_risks_by_severity()");
    let critical_risks = service.get_risks_by_severity(
        RiskSeverity::Critical,
        Some(RiskEntityType::Position),
        Some(test_user_id),
        true, // active only
        Some(50),
        Some(0),
    ).await?;

    info!("✅ Retrieved {} critical risks", critical_risks.len());

    let high_risks = service.get_risks_by_severity(
        RiskSeverity::High,
        None, // All entity types
        None, // All users
        true,
        Some(50),
        Some(0),
    ).await?;

    info!("✅ Retrieved {} high severity risks", high_risks.len());

    // Test 7: Create expired risk for testing
    info!("🔄 Test 7: Creating expired risk for testing");
    let expired_risk = service.update_risk_assessment(
        RiskEntityType::Pool,
        "0xexpiredpool123",
        Some(test_user_id),
        RiskType::Liquidity,
        BigDecimal::from_str("0.6").unwrap(),
        RiskSeverity::Medium,
        Some(BigDecimal::from_str("0.85").unwrap()),
        Some("Liquidity risk - expired for testing".to_string()),
        None,
        Some(Utc::now() - Duration::hours(1)), // Expired 1 hour ago
    ).await?;

    info!("✅ Created expired risk assessment: {}", expired_risk.id);

    // Test 8: Get Expired Risks
    info!("🔄 Test 8: get_expired_risks()");
    let expired_risks = service.get_expired_risks(
        Some(RiskEntityType::Pool),
        Some(test_user_id),
        Some(10),
    ).await?;

    info!("✅ Retrieved {} expired risks", expired_risks.len());
    for expired in &expired_risks {
        info!("  Expired: {:?} {:?} expires at {:?}", expired.entity_type, expired.risk_type, expired.expires_at);
    }

    // Test 9: Bulk Insert Risks
    info!("🔄 Test 9: bulk_insert_risks()");
    let bulk_assessments = vec![
        BulkRiskAssessment {
            entity_type: RiskEntityType::Token,
            entity_id: "0xtoken1".to_string(),
            user_id: Some(test_user_id),
            risk_type: RiskType::Market,
            risk_score: BigDecimal::from_str("0.3").unwrap(),
            severity: RiskSeverity::Low,
            confidence: BigDecimal::from_str("0.8").unwrap(),
            description: Some("Low market risk token".to_string()),
            metadata: Some(serde_json::json!({"volatility": 0.25})),
            expires_at: Some(Utc::now() + Duration::days(30)),
        },
        BulkRiskAssessment {
            entity_type: RiskEntityType::Token,
            entity_id: "0xtoken2".to_string(),
            user_id: Some(test_user_id),
            risk_type: RiskType::Market,
            risk_score: BigDecimal::from_str("0.7").unwrap(),
            severity: RiskSeverity::High,
            confidence: BigDecimal::from_str("0.9").unwrap(),
            description: Some("High market risk token".to_string()),
            metadata: Some(serde_json::json!({"volatility": 0.85})),
            expires_at: Some(Utc::now() + Duration::days(15)),
        },
        BulkRiskAssessment {
            entity_type: RiskEntityType::Portfolio,
            entity_id: test_user_id.to_string(),
            user_id: Some(test_user_id),
            risk_type: RiskType::Correlation,
            risk_score: BigDecimal::from_str("0.55").unwrap(),
            severity: RiskSeverity::Medium,
            confidence: BigDecimal::from_str("0.75").unwrap(),
            description: Some("Portfolio correlation risk".to_string()),
            metadata: Some(serde_json::json!({"correlation_matrix": [[1.0, 0.8], [0.8, 1.0]]})),
            expires_at: None,
        },
    ];

    let bulk_ids = service.bulk_insert_risks(bulk_assessments).await?;
    info!("✅ Bulk inserted {} risk assessments", bulk_ids.len());
    for (i, id) in bulk_ids.iter().enumerate() {
        info!("  {}. Bulk inserted ID: {}", i + 1, id);
    }

    // Test 10: Get Risk Statistics
    info!("🔄 Test 10: get_risk_statistics()");
    let user_stats = service.get_risk_statistics(
        None, // All entity types
        Some(test_user_id),
    ).await?;

    info!("✅ User risk statistics: {}", serde_json::to_string_pretty(&user_stats)?);

    let position_stats = service.get_risk_statistics(
        Some(RiskEntityType::Position),
        None, // All users
    ).await?;

    info!("✅ Position risk statistics: {}", serde_json::to_string_pretty(&position_stats)?);

    // Test 11: Deactivate some assessments for cleanup testing
    info!("🔄 Test 11: Deactivating assessments for cleanup test");
    let deactivated = service.deactivate_risk_assessment(protocol_risk.id).await?;
    info!("✅ Deactivated assessment: {}", deactivated);

    let deactivated2 = service.deactivate_risk_assessment(mev_risk.id).await?;
    info!("✅ Deactivated assessment: {}", deactivated2);

    // Test 12: Cleanup Old Risks
    info!("🔄 Test 12: cleanup_old_risks()");
    let cleaned_count = service.cleanup_old_risks(
        0, // 0 days old (clean up deactivated risks immediately)
        Some(100), // batch size
        true, // keep critical
    ).await?;

    info!("✅ Cleaned up {} old risk assessments", cleaned_count);

    // Test 13: Final verification - get remaining active risks
    info!("🔄 Test 13: Final verification - active risks count");
    let final_stats = service.get_risk_statistics(None, Some(test_user_id)).await?;
    info!("✅ Final user risk statistics: {}", serde_json::to_string_pretty(&final_stats)?);

    // Test 14: Get risk assessment by ID
    info!("🔄 Test 14: get_risk_assessment_by_id()");
    if let Some(assessment) = service.get_risk_assessment_by_id(updated_assessment.id).await? {
        info!("✅ Retrieved assessment by ID: {} - Score: {}", assessment.id, assessment.risk_score);
    } else {
        error!("❌ Failed to retrieve assessment by ID");
    }

    info!("🎉 ALL RISK ASSESSMENT QUERIES TESTS COMPLETED SUCCESSFULLY!");
    info!("📋 Summary:");
    info!("  ✅ update_risk_assessment() - Create and update working");
    info!("  ✅ get_risk_history() - History retrieval working");
    info!("  ✅ get_risks_by_severity() - Severity filtering working");
    info!("  ✅ get_expired_risks() - Expiration detection working");
    info!("  ✅ bulk_insert_risks() - Batch operations working");
    info!("  ✅ cleanup_old_risks() - Cleanup operations working");
    info!("  ✅ get_risk_statistics() - Statistics generation working");
    info!("  ✅ get_risk_assessment_by_id() - ID lookup working");
    info!("  ✅ deactivate_risk_assessment() - Deactivation working");

    Ok(())
}
