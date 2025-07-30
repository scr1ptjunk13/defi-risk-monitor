# Makefile for DeFi Risk Monitor Smart Contracts

# Load environment variables
include .env
export

# Default target
.PHONY: help
help:
	@echo "Available commands:"
	@echo "  install     - Install Foundry dependencies"
	@echo "  build       - Build contracts"
	@echo "  test        - Run all tests"
	@echo "  test-v      - Run tests with verbose output"
	@echo "  coverage    - Generate test coverage report"
	@echo "  deploy-sepolia - Deploy to Sepolia testnet"
	@echo "  deploy-mainnet - Deploy to mainnet"
	@echo "  verify      - Verify contracts on Etherscan"
	@echo "  clean       - Clean build artifacts"

# Install dependencies
.PHONY: install
install:
	forge install foundry-rs/forge-std
	forge install OpenZeppelin/openzeppelin-contracts
	forge install Uniswap/v3-core
	forge install Uniswap/v3-periphery

# Build contracts
.PHONY: build
build:
	forge build

# Run tests
.PHONY: test
test:
	forge test

# Run tests with verbose output
.PHONY: test-v
test-v:
	forge test -vvv

# Generate coverage report
.PHONY: coverage
coverage:
	forge coverage --report lcov
	genhtml lcov.info -o coverage/

# Deploy to Sepolia testnet
.PHONY: deploy-sepolia
deploy-sepolia:
	forge script script/Deploy.s.sol:DeployScript --rpc-url $(SEPOLIA_RPC_URL) --broadcast --verify

# Deploy to mainnet
.PHONY: deploy-mainnet
deploy-mainnet:
	@echo "WARNING: Deploying to mainnet!"
	@read -p "Are you sure? (y/N): " confirm && [ "$$confirm" = "y" ]
	forge script script/Deploy.s.sol:DeployScript --rpc-url $(MAINNET_RPC_URL) --broadcast --verify

# Verify contracts
.PHONY: verify
verify:
	forge verify-contract $(CONTRACT_ADDRESS) src/LiquidityPoolFactory.sol:LiquidityPoolFactory --etherscan-api-key $(ETHERSCAN_API_KEY)

# Clean build artifacts
.PHONY: clean
clean:
	forge clean
	rm -rf coverage/

# Format code
.PHONY: fmt
fmt:
	forge fmt

# Run static analysis
.PHONY: analyze
analyze:
	slither src/
