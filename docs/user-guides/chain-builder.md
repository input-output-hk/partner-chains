# Chain Builder (governance authority) onboarding

Partner Chain builders are organizations that want to build their own blockchains according to their business cases. They are the governance authorities for their new chains. They can utilize the Partner Chains toolkit and other tools to build and run a separate blockchain that can be interoperable with the Cardano network. Each builder can have their own operation and business model. The Partner Chain toolkit aims to be versatile enough to support a wide range of use cases.

## Order of Operations
1. Install dependencies
    1. Cardano node v10.1.4
        1. Ogmios v6.11.0
        2. DB Sync  v13.6.0.4 (PostgreSQLv15.3+)
    2. Download the partner chain node v1.4.0
2. Run the generate-keys wizard
3. Run the prepare-configuration wizard
    1. Set chain parameters
    2. Provide signing key
4. Run the create-chain-spec wizard
5. Run the setup-main-chain-state wizard
6. Run the start-node wizard
7. Distribute chain files to participants

### 1. Install Partner Chains dependencies

To run the Partner Chains stack, several dependencies need to be installed on a `cardano-node`.

Ogmios and DB Sync are essential to enable registration communication with the main chain (Cardano). Ogmios is used for submitting transactions to Cardano, and DB Sync is for observation of main chain state.

### 1.1 Cardano node v10.1.4

