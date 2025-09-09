# Jolteon Consensus Testing

This directory contains test cases for the Jolteon consensus protocol implementation (Tachyeon) based on the comprehensive test case document.

## Test Categories

### 1. Basic Consensus Tests (`test_jolteon_consensus.py`)
**Test Key: JOLTEON-001** - QC Formation and Round Advancement
- Verifies core Jolteon protocol functionality
- Tests Quorum Certificate (QC) formation
- Monitors round advancement
- **Complexity**: Low - builds on existing block monitoring infrastructure

**Test Key: JOLTEON-002** - Consensus Authority Rotation
- Tests consensus authority management
- Verifies proper leader rotation
- **Complexity**: Low - uses existing authority retrieval methods

**Test Key: JOLTEON-003** - Consensus Metadata Availability
- Exploratory test to understand block structure
- Identifies where consensus information is stored
- **Complexity**: Low - informational only

### 2. Advanced Consensus Tests (`test_jolteon_advanced.py`)
**Test Key: JOLTEON-101** - 2-Chain Commit Rule
- Tests Jolteon's core differentiation from 3-chain HotStuff
- Monitors for consecutive certified blocks
- **Complexity**: Medium - requires understanding of certification vs. commitment

**Test Key: JOLTEON-102** - Consensus Safety Properties
- Verifies fundamental safety guarantees
- Tests for forks, duplicate blocks, sequential numbering
- **Complexity**: Medium - requires historical block analysis

**Test Key: JOLTEON-103** - Consensus Liveness
- Tests system progress over time
- Monitors block production rate and round advancement
- **Complexity**: Medium - requires extended monitoring

## Running the Tests

### Prerequisites
- Jolteon environment running (`jolteon_docker` or similar)
- Access to Substrate node RPC endpoints
- Python test environment with required dependencies

### Basic Test Execution
```bash
# Run all Jolteon consensus tests
pytest tests/ -m jolteon

# Run only basic consensus tests
pytest tests/test_jolteon_consensus.py

# Run only advanced consensus tests
pytest tests/test_jolteon_advanced.py

# Run specific test by key
pytest tests/ -k "JOLTEON-001"
```

### Environment-Specific Execution
```bash
# For jolteon_docker environment
pytest tests/ -m jolteon --env jolteon_docker

# With specific blockchain type
pytest tests/ -m jolteon --blockchain substrate
```

## Understanding the Tests

### What These Tests Verify
1. **Happy Path**: Normal operation under good network conditions
2. **Safety**: No forks, proper block ordering, no duplicates
3. **Liveness**: Continuous progress and round advancement
4. **Protocol Compliance**: Jolteon-specific rules (2-chain commit)

### What These Tests Don't Cover Yet
1. **Fault Tolerance**: Leader failures, network partitions
2. **Performance**: Throughput under various conditions
3. **Edge Cases**: Extreme network conditions, DDoS scenarios
4. **Threshold Signatures**: Cryptographic verification of QCs/TCs

## Customization and Extension

### Adding New Test Cases
1. Follow the existing test structure and naming conventions
2. Include comprehensive logging for debugging
3. Add proper error handling and graceful degradation

### Modifying Test Parameters
- Adjust wait times based on your network characteristics
- Modify block sampling intervals for different consensus speeds
- Customize assertion thresholds based on expected performance

### Environment-Specific Adaptations
- Modify RPC endpoint handling for different Jolteon implementations
- Adjust consensus metadata extraction for different block formats
- Customize authority verification for different governance models

## Troubleshooting

### Common Issues
1. **Round Number Extraction**: May need adjustment based on actual block structure
2. **Certification Detection**: Heuristic-based approach may need refinement
3. **Timing Sensitivity**: Adjust wait times based on network performance
4. **RPC Compatibility**: Ensure Substrate interface version compatibility

### Debug Mode
```bash
# Run with verbose logging
pytest tests/ -m jolteon -v -s --log-cli-level=DEBUG

# Run single test with detailed output
pytest tests/test_jolteon_consensus.py::TestJolteonConsensus::test_qc_formation_and_round_advancement -v -s
```

## Contributing

When adding new test cases:
1. Implement tests incrementally (simple → complex)
2. Include proper documentation and logging
3. Test thoroughly in your Jolteon environment
4. Update this README with new test information

## References

- [Jolteon Consensus Protocol Specification](https://eprint.iacr.org/2021/319)
- [Substrate Interface Documentation](https://github.com/polkascan/py-substrate-interface)
- [Partner Chains E2E Testing Guide](../README.md)
