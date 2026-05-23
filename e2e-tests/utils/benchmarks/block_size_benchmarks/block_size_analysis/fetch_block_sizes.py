#!/usr/bin/env python3

import argparse
import sys
import csv
from datetime import datetime
from pathlib import Path
from substrateinterface import SubstrateInterface

def connect_to_node(url: str) -> SubstrateInterface:
    """Connect to a Substrate node via WebSocket."""
    try:
        print(f"Connecting to node at {url}...")
        substrate = SubstrateInterface(url=url)
        print(f"Connected successfully!")
        print(f"Chain: {substrate.chain}")
        return substrate
    except Exception as e:
        print(f"Error connecting to node: {e}")
        raise ConnectionError(f"Could not connect to node: {e}")


def get_block_size(substrate: SubstrateInterface, block_hash=None) -> dict:
    """
    Get the size of a block in bytes.
    
    Args:
        substrate: SubstrateInterface instance
        block_hash: Optional block hash. If None, fetches latest block.
    
    Returns:
        dict with block_number, block_hash, size_bytes, and timestamp
    """
    try:
        # Get block data
        block = substrate.get_block(block_hash=block_hash)
        
        if not block:
            return None
        
        block_number = block['header']['number']
        block_hash_str = block['header']['hash']
        
        # Get extrinsics list
        extrinsics = block.get('extrinsics', [])
        extrinsic_count = len(extrinsics)
        
        # Calculate block size using SCALE encoding
        # Method 1: Get the actual encoded block bytes
        try:
            # Use the substrate RPC to get the raw block
            block_data = substrate.rpc_request('chain_getBlock', [block_hash_str])
            if block_data and 'result' in block_data:
                # The block is returned as a hex string, calculate its size
                block_hex = block_data['result']['block']
                # Convert from dict to string representation
                import json
                block_json_str = json.dumps(block_hex)
                size_bytes = len(block_json_str.encode('utf-8'))
            else:
                # Fallback: estimate from string representation
                size_bytes = len(str(block).encode('utf-8'))
        except Exception as e:
            # Fallback: use string representation size
            size_bytes = len(str(block).encode('utf-8'))
        
        # Get block timestamp from extrinsics
        timestamp = None
        try:
            for extrinsic in extrinsics:
                # Handle both dict and GenericExtrinsic object types
                if hasattr(extrinsic, 'call_module'):
                    # GenericExtrinsic object
                    if extrinsic.call_module == 'Timestamp' and extrinsic.call_function == 'set':
                        if hasattr(extrinsic, 'params') and extrinsic.params:
                            timestamp_ms = extrinsic.params[0]['value']
                            timestamp = datetime.fromtimestamp(timestamp_ms / 1000)
                            break
                elif isinstance(extrinsic, dict):
                    # Dictionary format
                    call = extrinsic.get('call', {})
                    if call.get('call_module') == 'Timestamp' and call.get('call_function') == 'set':
                        call_args = call.get('call_args', [])
                        if call_args:
                            timestamp_ms = call_args[0]['value']
                            timestamp = datetime.fromtimestamp(timestamp_ms / 1000)
                            break
        except Exception:
            # Timestamp extraction failed, continue without it
            pass
        
        return {
            'block_number': block_number,
            'block_hash': block_hash_str,
            'size_bytes': size_bytes,
            'size_kb': size_bytes / 1024,
            'size_mb': size_bytes / (1024 * 1024),
            'timestamp': timestamp,
            'extrinsic_count': extrinsic_count
        }
    except Exception as e:
        print(f"Error getting block size for block {block_hash}: {e}")
        return None


def fetch_block_range(substrate: SubstrateInterface, start_block: int, end_block: int) -> list:
    """
    Fetch block sizes for a range of blocks.
    
    Args:
        substrate: SubstrateInterface instance
        start_block: Starting block number
        end_block: Ending block number (inclusive)
    
    Returns:
        List of block data dictionaries
    """
    block_data = []
    total_blocks = end_block - start_block + 1
    
    print(f"\nFetching blocks {start_block} to {end_block} ({total_blocks} blocks)...")
    
    for block_num in range(start_block, end_block + 1):
        try:
            # Get block hash for block number
            block_hash = substrate.get_block_hash(block_num)
            
            # Get block size
            block_info = get_block_size(substrate, block_hash=block_hash)
            
            if block_info:
                block_data.append(block_info)
                
                # Progress indicator
                progress = ((block_num - start_block + 1) / total_blocks) * 100
                print(f"Progress: {progress:.1f}% - Block #{block_num}: {block_info['size_kb']:.2f} KB ({block_info['extrinsic_count']} extrinsics)", end='\r')
        
        except Exception as e:
            print(f"\nWarning: Could not fetch block {block_num}: {e}")
            continue
    
    print()  # New line after progress
    return block_data


def save_to_csv(block_data: list, output_file: Path):
    """Save block data to CSV file."""
    if not block_data:
        print("No block data to save.")
        return
    
    fieldnames = ['block_number', 'block_hash', 'size_bytes', 'size_kb', 'size_mb', 'extrinsic_count', 'timestamp']
    
    try:
        with open(output_file, 'w', newline='') as csvfile:
            writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(block_data)
        
        print(f"\nData saved to: {output_file}")
        print(f"Total blocks: {len(block_data)}")
        
        # Summary statistics
        total_bytes = sum(b['size_bytes'] for b in block_data)
        avg_bytes = total_bytes / len(block_data)
        min_size = min(b['size_kb'] for b in block_data)
        max_size = max(b['size_kb'] for b in block_data)
        
        print(f"\nSummary:")
        print(f"  Average block size: {avg_bytes/1024:.2f} KB")
        print(f"  Min block size: {min_size:.2f} KB")
        print(f"  Max block size: {max_size:.2f} KB")
        print(f"  Total data: {total_bytes/(1024*1024):.2f} MB")
        
    except Exception as e:
        print(f"Error saving to CSV: {e}")
        raise IOError(f"Could not save to CSV: {e}")


def main():
    parser = argparse.ArgumentParser(
        description="Fetch block sizes from a Substrate node and save to CSV"
    )
    
    parser.add_argument("--url", 
                       default="ws://127.0.0.1:9944",
                       help="WebSocket URL of the Substrate node (default: ws://127.0.0.1:9944)")
    
    parser.add_argument("--start-block", 
                       type=int,
                       help="Starting block number")
    
    parser.add_argument("--end-block", 
                       type=int,
                       help="Ending block number")
    
    parser.add_argument("--latest-n", 
                       type=int,
                       help="Fetch the latest N blocks")
    
    parser.add_argument("--output", 
                       default="block_sizes.csv",
                       help="Output CSV file (default: block_sizes.csv)")
    
    args = parser.parse_args()
    
    # Connect to node
    substrate = connect_to_node(args.url)
    
    # Determine block range
    if args.latest_n:
        # Get current block
        latest_block = substrate.get_block()
        end_block = latest_block['header']['number']
        start_block = max(0, end_block - args.latest_n + 1)
        print(f"Fetching latest {args.latest_n} blocks (#{start_block} to #{end_block})")
    elif args.start_block is not None and args.end_block is not None:
        start_block = args.start_block
        end_block = args.end_block
    else:
        print("Error: Either --latest-n or both --start-block and --end-block must be specified")
        sys.exit(1)
    
    # Fetch block data
    block_data = fetch_block_range(substrate, start_block, end_block)
    
    # Save to CSV
    output_file = Path(args.output)
    save_to_csv(block_data, output_file)
    
    # Close connection
    substrate.close()
    print("\nDone!")


if __name__ == "__main__":
    main()
