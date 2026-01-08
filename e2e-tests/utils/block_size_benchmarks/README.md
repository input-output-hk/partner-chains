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

### Downloading Logs

Download logs from Loki/Grafana using `download_logs.py` (located in the parent `utils/` directory):

   **Using encrypted config file (recommended):**
   ```bash
   python3 ../download_logs.py \
     --config ../../secrets/substrate/performance/performance.json \
     --from-time "2026-01-07T10:00:00Z" \
     --to-time "2026-01-07T10:10:00Z" \
     --node alice --node bob --node charlie
   ```
   
   The config file is encrypted with sops and contains:
   - Grafana/Loki URL
   - Authentication token
   
   The script automatically decrypts the config using sops (requires `sops` and proper age/pgp keys).
   
   **Using command-line arguments (alternative):**
   ```bash
   python3 ../download_logs.py \
     --url "https://tools.node.sc.iog.io/api/datasources/proxy/uid/P8E80F9AEF21F6940" \
     --header "Authorization: Bearer <your_token>" \
     --from-time "2026-01-07T10:00:00Z" \
     --to-time "2026-01-07T10:10:00Z" \
     --node alice --node bob --node charlie
   ```
   
   Command-line arguments override values from the config file.
   
   **Node selection options:**
   - `--node <name>`: Specify individual nodes (can be used multiple times)
   - `--nodes-file <file>`: Read node list from a file (one node per line)
   - If neither is specified, uses default list of 20 nodes: alice, bob, charlie, dave, eve, ferdie, george, henry, iris, jack, kate, leo, mike, nina, oliver, paul, quinn, rita, sam, tom
   
   **Output directory:**
   - `--output-dir <path>`: Base directory for log output (default: `logs/`)
   - A timestamped subdirectory is created for each run: `<output-dir>/YYYY_MM_DD_HH_MM_SS/`
   - Log files are saved without timestamps: `<output-dir>/YYYY_MM_DD_HH_MM_SS/<node>.txt`
   - A `log_run_details.json` file is created in each run directory with the command parameters

4. Extract data from logs:
   ```bash
   python3 extractor.py alice bob charlie
   ```
   Note: extractor.py expects log files in the current directory. You'll need to navigate to the timestamped log directory (e.g., `logs/YYYY_MM_DD_HH_MM_SS/`) before running the extractor.

5. Generate statistics by node:
   ```bash
   python3 analyzer.py block_propagation_report.txt analysis.txt alice bob charlie
   ```
