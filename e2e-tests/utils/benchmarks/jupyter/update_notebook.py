import json
import os

path = "/Users/larry/Project/iohk/partner-chains/e2e-tests/utils/benchmarks/jupyter/performance_analysis.ipynb"

with open(path, 'r') as f:
    nb = json.load(f)

# Update Time Range and Add Reload Logic to first code cell
setup_cell = nb['cells'][1]
source = setup_cell['source']

new_source = []
for line in source:
    # Update time range lines
    if '"from":"2026' in line:
        line = 'TIME_RANGE = {"from":"2026-02-04 16:16:55","to":"2026-02-04 16:19:41"}\n'
    
    new_source.append(line)
    
    # Insert reload logic after end_time definition
    if "end_time = TIME_RANGE['to']" in line:
        new_source.append("\n")
        new_source.append("# Force reload modules to pick up our changes\n")
        new_source.append("for mod in ['traffic_benchmarks.traffic_analyzer', 'mempool_benchmarks.analyzer']:\n")
        new_source.append("    if mod in sys.modules:\n")
        new_source.append("        importlib.reload(sys.modules[mod])\n")

setup_cell['source'] = new_source

# Update the mempool analysis cell to use the fixed chart filename
for cell in nb['cells']:
    if cell['cell_type'] == 'code' and 'analyzer.plot_throughput_and_mempool' in ''.join(cell['source']):
        source = cell['source']
        new_source = []
        for line in source:
            if 'mempool_analysis_' in line:
                line = line.replace('mempool_analysis_', 'mempool_analysis_fixed_')
            new_source.append(line)
        cell['source'] = new_source

with open(path, 'w') as f:
    json.dump(nb, f, indent=1)

print("Notebook updated successfully.")