#!/usr/bin/env python3

import argparse
import sys
import csv
from pathlib import Path
import matplotlib
matplotlib.use('Agg')  # Non-interactive backend
import matplotlib.pyplot as plt
import numpy as np
from datetime import datetime


def load_csv_data(csv_file: Path) -> list:
    """Load block size data from CSV file."""
    try:
        with open(csv_file, 'r') as f:
            reader = csv.DictReader(f)
            data = list(reader)
        
        # Convert numeric fields
        for row in data:
            row['block_number'] = int(row['block_number'])
            row['size_bytes'] = int(row['size_bytes'])
            row['size_kb'] = float(row['size_kb'])
            row['size_mb'] = float(row['size_mb'])
            row['extrinsic_count'] = int(row['extrinsic_count'])
        
        return data
    except Exception as e:
        print(f"Error loading CSV file: {e}")
        sys.exit(1)


def plot_block_sizes_over_time(data: list, output_dir: Path):
    """Generate a line plot showing block sizes over block numbers."""
    
    block_numbers = [d['block_number'] for d in data]
    sizes_kb = [d['size_kb'] for d in data]
    
    plt.figure(figsize=(14, 6))
    plt.plot(block_numbers, sizes_kb, marker='o', markersize=3, linewidth=1.5, color='#2ca02c', alpha=0.7)
    
    plt.xlabel('Block Number', fontsize=12)
    plt.ylabel('Block Size (KB)', fontsize=12)
    plt.title('Block Size Over Time', fontsize=14, fontweight='bold')
    plt.grid(True, alpha=0.3, linestyle='--')
    
    # Add average line
    avg_size = np.mean(sizes_kb)
    plt.axhline(y=avg_size, color='r', linestyle='--', linewidth=2, label=f'Average: {avg_size:.2f} KB', alpha=0.7)
    plt.legend(fontsize=10)
    
    plt.tight_layout()
    
    output_file = output_dir / "block_sizes_over_time.png"
    plt.savefig(output_file, dpi=150, bbox_inches='tight')
    plt.close()
    
    print(f"Graph saved: {output_file}")
    return output_file


def plot_block_size_distribution(data: list, output_dir: Path):
    """Generate a histogram showing the distribution of block sizes."""
    
    sizes_kb = [d['size_kb'] for d in data]
    
    plt.figure(figsize=(12, 6))
    
    # Create histogram
    n, bins, patches = plt.hist(sizes_kb, bins=30, color='#1f77b4', alpha=0.7, edgecolor='black')
    
    # Add statistics
    avg_size = np.mean(sizes_kb)
    median_size = np.median(sizes_kb)
    
    plt.axvline(avg_size, color='r', linestyle='--', linewidth=2, label=f'Average: {avg_size:.2f} KB')
    plt.axvline(median_size, color='g', linestyle='--', linewidth=2, label=f'Median: {median_size:.2f} KB')
    
    plt.xlabel('Block Size (KB)', fontsize=12)
    plt.ylabel('Frequency', fontsize=12)
    plt.title('Block Size Distribution', fontsize=14, fontweight='bold')
    plt.legend(fontsize=10)
    plt.grid(True, alpha=0.3, linestyle='--', axis='y')
    
    plt.tight_layout()
    
    output_file = output_dir / "block_size_distribution.png"
    plt.savefig(output_file, dpi=150, bbox_inches='tight')
    plt.close()
    
    print(f"Graph saved: {output_file}")
    return output_file


def plot_block_size_vs_extrinsics(data: list, output_dir: Path):
    """Generate a scatter plot showing block size vs number of extrinsics."""
    
    extrinsic_counts = [d['extrinsic_count'] for d in data]
    sizes_kb = [d['size_kb'] for d in data]
    
    plt.figure(figsize=(12, 6))
    
    plt.scatter(extrinsic_counts, sizes_kb, alpha=0.5, color='#ff7f0e', s=50, edgecolors='black', linewidth=0.5)
    
    # Add trend line
    if len(extrinsic_counts) > 1:
        z = np.polyfit(extrinsic_counts, sizes_kb, 1)
        p = np.poly1d(z)
        x_trend = np.linspace(min(extrinsic_counts), max(extrinsic_counts), 100)
        plt.plot(x_trend, p(x_trend), "r--", linewidth=2, label=f'Trend: y = {z[0]:.2f}x + {z[1]:.2f}')
    
    plt.xlabel('Number of Extrinsics', fontsize=12)
    plt.ylabel('Block Size (KB)', fontsize=12)
    plt.title('Block Size vs Number of Extrinsics', fontsize=14, fontweight='bold')
    plt.legend(fontsize=10)
    plt.grid(True, alpha=0.3, linestyle='--')
    
    plt.tight_layout()
    
    output_file = output_dir / "block_size_vs_extrinsics.png"
    plt.savefig(output_file, dpi=150, bbox_inches='tight')
    plt.close()
    
    print(f"Graph saved: {output_file}")
    return output_file


