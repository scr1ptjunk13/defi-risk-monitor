# DeFi Risk Monitor

A real-time risk monitoring system for DeFi positions across multiple blockchains.

## Features

- Real-time position monitoring across Ethereum, Polygon, and Arbitrum
- Advanced risk calculation algorithms
- Automated alert system (Slack, Discord, Email)
- RESTful API for position and risk data
- PostgreSQL database for historical data
- Comprehensive test suite and benchmarks

## Quick Start

### Prerequisites

- Rust 1.70+
- PostgreSQL 13+
- Docker (optional)

### Setup

1. Clone the repository:
```bash
git clone <repository-url>
cd defi-risk-monitor
```

2. Copy environment configuration:
```bash
cp .env.example .env
```

3. Update `.env` with your configuration values

4. Setup database:
```bash
./scripts/setup_db.sh
```

5. Run the application:
```bash
cargo run
```

### Docker Setup

```bash
docker-compose up -d
```

## API Endpoints

- `GET /health` - Health check
- `GET /positions` - List all positions
- `GET /positions/{id}` - Get specific position
- `GET /risk/calculate` - Calculate risk metrics
- `GET /alerts` - List alerts
- `POST /alerts` - Create alert

## Testing

```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test integration

# Run benchmarks
cargo bench
```

## Architecture

The application is structured into several key modules:

- **Config**: Application configuration management
- **Models**: Data structures for positions, risk configs, and alerts
- **Services**: Core business logic (blockchain, risk calculation, monitoring, alerts)
- **Handlers**: HTTP request handlers
- **Database**: Database connection and migration management
- **Utils**: Utility functions for math and time operations
- **Error**: Custom error types and handling

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

MIT License
