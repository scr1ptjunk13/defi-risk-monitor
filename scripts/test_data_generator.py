#!/usr/bin/env python3
"""
Test Data Generator for DeFi Risk Monitor

This script generates realistic test data for the DeFi Risk Monitor application,
including positions, pool states, and risk configurations.
"""

import psycopg2
import uuid
import random
import json
from datetime import datetime, timedelta
from decimal import Decimal
import os
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

# Database configuration
DB_CONFIG = {
    'host': os.getenv('DB_HOST', 'localhost'),
    'port': os.getenv('DB_PORT', '5432'),
    'database': os.getenv('DB_NAME', 'defi_risk_monitor'),
    'user': os.getenv('DB_USER', 'postgres'),
    'password': os.getenv('DB_PASSWORD', 'password')
}

# Test data configuration
PROTOCOLS = ['Uniswap V3', 'PancakeSwap V3', 'SushiSwap V3']
CHAINS = [1, 137, 42161]  # Ethereum, Polygon, Arbitrum
FEE_TIERS = [500, 3000, 10000]

# Popular token addresses (mock)
TOKENS = {
    1: {  # Ethereum
        'WETH': '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
        'USDC': '0xA0b86a33E6417c4b0c2b2B82c5c3c8c8c8c8c8c8',
        'USDT': '0xdAC17F958D2ee523a2206206994597C13D831ec7',
        'WBTC': '0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599',
    },
    137: {  # Polygon
        'WMATIC': '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270',
        'USDC': '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174',
        'USDT': '0xc2132D05D31c914a87C6611C10748AEb04B58e8F',
        'WETH': '0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619',
    },
    42161: {  # Arbitrum
        'WETH': '0x82aF49447D8a07e3bd95BD0d56f35241523fBab1',
        'USDC': '0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8',
        'USDT': '0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9',
        'ARB': '0x912CE59144191C1204E64559FE8253a0e49E6548',
    }
}

def connect_db():
    """Connect to the PostgreSQL database."""
    try:
        conn = psycopg2.connect(**DB_CONFIG)
        return conn
    except Exception as e:
        print(f"Error connecting to database: {e}")
        return None

def generate_address():
    """Generate a random Ethereum address."""
    return '0x' + ''.join(random.choices('0123456789abcdef', k=40))

def generate_pool_address():
    """Generate a random pool address."""
    return generate_address()

def generate_user_address():
    """Generate a random user address."""
    return generate_address()

def generate_positions(conn, num_positions=50):
    """Generate test positions."""
    cursor = conn.cursor()
    
    print(f"Generating {num_positions} test positions...")
    
    for i in range(num_positions):
        position_id = str(uuid.uuid4())
        user_address = generate_user_address()
        protocol = random.choice(PROTOCOLS)
        chain_id = random.choice(CHAINS)
        pool_address = generate_pool_address()
        
        # Get random tokens for this chain
        chain_tokens = list(TOKENS[chain_id].values())
        token0_address = random.choice(chain_tokens)
        token1_address = random.choice([t for t in chain_tokens if t != token0_address])
        
        # Generate realistic amounts
        token0_amount = Decimal(str(random.uniform(100, 10000)))
        token1_amount = Decimal(str(random.uniform(1000, 50000)))
        liquidity = Decimal(str(random.uniform(10000, 1000000)))
        
        # Generate tick range
        tick_lower = random.randint(-10000, 0)
        tick_upper = random.randint(0, 10000)
        fee_tier = random.choice(FEE_TIERS)
        
        created_at = datetime.now() - timedelta(days=random.randint(1, 30))
        
        cursor.execute("""
            INSERT INTO positions (
                id, user_address, protocol, pool_address, token0_address, token1_address,
                token0_amount, token1_amount, liquidity, tick_lower, tick_upper,
                fee_tier, chain_id, created_at, updated_at
            ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        """, (
            position_id, user_address, protocol, pool_address, token0_address, token1_address,
            token0_amount, token1_amount, liquidity, tick_lower, tick_upper,
            fee_tier, chain_id, created_at, created_at
        ))
    
    conn.commit()
    print(f"Generated {num_positions} positions successfully!")

def generate_pool_states(conn, num_states=200):
    """Generate test pool states."""
    cursor = conn.cursor()
    
    # Get existing pool addresses
    cursor.execute("SELECT DISTINCT pool_address, chain_id FROM positions")
    pools = cursor.fetchall()
    
    if not pools:
        print("No positions found. Generate positions first.")
        return
    
    print(f"Generating {num_states} pool states for {len(pools)} pools...")
    
    for i in range(num_states):
        pool_address, chain_id = random.choice(pools)
        state_id = str(uuid.uuid4())
        
        # Generate realistic pool state data
        current_tick = random.randint(-10000, 10000)
        sqrt_price_x96 = Decimal(str(random.uniform(1000000, 10000000)))
        liquidity = Decimal(str(random.uniform(100000, 10000000)))
        
        # Generate token prices
        token0_price = Decimal(str(random.uniform(0.1, 5000)))
        token1_price = Decimal(str(random.uniform(0.1, 5000)))
        
        # Calculate TVL and volume
        tvl_usd = Decimal(str(random.uniform(100000, 50000000)))
        volume_24h_usd = Decimal(str(random.uniform(10000, 5000000)))
        fees_24h_usd = volume_24h_usd * Decimal('0.003')  # 0.3% fee
        
        timestamp = datetime.now() - timedelta(hours=random.randint(1, 168))  # Last week
        
        cursor.execute("""
            INSERT INTO pool_states (
                id, pool_address, chain_id, current_tick, sqrt_price_x96, liquidity,
                token0_price_usd, token1_price_usd, tvl_usd, volume_24h_usd, fees_24h_usd, timestamp
            ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
            ON CONFLICT (pool_address, chain_id, timestamp) DO NOTHING
        """, (
            state_id, pool_address, chain_id, current_tick, sqrt_price_x96, liquidity,
            token0_price, token1_price, tvl_usd, volume_24h_usd, fees_24h_usd, timestamp
        ))
    
    conn.commit()
    print(f"Generated pool states successfully!")