Cardano node is required to start a partner chain. The installation of `cardano-node` is out of the scope of this guide. Refer to our [Cardano course handbook](https://cardano-course.gitbook.io/cardano-course/handbook) for documentation and video instruction.

Once your node is synced with the preview network, you are ready to continue with this guide.

### 1.1 Cardano node dependencies

---
**NOTE**

Be mindful of file paths in the instruction sets below. Your Cardano node may have slightly different paths for certain files. Replace file paths below with the paths relevant to your node.

---

### 1.1.1 Ogmios - v6.11.0

Ogmios is a lightweight bridge interface for `cardano-node`. It offers a WebSocket API that enables local clients to speak to the main chain via JSON/RPC.

You will find it convenient to install [Ogmios](https://github.com/CardanoSolutions/ogmios) using pre-built binaries.

You can also build from source, although it requires a significant number of dependencies.

1. Obtain the [binary](https://github.com/CardanoSolutions/ogmios/releases)
2. Change the file to an executable: `sudo chmod +x /home/ubuntu/ogmios`
3. Add executable to PATH: `sudo mv ogmios /usr/local/bin`
3. Run Ogmios as a service:

```
sudo tee /etc/systemd/system/ogmios.service > /dev/null <<EOF
[Unit]
Description=Ogmios Service
After=network.target

[Service]
User=ubuntu
Type=simple
ExecStart=/usr/local/bin/ogmios \
  --host=0.0.0.0 \
  --node-config=/home/ubuntu/preview/configs/config.json \
  --node-socket=/home/ubuntu/preview/node.socket
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload && \
sudo systemctl enable ogmios.service && \
sudo systemctl start ogmios.service
```

4. Observe logs

```
journalctl -fu ogmios.service
```

For further instructions, please see [Ogmios](https://ogmios.dev/getting-started/building/).

### 1.1.2 Cardano DB Sync v13.6.0.4

The partner chain needs DB Sync on `cardano-node` to observe Cardano's state.

#### A critical note on Cardano DB Sync!

> Before starting the partner chain node, and during normal operations, it is essential that the DB Sync component is fully synchronized.
> Running the node with lagging or not fully synced DB Sync will result in consensus errors, decreased peer reputation, and possibly a temporary ban by network peers.
> Sync time depends on hardware and network conditions, but here are approximate estimations for each network:

#### Sync time required

- Preview: a few hours
- Pre-production: usually ranges from several hours to a day
- Mainnet: two or more days.

Typical error message if DB Sync is behind:

```
💔 Verification failed for block 0x151ed479f5766f8dc56fa3626329baa77292d5a692cf7fb9d24e743ae57fe71c received from (12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp): "Main chain state e04eea9347162cd773a3505692d0aaee3d49b2c61f21a5b8a95f3d5711a63961 referenced in imported block at slot 286497345 with timestamp 1718984070000 not found"
```

In this case, it's best to stop the partner chain node and make sure DB Sync is healthy and synchronized before restarting the node.

1. Download the [binary](https://github.com/IntersectMBO/cardano-db-sync/releases) and add it to the PATH
2. Set up a PostgreSQL server:

```
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql.service
```

Enter shell as default postgres user: `sudo -i -u postgres`

Enter the postgres CLI: `psql`

Create user: `CREATE USER ubuntu WITH PASSWORD 'XXXXXXXXXXXXX';`

Change role permissions:
`ALTER ROLE ubuntu WITH SUPERUSER;`
`ALTER ROLE ubuntu WITH CREATEDB;`

Verify user is created and has role permissions: `\du`

Create database: `CREATE DATABASE cexplorer;`

Verify the database is created: `\l`

Sample correct return:

```
List of databases
   Name    |  Owner   | Encoding | Collate |  Ctype  |   Access privileges
-----------+----------+----------+---------+---------+-----------------------
 cexplorer | postgres | UTF8     | C.UTF-8 | C.UTF-8 |
 postgres  | postgres | UTF8     | C.UTF-8 | C.UTF-8 |
 template0 | postgres | UTF8     | C.UTF-8 | C.UTF-8 | =c/postgres          +
           |          |          |         |         | postgres=CTc/postgres
 template1 | postgres | UTF8     | C.UTF-8 | C.UTF-8 | =c/postgres          +
           |          |          |         |         | postgres=CTc/postgres
(4 rows)
```

If any command fails, restart postgres service:
`sudo systemctl restart postgresql.service`

This check should return empty. It will be filled with db sync relations:
`PGPASSFILE=~/cardano-db-sync/config/pgpass-preview ./postgresql-setup.sh --check`

3. Run DB Sync as a service

```
sudo tee /etc/systemd/system/cardano-db-sync.service > /dev/null <<EOF
[Unit]
Description=Cardano DB Sync Service
After=network.target

[Service]
Environment=PGPASSFILE=/home/ubuntu/cardano-db-sync/config/pgpass-preview
ExecStart=/usr/local/bin/cardano-db-sync --config /home/ubuntu/preview/configs/db-sync-config.json --socket-path /home/ubuntu/preview/node.socket --state-dir /home/ubuntu/preview/db-sync/ledger-state --schema-dir /home/ubuntu/cardano-db-sync/schema/
User=ubuntu
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF
```

Enable and start service:
```
sudo systemctl daemon-reload && \
sudo systemctl enable cardano-db-sync.service && \
sudo systemctl start cardano-db-sync.service
```

4. Observe logs

```
journalctl -fu cardano-db-sync.service
```

---
**WARNING**

Ensure that the node is synced with the network to 100% as well as DB Sync before continuing beyond this point. On preview network, it is roughly 24 hours before sync is complete.

---

### 1.2 Download the Partner Chains node

1. Download the Partner Chains node from the [official repository](https://github.com/input-output-hk/partner-chains)

### 2. Run the generate-keys wizard

The generate-keys wizard is designed to simplify the process of getting started with a partner chains node. This is the initial step for network participants who do not yet have keys.

The generate-keys wizard will generate necessary keys and save them to your node’s keystore. The following keys will be created:

1. ECDSA cross-chain key
2. ED25519 Grandpa key
3. SR25519 Aura key

If these keys already exist in the node’s keystore, you will be asked to overwrite existing keys. The wizard will also generate a network key for your node if needed.

1. Start the wizard: `./partner-chains-node wizards generate-keys`
2. Input the node base path. It is saved in `pc-resources-config.json`.

Now the wizard will output `partner-chains-public-keys.json` containing three keys:

```javascript
{
	"sidechain_pub_key": "0x<key>",
	"aura_pub_key": "0x<key>",
	"grandpa_pub_key": "0x<key>"
}
```

### 3. Run the prepare-configuration wizard

Before running this wizard, be sure that `ogmios` is available by host and port.

1. Start the wizard:`./partner-chains-node wizards prepare-configuration`
2. Update the bootnodes array and provide public ip or hostname
3. Provide required payment keys and select genesis utxo to initialize the chain
4. Configure initial native token supply address (optional)
5. Store the main chain configuration

This wizard will submit a governance initialization transaction, that spends the genesis utxo. It will also result in a `pc-chain-config.json` file. The wizard also adds required cardano addresses and policy ids to the configuration file.

After chain-config file has been generated, it should be updated with your keys and the keys of other *permissioned* candidates in the `initial_permissioned_candidates` array.

Example:

```
    "initial_permissioned_candidates": [
		  {
			  "aura_pub_key": "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
			  "grandpa_pub_key": "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee",
			  "sidechain_pub_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1"
			},
			{
			  "aura_pub_key": "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48",
			  "grandpa_pub_key": "0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69",
			  "sidechain_pub_key": "0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27"
			}
	  ],
```

#### Further notes from the prepare-configuration wizard

##### Setting the partner chain parameters

The wizard asks for the genesis utxo that identifies a partner chain.

##### Storing the main chain configuration

The wizard completes by reporting that the `pc-chain-config.json` file is ready for distribution to network participants and also that the `create-chain-spec` wizard should be executed when keys of permissioned candidates are gathered.

A sample file:

```
{
  "bootnodes": [
    "/dns/myhost/tcp/3033/p2p/12D3KooWHBpeL1GgfnuykXzSvNt9wCbb1j9SEG6d4DJu5cnJR7sh"
  ],
  "cardano": {
    "active_slots_coeff": 0.05,
    "epoch_duration_millis": 432000000,
    "first_epoch_number": 208,
    "first_epoch_timestamp_millis": 1596059091000,
    "first_slot_number": 4492800,
    "main_chain_slot_duration_millis": 1000,
    "network": 0,
    "security_parameter": 2160
  },
  "cardano_addresses": {
    "committee_candidates_address": "addr_...",
    "d_parameter_policy_id": "bd292a4e...",
    "native_token": {
      "asset": {
        "asset_name": "0x",
        "policy_id": "0x00000000000000000000000000000000000000000000000000000000"
      },
      "illiquid_supply_address": "addr_..."
    },
    "permissioned_candidates_policy_id": "63ecb396..."
  },
  "chain_parameters": {
    "genesis_utxo": "0000000000000000000000000000000000000000000000000000000000000000#0",
  },
  "initial_permissioned_candidates": [
		  {
			  "aura_pub_key": "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
			  "grandpa_pub_key": "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee",
			  "sidechain_pub_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1"
			},
			{
			  "aura_pub_key": "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48",
			  "grandpa_pub_key": "0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69",
			  "sidechain_pub_key": "0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27"
			}
	  ],
}
```

### 4. Run the create-chain-spec wizard

The wizard reads the file `pc-chain-config.json`. This file should be present and identical for every node participating in the chain.

1. Start the wizard: `./partner-chains-node wizards create-chain-spec`

The wizard displays the contents of `chain_parameters` and `initial_permissioned_candidates` from the `pc-chain-config.json` file. You can manually modify these values before running this wizard.

The wizard creates the chain specification file `chain-spec.json` using these values.

The wizard informs you of the full path to the `chain-spec.json` file. You can now distribute this file to block production committee candidates.

### 5. Run the setup-main-chain-state wizard

1. Start the wizard: `./partner-chains-node wizards setup-main-chain-state`

The wizard reads the permissioned candidates list from the chain config file and Cardano. If it finds any discrepancy, it allows you to update the list. To update the list, add to the `initial_permissioned_candidates` array in `pc-chain-config.json` and re-run the setup-main-chain-state wizard.

Next, the wizard deals with the D parameter. If it is present on the main chain, the wizard displays its value and allows you to update it.

The D parameter has two values:

   - R, the number of registered candidate seats, and
   - P, the number of permissioned candidate seats.

The default value of R is zero, and the default value of P is the number of entries in the list of permissioned candidates.

The configuration of the chain is stored in the file `pc-chain-config.json`. This file should be present and identical for every node participating in the network.

Information about the resources used by each node is stored in the file `partner-chain-cli-resources-config.json`. This file should be present for every node participating in the chain, but its contents are specific to each node.

### 6. Run the partner chain node

The start-node wizard is used to start a partner chain node. Make sure that `cardano-node` is running with DB Sync running and fully synced. You will need to provide a link to a PostgreSQL server running with DB Sync as part of starting the node.

Be sure two main chain (Cardano) epochs have passed since the registration of a new partner chain before running the start-node wizard. On the preview network, this is between 1-2 days.

1. Start the wizard: `./partner-chains-node wizards start-node`
2. The wizard checks if all required keys are present. If not, it reminds you to run the generate-keys wizard first, and exits.
3. If the `chain-spec` file is not present, it should be generated with the create-chain-spec wizard.
4. The wizard checks the `pc-chain-config.json` file. If it is missing or invalid, it should be generated with the prepare-configuration wizard.
5. If the `db_sync_postgres_connection_string` is missing from the `partner-chain-cli-resources-config.json` file, the wizard prompts for it, using the default value `postgresql://postgres-user:postgres-password@localhost:5432/cexplorer`.
6. The wizard outputs all relevant parameters and asks if they are correct. If not, you should edit the `pc-chain-config.json` and/or `partner-chain-cli-resources-config.json` files and run the wizard again.

### 7. Distribute chain files to participants

The partner chain is now ready to start accepting registered validator nodes. [Permissioned candidates](./docs/user-guides/permissioned.md) and [Registered candidates](./docs/user-guides/registered.md) have different onboarding processes. Please follow the respective steps for the corresponding type of user.

Be prepared to share `chain-spec.json` and `pc-chain-config.json` files to both types of users.
