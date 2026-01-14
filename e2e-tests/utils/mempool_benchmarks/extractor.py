#!/usr/bin/env python3

import re
import sys
from datetime import datetime
from typing import Dict, List, Optional, Tuple


class MempoolEvent:
    """Represents a mempool state observation at a point in time."""
    def __init__(self, timestamp: datetime, node: str, ready: int, future: int,
                 mempool_len: Optional[int] = None, submitted_count: Optional[int] = None,
                 validated_count: Optional[int] = None, revalidated: Optional[int] = None,
                 pruned_count: Optional[int] = None, reverified_txs: Optional[int] = None):
        self.timestamp = timestamp
        self.node = node
        self.ready = ready
        self.future = future
        self.mempool_len = mempool_len
        self.submitted_count = submitted_count
        self.validated_count = validated_count
        self.revalidated = revalidated
        self.pruned_count = pruned_count
        self.reverified_txs = reverified_txs


def parse_timestamp(timestamp_str: str) -> Optional[datetime]:
    """Parse timestamp from log line."""
    formats = [
        "%Y-%m-%d %H:%M:%S.%f",  # 2026-01-07 12:32:55.905
        "%Y-%m-%d %H:%M:%S",      # 2026-01-07 12:32:55
    ]
    
    for fmt in formats:
        try:
            return datetime.strptime(timestamp_str, fmt)
        except ValueError:
            continue
    return None


def extract_maintain_event(line: str, timestamp: datetime, node: str) -> Optional[MempoolEvent]:
    """
    Extract data from 'maintain' event.
    Example: 2026-01-07 12:32:55.905 INFO txpool maintain txs=(5, 2) ...
    """
    if "maintain" not in line or "txs=" not in line:
        return None
    
    # Look for txs=(A, B) pattern
    match = re.search(r'txs=\((\d+),\s*(\d+)\)', line)
    if match:
        ready = int(match.group(1))
        future = int(match.group(2))
        
        # Also extract additional metrics if present
        validated_count = extract_value(line, 'validated_count')
        revalidated = extract_value(line, 'revalidated')
        pruned_count = extract_value(line, 'count')
        reverified_txs = extract_value(line, 'reverified_transactions')
        
        return MempoolEvent(timestamp, node, ready, future, 
                          validated_count=validated_count, revalidated=revalidated,
                          pruned_count=pruned_count, reverified_txs=reverified_txs)
    
    return None


def extract_value(line: str, key: str) -> Optional[int]:
    """Extract integer value for a given key from log line."""
    pattern = f'{key}=(\\d+)'
    match = re.search(pattern, line)
    return int(match.group(1)) if match else None


def extract_xts_count_event(line: str, timestamp: datetime, node: str) -> Optional[MempoolEvent]:
    """
    Extract data from 'xts_count' event.
    Example: 2026-01-07 12:32:54.150 DEBUG txpool update_view_with_mempool xts_count=(5, 2)
    """
    if "xts_count=" not in line:
        return None
    
    # Look for xts_count=(A, B) pattern
    match = re.search(r'xts_count=\((\d+),\s*(\d+)\)', line)
    if match:
        ready = int(match.group(1))
        future = int(match.group(2))
        
        # Extract all available metrics
        mempool_len = extract_value(line, 'mempool_len')
        submitted_count = extract_value(line, 'submitted_count')
        validated_count = extract_value(line, 'validated_count')
        revalidated = extract_value(line, 'revalidated')
        pruned_count = extract_value(line, 'count') if 'purge_finalized_transactions' in line else None
        reverified_txs = extract_value(line, 'reverified_transactions')
        
        return MempoolEvent(timestamp, node, ready, future, mempool_len, submitted_count,
                          validated_count, revalidated, pruned_count, reverified_txs)
    
    return None


def extract_update_view_event(line: str, timestamp: datetime, node: str) -> Optional[MempoolEvent]:
    """
    Extract data from 'update_view_with_mempool' event (when it doesn't have xts_count).
    Example: 2026-01-07 12:32:54.150 DEBUG txpool update_view_with_mempool submitted_count=1 mempool_len=7
    """
    if "update_view_with_mempool" not in line:
        return None
    
    # Skip if this line has xts_count (will be handled by extract_xts_count_event)
    if "xts_count=" in line:
        return None
    
    # Extract all available metrics
    mempool_len = extract_value(line, 'mempool_len')
    submitted_count = extract_value(line, 'submitted_count')
    validated_count = extract_value(line, 'validated_count')
    revalidated = extract_value(line, 'revalidated')
    
    # Only return an event if we found at least one metric
    if any(v is not None for v in [mempool_len, submitted_count, validated_count, revalidated]):
        # We don't have ready/future counts here, use -1 as placeholder
        return MempoolEvent(timestamp, node, -1, -1, mempool_len, submitted_count,
                          validated_count, revalidated, None, None)
    
    return None


