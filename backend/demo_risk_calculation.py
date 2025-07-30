#!/usr/bin/env python3
"""
REAL DeFi Risk Management Example
This demonstrates exactly what your DeFi Risk Monitor calculates
"""

import json
from decimal import Decimal, getcontext

# Set high precision for financial calculations
getcontext().prec = 28

def calculate_impermanent_loss(initial_price_ratio, current_price_ratio):
    """Calculate impermanent loss percentage"""
    if initial_price_ratio == 0 or current_price_ratio == 0:
        return 0
    
    price_change = current_price_ratio / initial_price_ratio
    sqrt_price_change = price_change ** 0.5
    
    # Impermanent loss formula for 50/50 pools
    il = (2 * sqrt_price_change) / (1 + price_change) - 1
    return il * 100  # Convert to percentage

def calculate_liquidation_risk(current_price, liquidation_price):
    """Calculate how close we are to liquidation"""
    if liquidation_price == 0:
        return 0
    
    distance_to_liquidation = abs(current_price - liquidation_price) / current_price
    risk_percentage = max(0, 100 - (distance_to_liquidation * 100))
    return min(risk_percentage, 100)

def calculate_var_95(position_value, volatility_30d):
    """Calculate 95% Value at Risk"""
    # 95% VaR = position_value * volatility * 1.645 (95% confidence)
    var_95 = position_value * volatility_30d * 1.645
    return var_95

# REAL SCENARIO: $500K Uniswap V3 WETH/USDC Position
print("🚨 REAL DeFi RISK MANAGEMENT EXAMPLE")
print("=" * 50)

# Position Details
position_value = 500000  # $500K USD
weth_amount = 125.0      # 125 WETH
usdc_amount = 250000.0   # 250K USDC

# Market Conditions
initial_eth_price = 2000  # Entry price: $2000/ETH
current_eth_price = 1800  # Current price: $1800/ETH (10% drop)
liquidation_price = 1500 # Liquidation at $1500/ETH
eth_volatility_30d = 0.65 # 65% annualized volatility

print(f"📊 POSITION DETAILS:")
print(f"   Position Value: ${position_value:,}")
print(f"   Assets: {weth_amount} WETH + ${usdc_amount:,} USDC")
print(f"   Protocol: Uniswap V3")
print(f"   Chain: Ethereum Mainnet")
print()

print(f"📈 MARKET CONDITIONS:")
print(f"   Entry ETH Price: ${initial_eth_price}")
print(f"   Current ETH Price: ${current_eth_price}")
print(f"   Liquidation Price: ${liquidation_price}")
print(f"   30-day Volatility: {eth_volatility_30d*100}%")
print()

# RISK CALCULATIONS
print("🚨 RISK ANALYSIS RESULTS:")
print("-" * 30)

# 1. Impermanent Loss
initial_ratio = initial_eth_price / 1  # ETH/USD ratio
current_ratio = current_eth_price / 1
il_percentage = calculate_impermanent_loss(initial_ratio, current_ratio)
il_dollar_amount = position_value * (il_percentage / 100)

print(f"1. IMPERMANENT LOSS:")
print(f"   Percentage: {il_percentage:.2f}%")
print(f"   Dollar Amount: -${abs(il_dollar_amount):,.2f}")
print()

# 2. Liquidation Risk
liq_risk = calculate_liquidation_risk(current_eth_price, liquidation_price)
price_drop_to_liquidation = ((current_eth_price - liquidation_price) / current_eth_price) * 100

print(f"2. LIQUIDATION RISK:")
print(f"   Risk Level: {liq_risk:.1f}%")
print(f"   Price drop to liquidation: {price_drop_to_liquidation:.1f}%")
print(f"   Status: {'🔴 HIGH RISK' if liq_risk > 50 else '🟡 MEDIUM RISK' if liq_risk > 25 else '🟢 LOW RISK'}")
print()

# 3. Value at Risk (95% confidence)
var_95 = calculate_var_95(position_value, eth_volatility_30d)
var_percentage = (var_95 / position_value) * 100

print(f"3. VALUE AT RISK (95% confidence):")
print(f"   Daily VaR: ${var_95:,.2f}")
print(f"   VaR Percentage: {var_percentage:.1f}%")
print()

# 4. Overall Risk Score (0-100)
# Weighted combination of all risk factors
il_weight = 0.3
liq_weight = 0.5
var_weight = 0.2

overall_risk = (
    (abs(il_percentage) * il_weight) +
    (liq_risk * liq_weight) +
    (var_percentage * var_weight)
)
overall_risk = min(overall_risk, 100)

print(f"4. OVERALL RISK SCORE:")
print(f"   Score: {overall_risk:.1f}/100")
print(f"   Level: {'🔴 CRITICAL' if overall_risk > 75 else '🟠 HIGH' if overall_risk > 50 else '🟡 MEDIUM' if overall_risk > 25 else '🟢 LOW'}")
print()

# RISK MANAGEMENT RECOMMENDATIONS
print("💡 RISK MANAGEMENT RECOMMENDATIONS:")
print("-" * 40)

if overall_risk > 75:
    print("🚨 IMMEDIATE ACTION REQUIRED:")
    print("   • Consider closing position or reducing size")
    print("   • Add more collateral to avoid liquidation")
    print("   • Set stop-loss orders")
elif overall_risk > 50:
    print("⚠️  HIGH RISK - MONITOR CLOSELY:")
    print("   • Consider partial position closure")
    print("   • Increase monitoring frequency")
    print("   • Prepare contingency plans")
elif overall_risk > 25:
    print("🟡 MEDIUM RISK - STANDARD MONITORING:")
    print("   • Continue regular monitoring")
    print("   • Review position weekly")
else:
    print("🟢 LOW RISK - NORMAL OPERATIONS:")
    print("   • Position is within acceptable risk parameters")

print()
print("📊 JSON OUTPUT (for API integration):")
risk_data = {
    "position_id": "5989026d-733c-4a89-9e64-744d06361a95",
    "timestamp": "2025-07-24T03:10:00Z",
    "position_value_usd": position_value,
    "risk_metrics": {
        "impermanent_loss_percentage": round(il_percentage, 2),
        "impermanent_loss_usd": round(il_dollar_amount, 2),
        "liquidation_risk_percentage": round(liq_risk, 1),
        "value_at_risk_95_usd": round(var_95, 2),
        "overall_risk_score": round(overall_risk, 1)
    },
    "market_data": {
        "current_eth_price": current_eth_price,
        "liquidation_price": liquidation_price,
        "price_drop_to_liquidation_percent": round(price_drop_to_liquidation, 1)
    },
    "risk_level": "CRITICAL" if overall_risk > 75 else "HIGH" if overall_risk > 50 else "MEDIUM" if overall_risk > 25 else "LOW",
    "alerts_triggered": overall_risk > 50
}

print(json.dumps(risk_data, indent=2))
