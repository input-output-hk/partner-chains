#!/bin/bash

# BEEFY Upgrade Network Monitoring Script
echo "=== Partner Chains Network Monitor ===" 
echo "Monitoring network health during BEEFY upgrade process"
echo "$(date): Starting monitoring..."
echo ""

# Function to get current block info from a node
get_block_info() {
    local node_num=$1
    local port=$((9933 + node_num - 1))
    if [ $node_num -eq 1 ]; then port=9933; fi
    if [ $node_num -eq 2 ]; then port=9934; fi  
    if [ $node_num -eq 3 ]; then port=9935; fi
    if [ $node_num -eq 4 ]; then port=9936; fi
    if [ $node_num -eq 5 ]; then port=9937; fi
    if [ $node_num -eq 6 ]; then port=9938; fi
    
    curl -s -H "Content-Type: application/json" -d '{
        "jsonrpc": "2.0",
        "method": "chain_getBlock",
        "params": [],
        "id": 1
    }' http://localhost:$port 2>/dev/null | jq -r '.result.block.header.number // "ERROR"' 2>/dev/null || echo "OFFLINE"
}

# Function to get node version
get_node_version() {
    local node_num=$1
    docker exec partner-chains-node-$node_num /usr/local/bin/partner-chains-node --version 2>/dev/null | awk '{print $2}' || echo "ERROR"
}

# Function to get connected peers count
get_peers_count() {
    local node_num=$1
    docker logs --tail=10 partner-chains-node-$node_num 2>/dev/null | grep -o "([0-9]* peers)" | tail -1 | grep -o "[0-9]*" || echo "?"
}

# Function to display network status
show_network_status() {
    echo "$(date) - Network Status:"
    echo "┌──────┬─────────────────────┬──────────┬───────┬────────────────────┐"
    echo "│ Node │ Version             │ Block    │ Peers │ Status             │"
    echo "├──────┼─────────────────────┼──────────┼───────┼────────────────────┤"
    
    for i in {1..6}; do
        version=$(get_node_version $i)
        block=$(get_block_info $i)
        peers=$(get_peers_count $i)
        
        # Determine node type based on version
        if [[ "$version" == "1.7.0-5f2afb1b15a" ]]; then
            node_type="BEEFY-ENABLED"
        elif [[ "$version" == "1.7.0-de400f4b0cf" ]]; then
            node_type="MASTER (pre-BEEFY)"
        else
            node_type="UNKNOWN"
        fi
        
        printf "│ %-4s │ %-19s │ %-8s │ %-5s │ %-18s │\n" "$i" "$version" "$block" "$peers" "$node_type"
    done
    
    echo "└──────┴─────────────────────┴──────────┴───────┴────────────────────┘"
    echo ""
}

# Monitor function - show status every N seconds
monitor_continuous() {
    local interval=${1:-10}
    echo "Starting continuous monitoring (every ${interval}s). Press Ctrl+C to stop."
    
    while true; do
        clear
        echo "=== Partner Chains BEEFY Upgrade Monitor ==="
        echo ""
        show_network_status
        sleep $interval
    done
}

# Single status check
if [ "$1" == "--continuous" ]; then
    monitor_continuous ${2:-10}
else
    show_network_status
fi
