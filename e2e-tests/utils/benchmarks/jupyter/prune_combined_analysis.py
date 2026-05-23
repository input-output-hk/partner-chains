import json
import os

path = "/Users/larry/Project/iohk/partner-chains/e2e-tests/utils/benchmarks/jupyter/performance_analysis.ipynb"

with open(path, 'r') as f:
    nb = json.load(f)

new_cells = []
for cell in nb['cells']:
    # Remove cells related to COMBINED throughput or analysis
    source_text = "".join(cell['source']).lower()
    if "combined" in source_text and ("analysis" in source_text or "throughput" in source_text):
        continue
    
    # Keep the node-specific sections
    new_cells.append(cell)

nb['cells'] = new_cells

with open(path, 'w') as f:
    json.dump(nb, f, indent=1)

print("Notebook updated: Removed combined analysis, kept Charlie and Ferdie.")