def plot_combined_analysis(data: list, output_dir: Path):
    """Generate a combined plot with multiple subplots."""
    
    block_numbers = [d['block_number'] for d in data]
    sizes_kb = [d['size_kb'] for d in data]
    sizes_mb = [d['size_mb'] for d in data]
    extrinsic_counts = [d['extrinsic_count'] for d in data]
    
    fig = plt.figure(figsize=(16, 10))
    
    # 1. Block size over time
    ax1 = plt.subplot(2, 2, 1)
    ax1.plot(block_numbers, sizes_kb, marker='o', markersize=2, linewidth=1, color='#2ca02c', alpha=0.7)
    avg_size = np.mean(sizes_kb)
    ax1.axhline(y=avg_size, color='r', linestyle='--', linewidth=1.5, alpha=0.7)
    ax1.set_xlabel('Block Number', fontsize=10)
    ax1.set_ylabel('Block Size (KB)', fontsize=10)
    ax1.set_title('Block Size Over Time', fontsize=11, fontweight='bold')
    ax1.grid(True, alpha=0.3, linestyle='--')
    ax1.text(0.02, 0.98, f'Avg: {avg_size:.2f} KB', transform=ax1.transAxes, 
             verticalalignment='top', bbox=dict(boxstyle='round', facecolor='wheat', alpha=0.5))
    
    # 2. Block size distribution
    ax2 = plt.subplot(2, 2, 2)
    n, bins, patches = ax2.hist(sizes_kb, bins=20, color='#1f77b4', alpha=0.7, edgecolor='black')
    median_size = np.median(sizes_kb)
    ax2.axvline(avg_size, color='r', linestyle='--', linewidth=1.5, label=f'Avg: {avg_size:.2f} KB')
    ax2.axvline(median_size, color='g', linestyle='--', linewidth=1.5, label=f'Median: {median_size:.2f} KB')
    ax2.set_xlabel('Block Size (KB)', fontsize=10)
    ax2.set_ylabel('Frequency', fontsize=10)
    ax2.set_title('Block Size Distribution', fontsize=11, fontweight='bold')
    ax2.legend(fontsize=8)
    ax2.grid(True, alpha=0.3, linestyle='--', axis='y')
    
    # 3. Block size vs extrinsics
    ax3 = plt.subplot(2, 2, 3)
    ax3.scatter(extrinsic_counts, sizes_kb, alpha=0.5, color='#ff7f0e', s=30, edgecolors='black', linewidth=0.5)
    if len(extrinsic_counts) > 1:
        z = np.polyfit(extrinsic_counts, sizes_kb, 1)
        p = np.poly1d(z)
        x_trend = np.linspace(min(extrinsic_counts), max(extrinsic_counts), 100)
        ax3.plot(x_trend, p(x_trend), "r--", linewidth=1.5, alpha=0.7)
    ax3.set_xlabel('Number of Extrinsics', fontsize=10)
    ax3.set_ylabel('Block Size (KB)', fontsize=10)
    ax3.set_title('Block Size vs Extrinsics', fontsize=11, fontweight='bold')
    ax3.grid(True, alpha=0.3, linestyle='--')
    
    # 4. Statistics table
    ax4 = plt.subplot(2, 2, 4)
    ax4.axis('off')
    
    # Calculate statistics
    total_blocks = len(data)
    total_size_mb = sum(sizes_mb)
    min_size = min(sizes_kb)
    max_size = max(sizes_kb)
    std_dev = np.std(sizes_kb)
    total_extrinsics = sum(extrinsic_counts)
    avg_extrinsics = np.mean(extrinsic_counts)
    
    stats_text = f"""
    STATISTICS SUMMARY
    ═══════════════════════════════
    
    Total Blocks:           {total_blocks:,}
    Block Range:            #{min(block_numbers)} - #{max(block_numbers)}
    
    BLOCK SIZE
    ───────────────────────────────
    Average:                {avg_size:.2f} KB
    Median:                 {median_size:.2f} KB
    Minimum:                {min_size:.2f} KB
    Maximum:                {max_size:.2f} KB
    Std Deviation:          {std_dev:.2f} KB
    Total Data:             {total_size_mb:.2f} MB
    
    EXTRINSICS
    ───────────────────────────────
    Total Extrinsics:       {total_extrinsics:,}
    Avg per Block:          {avg_extrinsics:.1f}
    """
    
    ax4.text(0.1, 0.5, stats_text, fontsize=10, family='monospace',
             verticalalignment='center', bbox=dict(boxstyle='round', facecolor='lightblue', alpha=0.3))
    
    plt.suptitle('Block Size Analysis Dashboard', fontsize=14, fontweight='bold', y=0.995)
    plt.tight_layout(rect=[0, 0, 1, 0.99])
    
    output_file = output_dir / "block_size_analysis_dashboard.png"
    plt.savefig(output_file, dpi=150, bbox_inches='tight')
    plt.close()
    
    print(f"Dashboard saved: {output_file}")
    return output_file


