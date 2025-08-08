# DeFi Risk Monitor - Complete Usage Guide

## 🚀 Quick Start Overview

Your DeFi Risk Monitor consists of three main components:
1. **PostgreSQL Database** (Docker container)
2. **Rust Backend** (API server on port 8080)
3. **Next.js Frontend** (Web app on port 3000)

---

## 📋 Prerequisites

Make sure you have these installed:
- **Docker & Docker Compose** (for PostgreSQL database)
- **Rust & Cargo** (for backend)
- **Node.js & npm** (for frontend)

---

## 🗄️ Step 1: Start the PostgreSQL Database

The database runs in a Docker container and must be started first.

### Start Database:
```bash
cd /home/junk/RustroverProjects/defi-risk-monitor/backend
docker-compose up postgres -d
```

### Verify Database is Running:
```bash
docker ps
# Should show postgres container running on port 5434
```

### Check Database Health:
```bash
docker-compose logs postgres
# Should show "database system is ready to accept connections"
```

### Connect to Database (Optional):
```bash
# Using psql if you have it installed
psql -h localhost -p 5434 -U postgres -d defi_risk_monitor
# Password: password

# Or using Docker:
docker exec -it backend-postgres-1 psql -U postgres -d defi_risk_monitor
```

---

## 🦀 Step 2: Start the Rust Backend

The backend provides the API and handles all DeFi risk calculations.

### Navigate to Backend Directory:
```bash
cd /home/junk/RustroverProjects/defi-risk-monitor/backend
```

### Run Database Migrations (First Time Only):
```bash
# Install sqlx-cli if you don't have it
cargo install sqlx-cli

# Run migrations to set up database schema
sqlx migrate run
```

### Start the Backend Server:
```bash
# Development mode with hot reload
cargo run

# Or build and run optimized version
cargo build --release
./target/release/defi-risk-monitor
```

### Verify Backend is Running:
```bash
# Check if API is responding
curl http://localhost:8080/health
# Should return: {"status":"healthy","timestamp":"..."}
```

### Backend Logs:
The backend will show logs like:
```
[INFO] Starting DeFi Risk Monitor Backend
[INFO] Database connected successfully
[INFO] Server running on http://0.0.0.0:8080
```

---

## 🌐 Step 3: Start the Frontend

The frontend is a Next.js web application that provides the user interface.

### Navigate to Frontend Directory:
```bash
cd /home/junk/RustroverProjects/defi-risk-monitor/frontend
```

### Install Dependencies (First Time Only):
```bash
npm install
# This will install all required packages
```

### Start the Development Server:
```bash
npm run dev
```

### Access the Application:
Open your browser and go to: **http://localhost:3000**

### Frontend Logs:
You'll see output like:
```
ready - started server on 0.0.0.0:3000, url: http://localhost:3000
event - compiled client and server successfully
```

---

## 🔗 Step 4: Verify Everything is Working

### Check All Services:
1. **Database**: `docker ps` should show postgres container
2. **Backend**: `curl http://localhost:8080/health` should return JSON
3. **Frontend**: `http://localhost:3000` should load the web app

### Test the Full Stack:
1. Open **http://localhost:3000** in your browser
2. Navigate to the Risk Dashboard
3. Try creating a test position
4. Check if risk metrics are calculated

---

## 🛠️ Development Workflow

### Daily Development Routine:
```bash
# 1. Start database (if not running)
cd backend && docker-compose up postgres -d

# 2. Start backend (in one terminal)
cd backend && cargo run

# 3. Start frontend (in another terminal)
cd frontend && npm run dev

# 4. Open browser to http://localhost:3000
```

### Making Changes:
- **Backend**: Rust code changes require restart (`Ctrl+C` then `cargo run`)
- **Frontend**: Next.js has hot reload, changes appear automatically
- **Database**: Schema changes require new migrations

---

## 🔧 Troubleshooting

### Database Issues:
```bash
# Database won't start
docker-compose down
docker-compose up postgres -d

# Reset database completely
docker-compose down -v
docker-compose up postgres -d
cd backend && sqlx migrate run
```

### Backend Issues:
```bash
# Compilation errors
cargo check
cargo build

# Database connection errors
# Check .env file has correct DATABASE_URL
# Verify postgres container is running on port 5434
```

### Frontend Issues:
```bash
# Clear cache and reinstall
npm run clear
npm install
npm run dev

# Port already in use
# Kill process on port 3000 or use different port
npm run dev -- -p 3001
```

### Common Port Conflicts:
- **5434**: PostgreSQL database
- **8080**: Rust backend API
- **3000**: Next.js frontend
- **8001**: AI service (if running)

---

## 📊 Using the Application

### Main Features:
1. **Risk Dashboard**: Real-time risk metrics and alerts
2. **Position Management**: Add/edit DeFi positions
3. **Analytics**: Portfolio performance and risk analysis
4. **Monitoring**: System health and alerts

### API Endpoints:
- **Health Check**: `GET http://localhost:8080/health`
- **Positions**: `GET http://localhost:8080/api/positions`
- **Risk Assessment**: `GET http://localhost:8080/api/risk`
- **WebSocket**: `ws://localhost:8080/ws/stream`

### Database Tables:
Key tables in your PostgreSQL database:
- `positions`: DeFi positions and liquidity pools
- `risk_assessments`: Risk calculations and history
- `users`: User accounts and preferences
- `alerts`: Risk alerts and notifications

---

## 🚀 Production Deployment

### Environment Variables:
Update `.env` files with production values:
- Database URLs
- API keys (Alchemy, Infura)
- JWT secrets
- RPC endpoints

### Build for Production:
```bash
# Backend
cd backend
cargo build --release

# Frontend
cd frontend
npm run build
npm start
```

### Docker Deployment:
```bash
# Build and run everything with Docker
docker-compose up --build
```

---

## 📚 Additional Resources

### Project Structure:
- `/backend`: Rust API server and business logic
- `/frontend`: Next.js web application
- `/contracts`: Smart contracts (if any)
- `/ai-service`: AI/ML service for risk analysis

### Key Configuration Files:
- `backend/.env`: Environment variables
- `backend/docker-compose.yml`: Database configuration
- `frontend/.env.local`: Frontend environment
- `backend/Cargo.toml`: Rust dependencies

### Testing:
```bash
# Run backend tests
cd backend && cargo test

# Run comprehensive test suite
./run_comprehensive_tests.sh
```

---

## 🎯 Success Checklist

- [ ] PostgreSQL container running on port 5434
- [ ] Backend API responding on port 8080
- [ ] Frontend loading on port 3000
- [ ] Database migrations applied
- [ ] Can create and view positions
- [ ] Risk calculations working
- [ ] No compilation errors

**You're ready to monitor DeFi risks! 🎉**