def extract_pruned_event(line: str, timestamp: datetime, node: str) -> Optional[MempoolEvent]:
    """
    Extract data from 'purge_finalized_transactions' event.
    Example: purge_finalized_transactions count=2
    """
    if "purge_finalized_transactions" not in line or "count=" not in line:
        return None
    
    pruned_count = extract_value(line, 'count')
    if pruned_count is not None:
        return MempoolEvent(timestamp, node, -1, -1, None, None,
                          None, None, pruned_count, None)
    
    return None


def extract_reverified_event(line: str, timestamp: datetime, node: str) -> Optional[MempoolEvent]:
    """
    Extract data from 'reverified_transactions' event.
    Example: reverified_transactions=1
    """
    if "reverified_transactions=" not in line:
        return None
    
    reverified_txs = extract_value(line, 'reverified_transactions')
    if reverified_txs is not None:
        return MempoolEvent(timestamp, node, -1, -1, None, None,
                          None, None, None, reverified_txs)
    
    return None


def extract_revalidation_event(line: str, timestamp: datetime, node: str) -> Optional[MempoolEvent]:
    """
    Extract data from 'view::finish_revalidation' event.
    Example: view::finish_revalidation: applying revalidation result invalid=0 revalidated=22 at_hash=...
    """
    if "view::finish_revalidation" not in line or "revalidated=" not in line:
        return None
    
    revalidated = extract_value(line, 'revalidated')
    if revalidated is not None and revalidated > 0:  # Only capture non-zero revalidations
        return MempoolEvent(timestamp, node, -1, -1, None, None,
                          None, revalidated, None, None)
    
    return None


def extract_validation_event(line: str, timestamp: datetime, node: str) -> Optional[MempoolEvent]:
    """
    Extract data from 'mempool::revalidate_inner' event.
    Example: mempool::revalidate_inner finalized_block=... validated_count=5 total_count=10 ...
    """
    if "mempool::revalidate_inner" not in line or "validated_count=" not in line:
        return None
    
    validated_count = extract_value(line, 'validated_count')
    if validated_count is not None and validated_count > 0:  # Only capture non-zero validations
        return MempoolEvent(timestamp, node, -1, -1, None, None,
                          validated_count, None, None, None)
    
    return None


def parse_logs(nodes: List[str]) -> List[MempoolEvent]:
    """Parse log files for all nodes and extract mempool events."""
    events = []
    
    for node_name in nodes:
        log_file = f"{node_name}.txt"
        
        try:
            with open(log_file, "r") as f:
                for line in f:
                    # Extract timestamp
                    timestamp_match = re.search(
                        r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d{3,6})?)", line
                    )
                    if not timestamp_match:
                        continue
                    
                    timestamp = parse_timestamp(timestamp_match.group(1))
                    if not timestamp:
                        continue
                    
                    # Try to extract different event types
                    event = None
                    
                    # Priority order: maintain > xts_count > update_view_with_mempool > revalidation > validation > pruned > reverified
                    event = extract_maintain_event(line, timestamp, node_name)
                    
                    if not event:
                        event = extract_xts_count_event(line, timestamp, node_name)
                    
                    if not event:
                        event = extract_update_view_event(line, timestamp, node_name)
                    
                    if not event:
                        event = extract_revalidation_event(line, timestamp, node_name)
                    
                    if not event:
                        event = extract_validation_event(line, timestamp, node_name)
                    
                    if not event:
                        event = extract_pruned_event(line, timestamp, node_name)
                    
                    if not event:
                        event = extract_reverified_event(line, timestamp, node_name)
                    
                    if event:
                        events.append(event)
        
        except FileNotFoundError:
            print(f"Warning: Could not find log file {log_file}")
            continue
        except Exception as e:
            print(f"Error reading {log_file}: {e}")
            continue
    
    # Sort events by timestamp
    events.sort(key=lambda e: e.timestamp)
    
    return events


