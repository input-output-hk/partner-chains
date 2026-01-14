#!/usr/bin/env python3

import sys
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple

import pandas as pd


@dataclass
class MempoolPoint:
    timestamp: datetime
    node: str
    ready: int
    future: int
    mempool_len: Optional[int]
    submitted_count: Optional[int]
    validated_count: Optional[int]
    revalidated: Optional[int]
    pruned_count: Optional[int]
    reverified_txs: Optional[int]


TS_FMT = "%Y-%m-%d %H:%M:%S.%f"


def parse_report(path: str) -> List[MempoolPoint]:
    points: List[MempoolPoint] = []
    with open(path, "r") as f:
        lines = f.readlines()
    for line in lines:
        line = line.strip()
        if not line or line.startswith("=") or line.startswith("-") or line.startswith("Time range:") or line.startswith("Total events:"):
            continue
        try:
            # Expected columns: Timestamp Node Ready Future MemLen Submit Valid Reval Pruned Reverif
            parts = [p for p in line.split(" ") if p]
            # The formatted line uses aligned spacing; rebuild by slices
            # Timestamp is first two tokens (date + time.millis)
            ts = " ".join(parts[0:2])
            node = parts[2]
            ready = parts[3]
            future = parts[4]
            mempool_len = parts[5]
            submitted = parts[6]
            validated = parts[7] if len(parts) > 7 else "N/A"
            revalidated = parts[8] if len(parts) > 8 else "N/A"
            pruned = parts[9] if len(parts) > 9 else "N/A"
            reverified = parts[10] if len(parts) > 10 else "N/A"

            # Normalize values (N/A -> None)
            def parse_int(v: str):
                return None if v == "N/A" else int(v)

            points.append(
                MempoolPoint(
                    timestamp=datetime.strptime(ts, TS_FMT),
                    node=node,
                    ready=parse_int(ready),
                    future=parse_int(future),
                    mempool_len=parse_int(mempool_len),
                    submitted_count=parse_int(submitted),
                    validated_count=parse_int(validated),
                    revalidated=parse_int(revalidated),
                    pruned_count=parse_int(pruned),
                    reverified_txs=parse_int(reverified),
                )
            )
        except Exception:
            # Ignore non-data lines
            continue
    return points


def to_dataframe(points: List[MempoolPoint]) -> pd.DataFrame:
    rows = []
    for p in points:
        rows.append({
            "timestamp": p.timestamp,
            "node": p.node,
            "ready": p.ready,
            "future": p.future,
            "mempool_len": p.mempool_len,
            "submitted": p.submitted_count,
            "validated": p.validated_count,
            "revalidated": p.revalidated,
            "pruned": p.pruned_count,
            "reverified": p.reverified_txs,
        })
    df = pd.DataFrame(rows)
    df.sort_values(["node", "timestamp"], inplace=True)
    return df


def resample_metrics(df: pd.DataFrame, window_ms: int) -> pd.DataFrame:
    # Resample per node
    all_nodes = []
    for node, group in df.groupby("node"):
        g = group.set_index("timestamp").copy()
        # Forward fill counts to build step-series of ready/future. Use last observed value.
        for col in ["ready", "future", "mempool_len"]:
            if col in g:
                g[col] = g[col].astype("Float64")
                g[col] = g[col].ffill()
        # submitted is count in event; treat as delta events per log line
        # Build rate by counting events per window
        event_counts = g["submitted"].fillna(0)
        event_counts[event_counts > 0] = 1
        submitted_rate = event_counts.resample(f"{window_ms}ms").sum() * (1000.0 / window_ms)
        # Resample ready/future via last
        ready = g["ready"].resample(f"{window_ms}ms").last()
        future = g["future"].resample(f"{window_ms}ms").last()
        mempool_len = g["mempool_len"].resample(f"{window_ms}ms").last()
        out = pd.DataFrame({
            "ready": ready,
            "future": future,
            "mempool_len": mempool_len,
            "admission_tps": submitted_rate,
        })
        out["node"] = node
        all_nodes.append(out.reset_index())
    return pd.concat(all_nodes, ignore_index=True)


