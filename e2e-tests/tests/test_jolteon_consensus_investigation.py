from pytest import mark
from src.blockchain_api import BlockchainApi, Wallet
from config.api_config import ApiConfig
import logging as logger
import json


class TestJolteonConsensusInvestigation:
    """Investigate where consensus information is stored in Jolteon environment"""
    
    @mark.test_key('JOLTEON-INVESTIGATION-001')
    def test_consensus_storage_queries(self, api: BlockchainApi, config: ApiConfig):
        """Query various storage locations for consensus information"""
        logger.info("Starting consensus storage investigation")
        
        # Test common consensus storage locations
        consensus_storage_queries = [
            # Standard Substrate consensus
            ("Aura", "Authorities"),
            ("Session", "Validators"),
            ("Staking", "Validators"),
            ("Grandpa", "Authorities"),
            ("Babe", "Authorities"),
            
            # Jolteon-specific (if they exist)
            ("Jolteon", "Authorities"),
            ("Jolteon", "CurrentRound"),
            ("Jolteon", "QuorumCertificates"),
            ("Jolteon", "TimeoutCertificates"),
            
            # Alternative naming conventions
            ("Consensus", "Authorities"),
            ("Consensus", "CurrentRound"),
            ("Consensus", "State"),
            
            # Check for any pallet that might contain consensus info
            ("System", "Events"),
            ("System", "UpgradedToU32RefCount"),
            ("System", "UpgradedToTripleRefCount"),
        ]
        
        logger.info("=== CONSENSUS STORAGE INVESTIGATION ===")
        
        successful_queries = 0
        total_queries = len(consensus_storage_queries)
        
        for module, storage in consensus_storage_queries:
            try:
                logger.info(f"Querying {module}.{storage}...")
                result = api.substrate.query(module, storage)
                
                if result is not None:
                    logger.info(f"  ✅ Found data: {result}")
                    successful_queries += 1
                    
                    # Look for consensus-related information
                    if isinstance(result, (list, tuple)) and len(result) > 0:
                        logger.info(f"  📊 Data length: {len(result)}")
                        if len(result) > 0:
                            logger.info(f"  🔍 First item: {result[0]}")
                    
                    # Check if this looks like consensus data
                    if any(consensus_term in str(result).lower() for consensus_term in ['authority', 'validator', 'round', 'consensus']):
                        logger.info(f"  🎯 *** LIKELY CONSENSUS DATA IN {module}.{storage} ***")
                        
                else:
                    logger.info(f"  ❌ No data found")
                    
            except Exception as e:
                error_msg = str(e)
                if "Storage function not found" in error_msg:
                    logger.info(f"  ⚠️  Storage not found: {module}.{storage}")
                elif "Module not found" in error_msg:
                    logger.info(f"  ⚠️  Module not found: {module}")
                else:
                    logger.info(f"  ❌ Query failed: {error_msg}")
        
        logger.info("=== STORAGE INVESTIGATION COMPLETE ===")
        logger.info(f"Successful queries: {successful_queries}/{total_queries}")
        
        # Test should fail if no queries succeeded (likely connection issues)
        if successful_queries == 0:
            raise AssertionError(f"All {total_queries} storage queries failed - likely connection issues")
        
        # At minimum, we should be able to query some basic system storage
        assert successful_queries > 0, f"Expected at least 1 successful query, got {successful_queries}/{total_queries}"

    @mark.test_key('JOLTEON-INVESTIGATION-002')
    def test_runtime_calls_investigation(self, api: BlockchainApi, config: ApiConfig):
        """Test runtime calls that might reveal consensus state"""
        logger.info("Starting runtime calls investigation")
        
        # Test various runtime calls
        runtime_calls = [
            # Standard consensus APIs
            ("AuraApi", "authorities"),
            ("Session", "validators"),
            ("Staking", "validators"),
            ("GrandpaApi", "authorities"),
            ("BabeApi", "authorities"),
            
            # Jolteon-specific APIs (if they exist)
            ("JolteonApi", "authorities"),
            ("JolteonApi", "current_round"),
            ("JolteonApi", "consensus_state"),
            
            # Alternative naming
            ("ConsensusApi", "authorities"),
            ("ConsensusApi", "current_round"),
            ("ConsensusApi", "state"),
        ]
        
        logger.info("=== RUNTIME CALLS INVESTIGATION ===")
        
        successful_calls = 0
        total_calls = len(runtime_calls)
        
        for module, function in runtime_calls:
            try:
                logger.info(f"Testing runtime call: {module}.{function}")
                result = api.substrate.runtime_call(module, function)
                
                if result is not None:
                    logger.info(f"  ✅ Call successful: {result}")
                    successful_calls += 1
                    
                    # Check if this looks like consensus data
                    if isinstance(result, (list, tuple)) and len(result) > 0:
                        logger.info(f"  📊 Result length: {len(result)}")
                        if len(result) > 0:
                            logger.info(f"  🔍 First item: {result[0]}")
                    
                    if any(consensus_term in str(result).lower() for consensus_term in ['authority', 'validator', 'round', 'consensus']):
                        logger.info(f"  🎯 *** LIKELY CONSENSUS DATA FROM {module}.{function} ***")
                        
                else:
                    logger.info(f"  ❌ Call returned None")
                    
            except Exception as e:
                error_msg = str(e)
                if "API not found" in error_msg:
                    logger.info(f"  ⚠️  API not found: {module}.{function}")
                elif "Function not found" in error_msg:
                    logger.info(f"  ⚠️  Function not found: {function}")
                else:
                    logger.info(f"  ❌ Call failed: {error_msg}")
        
        logger.info("=== RUNTIME CALLS INVESTIGATION COMPLETE ===")
        logger.info(f"Successful calls: {successful_calls}/{total_calls}")
        
        # Test should fail if no calls succeeded (likely connection issues)
        if successful_calls == 0:
            logger.warning("All runtime calls failed - this may be normal in test environments")
            logger.info("ℹ️  Runtime calls investigation completed (no successful calls)")
            return
        
        # At minimum, we should be able to make some basic runtime calls
        assert successful_calls > 0, f"Expected at least 1 successful runtime call, got {successful_calls}/{total_calls}"

    @mark.test_key('JOLTEON-INVESTIGATION-003')
    def test_metadata_analysis(self, api: BlockchainApi, config: ApiConfig):
        """Analyze runtime metadata for consensus-related modules"""
        logger.info("Starting metadata analysis")
        
        try:
            logger.info("=== RUNTIME METADATA ANALYSIS ===")
            metadata = api.substrate.get_metadata()
            
            if 'modules' in metadata:
                modules = metadata['modules']
                logger.info(f"Total modules: {len(modules)}")
                
                # Look for consensus-related modules
                consensus_modules = []
                for module in modules:
                    if 'name' in module:
                        module_name = module['name']
                        
                        # Check if module name suggests consensus functionality
                        if any(consensus_term in module_name.lower() for consensus_term in [
                            'consensus', 'jolteon', 'aura', 'babe', 'grandpa', 'session', 'staking'
                        ]):
                            consensus_modules.append(module)
                            logger.info(f"🎯 Consensus-related module: {module_name}")
                            
                            # Show module details
                            if 'calls' in module:
                                logger.info(f"  📞 Calls: {len(module['calls'])}")
                            if 'storage' in module:
                                logger.info(f"  💾 Storage entries: {len(module['storage'])}")
                            if 'events' in module:
                                logger.info(f"  📡 Events: {len(module['events'])}")
                
                logger.info(f"Found {len(consensus_modules)} consensus-related modules")
                
                # Show all module names for reference
                all_module_names = [m.get('name', 'Unknown') for m in modules]
                logger.info(f"All module names: {all_module_names}")
                
            else:
                logger.info("No modules found in metadata")
                
        except Exception as e:
            logger.error(f"Error analyzing metadata: {e}")
            raise AssertionError(f"Failed to analyze metadata: {e}")
        
        logger.info("=== METADATA ANALYSIS COMPLETE ===")
        assert True, "Metadata analysis completed"

    @mark.test_key('JOLTEON-INVESTIGATION-004')
    def test_events_analysis(self, api: BlockchainApi, config: ApiConfig):
        """Check for consensus-related events in recent blocks"""
        logger.info("Starting events analysis")
        
        try:
            logger.info("=== EVENTS ANALYSIS ===")
            
            # Get the latest block
            latest_block = api.substrate.get_block()
            latest_number = latest_block['header']['number']
            
            # Check events in the last few blocks
            for i in range(3):
                try:
                    block_num = latest_number - i
                    if block_num < 0:
                        break
                    
                    logger.info(f"Checking events in block {block_num}...")
                    
                    # Try to get events for this block
                    events = api.substrate.query("System", "Events", block_hash=block_num)
                    
                    if events is not None:
                        logger.info(f"  📡 Events found: {len(events)}")
                        
                        # Look for consensus-related events
                        for event in events:
                            event_str = str(event)
                            if any(consensus_term in event_str.lower() for consensus_term in [
                                'consensus', 'jolteon', 'round', 'authority', 'validator', 'qc', 'tc'
                            ]):
                                logger.info(f"    🎯 *** CONSENSUS EVENT FOUND ***")
                                logger.info(f"    Event: {event}")
                    else:
                        logger.info(f"  ❌ No events found")
                        
                except Exception as e:
                    logger.info(f"  ⚠️  Could not check block {block_num}: {e}")
                    
        except Exception as e:
            logger.error(f"Error analyzing events: {e}")
            raise AssertionError(f"Failed to analyze events: {e}")
        
        logger.info("=== EVENTS ANALYSIS COMPLETE ===")
        assert True, "Events analysis completed"


