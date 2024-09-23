# Permissioned candidate onboarding

A blockchain validator is a network node that helps process and validate transaction blocks on the platform according to the protocol consensus mechanism. A permissioned candidate is a block-producing validator that has been whitelisted by the chain builder so it can validate partner chain blocks and secure the network.

Before you begin, some software must be installed.

## Order of Operations

1. Install dependencies
    1. Cardano node v9.1.1
        1. DB Sync v13.5.0.2 (PostgreSQL v15.3)
2. Download the partner chain node v1.1.0
3. Run the generate-keys wizard
4. Share keys with the chain builder
5. Obtain the chain configuration and specification files
6. Run the partner chain node

---
**NOTE**

This guide is currently aimed at the **preview testnet only**. In most `cardano-node` operations, this can be set with `--testnet-magic 2`. If you need to set up an SPO, be sure it's set up on **preview**.

---

### 1. Install partner chains dependencies

To run the partner chains stack, several dependencies need to be installed on the Cardano node.

### 1.1 Cardano node v9.1.1

A passive Cardano node is required to validate a partner chain. The installation of `cardano-node` is out of the scope of this guide. Refer to the [Cardano course handbook](https://cardano-course.gitbook.io/cardano-course/handbook) for documentation and video instruction.

Once your node is synced with the preview testnet, you are ready to continue with this guide.

### 1.1.1 DB Sync

The partner chain needs DB Sync on a `cardano-node` to observe Cardano's state.

#### A critical note on Cardano DB Sync!

> Before starting the partner chain node, and during normal operations, it is essential that the DB Sync component is fully synchronized.
> Running the node with lagging or not fully synced DB Sync will result in consensus errors, decreased peer reputation, and possibly a temporary ban by network peers. 
> Sync time depends on hardware and network conditions, but here are approximate estimations for each network:

#### Sync time required

- Preview: About 10 hours
- Pre-production: usually ranges from several hours to a day
- Mainnet: two or more days.
  
Typical error message if db-sync is behind:
```
💔 Verification failed for block 0x151ed479f5766f8dc56fa3626329baa77292d5a692cf7fb9d24e743ae57fe71c received from (12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp): "Main chain state e04eea9347162cd773a3505692d0aaee3d49b2c61f21a5b8a95f3d5711a63961 referenced in imported block at slot 286497345 with timestamp 1718984070000 not found"
```

1. Download the [binary](https://github.com/IntersectMBO/cardano-db-sync/releases) and add it to the PATH
2. Set up PostgreSQL server:
```
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql.service
```

Enter shell as default postgres user:
`sudo -i -u postgres`

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
journalctl -fu cardano-dy-sync.service
```

---
**IMPORTANT NOTE**

Ensure that the node is synced with the network to 100% as well as Kupo and DB Sync before continuing beyond this point. On preview network, it is roughly 24 hours before sync is complete.

---

### 1.2 Download the Partner Chain node

1. Download the Partner Chain node from the [official repository](https://github.com/input-output-hk/partner-chains)

### 2. Run the generate-keys wizard

The generate-keys wizard is designed to simplify the process of getting started with a partner chains node. This is the initial step for network participants who do not yet have keys.

The generate-keys wizard will generate necessary keys and save them to your node’s keystore. The following keys will be created:

1. ECDSA cross-chain key
2. ED25519 Grandpa key
3. SR25519 Aura key

If these keys already exist in the node’s keystore, you will be asked to overwrite existing keys. The wizard will also generate a network key for your node if needed.

1. Start the wizard: `./partner-chains-cli generate-keys`
2. Input the node base path. It is saved in `partner-chains-cli-resources-config.json`.

Now the wizard will output `partner-chains-public-keys.json` containing three keys.
``` javascript
{
	"sidechain_pub_key": "0x<key>",
	"aura_pub_key": "0x<key>",
	"grandpa_pub_key": "0x<key>"
}
```

### 4. Share keys with the chain builder

In this step, the `partner-chains-cli-public-keys.json` file needs to be shared with the chain builder so that the permissioned validator can be whitelisted.

Contact the chain builder and provide the `partner-chains-cli-public-keys.json` file.

### 5. Obtain the chain configuration and specification files

Obtaining these files is as simple as getting the file from the chain builder.

Contact the chain builder and request the `chain-spec.json` and `partner-chains-cli-chain-config.json` files.

### 6. Run the partner chain node

The start-node wizard is used to start a partner chain node. Make sure that `cardano-node` is running with DB Sync running and fully synced. You will need to provide a link to postgreSQL server running with DB Sync as part of starting the node.

1. Start the wizard: `./partner-chains-cli start-node`
2. The wizard checks if all required keys are present. If not, it reminds you to run the generate-keys wizard first, and exits.
3. If the `chain-spec` file is not present, you should obtain it from the chain builder.
4. The wizard checks the `partner-chains-cli-chain-config.json` file. If it is missing or invalid, you should obtain it from the chain builder.
5. If the `db_sync_postgres_connection_string` is missing from the `partner-chain-cli-resources-config.json` file, the wizard prompts for it, using the default value `postgresql://postgres-user:postgres-password@localhost:5432/cexplorer`.
6. The wizard outputs all relevant parameters and asks if they are correct. If not, you should edit the `partner-chains-cli-chain-config.json` and/or `partner-chain-cli-resources-config.json` files and run the wizard again.

The wizard sets the required environment variables and starts the node.

---
**NOTE**

The configuration of the chain is stored in the file `partner-chains-cli-chain-config.json`. This file needs to remain idential with other nodes in the network.

---

---
**NOTE**

Information about the resources used by each node is stored in the file `partner-chain-cli-resources-config.json`. This file should be present on every node in the network, but its contents are specific to each node.

---