def generate_report(events: List[MempoolEvent]) -> str:
    """Generate a text report from mempool events."""
    report_lines = []
    
    report_lines.append("=== MEMPOOL TIME SERIES DATA ===")
    report_lines.append("")
    report_lines.append(f"Total events: {len(events)}")
    report_lines.append("")
    
    if not events:
        report_lines.append("No mempool events found in logs.")
        return "\n".join(report_lines)
    
    report_lines.append(f"Time range: {events[0].timestamp} to {events[-1].timestamp}")
    report_lines.append("")
    report_lines.append("Column Definitions:")
    report_lines.append("  Ready    - Number of ready transactions in the mempool (transactions that can be included in the next block)")
    report_lines.append("  Future   - Number of future transactions waiting for dependencies (e.g., higher nonce transactions)")
    report_lines.append("  MemLen   - Total number of transactions being tracked in the mempool")
    report_lines.append("  Submit   - Number of new transactions submitted to the mempool in this event")
    report_lines.append("  Valid    - Number of newly submitted transactions validated (initial validation check)")
    report_lines.append("  Reval    - Number of existing transactions revalidated after new blocks are imported")
    report_lines.append("  Pruned   - Number of transactions removed because they were included in finalized blocks")
    report_lines.append("  Reverif  - Number of transactions reverified after a chain reorganization")
    report_lines.append("")
    report_lines.append("-" * 160)
    report_lines.append(
        f"{'Timestamp':<26} {'Node':<10} {'Ready':>7} {'Future':>7} {'MemLen':>7} "
        f"{'Submit':>7} {'Valid':>7} {'Reval':>7} {'Pruned':>7} {'Reverif':>7}"
    )
    report_lines.append("-" * 160)
    
    for event in events:
        timestamp_str = event.timestamp.strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
        ready_str = str(event.ready) if event.ready >= 0 else "N/A"
        future_str = str(event.future) if event.future >= 0 else "N/A"
        mempool_str = str(event.mempool_len) if event.mempool_len is not None else "N/A"
        submitted_str = str(event.submitted_count) if event.submitted_count is not None else "N/A"
        validated_str = str(event.validated_count) if event.validated_count is not None else "N/A"
        revalidated_str = str(event.revalidated) if event.revalidated is not None else "N/A"
        pruned_str = str(event.pruned_count) if event.pruned_count is not None else "N/A"
        reverified_str = str(event.reverified_txs) if event.reverified_txs is not None else "N/A"
        
        # Skip rows where all data columns are N/A
        all_na = all([
            ready_str == "N/A",
            future_str == "N/A",
            mempool_str == "N/A",
            submitted_str == "N/A",
            validated_str == "N/A",
            revalidated_str == "N/A",
            pruned_str == "N/A",
            reverified_str == "N/A"
        ])
        if all_na:
            continue
        
        report_lines.append(
            f"{timestamp_str:<26} {event.node:<10} {ready_str:>7} {future_str:>7} {mempool_str:>7} "
            f"{submitted_str:>7} {validated_str:>7} {revalidated_str:>7} {pruned_str:>7} {reverified_str:>7}"
        )
    
    report_lines.append("-" * 160)
    report_lines.append("")
    
    return "\n".join(report_lines)


def export_events_csv(events: List[MempoolEvent], output_file: str):
    """Export events to CSV for analysis."""
    with open(output_file, 'w') as f:
        # Write header
        f.write("timestamp,node,ready,future,mempool_len,submitted_count,validated_count,revalidated,pruned_count,reverified_txs\n")
        
        # Write data
        for event in events:
            timestamp_str = event.timestamp.strftime("%Y-%m-%d %H:%M:%S.%f")
            ready = event.ready if event.ready >= 0 else ""
            future = event.future if event.future >= 0 else ""
            mempool_len = event.mempool_len if event.mempool_len is not None else ""
            submitted = event.submitted_count if event.submitted_count is not None else ""
            validated = event.validated_count if event.validated_count is not None else ""
            revalidated = event.revalidated if event.revalidated is not None else ""
            pruned = event.pruned_count if event.pruned_count is not None else ""
            reverified = event.reverified_txs if event.reverified_txs is not None else ""
            
            f.write(f"{timestamp_str},{event.node},{ready},{future},{mempool_len},{submitted},{validated},{revalidated},{pruned},{reverified}\n")


def main():
    # Default to ferdie since that's the only node with detailed txpool logs enabled
    nodes = ["ferdie"]
    
    if len(sys.argv) > 1:
        nodes = sys.argv[1:]
    
    print(f"Parsing mempool events from the following nodes: {', '.join(nodes)}")
    print("Note: Only Ferdie has detailed txpool logs enabled by default\n")
    
    events = parse_logs(nodes)
    
    print(f"Found {len(events)} mempool events\n")
    
    print("Generating report...\n")
    report = generate_report(events)
    
    output_file = "mempool_report.txt"
    with open(output_file, "w") as f:
        f.write(report)
    
    print(f"Report saved to {output_file}")
    
    # Export CSV
    csv_file = "mempool_events.csv"
    export_events_csv(events, csv_file)
    print(f"Event CSV saved to {csv_file}")


if __name__ == "__main__":
    main()
