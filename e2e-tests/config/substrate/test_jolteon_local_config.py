#!/usr/bin/env python3
"""
Test script to verify Jolteon local environment configuration.
"""

import json
import os
import sys
from pathlib import Path

def test_config_loading():
    """Test that the Jolteon local configuration files can be loaded."""
    
    # Get the current directory
    current_dir = Path(__file__).parent
    
    # Test nodes configuration
    nodes_config_path = current_dir / "jolteon_local_nodes.json"
    if not nodes_config_path.exists():
        print(f"❌ Nodes config file not found: {nodes_config_path}")
        return False
    
    try:
        with open(nodes_config_path, 'r') as f:
            nodes_config = json.load(f)
        print(f"✅ Nodes config loaded successfully")
        
        # Verify key fields
        if 'test_environment' not in nodes_config:
            print("❌ Missing 'test_environment' field")
            return False
        
        if nodes_config['test_environment'] != 'jolteon_local':
            print(f"❌ Wrong test environment: {nodes_config['test_environment']}")
            return False
        
        # Check nodes configuration
        nodes = nodes_config.get('nodes_config', {}).get('nodes', {})
        if not nodes:
            print("❌ No nodes configured")
            return False
        
        for node_name, node_config in nodes.items():
            if node_config.get('host') != '127.0.0.1':
                print(f"❌ Node {node_name} host is not 127.0.0.1: {node_config.get('host')}")
                return False
            if node_config.get('port') != '9933':
                print(f"❌ Node {node_name} port is not 9933: {node_config.get('port')}")
                return False
        
        print(f"✅ All {len(nodes)} nodes configured correctly")
        
    except json.JSONDecodeError as e:
        print(f"❌ Invalid JSON in nodes config: {e}")
        return False
    except Exception as e:
        print(f"❌ Error loading nodes config: {e}")
        return False
    
    # Test stack configuration
    stack_config_path = current_dir / "jolteon_local_stack.json"
    if not stack_config_path.exists():
        print(f"❌ Stack config file not found: {stack_config_path}")
        return False
    
    try:
        with open(stack_config_path, 'r') as f:
            stack_config = json.load(f)
        print(f"✅ Stack config loaded successfully")
        
        # Verify key fields
        stack_config_data = stack_config.get('stack_config', {})
        if not stack_config_data:
            print("❌ Missing 'stack_config' field")
            return False
        
        tools = stack_config_data.get('tools', {})
        if not tools:
            print("❌ No tools configured")
            return False
        
        print(f"✅ Tools configured: {list(tools.keys())}")
        
    except json.JSONDecodeError as e:
        print(f"❌ Invalid JSON in stack config: {e}")
        return False
    except Exception as e:
        print(f"❌ Error loading stack config: {e}")
        return False
    
    # Test CI configuration
    ci_config_path = current_dir / "jolteon_local-ci.json"
    if not ci_config_path.exists():
        print(f"❌ CI config file not found: {ci_config_path}")
        return False
    
    try:
        with open(ci_config_path, 'r') as f:
            ci_config = json.load(f)
        print(f"✅ CI config loaded successfully")
        
    except json.JSONDecodeError as e:
        print(f"❌ Invalid JSON in CI config: {e}")
        return False
    except Exception as e:
        print(f"❌ Error loading CI config: {e}")
        return False
    
    return True

def test_secrets_structure():
    """Test that the secrets directory structure exists."""
    
    # Get the secrets directory
    secrets_dir = Path(__file__).parent.parent.parent / "secrets" / "substrate" / "jolteon_local"
    
    if not secrets_dir.exists():
        print(f"❌ Secrets directory not found: {secrets_dir}")
        return False
    
    print(f"✅ Secrets directory exists: {secrets_dir}")
    
    # Check for required files
    required_files = ['jolteon_local.json', 'jolteon_local-ci.json']
    for file_name in required_files:
        file_path = secrets_dir / file_name
        if not file_path.exists():
            print(f"❌ Secrets file not found: {file_path}")
            return False
        print(f"✅ Secrets file exists: {file_name}")
    
    # Check for keys directory
    keys_dir = secrets_dir / "keys"
    if not keys_dir.exists():
        print(f"❌ Keys directory not found: {keys_dir}")
        return False
    
    print(f"✅ Keys directory exists: {keys_dir}")
    
    return True

def main():
    """Main test function."""
    print("Testing Jolteon Local Environment Configuration")
    print("=" * 50)
    
    config_ok = test_config_loading()
    secrets_ok = test_secrets_structure()
    
    print("\n" + "=" * 50)
    if config_ok and secrets_ok:
        print("✅ All tests passed! Jolteon local environment is ready.")
        print("\nTo run tests:")
        print("pytest --env=jolteon_local --blockchain=substrate")
        return 0
    else:
        print("❌ Some tests failed. Please check the configuration.")
        return 1

if __name__ == "__main__":
    sys.exit(main())
