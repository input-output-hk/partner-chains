import re
import sys

def count_block_transactions(file_path):
    """
    Parses a Midnight/Substrate log file to count distinct validated transactions.
    """
    # A set is used to automatically handle deduplication
    distinct_hashes = set()

    # Regex to find the specific confirmation message and capture the hex hash
    # Example match: Validated Midnight transaction "618804..."
    tx_pattern = re.compile(r'Validated Midnight transaction "([a-fA-F0-9]+)"')

    try:
        with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
            for line in f:
                # search() finds the pattern anywhere in the line
                match = tx_pattern.search(line)
                if match:
                    # distinct_hashes stores the unique ID found in group 1
                    distinct_hashes.add(match.group(1))
        
        return distinct_hashes

    except FileNotFoundError:
        print(f"❌ Error: The file '{file_path}' was not found.")
        return set()
    except Exception as e:
        print(f"❌ An error occurred: {e}")
        return set()

if __name__ == "__main__":
    # You can change this filename to match your local file
    filename = '490txs.txt'
    
    # Allow running from command line: python script.py my_logs.log
    if len(sys.argv) > 1:
        filename = sys.argv[1]

    print(f"Scanning '{filename}' for transactions...")
    
    unique_txs = count_block_transactions(filename)
    count = len(unique_txs)

    print("-" * 30)
    print(f"Total Distinct Txs: {count}")
    print("-" * 30)

    # Optional: Print the first few hashes to verify
    if count > 0:
        print("First 5 hashes found:")
        for tx in list(unique_txs)[:5]:
            print(f" - {tx}")