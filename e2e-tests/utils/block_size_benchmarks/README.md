# Block Size Benchmarking Scripts

Script calculates block propagation time as a timestamp difference between “Pre-sealed block for proposal” and “Imported #XXX” lines from partner-chains node logs.

## How to use

### Prerequisites

1. Install `python3`, `pip`
2. Install dependencies:
   ```bash
   pip install pandas requests
   ```
3. Install `sops` for encrypted config files:
   ```bash
   brew install sops
   ```

### Setting up Grafana Access

To download logs from Grafana, you need a service account token:

1. Log in to your Grafana instance (e.g., https://tools.node.sc.iog.io)
2. Navigate to **Administration** → **Service accounts** (or **Configuration** → **Service accounts**)
3. Click **Add service account**
4. Enter a name (e.g., "Performance Log Downloader") and role (typically **Viewer** is sufficient)
5. Click **Create**
6. Click **Add service account token**
7. Set an expiration time and click **Generate token**
8. **Copy the token immediately** (it won't be shown again)
9. Add the token to the encrypted config file in `secrets/substrate/performance/performance.json`:
   ```bash
   # Decrypt, edit, and re-encrypt the config file
   cd /path/to/e2e-tests
   sops secrets/substrate/performance/performance.json
   # Update the "token" field with your new service account token
   # Save and exit (sops will automatically re-encrypt)
   ```

### Running the Benchmark

The `run_benchmark.py` script automates the entire workflow: downloading logs, extracting data, and generating analysis.

**Using encrypted config file (recommended):**
```bash
python3 run_benchmark.py \
  --config ../../secrets/substrate/performance/performance.json \
  --from-time "2026-01-07T10:00:00Z" \
  --to-time "2026-01-07T10:10:00Z" \
  --node alice --node bob --node charlie
```

**Using command-line arguments (alternative):**
```bash
python3 run_benchmark.py \
  --url "https://tools.node.sc.iog.io/api/datasources/proxy/uid/P8E80F9AEF21F6940" \
  --header "Authorization: Bearer <your_token>" \
  --from-time "2026-01-07T10:00:00Z" \
  --to-time "2026-01-07T10:10:00Z" \
  --node alice --node bob --node charlie
```

**Re-analyzing existing logs:**

If you already have logs downloaded, you can skip the download step:
```bash
python3 run_benchmark.py \
  --skip-download \
  --log-dir logs/2026_01_07_10_00_00 \
  --from-time "2026-01-07T10:00:00Z" \
  --to-time "2026-01-07T10:10:00Z"
```

**Node selection options:**
- `--node <name>`: Specify individual nodes (can be used multiple times)
- `--nodes-file <file>`: Read node list from a file (one per line)
- If neither is specified, uses default list of 20 nodes: alice, bob, charlie, dave, eve, ferdie, george, henry, iris, jack, kate, leo, mike, nina, oliver, paul, quinn, rita, sam, tom
- When using `--skip-download`, nodes are automatically detected from the log files

**Output:**

All files are saved in a timestamped directory: `logs/YYYY_MM_DD_HH_MM_SS/`
- `<node>.txt` - Downloaded logs for each node
- `log_run_details.json` - Run metadata
- `block_propagation_report.txt` - Block propagation report
- `analysis.txt` - Summary statistics by node
