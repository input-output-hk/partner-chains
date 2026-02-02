
import os
import re
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from datetime import datetime

# Regex for block imports
log_pattern = re.compile(
    r'(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}).*?' 
    r'Imported #(\d+)\s*' 
    r'\((0x[a-f0-9]{4})[^\)]*?' 
    r'(?:→|->|\\xe2\\x86\\x92)\s*' 
    r'(0x[a-f0-9]{4})[^\)]*\)'
)

# Regex for finalization events
finalized_pattern = re.compile(
    r'(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}).*?' 
    r'finalized #(\d+)'
)

def load_logs(log_dir, node_list):
    imports = []
    finalizations = []
    print(f"Reading logs from: {log_dir}")
    
    files_read = 0
    for node in node_list:
        filename = f"{node}.txt"
        file_path = os.path.join(log_dir, filename)
        
        if os.path.exists(file_path):
            files_read += 1
            with open(file_path, 'r', encoding='utf-8', errors='replace') as f:
                for line in f:
                    if "Imported #" in line:
                        match = log_pattern.search(line)
                        if match:
                            ts, height, parent, hash_prefix = match.groups()
                            # Handle different arrow encodings if necessary, but regex handles it
                            dt = datetime.strptime(ts, '%Y-%m-%d %H:%M:%S.%f')
                            imports.append({
                                'Node': node.capitalize(),
                                'Time': dt,
                                'Height': int(height),
                                'Hash': hash_prefix,
                                'Parent': parent,
                            })
                    
                    if "finalized #" in line:
                        match = finalized_pattern.search(line)
                        if match:
                            ts, height = match.groups()
                            dt = datetime.strptime(ts, '%Y-%m-%d %H:%M:%S.%f')
                            finalizations.append({
                                'Node': node.capitalize(),
                                'Time': dt,
                                'FinalizedHeight': int(height),
                            })
    
    imports_df = pd.DataFrame(imports)
    finals_df = pd.DataFrame(finalizations)
    
    # CRITICAL: Deduplicate imports - keep first occurrence of each (Node, Height, Hash)
    if not imports_df.empty:
        before = len(imports_df)
        imports_df = imports_df.sort_values('Time').drop_duplicates(
            subset=['Node', 'Height', 'Hash'], 
            keep='first'
        )
        after = len(imports_df)
        print(f"Processed {files_read} files.")
        print(f"  Raw block imports: {before}")
        print(f"  After dedup: {after} (removed {before - after} duplicates)")
        print(f"  Finalization events: {len(finals_df)}")
    
    return imports_df, finals_df

def calculate_concurrent_tips(imports_df, finalizations_df):
    """
    Calculate concurrent active chain tips over time per node.
    A tip is active from when it's imported until its height is finalized.
    """
    if imports_df.empty:
        return pd.DataFrame()
    
    results = []
    
    for node in imports_df['Node'].unique():
        node_imports = imports_df[imports_df['Node'] == node].copy()
        node_finals = finalizations_df[finalizations_df['Node'] == node].copy() if not finalizations_df.empty else pd.DataFrame()
        
        # Build event timeline
        events = []
        for _, row in node_imports.iterrows():
            events.append({
                'time': row['Time'],
                'type': 'import',
                'height': row['Height'],
                'hash': row['Hash']
            })
        
        if not node_finals.empty:
            for _, row in node_finals.iterrows():
                events.append({
                    'time': row['Time'],
                    'type': 'finalize',
                    'height': row['FinalizedHeight'],
                    'hash': None
                })
        
        events.sort(key=lambda x: x['time'])
        
        # Track state
        active_blocks = {}  # (height, hash) -> True
        finalized_height = 0
        
        for evt in events:
            if evt['type'] == 'import':
                key = (evt['height'], evt['hash'])
                if key not in active_blocks:
                    active_blocks[key] = True
            
            elif evt['type'] == 'finalize':
                finalized_height = max(finalized_height, evt['height'])
                # Remove all blocks at or below finalized height
                active_blocks = {k: v for k, v in active_blocks.items() 
                                if k[0] > finalized_height}
            
            # Calculate metrics at this point in time
            if active_blocks:
                # Group by height
                by_height = {}
                for (h, bhash) in active_blocks.keys():
                    if h not in by_height:
                        by_height[h] = set()
                    by_height[h].add(bhash)
                
                # Total unique tips across all unfinalized heights
                total_tips = sum(len(hashes) for hashes in by_height.values())
                
                # Max competing at any single height
                max_at_height = max(len(hashes) for hashes in by_height.values())
                
                # Number of heights with forks (>1 block)
                forked_heights = sum(1 for hashes in by_height.values() if len(hashes) > 1)
            else:
                total_tips = 0
                max_at_height = 0
                forked_heights = 0
            
            results.append({
                'Node': node,
                'Time': evt['time'],
                'TotalTips': total_tips,
                'MaxAtHeight': max_at_height,
                'ForkedHeights': forked_heights,
                'FinalizedHeight': finalized_height,
            })
    
    return pd.DataFrame(results)

