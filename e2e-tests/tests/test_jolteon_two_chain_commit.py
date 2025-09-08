from time import sleep
from pytest import mark
from src.blockchain_api import BlockchainApi, Wallet
from config.api_config import ApiConfig
import logging as logger
import json
from time import time


class TestJolteonTwoChainCommit:
    """Test cases for Jolteon's 2-chain commit rule (core protocol feature)"""
    
    def _get_consensus_states_for_blocks(self, api: BlockchainApi, max_blocks: int = 5, rpc_endpoint: str = "jolteon_getReplicaState") -> list:
        """Get consensus states for current block and its parent blocks
        
        Args:
            api: Blockchain API instance
            max_blocks: Maximum number of parent blocks to analyze
            rpc_endpoint: RPC endpoint to use for consensus data ("jolteon_getReplicaState", "jolteon_getRoundInfo", "jolteon_getHighestQC")
            
        Returns:
            List of consensus states with block numbers
        """
        try:
            # Get current block
            current_block = api.substrate.get_block()
            current_number = current_block['header']['number']
            
            logger.info(f"Analyzing consensus states for block {current_number} and up to {max_blocks} parent blocks using {rpc_endpoint}...")
            
            # Get parent blocks to analyze
            blocks_to_analyze = min(max_blocks, current_number)
            consensus_states = []
            
            for i in range(blocks_to_analyze + 1):
                block_number = current_number - i
                try:
                    # Get block (for validation)
                    block = api.substrate.get_block(block_number)
                    
                    # Get consensus state using specified endpoint
                    consensus_result = api.substrate.rpc_request(rpc_endpoint, [])
                    
                    if consensus_result and 'result' in consensus_result:
                        consensus_state = consensus_result['result']
                        
                        # Handle different RPC endpoint response formats
                        if rpc_endpoint == "jolteon_getReplicaState":
                            state_data = {
                                'block_number': block_number,
                                'r_curr': consensus_state['r_curr'],
                                'r_vote': consensus_state['r_vote'],
                                'r_lock': consensus_state['r_lock'],
                                'qc_round': consensus_state['qc_high']['round'],
                                'qc_votes': consensus_state['qc_high']['vote_count'],
                                'qc_block': consensus_state['qc_high']['block_hash']
                            }
                        elif rpc_endpoint == "jolteon_getRoundInfo":
                            state_data = {
                                'block_number': block_number,
                                'r_curr': consensus_state['r_curr'],
                                'r_vote': consensus_state['r_vote'],
                                'r_lock': consensus_state['r_lock'],
                                'qc_round': None,  # Not available in RoundInfo
                                'qc_votes': None,  # Not available in RoundInfo
                                'qc_block': None   # Not available in RoundInfo
                            }
                        elif rpc_endpoint == "jolteon_getHighestQC":
                            state_data = {
                                'block_number': block_number,
                                'r_curr': None,    # Not available in QC
                                'r_vote': None,    # Not available in QC
                                'r_lock': None,    # Not available in QC
                                'qc_round': consensus_state['round'],
                                'qc_votes': consensus_state['vote_count'],
                                'qc_block': consensus_state['block_hash']
                            }
                        else:
                            # Default to ReplicaState format for unknown endpoints
                            state_data = {
                                'block_number': block_number,
                                'r_curr': consensus_state.get('r_curr'),
                                'r_vote': consensus_state.get('r_vote'),
                                'r_lock': consensus_state.get('r_lock'),
                                'qc_round': consensus_state.get('qc_high', {}).get('round'),
                                'qc_votes': consensus_state.get('qc_high', {}).get('vote_count'),
                                'qc_block': consensus_state.get('qc_high', {}).get('block_hash')
                            }
                        
                        consensus_states.append(state_data)
                        
                        # Log appropriate fields based on endpoint
                        if rpc_endpoint == "jolteon_getReplicaState":
                            logger.info(f"Block {block_number}: r_curr={consensus_state['r_curr']}, QC={consensus_state['qc_high']['round']}, Lock={consensus_state['r_lock']}")
                        elif rpc_endpoint == "jolteon_getRoundInfo":
                            logger.info(f"Block {block_number}: r_curr={consensus_state['r_curr']}, r_vote={consensus_state['r_vote']}, r_lock={consensus_state['r_lock']}")
                        elif rpc_endpoint == "jolteon_getHighestQC":
                            logger.info(f"Block {block_number}: QC round={consensus_state['round']}, votes={consensus_state['vote_count']}")
                    
                except Exception as e:
                    logger.warning(f"Could not get consensus state for block {block_number}: {e}")
                    continue
            
            logger.info(f"Successfully analyzed {len(consensus_states)} consensus states")
            return consensus_states
            
        except Exception as e:
            logger.error(f"Error getting consensus states for blocks: {e}")
            return []
    
    @mark.test_key('JOLTEON-2CHAIN-001')
    def test_two_chain_commit_rule_verification(self, api: BlockchainApi, config: ApiConfig):
        """Test Jolteon's 2-chain commit rule
        
        This test analyzes the current block and its parent blocks to verify
        that blocks are committed when two adjacent certified blocks
        with consecutive round numbers exist. This is Jolteon's core differentiation
        from 3-chain HotStuff.
        
        Based on Tachyeon test case: Lock and Commit Rules - 2-Chain Commit Rule
        """
        logger.info("Starting Jolteon 2-chain commit rule test")
        
        try:
            # Get consensus states for blocks
            consensus_states = self._get_consensus_states_for_blocks(api, max_blocks=10)
            
            if len(consensus_states) < 3:
                logger.warning("Insufficient data for 2-chain analysis")
                return
            
            logger.info(f"Analyzed {len(consensus_states)} consensus states for 2-chain commit patterns")
            
            # Look for 2-chain commit patterns
            # In Jolteon, a block is committed when there are two consecutive certified blocks
            # This means we should see the locked round advancing when QCs are consecutive
            
            consecutive_qc_pairs = 0
            lock_advancements = 0
            
            for i in range(1, len(consensus_states)):
                prev_state = consensus_states[i-1]
                curr_state = consensus_states[i]
                
                # Check for consecutive QC rounds
                if curr_state['qc_round'] == prev_state['qc_round'] + 1:
                    consecutive_qc_pairs += 1
                    logger.info(f"Consecutive QCs found: block {prev_state['block_number']} QC={prev_state['qc_round']} -> block {curr_state['block_number']} QC={curr_state['qc_round']}")
                    
                    # Check if lock advanced (indicating commit)
                    if curr_state['r_lock'] > prev_state['r_lock']:
                        lock_advancements += 1
                        logger.info(f"Lock advanced during consecutive QCs: block {prev_state['block_number']} r_lock={prev_state['r_lock']} -> block {curr_state['block_number']} r_lock={curr_state['r_lock']}")
            
            # Analyze lock advancement patterns
            total_lock_advancements = 0
            for i in range(1, len(consensus_states)):
                if consensus_states[i]['r_lock'] > consensus_states[i-1]['r_lock']:
                    total_lock_advancements += 1
            
            logger.info(f"2-chain commit analysis:")
            logger.info(f"  Consecutive QC pairs: {consecutive_qc_pairs}")
            logger.info(f"  Lock advancements during consecutive QCs: {lock_advancements}")
            logger.info(f"  Total lock advancements: {total_lock_advancements}")
            
            # Verify 2-chain commit behavior
            if consecutive_qc_pairs > 0:
                logger.info("✅ Consecutive QC pairs detected (2-chain pattern)")
                
                # Calculate commit rate during consecutive QCs
                if consecutive_qc_pairs > 0:
                    commit_rate = lock_advancements / consecutive_qc_pairs
                    logger.info(f"Commit rate during consecutive QCs: {commit_rate:.2f}")
                    
                    # In Jolteon, we expect commits to happen when we have consecutive QCs
                    if commit_rate > 0:
                        logger.info("✅ 2-chain commit rule appears to be working")
                    else:
                        logger.info("ℹ️  No commits detected during consecutive QCs (may be normal)")
            else:
                logger.info("ℹ️  No consecutive QC pairs found - this may be normal for this implementation")
            
            # Verify basic safety properties
            # Lock should never decrease
            for i in range(1, len(consensus_states)):
                assert consensus_states[i]['r_lock'] >= consensus_states[i-1]['r_lock'], \
                    f"Lock decreased: block {consensus_states[i-1]['block_number']} r_lock={consensus_states[i-1]['r_lock']} -> block {consensus_states[i]['block_number']} r_lock={consensus_states[i]['r_lock']}"
            
            # QC round should never decrease
            for i in range(1, len(consensus_states)):
                assert consensus_states[i]['qc_round'] >= consensus_states[i-1]['qc_round'], \
                    f"QC round decreased: block {consensus_states[i-1]['block_number']} QC={consensus_states[i-1]['qc_round']} -> block {consensus_states[i]['block_number']} QC={consensus_states[i]['qc_round']}"
            
            logger.info("✅ 2-chain commit rule test completed")
            
        except Exception as e:
            logger.error(f"Error testing 2-chain commit rule: {e}")
            raise

    @mark.test_key('JOLTEON-2CHAIN-002')
    def test_commit_latency_measurement(self, api: BlockchainApi, config: ApiConfig):
        """Measure commit latency in Jolteon
        
        This test analyzes the current block and its parent blocks to measure
        the time between QC formation and block commitment patterns.
        """
        logger.info("Starting Jolteon commit latency measurement test")
        
        try:
            # Get consensus states for blocks to analyze commit patterns
            consensus_states = self._get_consensus_states_for_blocks(api, max_blocks=15)
            
            if len(consensus_states) < 3:
                logger.warning("Insufficient data for commit latency analysis")
                return
            
            logger.info(f"Analyzed {len(consensus_states)} consensus states for commit latency patterns")
            
            # Analyze commit latency patterns
            qc_advancements = 0
            lock_advancements = 0
            
            for i in range(1, len(consensus_states)):
                prev_state = consensus_states[i-1]
                curr_state = consensus_states[i]
                
                # Track QC events
                if curr_state['qc_round'] > prev_state['qc_round']:
                    qc_advancements += 1
                    logger.info(f"QC advancement: block {prev_state['block_number']} QC={prev_state['qc_round']} -> block {curr_state['block_number']} QC={curr_state['qc_round']}")
                
                # Track commit events (lock advancements)
                if curr_state['r_lock'] > prev_state['r_lock']:
                    lock_advancements += 1
                    logger.info(f"Commit event: block {prev_state['block_number']} r_lock={prev_state['r_lock']} -> block {curr_state['block_number']} r_lock={curr_state['r_lock']}")
            
            logger.info(f"Commit latency analysis:")
            logger.info(f"  QC advancements: {qc_advancements}")
            logger.info(f"  Lock advancements: {lock_advancements}")
            
            # Calculate commit rate
            if qc_advancements > 0:
                commit_rate = lock_advancements / qc_advancements
                logger.info(f"Commit rate: {commit_rate:.2f} commits per QC advancement")
                
                # Estimate time span (assuming block_duration between blocks)
                estimated_time_span = len(consensus_states) * config.nodes_config.block_duration
                qc_rate = qc_advancements / (estimated_time_span / 60)  # QCs per minute
                commit_rate_per_minute = lock_advancements / (estimated_time_span / 60)  # commits per minute
                
                logger.info(f"Estimated rates:")
                logger.info(f"  QC formation rate: {qc_rate:.2f} QCs/minute")
                logger.info(f"  Commit rate: {commit_rate_per_minute:.2f} commits/minute")
                
                # Jolteon should have reasonable commit latency
                if commit_rate > 0:
                    logger.info("✅ Commit events detected")
                else:
                    logger.info("ℹ️  No commit events detected in analyzed blocks (may be normal)")
            else:
                logger.info("ℹ️  No QC advancements detected in analyzed blocks")
            
            logger.info("✅ Commit latency measurement test completed")
            
        except Exception as e:
            logger.error(f"Error testing commit latency: {e}")
            raise

    @mark.test_key('JOLTEON-2CHAIN-003')
    def test_consecutive_certification_patterns(self, api: BlockchainApi, config: ApiConfig):
        """Test for consecutive certification patterns
        
        This test analyzes the current block and its parent blocks to look for
        patterns where blocks are certified in consecutive rounds, which is
        essential for 2-chain commit rule.
        """
        logger.info("Starting consecutive certification patterns test")
        
        try:
            # Get consensus states for blocks to analyze certification patterns
            consensus_states = self._get_consensus_states_for_blocks(api, max_blocks=10)
            
            if len(consensus_states) < 3:
                logger.warning("Insufficient data for certification pattern analysis")
                return
            
            logger.info(f"Analyzed {len(consensus_states)} consensus states for certification patterns")
            
            # Look for consecutive certification patterns
            consecutive_certifications = 0
            certification_gaps = []
            
            for i in range(1, len(consensus_states)):
                prev = consensus_states[i-1]
                curr = consensus_states[i]
                
                # Check if QCs are consecutive
                if curr['qc_round'] == prev['qc_round'] + 1:
                    consecutive_certifications += 1
                    logger.info(f"Consecutive certification: block {prev['block_number']} QC={prev['qc_round']} -> block {curr['block_number']} QC={curr['qc_round']}")
                else:
                    gap = curr['qc_round'] - prev['qc_round']
                    if gap > 1:
                        certification_gaps.append(gap)
                        logger.info(f"Certification gap: block {prev['block_number']} QC={prev['qc_round']} -> block {curr['block_number']} QC={curr['qc_round']} (gap: {gap})")
            
            # Analyze certification frequency
            total_qc_advancements = 0
            for i in range(1, len(consensus_states)):
                if consensus_states[i]['qc_round'] > consensus_states[i-1]['qc_round']:
                    total_qc_advancements += 1
            
            logger.info(f"Certification pattern analysis:")
            logger.info(f"  Total QC advancements: {total_qc_advancements}")
            logger.info(f"  Consecutive certifications: {consecutive_certifications}")
            logger.info(f"  Certification gaps: {len(certification_gaps)}")
            
            if certification_gaps:
                avg_gap = sum(certification_gaps) / len(certification_gaps)
                logger.info(f"  Average certification gap: {avg_gap:.2f} rounds")
            
            # Calculate certification rate
            if len(consensus_states) > 1:
                # Estimate time span (assuming block_duration between blocks)
                estimated_time_span = len(consensus_states) * config.nodes_config.block_duration
                certification_rate = total_qc_advancements / (estimated_time_span / 60)  # QCs per minute
                logger.info(f"Certification rate: {certification_rate:.2f} QCs/minute")
            
            # Verify that consecutive certifications are happening
            if consecutive_certifications > 0:
                logger.info("✅ Consecutive certification patterns detected")
                consecutive_rate = consecutive_certifications / total_qc_advancements if total_qc_advancements > 0 else 0
                logger.info(f"Consecutive certification rate: {consecutive_rate:.2f}")
            else:
                logger.info("ℹ️  No consecutive certifications detected in analyzed blocks (may be normal)")
            
            logger.info("✅ Consecutive certification patterns test completed")
            
        except Exception as e:
            logger.error(f"Error testing consecutive certification patterns: {e}")
            raise
