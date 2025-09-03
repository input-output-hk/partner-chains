#!/bin/bash

# Jolteon Consensus Test Runner
# This script helps run the Jolteon consensus tests with proper environment setup

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Jolteon Consensus Test Runner ===${NC}"

# Check if we're in the right directory
if [ ! -f "pytest.ini" ]; then
    echo -e "${RED}Error: Please run this script from the e2e-tests directory${NC}"
    exit 1
fi

# Check if virtual environment exists
if [ ! -d "venv" ]; then
    echo -e "${YELLOW}Virtual environment not found. Creating one...${NC}"
    python3 -m venv venv
    source venv/bin/activate
    pip install -r requirements.txt
else
    echo -e "${GREEN}Using existing virtual environment${NC}"
    source venv/bin/activate
fi

# Function to run tests
run_tests() {
    local test_type=$1
    local env=${2:-"local"}
    local blockchain=${3:-"substrate"}
    
    echo -e "${BLUE}Running ${test_type} tests...${NC}"
    echo -e "${BLUE}Environment: ${env}, Blockchain: ${blockchain}${NC}"
    
    case $test_type in
        "basic")
            pytest tests/test_jolteon_consensus.py -v -s --env $env --blockchain $blockchain
            ;;
        "advanced")
            pytest tests/test_jolteon_advanced.py -v -s --env $env --blockchain $blockchain
            ;;
        "rpc")
            pytest tests/test_jolteon_consensus_rpc.py -v -s --env $env --blockchain $blockchain
            ;;
        "2chain")
            pytest tests/test_jolteon_two_chain_commit.py -v -s --env $env --blockchain $blockchain
            ;;
        "all")
            pytest tests/ -m jolteon -v -s --env $env --blockchain $blockchain
            ;;
        "smoke")
            pytest tests/test_jolteon_consensus_rpc.py::TestJolteonConsensusRPC::test_replica_state_retrieval -v -s --env $env --blockchain $blockchain
            ;;
        *)
            echo -e "${RED}Unknown test type: ${test_type}${NC}"
            echo -e "${YELLOW}Available types: basic, advanced, all, smoke${NC}"
            exit 1
            ;;
    esac
}

# Function to show help
show_help() {
    echo -e "${BLUE}Usage: $0 [OPTIONS] <test_type>${NC}"
    echo ""
    echo -e "${BLUE}Test Types:${NC}"
    echo "  basic     - Run basic Jolteon consensus tests"
    echo "  advanced  - Run advanced Jolteon consensus tests"
    echo "  rpc       - Run Jolteon consensus RPC tests"
    echo "  2chain    - Run Jolteon 2-chain commit rule tests"
    echo "  all       - Run all Jolteon consensus tests"
    echo "  smoke     - Run single smoke test (JOLTEON-RPC-001)"
    echo ""
    echo -e "${BLUE}Options:${NC}"
    echo "  --env <environment>     - Set test environment (default: local)"
    echo "  --blockchain <type>    - Set blockchain type (default: substrate)"
    echo "  --help                 - Show this help message"
    echo ""
    echo -e "${BLUE}Examples:${NC}"
    echo "  $0 basic                           # Run basic tests with defaults"
    echo "  $0 all --env jolteon_docker       # Run all tests in jolteon_docker env"
    echo "  $0 smoke --blockchain substrate   # Run smoke test for substrate"
    echo "  $0 rpc --env jolteon_docker       # Run RPC-based consensus tests"
    echo "  $0 2chain --env jolteon_docker    # Run 2-chain commit rule tests"
    echo ""
}

# Parse command line arguments
TEST_TYPE=""
ENV="local"
BLOCKCHAIN="substrate"

while [[ $# -gt 0 ]]; do
    case $1 in
        --env)
            ENV="$2"
            shift 2
            ;;
        --blockchain)
            BLOCKCHAIN="$2"
            shift 2
            ;;
        --help)
            show_help
            exit 0
            ;;
        -*)
            echo -e "${RED}Unknown option: $1${NC}"
            show_help
            exit 1
            ;;
        *)
            if [ -z "$TEST_TYPE" ]; then
                TEST_TYPE="$1"
            else
                echo -e "${RED}Multiple test types specified${NC}"
                exit 1
            fi
            shift
            ;;
    esac
done

# Check if test type was specified
if [ -z "$TEST_TYPE" ]; then
    echo -e "${RED}Error: Test type is required${NC}"
    show_help
    exit 1
fi

# Validate test type
case $TEST_TYPE in
    basic|advanced|rpc|2chain|all|smoke)
        ;;
    *)
        echo -e "${RED}Error: Invalid test type: ${TEST_TYPE}${NC}"
        show_help
        exit 1
        ;;
esac

# Show configuration
echo -e "${GREEN}Configuration:${NC}"
echo "  Test Type:   $TEST_TYPE"
echo "  Environment: $ENV"
echo "  Blockchain:  $BLOCKCHAIN"
echo ""

# Run the tests
run_tests "$TEST_TYPE" "$ENV" "$BLOCKCHAIN"

echo -e "${GREEN}=== Test execution completed ===${NC}"
