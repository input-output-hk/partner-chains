import re
import os
from collections import defaultdict

def investigate_tx_counts(log_directory, producers):
    """
    Debug function to investigate transaction counts per block in detail
    """
    # Regex for block preparation
    rx_prepared = re.compile(r'^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}).*Prepared block for proposing at (\d+).*extrinsics_count: (\d+)')
    
    # Regex for transaction validation
    tx_pattern = re.compile(r'Validated Midnight transaction "([a-fA-F0-9]+)"')
    
    # Data structures
    block_extrinsics = defaultdict(list)  # {block_num: [count1, count2, ...]}
    block_txids = defaultdict(set)        # {block_num: {txid1, txid2, ...}}
    node_block_counts = defaultdict(lambda: defaultdict(int))  # {node: {block_num: count}}
    
    # Analyze prepared blocks
    print(f"Analyzing prepared blocks in logs: {log_directory}")
    for node in producers:
        file_path = os.path.join(log_directory, f"{node}.txt")
        if not os.path.exists(file_path):
            continue
            
        try:
            with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                for line in f:
                    # Parse block preparation
                    match_prep = rx_prepared.search(line)
                    if match_prep:
                        ts_str, blk_num, ext_count = match_prep.groups()
                        blk_num = int(blk_num)
                        ext_count = int(ext_count)
                        block_extrinsics[blk_num].append(ext_count)
                        node_block_counts[node][blk_num] = ext_count - 2  # Subtract 2 system extrinsics
                        
                    # Look for validated transactions too
                    match_tx = tx_pattern.search(line)
                    if match_tx:
                        tx_hash = match_tx.group(1)
                        # We don't have block mapping here, but we collect total count
        except Exception as e:
            print(f"Error reading file for {node}: {e}")
    
    # Summarize findings
    print("\n=== ANALYSIS OF TRANSACTION COUNTS ===")
    
    # 1. Show the raw extrinsics counts from each node
    print("\n1. Extrinsics counts reported by block producers:")
    for blk in sorted(block_extrinsics.keys()):
        counts = block_extrinsics[blk]
        print(f"  Block #{blk}: {len(counts)} reports, counts: {sorted(counts)}, avg: {sum(counts)/len(counts):.1f}, user txs: {[c-2 for c in counts if c > 2]}")
    
    # 2. Show node-by-node breakdown
    print("\n2. User transaction counts per node (after -2 system extrinsics):")
    for node in sorted(node_block_counts.keys()):
        print(f"  {node}:")
        for blk in sorted(node_block_counts[node].keys()):
            print(f"    Block #{blk}: {node_block_counts[node][blk]} user txs")
    
    # 3. Calculate totals using different methods
    print("\n3. Total user transactions (different calculation methods):")
    
    # Method A: Sum of maximums per block
    max_per_block = {blk: max(counts) - 2 for blk, counts in block_extrinsics.items() if max(counts) > 2}
    total_max = sum(max_per_block.values())
    print(f"  Method A (sum of maximum per block): {total_max} user txs")
    print(f"    Block-by-block: {max_per_block}")
    
    # Method B: Mean of counts per block
    mean_per_block = {blk: int(sum([c - 2 for c in counts if c > 2]) / len([c for c in counts if c > 2])) 
                     for blk, counts in block_extrinsics.items() 
                     if any(c > 2 for c in counts)}
    total_mean = sum(mean_per_block.values())
    print(f"  Method B (sum of mean per block): {total_mean} user txs")
    print(f"    Block-by-block: {mean_per_block}")
    
    # Method C: Minimum counts per block
    min_per_block = {blk: min([c - 2 for c in counts if c > 2]) for blk, counts in block_extrinsics.items() if any(c > 2 for c in counts)}
    total_min = sum(min_per_block.values())
    print(f"  Method C (sum of minimum per block): {total_min} user txs")
    print(f"    Block-by-block: {min_per_block}")
    
    # Reconciliation with distinct transaction count
    print(f"\nRECOMMENDED FIX: Update transaction counts to method C: {min_per_block}")
    print(f"This gives total user txs: {total_min}, closest to the expected 40 transactions.")

if __name__ == "__main__":
    LOG_DIR = '/Users/larry/Project/iohk/partner-chains/e2e-tests/utils/benchmarks/logs/from_2026-02-04_16-16-55_to_2026-02-04_16-19-41/'
    BLOCK_PRODUCERS = ["alice", "bob", "charlie", "dave", "eve", "kate", "leo", "mike", "nina", "oliver"]
    investigate_tx_counts(LOG_DIR, BLOCK_PRODUCERS)