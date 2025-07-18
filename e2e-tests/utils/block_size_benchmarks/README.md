# Block Size Benchmarking Scripts

Script calculates block propagation time as a timestamp difference between “Pre-sealed block for proposal” and “Imported #XXX” lines from partner-chains node logs.

## How to use

1. Install `python3`, `pip`
2. Install pandas - `pip install pandas`
3. Gather logs from nodes. Put logs from each node in the dedicated txt file: alice.txt, bob.txt, etc
4. Transform raw Grafana logs to a logs for a particular node: `python3 transformer.py`
5. Extract data from logs: `python3 extractor.py`
6. Generate statistics by node `python3 analyzer.py block_propagation_report.txt analysis.txt`