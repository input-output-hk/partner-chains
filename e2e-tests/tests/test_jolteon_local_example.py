#!/usr/bin/env python3
"""
Example test for Jolteon local environment.
This test demonstrates how to use the jolteon_local environment configuration.
"""

import pytest
from src.partner_chain_rpc import PartnerChainRpc


def test_jolteon_local_environment_connection(config):
    """Test that we can connect to the Jolteon local environment."""
    
    # Verify we're using the correct environment
    assert config.test_environment == "jolteon_local", f"Expected jolteon_local, got {config.test_environment}"
    
    # Verify the selected node configuration
    node = config.nodes_config.node
    assert node.host == "127.0.0.1", f"Expected 127.0.0.1, got {node.host}"
    assert node.port == 9933, f"Expected 9933, got {node.port}"
    
    # Verify RPC URL construction
    expected_rpc_url = "http://127.0.0.1:9933"
    assert node.rpc_url == expected_rpc_url, f"Expected {expected_rpc_url}, got {node.rpc_url}"
    
    print(f"✅ Environment configured correctly:")
    print(f"   - Environment: {config.test_environment}")
    print(f"   - Node: {config.nodes_config.selected_node}")
    print(f"   - Host: {node.host}")
    print(f"   - Port: {node.port}")
    print(f"   - RPC URL: {node.rpc_url}")


def test_jolteon_local_node_list(config):
    """Test that all nodes are configured for localhost."""
    
    nodes = config.nodes_config.nodes
    
    # Verify we have the expected nodes
    expected_nodes = ["alice", "bob", "charlie", "dave"]
    actual_nodes = list(nodes.keys())
    assert actual_nodes == expected_nodes, f"Expected {expected_nodes}, got {actual_nodes}"
    
    # Verify all nodes point to localhost
    for node_name, node_config in nodes.items():
        assert node_config.host == "127.0.0.1", f"Node {node_name} host is not 127.0.0.1"
        assert node_config.port == 9933, f"Node {node_name} port is not 9933"
    
    print(f"✅ All {len(nodes)} nodes configured for localhost")


def test_jolteon_local_network_config(config):
    """Test that the network configuration is correct for Jolteon."""
    
    # Verify main chain configuration
    main_chain = config.main_chain
    assert main_chain.network == "--testnet-magic 2", f"Expected --testnet-magic 2, got {main_chain.network}"
    
    # Verify deployment configuration
    assert config.deployment_version == "v1.7.0-rc2", f"Expected v1.7.0-rc2, got {config.deployment_version}"
    assert config.deployment_mc_epoch == 958, f"Expected 958, got {config.deployment_mc_epoch}"
    assert config.initial_pc_epoch == 4859579, f"Expected 4859579, got {config.initial_pc_epoch}"
    
    print(f"✅ Network configuration correct:")
    print(f"   - Network: {main_chain.network}")
    print(f"   - Deployment Version: {config.deployment_version}")
    print(f"   - Deployment MC Epoch: {config.deployment_mc_epoch}")
    print(f"   - Initial PC Epoch: {config.initial_pc_epoch}")


def test_jolteon_local_stack_config(config):
    """Test that the stack configuration is correct."""
    
    stack_config = config.stack_config
    
    # Verify ogmios configuration
    assert stack_config.ogmios_host == "localhost", f"Expected localhost, got {stack_config.ogmios_host}"
    assert stack_config.ogmios_port == 1337, f"Expected 1337, got {stack_config.ogmios_port}"
    assert stack_config.ogmios_scheme == "http", f"Expected http, got {stack_config.ogmios_scheme}"
    
    # Verify tools are configured
    tools = stack_config.tools
    assert hasattr(tools, 'runner'), "Missing runner tool"
    assert hasattr(tools, 'cardano_cli'), "Missing cardano_cli tool"
    assert hasattr(tools, 'node'), "Missing node tool"
    
    print(f"✅ Stack configuration correct:")
    print(f"   - Ogmios URL: {stack_config.ogmios_url}")
    print(f"   - Tools: {list(tools.__dict__.keys())}")


if __name__ == "__main__":
    # This allows running the test directly for debugging
    pytest.main([__file__, "-v"])