def plot_results(df):
    if df.empty:
        print("No data to plot")
        return
    
    fig, axes = plt.subplots(2, 1, figsize=(14, 10), sharex=True)
    nodes = sorted(df['Node'].unique())
    
    # Plot 1: Total concurrent tips (should match Grafana)
    ax1 = axes[0]
    for node in nodes:
        node_data = df[df['Node'] == node].sort_values('Time')
        ax1.step(node_data['Time'], node_data['TotalTips'], 
                 where='post', label=node, linewidth=1.2, alpha=0.8)
    ax1.set_title("Total Concurrent Chain Tips (Across All Unfinalized Heights)")
    ax1.set_ylabel("Active Tips")
    ax1.axhline(y=1, color='green', linestyle='--', alpha=0.5, label='No forks (1 tip)')
    ax1.set_ylim(0, 10)
    ax1.grid(True, linestyle='--', alpha=0.3)
    ax1.legend(bbox_to_anchor=(1.02, 1), loc='upper left', fontsize=7)
    
    # Plot 2: Max forks at single height
    ax2 = axes[1]
    for node in nodes:
        node_data = df[df['Node'] == node].sort_values('Time')
        ax2.step(node_data['Time'], node_data['MaxAtHeight'], 
                 where='post', label=node, linewidth=1.2, alpha=0.8)
    ax2.set_title("Max Competing Blocks at Any Single Height")
    ax2.set_ylabel("Fork Depth")
    ax2.set_xlabel("Time (UTC)")
    ax2.axhline(y=1, color='green', linestyle='--', alpha=0.5, label='No forks')
    ax2.set_ylim(0, 8)
    ax2.grid(True, linestyle='--', alpha=0.3)
    ax2.legend(bbox_to_anchor=(1.02, 1), loc='upper left', fontsize=7)
    
    for ax in axes:
        ax.xaxis.set_major_formatter(mdates.DateFormatter('%H:%M:%S'))
    
    plt.tight_layout()
    plt.show()

def print_summary(df, imports_df):
    print("\n" + "="*70)
    print("SUMMARY")
    print("="*70)
    
    # Verify deduplication worked
    print("\n--- Unique blocks per node per height ---")
    if not imports_df.empty:
        check = imports_df.groupby(['Node', 'Height'])['Hash'].nunique()
        forked = check[check > 1]
        print(forked)
    
    print("\n--- Max Total Concurrent Tips per Node ---")
    if not df.empty:
        max_tips = df.groupby('Node')['TotalTips'].max().sort_values(ascending=False)
        print(max_tips)
        print(f"\nOverall max concurrent tips: {df['TotalTips'].max()}")
    
    print("\n--- Max Fork Depth (at single height) per Node ---")
    if not df.empty:
        max_depth = df.groupby('Node')['MaxAtHeight'].max().sort_values(ascending=False)
        print(max_depth)
        print(f"\nOverall max fork depth: {df['MaxAtHeight'].max()}")

def analyze_forks(log_dir, nodes):
    imports_df, finals_df = load_logs(log_dir, nodes)
    
    if not imports_df.empty:
        print(f"\nHeight range: {imports_df['Height'].min()} to {imports_df['Height'].max()}")
        print(f"Time range: {imports_df['Time'].min()} to {imports_df['Time'].max()}")
        
        metrics_df = calculate_concurrent_tips(imports_df, finals_df)
        print_summary(metrics_df, imports_df)
        plot_results(metrics_df)
    else:
        print("No data found.")
