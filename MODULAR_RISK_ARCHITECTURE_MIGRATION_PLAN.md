# 🏗️ Modular Risk Architecture Migration Plan

## 🎯 **Executive Summary**

**Problem Identified**: Current adapters are calculating their own risk scores, bypassing the sophisticated risk infrastructure already built. Each protocol has unique risk profiles that require specialized risk models.

**Solution**: Implement a modular risk architecture where adapters only fetch position data, and protocol-specific risk calculators handle risk assessment.

**Strategy**: Build new architecture alongside existing system, test thoroughly, then migrate.

---

## 📋 **Current Architecture Issues**

### ❌ **What's Wrong:**
```
Adapter → Position Data + Risk Score (calculated internally)
```

**Problems:**
- Each adapter duplicates risk calculation logic
- Protocol-specific risk expertise is scattered across adapters
- Existing sophisticated risk services (49KB risk_calculator.rs, 32KB risk_handlers.rs) are unused
- No centralized risk management
- Difficult to maintain and test risk logic across 20+ protocols

### ✅ **Target Architecture:**
```
Adapter → Position Data → Protocol-Specific Risk Calculator → Risk Score → Risk Orchestrator → Portfolio Risk
```

**Benefits:**
- Clean separation of concerns
- Protocol-specific risk expertise in dedicated modules
- Reusable risk infrastructure
- Centralized risk management
- Easy to test and maintain
- Extensible for new protocols

---

## 🏗️ **New Modular Risk Architecture Design**

### **1. Core Components**

#### **A. Protocol Risk Calculator Trait**
```rust
pub trait ProtocolRiskCalculator: Send + Sync {
    async fn calculate_risk(&self, positions: &[Position]) -> Result<ProtocolRiskMetrics, RiskError>;
    fn protocol_name(&self) -> &'static str;
    fn supported_position_types(&self) -> Vec<&'static str>;
    async fn validate_position(&self, position: &Position) -> Result<bool, RiskError>;
}
```

#### **B. Protocol-Specific Risk Calculators**
```rust
// Each protocol gets specialized risk logic
pub struct LidoRiskCalculator {
    // Lido-specific: validator slashing, stETH depeg, withdrawal queue delays
}

pub struct UniswapV3RiskCalculator {
    base_calculator: RiskCalculator, // Reuse existing DEX logic
    // Uniswap-specific: impermanent loss, concentrated liquidity, MEV
}

pub struct AaveRiskCalculator {
    // Aave-specific: liquidation cascades, utilization rates, bad debt
}

pub struct MakerDAORiskCalculator {
    // MakerDAO-specific: collateralization ratios, stability fees, oracle attacks
}

pub struct EigenLayerRiskCalculator {
    // EigenLayer-specific: multi-AVS slashing, operator centralization
}
```

#### **C. Risk Orchestrator**
```rust
pub struct RiskOrchestrator {
    calculators: HashMap<String, Box<dyn ProtocolRiskCalculator>>,
    portfolio_risk_aggregator: PortfolioRiskAggregator,
}
```

#### **D. Clean Adapter Interface**
```rust
// Adapters ONLY fetch position data - NO risk calculation
impl DeFiAdapter for LidoAdapter {
    async fn get_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError>;
    // Remove all risk calculation methods
}
```

### **2. Risk Metrics Hierarchy**

#### **A. Protocol-Specific Risk Metrics**
```rust
pub enum ProtocolRiskMetrics {
    Lido(LidoRiskMetrics),
    UniswapV3(UniswapV3RiskMetrics),
    Aave(AaveRiskMetrics),
    MakerDAO(MakerDAORiskMetrics),
    EigenLayer(EigenLayerRiskMetrics),
    // ... 20+ protocols
}

pub struct LidoRiskMetrics {
    pub validator_slashing_risk: BigDecimal,
    pub steth_depeg_risk: BigDecimal,
    pub withdrawal_queue_risk: BigDecimal,
    pub protocol_governance_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
}

pub struct UniswapV3RiskMetrics {
    pub impermanent_loss_risk: BigDecimal,
    pub concentrated_liquidity_risk: BigDecimal,
    pub mev_risk: BigDecimal,
    pub price_impact_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
}
```

#### **B. Portfolio-Level Risk Aggregation**
```rust
pub struct PortfolioRiskMetrics {
    pub protocol_risks: HashMap<String, ProtocolRiskMetrics>,
    pub cross_protocol_correlation_risk: BigDecimal,
    pub concentration_risk: BigDecimal,
    pub overall_portfolio_risk: BigDecimal,
    pub risk_breakdown: RiskBreakdown,
}
```

---

## 📅 **Implementation Phases**

