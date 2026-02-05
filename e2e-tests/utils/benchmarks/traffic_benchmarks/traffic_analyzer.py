import os
import re
from datetime import datetime

def count_validated_transactions(log_directory, nodes):
    """
    Reads log files and counts unique 'Validated Midnight transaction' entries
    using regex. Returns a set of all unique TXs, a dict of counts per node,
    and number of files processed.
    """
    total_unique_txs = set()
    node_stats = {}
    files_processed = 0

    # Regex matches: 📋 Validated transaction 59bbd9c722cde20d... for mempool
    # Also supports old format: Validated Midnight transaction "618804..."
    tx_pattern = re.compile(r'Validated (?:Midnight )?transaction (?:")?([a-fA-F0-9]+)(?:")?(?: for mempool)?')

    print(f"Scanning logs in: {log_directory}\n")

    for node in nodes:
        file_path = os.path.join(log_directory, f"{node}.txt")
        # Fallback to .log if .txt doesn't exist (seen in other snippets)
        if not os.path.exists(file_path):
            file_path_log = os.path.join(log_directory, f"{node}.log")
            if os.path.exists(file_path_log):
                file_path = file_path_log
            else:
                node_stats[node] = 0
                continue
        
        node_unique_txs = set()
        try:
            with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                for line in f:
                    match = tx_pattern.search(line)
                    if match:
                        tx_hash = match.group(1)
                        node_unique_txs.add(tx_hash)
            
            # Update stats
            node_stats[node] = len(node_unique_txs)
            total_unique_txs.update(node_unique_txs)
            files_processed += 1
            
        except Exception as e:
            print(f"Error reading file for {node}: {e}")
            node_stats[node] = 0

    return total_unique_txs, node_stats, files_processed

def analyze_block_production(log_directory, producers):
    """
    Analyzes block production logs to determine traffic stats.
    Returns dictionaries for tx counts, creation times, and finalization times.
    """
    # # print("Scanning Block Producers for Active Window (Non-empty blocks)...")  # Disabled to match original
    
    # Regex definitions
    rx_prepared = re.compile(r'^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}).*Prepared block for proposing at (\d+).*extrinsics_count: (\d+)')
    rx_finalized_event = re.compile(r'^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}).*event=Finalized \{ hash: (0x[a-f0-9]+)')
    rx_finalized_idle = re.compile(r'^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}).*finalized #(\d+)')
    rx_imported = re.compile(r'Imported #(\d+) \(.*→ (0x[a-f0-9]+)\)')
    
    # Data structures
    block_tx_counts = {}      # {block_num: tx_count}
    block_creation_times = {} # {block_num: datetime_obj}
    block_finalization_times = {} # {block_num: datetime_obj}
    hash_to_block_num = {}

    for node in producers:
        file_path_txt = os.path.join(log_directory, f"{node}.txt")
        file_path_log = os.path.join(log_directory, f"{node}.log")
        file_path = file_path_txt if os.path.exists(file_path_txt) else file_path_log
        
        if not os.path.exists(file_path):
            continue

        try:
            with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                for line in f:
                    # 1. Parse Block Creation
                    match_prep = rx_prepared.search(line)
                    if match_prep:
                        ts_str, blk_num, ext_count = match_prep.groups()
                        blk_num = int(blk_num)
                        ext_count = int(ext_count)
                        
                        if ext_count > 2:
                            # Subtract 2 system extrinsics (Timestamp + Inherent)
                            user_txs = ext_count - 2
                            
                            # Use the minimum transaction count per block to avoid duplication
                            if blk_num not in block_tx_counts or user_txs < block_tx_counts[blk_num]:
                                block_tx_counts[blk_num] = user_txs
                            
                            ts = datetime.strptime(ts_str, "%Y-%m-%d %H:%M:%S.%f")
                            if blk_num not in block_creation_times or ts < block_creation_times[blk_num]:
                                block_creation_times[blk_num] = ts

                    # 2. Map Hashes to Block Numbers
                    match_imp = rx_imported.search(line)
                    if match_imp:
                        blk_num, blk_hash = match_imp.groups()
                        hash_to_block_num[blk_hash] = int(blk_num)

                    # 3. Parse Standard Finalization Event
                    match_fin = rx_finalized_event.search(line)
                    if match_fin:
                        ts_str, blk_hash = match_fin.groups()
                        if blk_hash in hash_to_block_num:
                            blk_num = hash_to_block_num[blk_hash]
                            ts = datetime.strptime(ts_str, "%Y-%m-%d %H:%M:%S.%f")
                            if blk_num not in block_finalization_times or ts < block_finalization_times[blk_num]:
                                block_finalization_times[blk_num] = ts

                    # 4. Parse Idle Status Finalization
                    match_idle = rx_finalized_idle.search(line)
                    if match_idle:
                        ts_str, blk_num = match_idle.groups()
                        blk_num = int(blk_num)
                        ts = datetime.strptime(ts_str, "%Y-%m-%d %H:%M:%S.%f")
                        if blk_num not in block_finalization_times or ts < block_finalization_times[blk_num]:
                            block_finalization_times[blk_num] = ts

        except Exception as e:
            print(f"Error reading file for {node}: {e}")

    return block_tx_counts, block_creation_times, block_finalization_times

