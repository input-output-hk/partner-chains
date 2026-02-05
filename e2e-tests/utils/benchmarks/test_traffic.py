import sys
import os

# Add traffic_benchmarks to path
sys.path.append(os.path.abspath("traffic_benchmarks"))

# Import the traffic_analyzer
from traffic_benchmarks import traffic_analyzer

# Define a simple test
def test_print_format():
    print("Testing traffic analyzer print format...")
    
    # Sample data
    tx_counts = {
        103968: 8,
        103969: 16,
        103970: 24,
        103971: 8
    }
    
    from datetime import datetime
    
    # Sample creation times
    creation_times = {
        103968: datetime.strptime("2026-02-04 21:17:08.672000", "%Y-%m-%d %H:%M:%S.%f"),
        103969: datetime.strptime("2026-02-04 21:17:20.096000", "%Y-%m-%d %H:%M:%S.%f"),
        103970: datetime.strptime("2026-02-04 21:17:34.552000", "%Y-%m-%d %H:%M:%S.%f"),
        103971: datetime.strptime("2026-02-04 21:17:56.895000", "%Y-%m-%d %H:%M:%S.%f")
    }
    
    # Sample finalization times
    finalization_times = {
        103968: datetime.strptime("2026-02-04 21:17:51.922000", "%Y-%m-%d %H:%M:%S.%f"),
        103969: datetime.strptime("2026-02-04 21:18:16.924000", "%Y-%m-%d %H:%M:%S.%f"),
        103970: datetime.strptime("2026-02-04 21:18:35.480000", "%Y-%m-%d %H:%M:%S.%f"),
        103971: datetime.strptime("2026-02-04 21:18:39.128000", "%Y-%m-%d %H:%M:%S.%f")
    }
    
    # Call the function
    print("\nPrinting traffic report...")
    traffic_analyzer.print_traffic_report(tx_counts, creation_times, finalization_times)

if __name__ == "__main__":
    test_print_format()