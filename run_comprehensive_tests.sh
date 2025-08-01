#!/bin/bash

# Comprehensive Test Suite for DeFi Risk Monitor
# This script runs all critical tests to battle-test the entire system

set -e  # Exit on any error

echo "🚀 Starting Comprehensive DeFi Risk Monitor Test Suite"
echo "======================================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results tracking
BACKEND_UNIT_PASSED=0
BACKEND_INTEGRATION_PASSED=0
BACKEND_SECURITY_PASSED=0
BACKEND_PERFORMANCE_PASSED=0
FRONTEND_E2E_PASSED=0
TOTAL_TESTS=5

print_status() {
    echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $1"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Function to check if PostgreSQL is running
check_database() {
    print_status "Checking database connection..."
    
    if ! docker ps | grep -q postgres; then
        print_warning "PostgreSQL container not running. Starting database..."
        cd backend && docker-compose up -d postgres
        sleep 5
    fi
    
    # Test database connection
    if cd backend && cargo run --bin test_comprehensive_database_fixed > /dev/null 2>&1; then
        print_success "Database connection verified"
        return 0
    else
        print_error "Database connection failed"
        return 1
    fi
}

# Function to run backend unit tests
run_backend_unit_tests() {
    print_status "Running Backend Unit Tests..."
    echo "============================================"
    
    cd backend
    
    # Run comprehensive service unit tests
    if cargo test unit::comprehensive_service_tests --lib --release; then
        print_success "Backend Unit Tests: PASSED"
        BACKEND_UNIT_PASSED=1
    else
        print_error "Backend Unit Tests: FAILED"
    fi
    
    cd ..
}

# Function to run backend integration tests
run_backend_integration_tests() {
    print_status "Running Backend Integration Tests..."
    echo "============================================"
    
    cd backend
    
    # Ensure database is ready
    if ! check_database; then
        print_error "Database not available for integration tests"
        return 1
    fi
    
    # Run comprehensive integration tests
    if cargo test integration::comprehensive_integration_tests --release; then
        print_success "Backend Integration Tests: PASSED"
        BACKEND_INTEGRATION_PASSED=1
    else
        print_error "Backend Integration Tests: FAILED"
    fi
    
    cd ..
}

# Function to run security tests
run_security_tests() {
    print_status "Running Security Tests..."
    echo "============================================"
    
    cd backend
    
    # Run security test suite
    if cargo test security::security_tests --release; then
        print_success "Security Tests: PASSED"
        BACKEND_SECURITY_PASSED=1
    else
        print_error "Security Tests: FAILED"
    fi
    
    cd ..
}

# Function to run performance/load tests
run_performance_tests() {
    print_status "Running Performance/Load Tests..."
    echo "============================================"
    
    cd backend
    
    # Ensure database is ready for load testing
    if ! check_database; then
        print_error "Database not available for performance tests"
        return 1
    fi
    
    # Run load tests
    if cargo test performance::load_tests --release -- --test-threads=1; then
        print_success "Performance/Load Tests: PASSED"
        BACKEND_PERFORMANCE_PASSED=1
    else
        print_error "Performance/Load Tests: FAILED"
    fi
    
    cd ..
}

# Function to run frontend E2E tests
run_frontend_e2e_tests() {
    print_status "Running Frontend E2E Tests..."
    echo "============================================"
    
    cd frontend
    
    # Check if dependencies are installed
    if [ ! -d "node_modules" ]; then
        print_status "Installing frontend dependencies..."
        npm install
    fi
    
    # Install Playwright browsers if needed
    if [ ! -d "node_modules/@playwright" ]; then
        print_status "Installing Playwright..."
        npm install --save-dev @playwright/test
        npx playwright install
    fi
    
    # Start frontend development server in background
    print_status "Starting frontend development server..."
    npm run dev &
    FRONTEND_PID=$!
    
    # Wait for frontend to be ready
    sleep 10
    
    # Run E2E tests
    if npx playwright test; then
        print_success "Frontend E2E Tests: PASSED"
        FRONTEND_E2E_PASSED=1
    else
        print_error "Frontend E2E Tests: FAILED"
    fi
    
    # Stop frontend server
    kill $FRONTEND_PID 2>/dev/null || true
    
    cd ..
}

# Function to run specific test based on argument
run_specific_test() {
    case $1 in
        "unit")
            run_backend_unit_tests
            ;;
        "integration")
            run_backend_integration_tests
            ;;
        "security")
            run_security_tests
            ;;
        "performance")
            run_performance_tests
            ;;
        "e2e")
            run_frontend_e2e_tests
            ;;
        *)
            echo "Unknown test type: $1"
            echo "Available options: unit, integration, security, performance, e2e"
            exit 1
            ;;
    esac
}