def print_traffic_report(tx_counts, creation_times, finalization_times):
    """
    Prints a detailed report of traffic analysis and returns summary stats.
    """
    TOTAL_TXS_VALIDATED = sum(tx_counts.values())
    MAX_TXS_IN_SINGLE_BLOCK = max(tx_counts.values()) if tx_counts else 0
    
    print(f"{'BLOCK':<8} | {'USER TXS':<10} | {'CREATED AT':<26} | {'FINALIZED AT':<26}")
    print("-" * 75)

    sorted_blocks = sorted(tx_counts.keys())
    
    for blk in sorted_blocks:
        c_time = creation_times.get(blk, "N/A")
        f_time = finalization_times.get(blk, "N/A")
        print(f"#{blk:<7} | {tx_counts[blk]:<10} | {str(c_time):<26} | {str(f_time):<26}")

    print("-" * 75)
    
    duration = 0
    avg_tps = 0
    peak_tps = 0
    start_time = None
    end_time = None
    last_finalized_block = None

    if sorted_blocks and len(sorted_blocks) > 1:
        start_time = creation_times.get(sorted_blocks[0])
        
        # Find the last block that has a finalization time
        for blk in reversed(sorted_blocks):
            if blk in finalization_times:
                last_finalized_block = blk
                break
        
        end_time = finalization_times.get(last_finalized_block) if last_finalized_block else creation_times.get(sorted_blocks[-1])
        
        if start_time and end_time and isinstance(start_time, datetime) and isinstance(end_time, datetime):
            duration = (end_time - start_time).total_seconds()
            if duration < 1: duration = 1
            avg_tps = TOTAL_TXS_VALIDATED / duration
            
            # Calculate Peak Instantaneous TPS for return stats (even if not printed)
            for i in range(1, len(sorted_blocks)):
                b_curr = sorted_blocks[i]
                b_prev = sorted_blocks[i-1]
                t_curr = creation_times.get(b_curr)
                t_prev = creation_times.get(b_prev)
                
                if t_curr and t_prev:
                    delta = (t_curr - t_prev).total_seconds()
                    if delta > 0:
                        inst_tps = tx_counts[b_curr] / delta
                        if inst_tps > peak_tps:
                            peak_tps = inst_tps

    print(f"TOTAL_TXS_VALIDATED: {TOTAL_TXS_VALIDATED}")
    print(f"MAX_TXS_IN_SINGLE_BLOCK: {MAX_TXS_IN_SINGLE_BLOCK}")
    
    if duration > 0:
        print(f"Time Duration (First Block Start -> Last Block Finalized): {duration:.3f} seconds")
        print(f"Start Block ({sorted_blocks[0]}): {start_time}")
        if last_finalized_block:
            print(f"End Block ({last_finalized_block}): {end_time}")
        else:
            print(f"End Block ({sorted_blocks[-1]}): {end_time}")

    print("=" * 50)
    
    return {
        "total_txs": TOTAL_TXS_VALIDATED,
        "max_txs_block": MAX_TXS_IN_SINGLE_BLOCK,
        "duration": duration,
        "avg_tps": avg_tps,
        "peak_tps": peak_tps,
        "start_time": start_time,
        "end_time": end_time,
    }