def summarize(resampled_df: pd.DataFrame, original_df: pd.DataFrame) -> str:
    lines = [
        "=== MEMPOOL STATISTICS BY NODE ===",
        "",
        "Metrics Explained:",
        "  Ready Txs     - Average number of ready transactions (can be included in next block)",
        "  Future Txs    - Average number of future transactions (waiting for dependencies like higher nonce)",
        "  Admission Rate- Average rate of new transaction submissions per second (TPS)",
        "  Validated     - Total count of newly submitted transactions that were validated",
        "  Revalidated   - Total count of existing transactions revalidated after new blocks imported",
        "  Pruned        - Total count of transactions removed (included in finalized blocks)",
        "  Reverified    - Total count of transactions reverified after chain reorganization",
        ""
    ]
    for node, group in resampled_df.groupby("node"):
        g = group.dropna(subset=["ready", "future"])
        avg_ready = g["ready"].mean() if not g.empty else 0
        avg_future = g["future"].mean() if not g.empty else 0
        peak_ready = g["ready"].max() if not g.empty else 0
        peak_future = g["future"].max() if not g.empty else 0
        avg_adm = group["admission_tps"].mean() if "admission_tps" in group else 0
        
        # Get event counts from original dataframe
        node_orig = original_df[original_df["node"] == node]
        total_validated = node_orig["validated"].sum() if "validated" in node_orig else 0
        total_revalidated = node_orig["revalidated"].sum() if "revalidated" in node_orig else 0
        total_pruned = node_orig["pruned"].sum() if "pruned" in node_orig else 0
        total_reverified = node_orig["reverified"].sum() if "reverified" in node_orig else 0
        
        lines.append(f"Node: {node}")
        lines.append(f"  Average Ready Txs: {avg_ready:.2f}")
        lines.append(f"  Average Future Txs: {avg_future:.2f}")
        lines.append(f"  Peak Ready Txs: {int(peak_ready) if pd.notna(peak_ready) else 0}")
        lines.append(f"  Peak Future Txs: {int(peak_future) if pd.notna(peak_future) else 0}")
        lines.append(f"  Avg Admission Rate: {avg_adm:.2f} TPS")
        lines.append(f"  Total Validated: {int(total_validated)}")
        lines.append(f"  Total Revalidated: {int(total_revalidated)}")
        lines.append(f"  Total Pruned: {int(total_pruned)}")
        lines.append(f"  Total Reverified: {int(total_reverified)}")
        lines.append("")
    return "\n".join(lines)


