#!/bin/bash
#
# Example: Time-based Block Size Analysis Workflow
#
# This script demonstrates how to analyze block sizes for a specific time range
# by first downloading logs and then extracting block numbers from those logs.
#

set -e

echo "============================================================"
echo "Time-Based Block Size Analysis Example"
echo "============================================================"
echo ""

# Configuration
TIME_RANGE='{"from":"2026-01-26 16:20:00","to":"2026-01-26 16:40:00"}'
NODE_URL="ws://127.0.0.1:9944"
CONFIG_FILE="../../secrets/substrate/performance/performance.json"

echo "Step 1: Download logs for the specified time range"
echo "------------------------------------------------------------"
echo "Time range: $TIME_RANGE"
echo ""

# Check if logs already exist
LOG_DIR="../../logs/from_2026-01-26_16-20-00_to_2026-01-26_16-40-00"

if [ -d "$LOG_DIR" ] && [ "$(ls -A $LOG_DIR/*.txt 2>/dev/null)" ]; then
    echo "Logs already exist in: $LOG_DIR"
    echo "Skipping download step..."
else
    echo "Downloading logs..."
    cd ../../
    python3 download_logs.py \
        --time-range "$TIME_RANGE" \
        --config "$CONFIG_FILE"
    cd block_size_benchmarks/block_size_analysis
    echo "Logs downloaded successfully!"
fi

echo ""
echo "Step 2: Extract block range from logs"
echo "------------------------------------------------------------"

python3 extract_block_range_from_logs.py \
    --log-dir "$LOG_DIR" \
    --time-range "$TIME_RANGE"

echo ""
echo "Step 3: Analyze block sizes for the extracted range"
echo "------------------------------------------------------------"
echo "This will fetch block data from the node and generate visualizations..."
echo ""

python3 analyze_block_sizes.py \
    --url "$NODE_URL" \
    --log-dir "$LOG_DIR" \
    --time-range "$TIME_RANGE"

echo ""
echo "============================================================"
echo "Analysis Complete!"
echo "============================================================"
echo ""
echo "Check the output directory for:"
echo "  - block_sizes.csv (raw data)"
echo "  - block_sizes_over_time.png"
echo "  - block_size_distribution.png"
echo "  - block_size_vs_extrinsics.png"
echo "  - block_size_analysis_dashboard.png"
echo "  - block_size_report.txt"
echo ""
