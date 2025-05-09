# Registered block producer onboarding

A registered block producer is a Cardano stake pool operator (SPO) that desires to process and validate transaction blocks on the partner chain. A registered block producer will use the partner chains toolkit (as well as other tools) to contribute to the validity of the partner chain ledger and, in turn, secure it.

## Order of Operations
1. Become a Cardano SPO
2. Install partner chains dependencies
    1. Cardano node
        1. Ogmios
        2. Cardano DB Sync
    2. Download the partner chain node
3. Run the generate-keys wizard
4. Obtain chain parameters from the chain builder
5. Register for the partner chain
6. Run the partner chain node
7. Optional: deregister from the partner chain

---
**NOTE**

This guide is currently aimed at the **preview testnet only**. In most `cardano-node` operations, this can be set with `--testnet-magic 2`. If you need to set up an SPO, be sure it's set up on **preview**.

---

### 1. Become a Cardano SPO on preview

An operational Cardano Stake Pool is required to validate a partner chain as a Registered user. The installation for a Cardano Stake Pool is out of the scope of this guide. Refer to the [Cardano course handbook](https://cardano-course.gitbook.io/cardano-course/handbook) for documentation and video instruction.

Once you have a Cardano Stake Pool registered on the preview network, you are ready to continue with this guide.

### 2. Install partner chain dependencies

To run the partner chains stack, several dependencies need to be installed on the same machine as the Cardano node.

Ogmios and DB Sync are essential to enable registration communication with the main chain (Cardano).

---
**NOTE**

Consult the Compatibility matrix on the releases page for dependency version compatibility for a particular release. These change with each [release](https://github.com/input-output-hk/partner-chains/releases).
---

### 2.1 Cardano node dependencies

The following tools are required for running a Cardano node.

---
**NOTE**

Be mindful of file paths in the instruction sets below. Your `cardano-node` may have slightly different paths for certain files. Replace file paths below with the paths relevant to your node.

---

### 2.1.1 Ogmios

Ogmios is a lightweight bridge interface for `cardano-node`. It offers a WebSocket API that enables local clients to speak to the main chain via JSON/RPC.

It is recommended to install [Ogmios](https://github.com/CardanoSolutions/ogmios) via pre-built binaries.

You can also build from source, though it requires a significant number of dependencies.

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

### 2.1.2 Cardano DB Sync

The partner chain needs DB Sync on a `cardano-node` to observe Cardano's state.

Cardano DB Sync is configurable in regards to the data it indexes.
The default configuration works well, if you don't use the default configuration,
then please read [partner-chains-db-sync-data-sources module header](../../toolkit/data-sources/db-sync/src/lib.rs)

#### A critical note on Cardano DB Sync!

> Before starting the partner chain node, and during normal operations, it is essential that the DB Sync component is fully synchronized.
> Running the node with lagging or not fully synced DB Sync will result in consensus errors, decreased peer reputation, and possibly a temporary ban by network peers.
> Sync time depends on hardware and network conditions, but here are approximate estimations for each network:

#### Sync time required

- Preview: a few hours
- Pre-production: usually ranges from several hours to a day
- Mainnet: two or more days.

Typical error message if db-sync is behind:

```
💔 Verification failed for block 0x151ed479f5766f8dc56fa3626329baa77292d5a692cf7fb9d24e743ae57fe71c received from (12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp): "Main chain state e04eea9347162cd773a3505692d0aaee3d49b2c61f21a5b8a95f3d5711a63961 referenced in imported block at slot 286497345 with timestamp 1718984070000 not found"
```

In this case, it's best to stop the partner chain node and make sure DB Sync is healthy and synchronized before restarting the node.

1. Download the [binary](https://github.com/IntersectMBO/cardano-db-sync/releases) and add it to the PATH
2. Set up PostgreSQL server:

```
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql.service
```

Enter shell as default postgres user: `sudo -i -u postgres`

Enter the postgres CLI: `psql`

Create User: `CREATE USER ubuntu WITH PASSWORD 'XXXXXXXXXXXXX';`

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
ExecStart=/usr/local/bin/cardano-db-sync --config /home/ubuntu/testnet/configs/db-sync-config.json --socket-path /home/ubuntu/testnet/node.socket --state-dir /home/ubuntu/testnet/db-sync/ledger-state --schema-dir /home/ubuntu/cardano-db-sync/schema/
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

### 2.2 Download the Partner Chains node

1. Download the Partner Chains node from the [official repository](https://github.com/input-output-hk/partner-chains)

### 3. Run the generate-keys wizard

The generate-keys wizard is designed to simplify the process of getting started with a partner chains node. This is the initial step for network participants who do not yet have keys.

The generate-keys wizard will generate necessary keys and save them to your node’s keystore. The following keys will be created:

1. ECDSA cross-chain key
2. ED25519 Grandpa key
3. SR25519 Aura key

If these keys already exist in the node’s keystore, you will be asked to overwrite existing keys. The wizard will also generate a network key for your node if needed.

1. To start the wizard, run the following command in the node repository:
`./partner-chains-node wizards generate-keys`
3. Input the node base path. It is saved in `pc-resources-config.json`

Now the wizard will output `partner-chains-public-keys.json` containing three keys:

``` javascript
{
	"sidechain_pub_key": "0x<key>",
	"aura_pub_key": "0x<key>",
	"grandpa_pub_key": "0x<key>"
}
```

### 4. Obtain chain parameters

Obtaining the chain parameters needs to be done manually.

Contact the chain builder and request the `chain-spec.json` file and the `pc-chain-config.json` file.

### 5. Register for the partner chain

Registration is a three-step process, with the second step executed on the 'cold' machine, so there are three wizards.

#### Register-1 wizard

The register-1 wizard obtains the registration UTXO.

1. Start the wizard: `./partner-chains-node wizards register1`
2. Follow the steps when prompted by the wizard

The wizard derives a payment address from the payment verification key and queries Ogmios for the UTXOs of the derived address.
It filters the UTXOs, retains only the ones without assets, and presents them to you, together with their lovelace balance, as a table. Use the up and down arrow keys to choose a row and press `enter` to select it.

---
**NOTE**

You must not spend the selected UTXO, because it needs to be consumed later in the registration process.

---

Finally, the wizard outputs the command for obtaining signatures, this command will be used as input in the next step (register-2 wizard). We suggest the command to be run on an offline machine, as to not expose the Cardano `cold.skey` to the internet, and return to the online machine to perform the register-3 wizard.

#### Register-2 wizard

The register-2 wizard obtains signatures for the registration message. It only requires the `partner-chain-cli` binary executable to be installed on the offline machine.

1. Follow the steps when prompted by the wizard

The wizard outputs the final command to be input to the register-3 wizard.

#### Register-3 wizard

The register-3 wizard executes the registration command. Be sure to have the `chain-spec.json` file present before continuing.

1. Follow the steps when prompted by the wizard

The wizard will give you the option of displaying the registration status. If you choose to display it, the wizard informs you that it will query the DB Sync PostgreSQL for the user registration status, and output it for the epoch that is two epochs ahead of the current one.

### 6. Run the partner chain node

The start-node wizard is used to start a partner chain node. Make sure that `cardano-node` is running with DB Sync running and fully synced. You will need to provide a link to postgreSQL server running with DB Sync as part of starting the node.

1. Start the wizard: `./partner-chains-node wizards start-node`.
2. The wizard checks if all required keys are present. If not, it reminds you to the run the generate-keys wizard first, and exits.
3. If the `chain-spec` file is not present, you should obtain it from the governance authority.
4. The wizard checks the `pc-chain-config.json` file. If it is missing or invalid, you should obtain it from the governance authority.
5. If the `db_sync_postgres_connection_string` is missing from the `partner-chain-cli-resources-config.json` file, the wizard prompts for it using the default value `postgresql://postgres-user:postgres-password@localhost:5432/cexplorer`.
6. The wizard outputs all relevant parameters and asks if they are correct. If not, you should edit the `pc-chain-config.json` and/or `partner-chain-cli-resources-config.json` files and run the wizard again.

The wizard sets the required environment variables and starts the node.

Registration is effective after 1-2 Cardano epochs. After the waiting period, the partner chain node is registered on the partner chain and is a selection option for the consensus committee.

### 7. Optional: deregister from the partner chain

To deregister from the list of block producer candidates, you need to run the deregister wizard.

1. Start the wizard: `./partner-chains-node wizards deregister`.
2. The wizard checks the `pc-chain-config.json` file.
3. The wizard prompts for the payment verification key file used during registration.
4. The wizard prompts for the cold verification key matching the cold signing key used during registration.
5. The wizard prompts for the Ogmios address.
6. The wizard executes the deregistration command. The change will be effective after two Cardano epochs boundaries.

---
**NOTE**

The configuration of the chain is stored in the file `pc-chain-config.json`. This file needs to remain identical with other nodes in the network.

---

**NOTE**
Information about the resources used by each node is stored in the file `partner-chain-cli-resources-config.json`. This file should be present on every node in the network, but its contents are specific to each node.

---