# Function to generate test report
generate_test_report() {
    echo ""
    echo "📊 COMPREHENSIVE TEST RESULTS SUMMARY"
    echo "======================================"
    
    local passed_tests=$((BACKEND_UNIT_PASSED + BACKEND_INTEGRATION_PASSED + BACKEND_SECURITY_PASSED + BACKEND_PERFORMANCE_PASSED + FRONTEND_E2E_PASSED))
    local success_rate=$((passed_tests * 100 / TOTAL_TESTS))
    
    echo "Backend Unit Tests:        $([ $BACKEND_UNIT_PASSED -eq 1 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
    echo "Backend Integration Tests: $([ $BACKEND_INTEGRATION_PASSED -eq 1 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
    echo "Security Tests:            $([ $BACKEND_SECURITY_PASSED -eq 1 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
    echo "Performance/Load Tests:    $([ $BACKEND_PERFORMANCE_PASSED -eq 1 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
    echo "Frontend E2E Tests:        $([ $FRONTEND_E2E_PASSED -eq 1 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
    echo ""
    echo "Overall Success Rate: ${success_rate}% (${passed_tests}/${TOTAL_TESTS} test suites passed)"
    
    if [ $success_rate -ge 80 ]; then
        print_success "🎉 DeFi Risk Monitor is BATTLE-TESTED and ready for production!"
    elif [ $success_rate -ge 60 ]; then
        print_warning "⚠️  DeFi Risk Monitor has some issues but core functionality works"
    else
        print_error "❌ DeFi Risk Monitor needs significant fixes before production"
    fi
    
    # Save results to file
    echo "Test Results Summary - $(date)" > test_results.txt
    echo "Backend Unit Tests: $([ $BACKEND_UNIT_PASSED -eq 1 ] && echo "PASSED" || echo "FAILED")" >> test_results.txt
    echo "Backend Integration Tests: $([ $BACKEND_INTEGRATION_PASSED -eq 1 ] && echo "PASSED" || echo "FAILED")" >> test_results.txt
    echo "Security Tests: $([ $BACKEND_SECURITY_PASSED -eq 1 ] && echo "PASSED" || echo "FAILED")" >> test_results.txt
    echo "Performance/Load Tests: $([ $BACKEND_PERFORMANCE_PASSED -eq 1 ] && echo "PASSED" || echo "FAILED")" >> test_results.txt
    echo "Frontend E2E Tests: $([ $FRONTEND_E2E_PASSED -eq 1 ] && echo "PASSED" || echo "FAILED")" >> test_results.txt
    echo "Overall Success Rate: ${success_rate}%" >> test_results.txt
    
    print_status "Test results saved to test_results.txt"
}

# Main execution
main() {
    # Check if specific test type is requested
    if [ $# -eq 1 ]; then
        print_status "Running specific test: $1"
        run_specific_test $1
        exit 0
    fi
    
    # Run all tests
    print_status "Running ALL comprehensive tests..."
    echo ""
    
    # Phase 1: Backend Unit Tests (Fastest, run first)
    run_backend_unit_tests
    echo ""
    
    # Phase 2: Security Tests (Critical for DeFi)
    run_security_tests
    echo ""
    
    # Phase 3: Backend Integration Tests (Require database)
    run_backend_integration_tests
    echo ""
    
    # Phase 4: Performance/Load Tests (Resource intensive)
    run_performance_tests
    echo ""
    
    # Phase 5: Frontend E2E Tests (Slowest, run last)
    run_frontend_e2e_tests
    echo ""
    
    # Generate final report
    generate_test_report
}

# Help function
show_help() {
    echo "DeFi Risk Monitor Comprehensive Test Suite"
    echo ""
    echo "Usage:"
    echo "  ./run_comprehensive_tests.sh                 # Run all tests"
    echo "  ./run_comprehensive_tests.sh [test_type]     # Run specific test"
    echo ""
    echo "Available test types:"
    echo "  unit         - Backend unit tests"
    echo "  integration  - Backend integration tests"
    echo "  security     - Security and authentication tests"
    echo "  performance  - Performance and load tests"
    echo "  e2e          - Frontend end-to-end tests"
    echo ""
    echo "Examples:"
    echo "  ./run_comprehensive_tests.sh unit"
    echo "  ./run_comprehensive_tests.sh security"
    echo "  ./run_comprehensive_tests.sh e2e"
}

# Check for help flag
if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
    show_help
    exit 0
fi

# Run main function
main "$@"
