import sys
import os

# Add traffic_benchmarks to path
sys.path.append(os.path.abspath("traffic_benchmarks"))

# Import the traffic_analyzer
from traffic_benchmarks import traffic_analyzer

# Test the fix with actual logs
def test_tx_counting_fix():
    print("Testing fixed transaction counting logic...")
    
    LOG_DIR = '/Users/larry/Project/iohk/partner-chains/e2e-tests/utils/benchmarks/logs/from_2026-02-04_16-16-55_to_2026-02-04_16-19-41/'
    BLOCK_PRODUCERS = ["alice", "bob", "charlie", "dave", "eve", "kate", "leo", "mike", "nina", "oliver"]
    
    tx_counts, creation_times, finalization_times = traffic_analyzer.analyze_block_production(LOG_DIR, BLOCK_PRODUCERS)
    
    print("\nPrinting fixed traffic report...")
    traffic_analyzer.print_traffic_report(tx_counts, creation_times, finalization_times)

if __name__ == "__main__":
    test_tx_counting_fix()