### **Phase 1: Foundation (Week 1)**
**Goal**: Build core risk architecture without breaking existing system

#### **Tasks:**
1. **Create Risk Architecture Foundation**
   - [ ] Create `src/risk/` module directory
   - [ ] Implement `ProtocolRiskCalculator` trait
   - [ ] Create `RiskOrchestrator` struct
   - [ ] Define protocol-specific risk metrics enums

2. **Protocol Risk Calculator Interfaces**
   - [ ] Create base interfaces for each protocol
   - [ ] Define risk metric structures for top 5 protocols
   - [ ] Implement validation logic

3. **Testing Infrastructure**
   - [ ] Create comprehensive test suite for risk architecture
   - [ ] Mock data generators for different protocols
   - [ ] Integration test framework

#### **Deliverables:**
- `src/risk/mod.rs` - Main risk module
- `src/risk/traits.rs` - Risk calculator traits
- `src/risk/orchestrator.rs` - Risk orchestration logic
- `src/risk/metrics.rs` - Risk metrics definitions
- Comprehensive test suite

### **Phase 2: Protocol-Specific Calculators (Week 2)**
**Goal**: Implement specialized risk calculators for major protocols

#### **Tasks:**
1. **Lido Risk Calculator**
   - [ ] Validator slashing risk assessment
   - [ ] stETH/ETH peg monitoring
   - [ ] Withdrawal queue analysis
   - [ ] Protocol governance risk

2. **Uniswap V3 Risk Calculator**
   - [ ] Reuse existing `RiskCalculator` for DEX logic
   - [ ] Impermanent loss calculations
   - [ ] Concentrated liquidity risk
   - [ ] MEV exposure assessment

3. **Aave Risk Calculator**
   - [ ] Liquidation risk assessment
   - [ ] Utilization rate monitoring
   - [ ] Bad debt risk analysis
   - [ ] Interest rate volatility

4. **MakerDAO Risk Calculator**
   - [ ] Collateralization ratio monitoring
   - [ ] Stability fee impact
   - [ ] Oracle manipulation risk
   - [ ] Liquidation cascade risk

5. **EigenLayer Risk Calculator**
   - [ ] Multi-AVS slashing risk
   - [ ] Operator centralization risk
   - [ ] Restaking penalty assessment

#### **Deliverables:**
- `src/risk/calculators/lido.rs`
- `src/risk/calculators/uniswap_v3.rs`
- `src/risk/calculators/aave.rs`
- `src/risk/calculators/makerdao.rs`
- `src/risk/calculators/eigenlayer.rs`
- Protocol-specific test suites

### **Phase 3: Integration & Testing (Week 3)**
**Goal**: Integrate new risk system with existing infrastructure

#### **Tasks:**
1. **Risk Orchestrator Implementation**
   - [ ] Protocol routing logic
   - [ ] Portfolio risk aggregation
   - [ ] Cross-protocol correlation analysis
   - [ ] Risk threshold management

2. **Service Integration**
   - [ ] Update `MonitoringService` to use new risk architecture
   - [ ] Integrate with existing `RiskAssessmentService`
   - [ ] Update `AlertEngine` to use protocol-specific risks
   - [ ] WebSocket integration for real-time risk updates

3. **API Integration**
   - [ ] Update risk handlers to use new architecture
   - [ ] Maintain backward compatibility
   - [ ] Add new protocol-specific risk endpoints

4. **Comprehensive Testing**
   - [ ] End-to-end integration tests
   - [ ] Performance benchmarking
   - [ ] Load testing with multiple protocols
   - [ ] Risk calculation accuracy validation

#### **Deliverables:**
- Fully integrated risk orchestrator
- Updated services using new architecture
- Comprehensive test results
- Performance benchmarks

### **Phase 4: Migration & Cleanup (Week 4)**
**Goal**: Migrate to new architecture and remove legacy code

#### **Tasks:**
1. **Adapter Cleanup**
   - [ ] Remove risk calculation logic from all adapters
   - [ ] Update adapter interfaces to be data-only
   - [ ] Ensure adapters only return position data
   - [ ] Update adapter tests

2. **Legacy Code Removal**
   - [ ] Remove duplicate risk logic from adapters
   - [ ] Clean up unused risk calculation methods
   - [ ] Update documentation

3. **Production Deployment**
   - [ ] Feature flag for new risk architecture
   - [ ] Gradual rollout strategy
   - [ ] Monitoring and alerting
   - [ ] Rollback plan

4. **Documentation & Training**
   - [ ] Update API documentation
   - [ ] Create protocol risk calculator development guide
   - [ ] Update deployment documentation

#### **Deliverables:**
- Clean, modular codebase
- Production-ready risk architecture
- Complete documentation
- Migration complete

---

