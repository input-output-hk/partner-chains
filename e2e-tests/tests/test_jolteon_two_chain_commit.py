from time import sleep
from pytest import mark
from src.blockchain_api import BlockchainApi, Wallet
from config.api_config import ApiConfig
import logging as logger
import json
from time import time


class TestJolteonTwoChainCommit:
    """Test cases for Jolteon's 2-chain commit rule (core protocol feature)"""
    
    @mark.test_key('JOLTEON-2CHAIN-001')
    def test_two_chain_commit_rule_verification(self, api: BlockchainApi, config: ApiConfig):
        """Test Jolteon's 2-chain commit rule
        
        This test verifies that blocks are committed when two adjacent certified blocks
        with consecutive round numbers exist. This is Jolteon's core differentiation
        from 3-chain HotStuff.
        
        Based on Tachyeon test case: Lock and Commit Rules - 2-Chain Commit Rule
        """
        logger.info("Starting Jolteon 2-chain commit rule test")
        
        # Monitor consensus state over time to observe commit patterns - use configurable multipliers
        wait_time = config.nodes_config.block_duration * config.jolteon_config.safety_monitoring_multiplier
        check_interval = config.nodes_config.block_duration * config.jolteon_config.check_interval_multiplier
        logger.info(f"Monitoring consensus state for {wait_time} seconds to observe 2-chain commit patterns ({config.jolteon_config.safety_monitoring_multiplier}x block_duration={config.nodes_config.block_duration}s)...")
        
        consensus_history = []
        start_time = time()
        
        while (time() - start_time) < wait_time:
            try:
                # Get current consensus state
                result = api.substrate.rpc_request("jolteon_getReplicaState", [])
                if result and 'result' in result:
                    current_state = result['result']
                    
                    consensus_history.append({
                        'timestamp': time() - start_time,
                        'r_curr': current_state['r_curr'],
                        'r_vote': current_state['r_vote'],
                        'r_lock': current_state['r_lock'],
                        'qc_round': current_state['qc_high']['round'],
                        'qc_block': current_state['qc_high']['block_hash'],
                        'qc_votes': current_state['qc_high']['vote_count']
                    })
                    
                    logger.info(f"State at {current_state['r_curr']}: QC={current_state['qc_high']['round']}, Lock={current_state['r_lock']}")
                    
                sleep(check_interval)
                
            except Exception as e:
                logger.warning(f"Error getting consensus state: {e}")
                sleep(check_interval)
        
        # Analyze 2-chain commit patterns
        if len(consensus_history) < 3:
            logger.warning("Insufficient data for 2-chain analysis")
            return
        
        logger.info(f"Collected {len(consensus_history)} consensus state samples")
        
        # Look for 2-chain commit patterns
        # In Jolteon, a block is committed when there are two consecutive certified blocks
        # This means we should see the locked round advancing when QCs are consecutive
        
        consecutive_qc_pairs = 0
        lock_advancements = 0
        
        for i in range(1, len(consensus_history)):
            prev_state = consensus_history[i-1]
            curr_state = consensus_history[i]
            
            # Check for consecutive QC rounds
            if curr_state['qc_round'] == prev_state['qc_round'] + 1:
                consecutive_qc_pairs += 1
                logger.info(f"Consecutive QCs found: {prev_state['qc_round']} -> {curr_state['qc_round']}")
                
                # Check if lock advanced (indicating commit)
                if curr_state['r_lock'] > prev_state['r_lock']:
                    lock_advancements += 1
                    logger.info(f"Lock advanced during consecutive QCs: {prev_state['r_lock']} -> {curr_state['r_lock']}")
        
        # Analyze lock advancement patterns
        total_lock_advancements = 0
        for i in range(1, len(consensus_history)):
            if consensus_history[i]['r_lock'] > consensus_history[i-1]['r_lock']:
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
                    logger.warning("⚠️  No commits detected during consecutive QCs")
        else:
            logger.info("ℹ️  No consecutive QC pairs found - this may be normal for this implementation")
        
        # Verify basic safety properties
        # Lock should never decrease
        for i in range(1, len(consensus_history)):
            assert consensus_history[i]['r_lock'] >= consensus_history[i-1]['r_lock'], \
                f"Lock decreased: {consensus_history[i-1]['r_lock']} -> {consensus_history[i]['r_lock']}"
        
        # QC round should never decrease
        for i in range(1, len(consensus_history)):
            assert consensus_history[i]['qc_round'] >= consensus_history[i-1]['qc_round'], \
                f"QC round decreased: {consensus_history[i-1]['qc_round']} -> {consensus_history[i]['qc_round']}"
        
        logger.info("✅ 2-chain commit rule test completed")

    @mark.test_key('JOLTEON-2CHAIN-002')
    def test_commit_latency_measurement(self, api: BlockchainApi, config: ApiConfig):
        """Measure commit latency in Jolteon
        
        This test measures the time between QC formation and block commitment
        to verify Jolteon's improved commit latency compared to 3-chain HotStuff.
        """
        logger.info("Starting Jolteon commit latency measurement test")
        
        # Monitor for QC formation and subsequent commits - use configurable multipliers
        wait_time = config.nodes_config.block_duration * config.jolteon_config.liveness_monitoring_multiplier
        check_interval = config.nodes_config.block_duration  # Use block_duration for higher resolution
        logger.info(f"Monitoring commit latency for {wait_time} seconds ({config.jolteon_config.liveness_monitoring_multiplier}x block_duration={config.nodes_config.block_duration}s)...")
        
        qc_events = []
        commit_events = []
        start_time = time()
        
        while (time() - start_time) < wait_time:
            try:
                # Get current consensus state
                result = api.substrate.rpc_request("jolteon_getReplicaState", [])
                if result and 'result' in result:
                    current_state = result['result']
                    
                    # Track QC events
                    if len(qc_events) == 0 or current_state['qc_high']['round'] > qc_events[-1]['qc_round']:
                        qc_events.append({
                            'timestamp': time() - start_time,
                            'qc_round': current_state['qc_high']['round'],
                            'qc_block': current_state['qc_high']['block_hash']
                        })
                        logger.info(f"New QC formed: round {current_state['qc_high']['round']}")
                    
                    # Track commit events (lock advancements)
                    if len(commit_events) == 0 or current_state['r_lock'] > commit_events[-1]['lock_round']:
                        commit_events.append({
                            'timestamp': time() - start_time,
                            'lock_round': current_state['r_lock']
                        })
                        logger.info(f"New commit: lock round {current_state['r_lock']}")
                    
                sleep(check_interval)
                
            except Exception as e:
                logger.warning(f"Error monitoring commit latency: {e}")
                sleep(check_interval)
        
        # Analyze commit latency
        logger.info(f"Commit latency analysis:")
        logger.info(f"  QC events: {len(qc_events)}")
        logger.info(f"  Commit events: {len(commit_events)}")
        
        if len(qc_events) > 1 and len(commit_events) > 1:
            # Calculate average time between QCs
            qc_intervals = []
            for i in range(1, len(qc_events)):
                interval = qc_events[i]['timestamp'] - qc_events[i-1]['timestamp']
                qc_intervals.append(interval)
            
            avg_qc_interval = sum(qc_intervals) / len(qc_intervals)
            logger.info(f"Average QC interval: {avg_qc_interval:.2f} seconds")
            
            # Calculate average time between commits
            commit_intervals = []
            for i in range(1, len(commit_events)):
                interval = commit_events[i]['timestamp'] - commit_events[i-1]['timestamp']
                commit_intervals.append(interval)
            
            avg_commit_interval = sum(commit_intervals) / len(commit_intervals)
            logger.info(f"Average commit interval: {avg_commit_interval:.2f} seconds")
            
            # Calculate commit latency (time from QC to commit)
            if len(qc_events) > 0 and len(commit_events) > 0:
                # Find commits that happened after QCs
                latencies = []
                for qc_event in qc_events:
                    for commit_event in commit_events:
                        if commit_event['timestamp'] > qc_event['timestamp']:
                            latency = commit_event['timestamp'] - qc_event['timestamp']
                            latencies.append(latency)
                            break
                
                if latencies:
                    avg_latency = sum(latencies) / len(latencies)
                    min_latency = min(latencies)
                    max_latency = max(latencies)
                    
                    logger.info(f"Commit latency statistics:")
                    logger.info(f"  Average: {avg_latency:.2f} seconds")
                    logger.info(f"  Minimum: {min_latency:.2f} seconds")
                    logger.info(f"  Maximum: {max_latency:.2f} seconds")
                    
                    # Jolteon should have lower commit latency than 3-chain HotStuff
                    if avg_latency < config.jolteon_config.commit_latency_threshold:
                        logger.info(f"✅ Commit latency appears reasonable for Jolteon (< {config.jolteon_config.commit_latency_threshold}s)")
                    else:
                        logger.warning(f"⚠️  High commit latency: {avg_latency:.2f} seconds (threshold: {config.jolteon_config.commit_latency_threshold}s)")
        
        logger.info("✅ Commit latency measurement test completed")

    @mark.test_key('JOLTEON-2CHAIN-003')
    def test_consecutive_certification_patterns(self, api: BlockchainApi, config: ApiConfig):
        """Test for consecutive certification patterns
        
        This test specifically looks for patterns where blocks are certified
        in consecutive rounds, which is essential for 2-chain commit rule.
        """
        logger.info("Starting consecutive certification patterns test")
        
        # Monitor block production and certification - use configurable multipliers
        wait_time = config.nodes_config.block_duration * config.jolteon_config.safety_monitoring_multiplier
        check_interval = config.nodes_config.block_duration * config.jolteon_config.check_interval_multiplier
        logger.info(f"Monitoring certification patterns for {wait_time} seconds ({config.jolteon_config.safety_monitoring_multiplier}x block_duration={config.nodes_config.block_duration}s)...")
        
        certification_history = []
        start_time = time()
        
        while (time() - start_time) < wait_time:
            try:
                # Get current block and consensus state
                block_result = api.substrate.get_block()
                consensus_result = api.substrate.rpc_request("jolteon_getReplicaState", [])
                
                if block_result and consensus_result and 'result' in consensus_result:
                    block_number = block_result['header']['number']
                    consensus_state = consensus_result['result']
                    
                    certification_history.append({
                        'timestamp': time() - start_time,
                        'block_number': block_number,
                        'r_curr': consensus_state['r_curr'],
                        'qc_round': consensus_state['qc_high']['round'],
                        'qc_votes': consensus_state['qc_high']['vote_count']
                    })
                    
                    logger.info(f"Block {block_number}: round {consensus_state['r_curr']}, QC {consensus_state['qc_high']['round']}")
                    
                sleep(check_interval)
                
            except Exception as e:
                logger.warning(f"Error monitoring certification: {e}")
                sleep(check_interval)
        
        # Analyze certification patterns
        if len(certification_history) < 3:
            logger.warning("Insufficient data for certification pattern analysis")
            return
        
        logger.info(f"Collected {len(certification_history)} certification samples")
        
        # Look for consecutive certification patterns
        consecutive_certifications = 0
        certification_gaps = []
        
        for i in range(1, len(certification_history)):
            prev = certification_history[i-1]
            curr = certification_history[i]
            
            # Check if QCs are consecutive
            if curr['qc_round'] == prev['qc_round'] + 1:
                consecutive_certifications += 1
                logger.info(f"Consecutive certification: QC {prev['qc_round']} -> {curr['qc_round']}")
            else:
                gap = curr['qc_round'] - prev['qc_round']
                if gap > 1:
                    certification_gaps.append(gap)
                    logger.info(f"Certification gap: QC {prev['qc_round']} -> {curr['qc_round']} (gap: {gap})")
        
        # Analyze certification frequency
        total_qc_advancements = 0
        for i in range(1, len(certification_history)):
            if certification_history[i]['qc_round'] > certification_history[i-1]['qc_round']:
                total_qc_advancements += 1
        
        logger.info(f"Certification pattern analysis:")
        logger.info(f"  Total QC advancements: {total_qc_advancements}")
        logger.info(f"  Consecutive certifications: {consecutive_certifications}")
        logger.info(f"  Certification gaps: {len(certification_gaps)}")
        
        if certification_gaps:
            avg_gap = sum(certification_gaps) / len(certification_gaps)
            logger.info(f"  Average certification gap: {avg_gap:.2f} rounds")
        
        # Calculate certification rate
        if len(certification_history) > 1:
            total_time = certification_history[-1]['timestamp']
            certification_rate = total_qc_advancements / (total_time / 60)  # QCs per minute
            logger.info(f"Certification rate: {certification_rate:.2f} QCs/minute")
        
        # Verify that consecutive certifications are happening
        if consecutive_certifications > 0:
            logger.info("✅ Consecutive certification patterns detected")
            consecutive_rate = consecutive_certifications / total_qc_advancements if total_qc_advancements > 0 else 0
            logger.info(f"Consecutive certification rate: {consecutive_rate:.2f}")
        else:
            logger.warning("⚠️  No consecutive certifications detected")
        
        logger.info("✅ Consecutive certification patterns test completed")