def generate_risk_configs(conn):
    """Generate risk configurations for users."""
    cursor = conn.cursor()
    
    # Get unique user addresses
    cursor.execute("SELECT DISTINCT user_address FROM positions")
    users = cursor.fetchall()
    
    if not users:
        print("No users found. Generate positions first.")
        return
    
    print(f"Generating risk configurations for {len(users)} users...")
    
    for (user_address,) in users:
        config_id = str(uuid.uuid4())
        
        # Generate realistic risk thresholds
        max_position_size = Decimal(str(random.uniform(500000, 2000000)))
        liquidation_threshold = Decimal(str(random.uniform(0.75, 0.95)))
        price_impact_threshold = Decimal(str(random.uniform(0.02, 0.10)))
        il_threshold = Decimal(str(random.uniform(0.05, 0.20)))
        volatility_threshold = Decimal(str(random.uniform(0.15, 0.30)))
        correlation_threshold = Decimal(str(random.uniform(0.70, 0.90)))
        
        created_at = datetime.now() - timedelta(days=random.randint(1, 10))
        
        cursor.execute("""
            INSERT INTO risk_configs (
                id, user_address, max_position_size_usd, liquidation_threshold,
                price_impact_threshold, impermanent_loss_threshold, volatility_threshold,
                correlation_threshold, created_at, updated_at
            ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
            ON CONFLICT (user_address) DO NOTHING
        """, (
            config_id, user_address, max_position_size, liquidation_threshold,
            price_impact_threshold, il_threshold, volatility_threshold,
            correlation_threshold, created_at, created_at
        ))
    
    conn.commit()
    print(f"Generated risk configurations successfully!")

def generate_alerts(conn, num_alerts=30):
    """Generate test alerts."""
    cursor = conn.cursor()
    
    # Get existing positions
    cursor.execute("SELECT id FROM positions")
    position_ids = [row[0] for row in cursor.fetchall()]
    
    if not position_ids:
        print("No positions found. Generate positions first.")
        return
    
    print(f"Generating {num_alerts} test alerts...")
    
    alert_types = [
        'risk_threshold_violation',
        'impermanent_loss_warning',
        'price_impact_alert',
        'volatility_spike',
        'liquidity_drop'
    ]
    
    severities = ['low', 'medium', 'high', 'critical']
    
    for i in range(num_alerts):
        alert_id = str(uuid.uuid4())
        position_id = random.choice(position_ids) if random.random() > 0.2 else None
        alert_type = random.choice(alert_types)
        severity = random.choice(severities)
        
        title = f"{alert_type.replace('_', ' ').title()} Alert"
        message = f"Alert triggered for {alert_type}: Risk threshold exceeded"
        
        risk_score = Decimal(str(random.uniform(0.1, 1.0)))
        current_value = Decimal(str(random.uniform(0.05, 0.50)))
        threshold_value = Decimal(str(random.uniform(0.10, 0.30)))
        
        is_resolved = random.random() > 0.7  # 30% chance of being resolved
        resolved_at = datetime.now() - timedelta(hours=random.randint(1, 24)) if is_resolved else None
        created_at = datetime.now() - timedelta(hours=random.randint(1, 72))
        
        cursor.execute("""
            INSERT INTO alerts (
                id, position_id, alert_type, severity, title, message,
                risk_score, current_value, threshold_value, is_resolved,
                resolved_at, created_at
            ) VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
        """, (
            alert_id, position_id, alert_type, severity, title, message,
            risk_score, current_value, threshold_value, is_resolved,
            resolved_at, created_at
        ))
    
    conn.commit()
    print(f"Generated {num_alerts} alerts successfully!")

def main():
    """Main function to generate all test data."""
    print("=== DeFi Risk Monitor Test Data Generator ===")
    
    conn = connect_db()
    if not conn:
        print("Failed to connect to database. Exiting.")
        return
    
    try:
        # Generate test data
        generate_positions(conn, 50)
        generate_pool_states(conn, 200)
        generate_risk_configs(conn)
        generate_alerts(conn, 30)
        
        print("\n=== Test Data Generation Complete ===")
        print("Generated:")
        print("- 50 test positions")
        print("- 200 pool states")
        print("- Risk configurations for all users")
        print("- 30 test alerts")
        print("\nYou can now test the application with realistic data!")
        
    except Exception as e:
        print(f"Error generating test data: {e}")
        conn.rollback()
    finally:
        conn.close()

if __name__ == "__main__":
    main()
