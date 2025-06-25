#!/usr/bin/env python3

import sys
import re
import statistics
from typing import Dict, List, Optional, Tuple


class Block:
    def __init__(self, number: int, hash_str: Optional[str] = None):
        self.number = number
        self.hash = hash_str
        self.creator: Optional[str] = None
        self.imports: Dict[str, float] = {}

    def add_import(self, node: str, delay_ms: float):
        self.imports[node] = delay_ms

    def has_all_nodes(self, required_nodes: List[str]) -> bool:
        return all(node in self.imports for node in required_nodes)

    def is_complete(self, required_nodes: List[str]) -> bool:
        return (self.creator and self.creator != 'unknown'
                and self.has_all_nodes(required_nodes))


class BlockPropagationAnalyzer:
    def __init__(self, nodes: List[str]):
        if not nodes:
            raise ValueError("At least one node must be specified")
        self.all_nodes = [node.lower() for node in nodes]
        self.blocks: List[Block] = []

    def parse_file(self, filename: str) -> None:
        try:
            with open(filename, 'r', encoding='utf-8') as file:
                content = file.read()
        except FileNotFoundError:
            print(f"Error: File '{filename}' not found.")
            sys.exit(1)
        except Exception as e:
            print(f"Error reading file '{filename}': {e}")
            sys.exit(1)
        self._parse_content(content)

    def _parse_content(self, content: str) -> None:
        lines = content.split('\n')
        current_block = None
        for line in lines:
            line = line.strip()
            if line.startswith('Block #'):
                current_block = self._parse_block_header(line)
                if current_block:
                    self.blocks.append(current_block)
            elif line.startswith('Created by:') and current_block:
                current_block.creator = self._parse_creator(line)
            elif line.startswith('Imported by') and current_block:
                node, delay = self._parse_import(line)
                if node:
                    current_block.add_import(node, delay)
            elif 'Creator unknown' in line and current_block:
                current_block.creator = 'unknown'

    def _parse_block_header(self, line: str) -> Optional[Block]:
        block_match = re.search(r'Block #(\d+)', line)
        hash_match = re.search(r'0x[a-f0-9]{4}…[a-f0-9]{4}', line)
        if block_match:
            number = int(block_match.group(1))
            hash_str = hash_match.group(0) if hash_match else None
            return Block(number, hash_str)
        return None

    def _parse_creator(self, line: str) -> Optional[str]:
        creator_match = re.search(r'Created by: (\w+)', line)
        return creator_match.group(1).lower() if creator_match else None

    def _parse_import(self, line: str) -> Tuple[Optional[str], float]:
        import_match = re.search(
            r'Imported by (\w+)'
            r'(?:\s+\(creator node\))?'
            r'(?:\s+after ([\d.]+) ms)?',
            line
        )
        if import_match:
            node = import_match.group(1).lower()
            delay_str = import_match.group(2)
            delay = float(delay_str) if delay_str else 0.0
            return node, delay
        return None, 0.0

    def get_complete_blocks(self) -> List[Block]:
        return [block for block in self.blocks
                if block.is_complete(self.all_nodes)]

    def _format_table_row(self, values: List[str], widths: List[int]) -> str:
        formatted_values = []
        for i, (value, width) in enumerate(zip(values, widths)):
            if i == 0:
                formatted_values.append(f"{value:<{width}}")
            else:
                formatted_values.append(f"{value:^{width}}")
        return "| " + " | ".join(formatted_values) + " |"

    def generate_summary_statistics(self, complete_blocks: List[Block]) -> str:
        lines = []
        lines.append("=== SUMMARY STATISTICS BY NODE ===")
        lines.append("")

        stats = {}
        for node in self.all_nodes:
            blocks_created = len([block for block in complete_blocks if block.creator == node])

            import_times = [
                float(block.imports[node])
                for block in complete_blocks
                if block.creator != node
            ]

            avg_import = statistics.mean(import_times) if import_times else 0

            stats[node] = {
                'blocks_created': blocks_created,
                'blocks_imported': len(import_times),
                'min_import': min(import_times) if import_times else 0,
                'max_import': max(import_times) if import_times else 0,
                'avg_import': avg_import
            }

        header = "| Node    | Blocks Created | Blocks Imported | Min Import Time | Max Import Time | Avg Import Time |"
        separator = "|---------|----------------|-----------------|-----------------|-----------------|-----------------|"
        lines.append(header)
        lines.append(separator)

        for node in self.all_nodes:
            s = stats[node]
            row = (f"| {node.capitalize():<7} | {s['blocks_created']:<14} | "
                   f"{s['blocks_imported']:<15} | {s['min_import']:<15.0f} | "
                   f"{s['max_import']:<15.0f} | {s['avg_import']:<15.1f} |")
            lines.append(row)

        return '\n'.join(lines)

    def run(self, input_filename: str, output_filename: str) -> None:
        """Main analysis function"""
        print(f"Analyzing nodes: {', '.join(self.all_nodes)}")
        print(f"Parsing file: {input_filename}")
        self.parse_file(input_filename)
        print(f"Total blocks parsed: {len(self.blocks)}")
        complete_blocks = self.get_complete_blocks()
        print(f"Complete blocks: {len(complete_blocks)}")
        if not complete_blocks:
            print("No complete blocks found. Exiting.")
            sys.exit(1)
        stats_table = self.generate_summary_statistics(complete_blocks)
        try:
            with open(output_filename, 'w', encoding='utf-8') as file:
                file.write("# Block Propagation Analysis\n\n")
                nodes = ', '.join(node.capitalize() for node in self.all_nodes)
                file.write(f"**Nodes analyzed:** {nodes}")
                file.write("\n\n")
                file.write(stats_table)
                file.write("\n\n")
            print(f"Analysis complete. Results saved to: {output_filename}")
        except Exception as e:
            print(f"Error writing output file '{output_filename}': {e}")
            sys.exit(1)


def main():
    nodes = [
        "alice",
        "bob",
        "charlie",
        "dave",
        "eve",
        "ferdie",
        "george",
        "henry",
        "iris",
        "jack"
    ]

    if len(sys.argv) < 3:
        print(
            "Usage: python analyzer.py <input_file.txt> <output_file.txt> "
            "[node1 node2 node3 ...]"
        )
        print(
            "Example: python analyzer.py data.txt results.txt "
            "alice bob charlie"
        )
        print(
            "If no nodes specified, default nodes will be used: "
            f"{', '.join(nodes)}"
        )
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2]

    if len(sys.argv) > 3:
        nodes = sys.argv[3:]

    try:
        analyzer = BlockPropagationAnalyzer(nodes)
        analyzer.run(input_file, output_file)
    except ValueError as e:
        print(f"Error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