def generate_report(data: list, output_dir: Path):
    """Generate a text report with statistics."""
    
    sizes_kb = [d['size_kb'] for d in data]
    sizes_mb = [d['size_mb'] for d in data]
    block_numbers = [d['block_number'] for d in data]
    extrinsic_counts = [d['extrinsic_count'] for d in data]
    
    report_lines = []
    report_lines.append("=" * 60)
    report_lines.append("BLOCK SIZE ANALYSIS REPORT")
    report_lines.append("=" * 60)
    report_lines.append("")
    report_lines.append(f"Generated at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    report_lines.append("")
    report_lines.append("DATASET SUMMARY")
    report_lines.append("-" * 60)
    report_lines.append(f"Total blocks analyzed:      {len(data):,}")
    report_lines.append(f"Block range:                #{min(block_numbers)} - #{max(block_numbers)}")
    report_lines.append("")
    report_lines.append("BLOCK SIZE STATISTICS")
    report_lines.append("-" * 60)
    report_lines.append(f"Average block size:         {np.mean(sizes_kb):.2f} KB  ({np.mean(sizes_mb):.4f} MB)")
    report_lines.append(f"Median block size:          {np.median(sizes_kb):.2f} KB")
    report_lines.append(f"Minimum block size:         {min(sizes_kb):.2f} KB")
    report_lines.append(f"Maximum block size:         {max(sizes_kb):.2f} KB")
    report_lines.append(f"Standard deviation:         {np.std(sizes_kb):.2f} KB")
    report_lines.append(f"Total data processed:       {sum(sizes_mb):.2f} MB")
    report_lines.append("")
    report_lines.append("EXTRINSIC STATISTICS")
    report_lines.append("-" * 60)
    report_lines.append(f"Total extrinsics:           {sum(extrinsic_counts):,}")
    report_lines.append(f"Average per block:          {np.mean(extrinsic_counts):.1f}")
    report_lines.append(f"Min extrinsics in block:    {min(extrinsic_counts)}")
    report_lines.append(f"Max extrinsics in block:    {max(extrinsic_counts)}")
    report_lines.append("")
    report_lines.append("TOP 5 LARGEST BLOCKS")
    report_lines.append("-" * 60)
    
    # Sort by size and get top 5
    sorted_data = sorted(data, key=lambda x: x['size_kb'], reverse=True)
    for i, block in enumerate(sorted_data[:5], 1):
        report_lines.append(f"{i}. Block #{block['block_number']:,}: {block['size_kb']:.2f} KB ({block['extrinsic_count']} extrinsics)")
    
    report_lines.append("")
    report_lines.append("=" * 60)
    
    report_text = "\n".join(report_lines)
    
    # Save to file
    report_file = output_dir / "block_size_report.txt"
    with open(report_file, 'w') as f:
        f.write(report_text)
    
    print(f"\nReport saved: {report_file}")
    
    # Also print to console
    print("\n" + report_text)


def main():
    parser = argparse.ArgumentParser(
        description="Visualize block size data from CSV"
    )
    
    parser.add_argument("--input", 
                       required=True,
                       help="Input CSV file with block size data")
    
    parser.add_argument("--output-dir", 
                       default=".",
                       help="Output directory for graphs and reports (default: current directory)")
    
    args = parser.parse_args()
    
    input_file = Path(args.input)
    output_dir = Path(args.output_dir)
    
    if not input_file.exists():
        print(f"Error: Input file not found: {input_file}")
        sys.exit(1)
    
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print(f"Loading data from {input_file}...")
    data = load_csv_data(input_file)
    print(f"Loaded {len(data)} blocks")
    
    print("\nGenerating visualizations...")
    plot_block_sizes_over_time(data, output_dir)
    plot_block_size_distribution(data, output_dir)
    plot_block_size_vs_extrinsics(data, output_dir)
    plot_combined_analysis(data, output_dir)
    
    print("\nGenerating report...")
    generate_report(data, output_dir)
    
    print("\n✓ All visualizations complete!")


if __name__ == "__main__":
    main()
