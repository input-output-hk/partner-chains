import re
import sys
from datetime import datetime


def parse_logs(nodes):
    blocks = {}
    pre_sealed_blocks = {}

    for node_name in nodes:
        log_file = f"{node_name}.txt"
        node_name = log_file.split(".")[0]

        with open(log_file, "r") as f:
            for line in f:
                timestamp_match = re.search(
                    r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d{3})?)", line
                )
                if not timestamp_match:
                    continue

                timestamp = parse_timestamp(timestamp_match)

                extract_pre_sealed_data(
                    pre_sealed_blocks, node_name, line, timestamp
                    )

                if "🏆 Imported #" in line:
                    import_match = re.search(
                        r"🏆 Imported #(\d+) \((.*) → (.*)\)", line
                    )

                    if import_match:
                        block_num = int(import_match.group(1))
                        block_hash = import_match.group(3)

                        block_key = (block_num, block_hash)

                        if block_key not in blocks:
                            blocks[block_key] = {
                                "number": block_num,
                                "hash": block_hash,
                                "creator": None,
                                "creation_time": None,
                                "import_times": {},
                            }

                        blocks[block_key]["import_times"][
                            node_name
                        ] = timestamp

    parse_pre_sealed_blocks(blocks, pre_sealed_blocks)

    return blocks


def parse_pre_sealed_blocks(blocks, pre_sealed_blocks):
    for node_name, node_blocks in pre_sealed_blocks.items():
        for block_num, pre_sealed_info in node_blocks.items():
            pre_sealed_hash = pre_sealed_info["hash"]

            for block_info in blocks.values():
                if block_info["number"] == block_num:
                    imported_hash = block_info["hash"]

                    if pre_sealed_hash[-4:] == imported_hash[-4:]:
                        block_info["creator"] = node_name
                        block_info["creation_time"] = pre_sealed_info["time"]
                        block_info["full_hash"] = pre_sealed_hash


def extract_pre_sealed_data(pre_sealed_blocks, node_name, line, timestamp):
    if "🔖 Pre-sealed block for proposal at" in line:
        block_num_match = re.search(r"at (\d+)", line)
        hash_match = re.search(r"Hash now (0x[0-9a-f]+)", line)

        if block_num_match and hash_match:
            block_num = int(block_num_match.group(1))
            block_hash = hash_match.group(1)

            if node_name not in pre_sealed_blocks:
                pre_sealed_blocks[node_name] = {}

            pre_sealed_blocks[node_name][block_num] = {
                            "hash": block_hash,
                            "time": timestamp,
                        }


def parse_timestamp(timestamp_match):
    timestamp_str = timestamp_match.group(1)
    format = (
        "%Y-%m-%d %H:%M:%S.%f"
        if "." in timestamp_str
        else "%Y-%m-%d %H:%M:%S"
    )
    return datetime.strptime(timestamp_str, format)


def calculate_propagation_times(blocks):
    results = []

    for block_info in blocks.values():
        result = {
            "block_num": block_info["number"],
            "block_hash": block_info["hash"],
            "creator": block_info["creator"],
            "import_times": block_info["import_times"].copy(),
        }

        if "full_hash" in block_info:
            result["full_hash"] = block_info["full_hash"]

        if block_info["creator"] and "creation_time" in block_info:
            result["creation_time"] = block_info["creation_time"]
            result["propagation_times"] = {}

            for node, import_time in block_info["import_times"].items():
                prop_time_delta = import_time - block_info["creation_time"]
                prop_time = (
                    prop_time_delta.total_seconds() * 1000
                )
                result["propagation_times"][node] = prop_time

        results.append(result)

    results.sort(key=lambda x: x["block_num"])

    return results


def generate_report(results):
    report_lines = []

    for result in results:
        block_num = result["block_num"]
        block_hash = result["block_hash"]

        if "full_hash" in result:
            report_lines.append(
                (
                    f"Block #{block_num} (Full Hash: {result['full_hash']}, "
                    f"Displayed as: {block_hash})"
                )
            )
        else:
            report_lines.append(f"Block #{block_num} (Hash: {block_hash})")

        if result["creator"]:
            report_lines.append(
                (
                    (
                        f"  Created by: {result['creator']} at "
                        f"{result['creation_time']}"
                    )
                )
            )

            for node, import_time in sorted(result["import_times"].items()):
                if node == result["creator"]:
                    report_lines.append(
                        f"  Imported by {node} (creator node) at {import_time}"
                    )
                else:
                    prop_time = result["propagation_times"][node]
                    report_lines.append(
                        (
                            f"  Imported by {node} after {prop_time:.3f} ms "
                            f"at {import_time}"
                        )
                    )
        else:
            report_lines.append("  Creator unknown")
            for node, import_time in sorted(result["import_times"].items()):
                report_lines.append(f"  Imported by {node} at {import_time}")

        report_lines.append("")

    return "\n".join(report_lines)


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

    if len(sys.argv) < 1:
        print("Usage: python extractor.py [node1 node2 node3 ...]")
        print("Example: python extractor.py alice bob charlie")
        print("If no nodes specified, default nodes will be used:")
        print(", ".join(nodes))
        sys.exit(1)

    if len(sys.argv) > 1:
        nodes = sys.argv[1:]
    print(f"Parsing the following nodes: {', '.join(nodes)}\n")

    blocks = parse_logs(nodes)

    print("Calculating propagation times...\n")
    results = calculate_propagation_times(blocks)

    print("Generating report...\n")
    report = generate_report(results)

    with open("block_propagation_report.txt", "w") as f:
        f.write(report)

    print("Report saved to block_propagation_report.txt")


if __name__ == "__main__":
    main()