def generate_insights(resampled_df: pd.DataFrame, original_df: pd.DataFrame) -> str:
    """Generate insights and observations from the mempool data."""
    insights = []
    insights.append("=== KEY INSIGHTS ===")
    insights.append("")
    
    for node, group in resampled_df.groupby("node"):
        insights.append(f"Node: {node}")
        insights.append("")
        
        # Get node data from original dataframe
        node_orig = original_df[original_df["node"] == node]
        
        # Mempool size analysis
        g = group.dropna(subset=["ready", "future"])
        if not g.empty:
            avg_ready = g["ready"].mean()
            peak_ready = g["ready"].max()
            avg_future = g["future"].mean()
            peak_future = g["future"].max()
            
            # Memory pool utilization
            if avg_ready < 10:
                insights.append("📊 Mempool Utilization: LOW")
                insights.append(f"   - Average ready transactions: {avg_ready:.1f}")
                insights.append("   - The mempool is relatively empty, indicating low transaction volume.")
                insights.append("   - This is typical for relay nodes or periods of low network activity.")
            elif avg_ready < 50:
                insights.append("📊 Mempool Utilization: MODERATE")
                insights.append(f"   - Average ready transactions: {avg_ready:.1f}")
                insights.append("   - Moderate transaction throughput with occasional spikes.")
            else:
                insights.append("📊 Mempool Utilization: HIGH")
                insights.append(f"   - Average ready transactions: {avg_ready:.1f}")
                insights.append("   - High mempool utilization may indicate network congestion.")
                insights.append("   - Consider monitoring block production capacity.")
            
            # Peak analysis
            if peak_ready > avg_ready * 3:
                insights.append("")
                insights.append(f"⚠️  Transaction Spikes Detected: Peak of {int(peak_ready)} transactions")
                insights.append(f"   - Peak is {peak_ready / avg_ready:.1f}x higher than average.")
                insights.append("   - May indicate burst transaction submission or temporary processing delays.")
            
            # Future transactions
            if avg_future > 1:
                insights.append("")
                insights.append(f"🔮 Future Transactions: Average {avg_future:.1f} transactions waiting")
                insights.append("   - Transactions are waiting for dependencies (e.g., nonce ordering).")
                insights.append("   - This is normal for accounts sending multiple transactions.")
        
        # Event-based metrics
        total_validated = node_orig["validated"].sum() if "validated" in node_orig else 0
        total_revalidated = node_orig["revalidated"].sum() if "revalidated" in node_orig else 0
        total_pruned = node_orig["pruned"].sum() if "pruned" in node_orig else 0
        total_reverified = node_orig["reverified"].sum() if "reverified" in node_orig else 0
        
        insights.append("")
        insights.append("📈 Transaction Lifecycle:")
        
        # Validation insights
        if total_validated == 0:
            insights.append("   - ✓ No new transaction validations detected.")
            insights.append("     → This node is likely a relay or not receiving direct tx submissions.")
        else:
            insights.append(f"   - ✓ {int(total_validated)} new transactions validated.")
        
        # Revalidation insights
        if total_revalidated > 0:
            insights.append(f"   - 🔄 {int(total_revalidated)} transactions revalidated after block imports.")
            if total_revalidated > total_pruned:
                insights.append("     → High revalidation suggests many transactions remain pending.")
            else:
                insights.append("     → Normal revalidation activity as blocks are processed.")
        
        # Pruning insights
        if total_pruned > 0:
            insights.append(f"   - ✂️  {int(total_pruned)} transactions pruned (included in finalized blocks).")
            avg_pruned_per_block = total_pruned / max(1, len(g) / 6)  # Approximate blocks
            if avg_pruned_per_block > 5:
                insights.append(f"     → High throughput: ~{avg_pruned_per_block:.1f} txs per block on average.")
        
        # Reorg insights
        if total_reverified > 0:
            insights.append(f"   - ⚡ {int(total_reverified)} transactions reverified after chain reorgs.")
            if total_reverified > 50:
                insights.append("     → Significant reorg activity detected. Monitor chain stability.")
            else:
                insights.append("     → Minor reorg activity is normal in blockchain networks.")
        
        # Efficiency ratio
        if total_pruned > 0 and total_revalidated > 0:
            efficiency = total_pruned / (total_revalidated + total_pruned) * 100
            insights.append("")
            insights.append(f"💡 Mempool Efficiency: {efficiency:.1f}%")
            if efficiency > 60:
                insights.append("   - High efficiency: Most revalidated transactions are eventually included.")
            elif efficiency > 30:
                insights.append("   - Moderate efficiency: Some transactions are revalidated multiple times.")
            else:
                insights.append("   - Low efficiency: Many revalidations without finalization.")
                insights.append("   - This may indicate transaction replacement or expiration.")
        
        insights.append("")
        insights.append("-" * 80)
        insights.append("")
    
    # Add visualization recommendations
    insights.append("")
    insights.append("=== RECOMMENDED VISUALIZATIONS ===")
    insights.append("")
    insights.append("Use the generated CSV files to create the following charts:")
    insights.append("")
    insights.append("1. 📊 Mempool Size Over Time (Line Chart)")
    insights.append("   - X-axis: timestamp")
    insights.append("   - Y-axis: ready, future")
    insights.append("   - Shows mempool growth and spikes")
    insights.append("   - File: *_timeseries.csv")
    insights.append("")
    insights.append("2. 🔄 Transaction Events Timeline (Stacked Area/Bar Chart)")
    insights.append("   - X-axis: timestamp")
    insights.append("   - Y-axis: pruned_count, reverified_txs, revalidated (stacked)")
    insights.append("   - Shows transaction processing activity")
    insights.append("   - File: mempool_events.csv")
    insights.append("")
    insights.append("3. 📈 Ready vs Future Transactions (Dual-Axis Line Chart)")
    insights.append("   - Primary Y-axis: ready transactions")
    insights.append("   - Secondary Y-axis: future transactions")
    insights.append("   - Shows dependency patterns")
    insights.append("   - File: *_timeseries.csv")
    insights.append("")
    insights.append("4. 🎯 Mempool Efficiency Trend (Line Chart)")
    insights.append("   - Calculate rolling ratio: pruned / (pruned + revalidated)")
    insights.append("   - Shows how efficiently transactions move through the mempool")
    insights.append("   - File: mempool_events.csv (requires calculation)")
    insights.append("")
    insights.append("5. ⚡ Reorg Impact (Event Timeline)")
    insights.append("   - Mark timestamps where reverified_txs > 0")
    insights.append("   - Shows when chain reorganizations occur")
    insights.append("   - File: mempool_events.csv")
    insights.append("")
    
    return "\n".join(insights)


def export_csv(df: pd.DataFrame, output_path: str):
    """Export resampled dataframe to CSV for graphing."""
    # Format timestamp for CSV
    df_export = df.copy()
    df_export['timestamp'] = df_export['timestamp'].dt.strftime('%Y-%m-%d %H:%M:%S.%f')
    df_export.to_csv(output_path, index=False)


def main():
    if len(sys.argv) < 3:
        print("Usage: python analyzer.py <mempool_report.txt> <analysis.txt> [window_ms]")
        sys.exit(1)
    report_path = sys.argv[1]
    out_path = sys.argv[2]
    window_ms = int(sys.argv[3]) if len(sys.argv) > 3 else 1000

    points = parse_report(report_path)
    df = to_dataframe(points)
    res = resample_metrics(df, window_ms)
    summary = summarize(res, df)
    insights = generate_insights(res, df)

    # Save summary analysis with insights
    with open(out_path, "w") as f:
        f.write("# Mempool Analysis\n\n")
        f.write(f"Window: {window_ms} ms\n\n")
        f.write(summary)
        f.write("\n\n")
        f.write(insights)

    print(f"Analysis saved to: {out_path}")
    
    # Export CSV for graphing
    csv_path = out_path.rsplit('.', 1)[0] + '_timeseries.csv'
    export_csv(res, csv_path)
    print(f"Time-series CSV saved to: {csv_path}")


if __name__ == "__main__":
    main()