## 🧪 **Testing Strategy**

### **Unit Tests**
- Each protocol risk calculator independently tested
- Risk metric calculations validated against known scenarios
- Edge case handling (zero positions, extreme values)

### **Integration Tests**
- Risk orchestrator routing logic
- Portfolio risk aggregation accuracy
- Service integration points
- API endpoint functionality

### **Performance Tests**
- Risk calculation speed for large portfolios
- Memory usage with 20+ protocols
- Concurrent risk calculations
- Cache effectiveness

### **Accuracy Tests**
- Compare risk scores with existing adapter calculations
- Validate against real-world scenarios
- Historical risk prediction accuracy

---

## 📊 **Success Metrics**

### **Technical Metrics**
- [ ] All 20+ protocols have dedicated risk calculators
- [ ] Risk calculation time < 500ms for typical portfolio
- [ ] 100% test coverage for risk calculation logic
- [ ] Zero breaking changes to existing APIs

### **Quality Metrics**
- [ ] Risk scores are more accurate than current adapter-based calculations
- [ ] Protocol experts can easily add new risk factors
- [ ] New protocols can be added without touching existing code
- [ ] Risk explanations are more detailed and protocol-specific

### **Operational Metrics**
- [ ] Successful migration with zero downtime
- [ ] All existing functionality preserved
- [ ] Improved observability and debugging
- [ ] Reduced maintenance overhead

---

## 🚨 **Risk Mitigation**

### **Technical Risks**
- **Risk**: New architecture introduces bugs
- **Mitigation**: Extensive testing, gradual rollout, feature flags

- **Risk**: Performance degradation
- **Mitigation**: Benchmarking, caching strategies, optimization

- **Risk**: Breaking changes to existing APIs
- **Mitigation**: Backward compatibility layer, versioned APIs

### **Business Risks**
- **Risk**: Delayed delivery due to complexity
- **Mitigation**: Phased approach, MVP for core protocols first

- **Risk**: Risk calculation accuracy issues
- **Mitigation**: Validation against existing calculations, expert review

---

## 🎯 **Next Steps**

### **Immediate Actions (Next 24 hours)**
1. [ ] Create `src/risk/` module structure
2. [ ] Implement `ProtocolRiskCalculator` trait
3. [ ] Create basic `RiskOrchestrator` skeleton
4. [ ] Set up testing infrastructure

### **Week 1 Goals**
1. [ ] Complete Phase 1 foundation
2. [ ] Begin Lido risk calculator implementation
3. [ ] Validate architecture with simple test cases

### **Success Criteria for Week 1**
- [ ] New risk architecture compiles and passes basic tests
- [ ] At least one protocol risk calculator working
- [ ] Clear path forward for remaining protocols
- [ ] No impact on existing system functionality

---

## 📝 **File Structure**

```
backend/src/
├── risk/                          # New risk architecture
│   ├── mod.rs                     # Main risk module
│   ├── traits.rs                  # ProtocolRiskCalculator trait
│   ├── orchestrator.rs            # Risk orchestration logic
│   ├── metrics.rs                 # Risk metrics definitions
│   ├── aggregator.rs              # Portfolio risk aggregation
│   └── calculators/               # Protocol-specific calculators
│       ├── mod.rs
│       ├── lido.rs                # Lido risk calculator
│       ├── uniswap_v3.rs          # Uniswap V3 risk calculator
│       ├── aave.rs                # Aave risk calculator
│       ├── makerdao.rs            # MakerDAO risk calculator
│       ├── eigenlayer.rs          # EigenLayer risk calculator
│       └── ...                    # 20+ protocol calculators
├── adapters/                      # Existing adapters (cleaned up)
│   ├── lido.rs                    # Data-only, no risk calculation
│   └── ...
└── services/                      # Existing services (updated)
    ├── risk_calculator.rs         # Keep for DEX protocols
    ├── monitoring_service.rs      # Updated to use new architecture
    └── ...
```

---

## 🎉 **Expected Outcomes**

### **Short Term (1 Month)**
- Clean, modular risk architecture
- Protocol-specific risk expertise properly organized
- Improved risk calculation accuracy
- Better maintainability and testability

### **Medium Term (3 Months)**
- Easy addition of new protocols
- Enhanced risk explanations for users
- Better risk-based alerting
- Improved system performance

### **Long Term (6+ Months)**
- Industry-leading risk assessment capabilities
- Protocol risk expertise becomes competitive advantage
- Easy integration with external risk data providers
- Foundation for advanced risk modeling (ML, AI)

---

**This migration plan transforms your DeFi risk monitor from a collection of independent adapters into a sophisticated, modular risk management platform capable of handling the complexity and nuance of 20+ different DeFi protocols.**
