# BEEFY Binary Upgrade Tools

This directory contains the BEEFY-enabled partner-chains-node binary and helper scripts for upgrading nodes during testing.

## Files

- `partner-chains-node-beefy` - BEEFY-enabled binary (version 1.7.0-5f2afb1b15a)
- `upgrade-node-binary.sh` - Upgrade single node binary
- `upgrade-all-nodes.sh` - Upgrade multiple nodes at once
- `check-node-versions.sh` - Check versions across all nodes

## Usage

### Check current versions
```bash
./check-node-versions.sh
```

### Upgrade a single node
```bash
./upgrade-node-binary.sh partner-chains-node-1
docker restart partner-chains-node-1
```

### Upgrade multiple nodes
```bash
./upgrade-all-nodes.sh partner-chains-node-1 partner-chains-node-2 partner-chains-node-3
docker restart partner-chains-node-1 partner-chains-node-2 partner-chains-node-3
```

### Testing Scenario

1. Start with master branch network (non-BEEFY nodes)
2. Verify all nodes are working with `./check-node-versions.sh`
3. Upgrade specific nodes: `./upgrade-all-nodes.sh partner-chains-node-1 partner-chains-node-2`
4. Restart upgraded nodes: `docker restart partner-chains-node-1 partner-chains-node-2`
5. Add BEEFY keys to upgraded nodes
6. Test BEEFY functionality
7. Gradually upgrade remaining nodes as needed

## Binary Information

- **Source**: Copied from partner-chains-node-1 container
- **Version**: 1.7.0-5f2afb1b15a (BEEFY-enabled)
- **Architecture**: Linux x86_64
- **Size**: ~88MB

## Notes

- Always backup original binaries (done automatically by upgrade script)
- Container restart is required after binary upgrade
- Check logs after restart to verify BEEFY functionality
