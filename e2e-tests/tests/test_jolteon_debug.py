from time import sleep
from pytest import mark
from src.blockchain_api import BlockchainApi, Wallet
from config.api_config import ApiConfig
import logging as logger
import json


class TestJolteonDebug:
    """Debug tests to understand Jolteon block structure and consensus metadata"""
    
    @mark.test_key('JOLTEON-DEBUG-001')
    def test_block_structure_analysis(self, api: BlockchainApi, config: ApiConfig):
        """Analyze the actual block structure to understand how consensus data is stored"""
        logger.info("Starting Jolteon block structure analysis")
        
        try:
            # Get a recent block and examine its complete structure
            block_info = api.substrate.get_block()
            
            logger.info("=== BLOCK STRUCTURE ANALYSIS ===")
            logger.info(f"Block number: {block_info['header']['number']}")
            
            # Safely check for hash - it might not exist
            if 'hash' in block_info:
                logger.info(f"Block hash: {block_info['hash']}")
            else:
                logger.info("No block hash found in response")
                logger.info(f"Available keys in block_info: {list(block_info.keys())}")
            
            # Analyze header structure
            header = block_info['header']
            logger.info(f"Header keys: {list(header.keys())}")
            
            # Examine digest items in detail
            if 'digest' in header:
                logger.info(f"Digest items count: {len(header['digest'])}")
                
                for i, item in enumerate(header['digest']):
                    logger.info(f"Digest item {i}: {item}")
                    
                    if isinstance(item, dict):
                        for key, value in item.items():
                            logger.info(f"  {key}: {value}")
                            
                            # Look for consensus-related data
                            if 'consensus' in key.lower() or 'jolteon' in key.lower():
                                logger.info(f"    *** CONSENSUS DATA FOUND: {key} ***")
                                logger.info(f"    Value: {value}")
                            
                            # Look for round information
                            if 'round' in key.lower():
                                logger.info(f"    *** ROUND DATA FOUND: {key} ***")
                                logger.info(f"    Value: {value}")
            else:
                logger.info("No digest found in header")
            
            # Check for other potential consensus indicators
            if 'extrinsicsRoot' in header:
                logger.info(f"Extrinsics root: {header['extrinsicsRoot']}")
            
            if 'stateRoot' in header:
                logger.info(f"State root: {header['stateRoot']}")
            
            # Try to get block by number to see if there's more detail
            try:
                current_number = header['number']
                block_by_number = api.substrate.get_block(current_number)
                logger.info("=== BLOCK BY NUMBER ANALYSIS ===")
                logger.info(f"Block by number keys: {list(block_by_number.keys())}")
                
                if 'header' in block_by_number:
                    num_header = block_by_number['header']
                    logger.info(f"Number header keys: {list(num_header.keys())}")
                    
                    if 'digest' in num_header:
                        logger.info(f"Number digest items: {num_header['digest']}")
            except Exception as e:
                logger.info(f"Could not get block by number: {e}")
            
            # Try to get multiple recent blocks to see patterns
            logger.info("=== MULTIPLE BLOCKS ANALYSIS ===")
            latest_number = header['number']
            
            for i in range(3):
                try:
                    block_num = latest_number - i
                    if block_num < 0:
                        break
                        
                    recent_block = api.substrate.get_block(block_num)
                    recent_header = recent_block['header']
                    
                    logger.info(f"Block {block_num}:")
                    if 'digest' in recent_header:
                        digest_items = recent_header['digest']
                        logger.info(f"  Digest items: {len(digest_items)}")
                        
                        # Look for consensus patterns
                        for item in digest_items:
                            if isinstance(item, dict):
                                for key in item.keys():
                                    if 'consensus' in key.lower() or 'jolteon' in key.lower():
                                        logger.info(f"    *** CONSENSUS PATTERN IN BLOCK {block_num}: {key} ***")
                except Exception as e:
                    logger.info(f"Could not analyze block {block_num}: {e}")
            
            # Try to query some storage to see if consensus data is stored there
            logger.info("=== STORAGE QUERY ANALYSIS ===")
            storage_queries = [
                ("System", "Events"),
                ("Aura", "Authorities"),
                ("Session", "Validators"),
                ("Staking", "Validators"),
                ("Grandpa", "Authorities"),
                ("Babe", "Authorities"),
            ]
            
            for module, storage in storage_queries:
                try:
                    logger.info(f"Querying {module}.{storage}...")
                    result = api.substrate.query(module, storage)
                    logger.info(f"  Result: {result}")
                    
                except Exception as e:
                    logger.info(f"  {module}.{storage} query failed: {e}")
            
            logger.info("=== BLOCK STRUCTURE ANALYSIS COMPLETE ===")
            
            # Test should fail if we couldn't get any block data
            assert block_info is not None, "Failed to retrieve block data"
            assert 'header' in block_info, "Block data missing header"
            
        except Exception as e:
            logger.error(f"Error in block structure analysis: {e}")
            raise

    @mark.test_key('JOLTEON-DEBUG-002')
    def test_consensus_rpc_methods(self, api: BlockchainApi, config: ApiConfig):
        """Test various RPC methods that might reveal consensus information"""
        logger.info("Starting Jolteon consensus RPC methods test")
        
        try:
            # Test standard Substrate RPC methods
            rpc_methods = [
                "system_chain",
                "system_version", 
                "system_health",
                "chain_getHeader",
                "chain_getBlock",
                "state_getRuntimeVersion",
                "author_hasKey",
                "author_hasSessionKeys"
            ]
            
            logger.info("=== RPC METHODS TESTING ===")
            
            successful_rpc_calls = 0
            
            for method in rpc_methods:
                try:
                    logger.info(f"Testing RPC method: {method}")
                    result = api.substrate.rpc_request(method, [])
                    logger.info(f"  Result: {result}")
                    successful_rpc_calls += 1
                    
                    # Look for consensus-related information in responses
                    if isinstance(result, dict) and 'result' in result:
                        result_data = result['result']
                        if isinstance(result_data, dict):
                            for key in result_data.keys():
                                if any(consensus_term in key.lower() for consensus_term in ['consensus', 'jolteon', 'round', 'authority']):
                                    logger.info(f"    *** CONSENSUS INFO IN {method}: {key} ***")
                    
                except Exception as e:
                    logger.info(f"  {method} failed: {e}")
            
            # Test runtime calls that might reveal consensus state
            runtime_calls = [
                ("AuraApi", "authorities"),
                ("Session", "validators"),
                ("Staking", "validators"),
                ("GrandpaApi", "authorities"),
                ("BabeApi", "authorities")
            ]
            
            logger.info("=== RUNTIME CALLS TESTING ===")
            
            successful_runtime_calls = 0
            
            for module, function in runtime_calls:
                try:
                    logger.info(f"Testing runtime call: {module}.{function}")
                    result = api.substrate.runtime_call(module, function)
                    logger.info(f"  Result: {result}")
                    successful_runtime_calls += 1
                    
                except Exception as e:
                    logger.info(f"  {module}.{function} failed: {e}")
            
            logger.info("=== RPC METHODS TESTING COMPLETE ===")
            logger.info(f"Successful RPC calls: {successful_rpc_calls}/{len(rpc_methods)}")
            logger.info(f"Successful runtime calls: {successful_runtime_calls}/{len(runtime_calls)}")
            
            # Test should fail if no calls succeeded (likely connection issues)
            if successful_rpc_calls == 0 and successful_runtime_calls == 0:
                raise AssertionError("All RPC and runtime calls failed - likely connection issues")
            
            # At minimum, we should be able to make some basic calls
            assert successful_rpc_calls > 0 or successful_runtime_calls > 0, "No successful calls made"
            
        except Exception as e:
            logger.error(f"Error in consensus RPC methods test: {e}")
            raise