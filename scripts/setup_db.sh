#!/bin/bash

# DeFi Risk Monitor Database Setup Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
DB_NAME="defi_risk_monitor"
DB_USER="postgres"
DB_PASSWORD="password"
DB_HOST="localhost"
DB_PORT="5432"

# Load environment variables if .env file exists
if [ -f .env ]; then
    echo -e "${YELLOW}Loading environment variables from .env file...${NC}"
    export $(grep -v '^#' .env | xargs)
    
    # Extract database connection details from DATABASE_URL if present
    if [ ! -z "$DATABASE_URL" ]; then
        # Parse DATABASE_URL (format: postgresql://user:password@host:port/database)
        DB_USER=$(echo $DATABASE_URL | sed -n 's/.*:\/\/\([^:]*\):.*/\1/p')
        DB_PASSWORD=$(echo $DATABASE_URL | sed -n 's/.*:\/\/[^:]*:\([^@]*\)@.*/\1/p')
        DB_HOST=$(echo $DATABASE_URL | sed -n 's/.*@\([^:]*\):.*/\1/p')
        DB_PORT=$(echo $DATABASE_URL | sed -n 's/.*:\([0-9]*\)\/.*/\1/p')
        DB_NAME=$(echo $DATABASE_URL | sed -n 's/.*\/\([^?]*\).*/\1/p')
    fi
fi

echo -e "${GREEN}=== DeFi Risk Monitor Database Setup ===${NC}"
echo "Database: $DB_NAME"
echo "Host: $DB_HOST:$DB_PORT"
echo "User: $DB_USER"
echo ""

# Check if PostgreSQL is running
echo -e "${YELLOW}Checking PostgreSQL connection...${NC}"
if ! pg_isready -h $DB_HOST -p $DB_PORT -U $DB_USER > /dev/null 2>&1; then
    echo -e "${RED}Error: Cannot connect to PostgreSQL at $DB_HOST:$DB_PORT${NC}"
    echo "Please ensure PostgreSQL is running and accessible."
    echo "You can start PostgreSQL with Docker using:"
    echo "  docker-compose up -d postgres"
    exit 1
fi

echo -e "${GREEN}PostgreSQL is running!${NC}"

# Check if database exists
echo -e "${YELLOW}Checking if database exists...${NC}"
if psql -h $DB_HOST -p $DB_PORT -U $DB_USER -lqt | cut -d \| -f 1 | grep -qw $DB_NAME; then
    echo -e "${GREEN}Database '$DB_NAME' already exists.${NC}"
else
    echo -e "${YELLOW}Creating database '$DB_NAME'...${NC}"
    createdb -h $DB_HOST -p $DB_PORT -U $DB_USER $DB_NAME
    echo -e "${GREEN}Database '$DB_NAME' created successfully!${NC}"
fi

# Run migrations
echo -e "${YELLOW}Running database migrations...${NC}"
if [ -d "migrations" ]; then
    for migration in migrations/*.sql; do
        if [ -f "$migration" ]; then
            echo "  Running $(basename $migration)..."
            psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f "$migration" > /dev/null
        fi
    done
    echo -e "${GREEN}Migrations completed successfully!${NC}"
else
    echo -e "${YELLOW}No migrations directory found. Skipping migrations.${NC}"
fi

# Verify tables were created
echo -e "${YELLOW}Verifying database setup...${NC}"
TABLE_COUNT=$(psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public';" | xargs)

if [ "$TABLE_COUNT" -gt 0 ]; then
    echo -e "${GREEN}Database setup completed successfully!${NC}"
    echo "Tables created: $TABLE_COUNT"
    
    # List created tables
    echo -e "${YELLOW}Created tables:${NC}"
    psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "\dt" | grep -E "^ public"
else
    echo -e "${RED}Warning: No tables found in the database.${NC}"
fi

echo ""
echo -e "${GREEN}=== Setup Complete ===${NC}"
echo "You can now run the application with:"
echo "  cargo run"
echo ""
echo "Or start the full stack with Docker:"
echo "  docker-compose up"
