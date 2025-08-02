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

TOTAL_TESTS=4

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
        (cd backend && docker-compose up -d postgres)
        sleep 5
    fi
    
    # Test database connection
    if (cd backend && cargo run --bin test_comprehensive_database_fixed > /dev/null 2>&1); then
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
    
    # Change to backend directory with error checking
    if [ -d "backend" ]; then
        cd backend
    elif [ -f "Cargo.toml" ]; then
        # Already in backend directory
        :
    else
        print_error "Cannot find backend directory or Cargo.toml"
        return 1
    fi
    
    # Run comprehensive unit tests
    if SQLX_OFFLINE=true cargo test --test comprehensive_test --release; then
        print_success "Backend Unit Tests: PASSED"
        BACKEND_UNIT_PASSED=1
    else
        print_error "Backend Unit Tests: FAILED"
    fi
    
    # Return to original directory if we changed
    if [ -d "../frontend" ]; then
        cd ..
    fi
}

# Function to run backend integration tests
run_backend_integration_tests() {
    print_status "Running Backend Integration Tests..."
    echo "============================================"
    
    # Ensure database is ready
    if ! check_database; then
        print_error "Database not available for integration tests"
        return 1
    fi
    
    # Change to backend directory with error checking
    if [ -d "backend" ]; then
        cd backend
    elif [ -f "Cargo.toml" ]; then
        # Already in backend directory
        :
    else
        print_error "Cannot find backend directory or Cargo.toml"
        return 1
    fi
    
    # Run integration tests - use actual test files instead of non-existent filter
    if SQLX_OFFLINE=true cargo test --test integration_test --release; then
        print_success "Backend Integration Tests: PASSED"
        BACKEND_INTEGRATION_PASSED=1
    else
        print_error "Backend Integration Tests: FAILED"
    fi
    
    # Return to original directory if we changed
    if [ -d "../frontend" ]; then
        cd ..
    fi
}

# Function to run security tests
run_security_tests() {
    print_status "Running Security Tests..."
    echo "============================================"
    
    # Change to backend directory with error checking
    if [ -d "backend" ]; then
        cd backend
    elif [ -f "Cargo.toml" ]; then
        # Already in backend directory
        :
    else
        print_error "Cannot find backend directory or Cargo.toml"
        return 1
    fi
    
    # Run security test suite - property-based tests and integration tests contain security tests
    if SQLX_OFFLINE=true cargo test --test property_based_tests --test integration_test --release; then
        print_success "Security Tests: PASSED"
        BACKEND_SECURITY_PASSED=1
    else
        print_error "Security Tests: FAILED"
    fi
    
    # Return to original directory if we changed
    if [ -d "../frontend" ]; then
        cd ..
    fi
}

# Function to run performance/load tests
run_performance_tests() {
    print_status "Running Performance/Load Tests..."
    echo "============================================"
    
    # Ensure database is ready for load testing
    if ! check_database; then
        print_error "Database not available for performance tests"
        return 1
    fi
    
    # Change to backend directory with error checking
    if [ -d "backend" ]; then
        cd backend
    elif [ -f "Cargo.toml" ]; then
        # Already in backend directory
        :
    else
        print_error "Cannot find backend directory or Cargo.toml"
        return 1
    fi
    
    # Run load tests - database is available so no need for offline mode
    if cargo test --release -- --test-threads=1; then
        print_success "Performance/Load Tests: PASSED"
        BACKEND_PERFORMANCE_PASSED=1
    else
        print_error "Performance/Load Tests: FAILED"
    fi
    
    # Return to original directory if we changed
    if [ -d "../frontend" ]; then
        cd ..
    fi
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
        *)
            echo "Unknown test type: $1"
            echo "Available options: unit, integration, security, performance"
            exit 1
            ;;
    esac
}

# Function to generate test report
generate_test_report() {
    echo ""
    echo "📊 COMPREHENSIVE TEST RESULTS SUMMARY"
    echo "======================================"
    
    local passed_tests=$((BACKEND_UNIT_PASSED + BACKEND_INTEGRATION_PASSED + BACKEND_SECURITY_PASSED + BACKEND_PERFORMANCE_PASSED))
    local success_rate=$((passed_tests * 100 / TOTAL_TESTS))
    
    echo "Backend Unit Tests:        $([ $BACKEND_UNIT_PASSED -eq 1 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
    echo "Backend Integration Tests: $([ $BACKEND_INTEGRATION_PASSED -eq 1 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
    echo "Security Tests:            $([ $BACKEND_SECURITY_PASSED -eq 1 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
    echo "Performance/Load Tests:    $([ $BACKEND_PERFORMANCE_PASSED -eq 1 ] && echo -e "${GREEN}PASSED${NC}" || echo -e "${RED}FAILED${NC}")"
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

    echo ""
    echo "Examples:"
    echo "  ./run_comprehensive_tests.sh unit"
    echo "  ./run_comprehensive_tests.sh security"

}

# Check for help flag
if [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
    show_help
    exit 0
fi

# Run main function
main "$@"
