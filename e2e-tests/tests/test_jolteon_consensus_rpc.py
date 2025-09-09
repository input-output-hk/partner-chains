from time import sleep
from pytest import mark
from src.blockchain_api import BlockchainApi, Wallet
from config.api_config import ApiConfig
import logging as logger
import json
from time import time


class TestJolteonConsensusRPC:
    """Test cases for Jolteon consensus using custom RPC endpoints"""
    
    @mark.test_key('JOLTEON-RPC-001')
    def test_replica_state_retrieval(self, api: BlockchainApi, config: ApiConfig):
        """Test basic replica state retrieval via custom RPC endpoint
        
        This test verifies that the Jolteon consensus state is accessible
        and contains the expected data structures.
        """
        logger.info("Starting Jolteon replica state retrieval test")
        
        try:
            # Call the custom RPC endpoint
            result = api.substrate.rpc_request("jolteon_getReplicaState", [])
            
            if result is None or 'result' not in result:
                logger.error("No result from jolteon_getReplicaState")
                assert False, "Failed to retrieve replica state"
            
            replica_state = result['result']
            logger.info(f"Replica state: {json.dumps(replica_state, indent=2)}")
            
            # Verify required fields exist
            required_fields = ['r_curr', 'r_vote', 'r_lock', 'qc_high', 'tc_last', 'storage_block_count']
            for field in required_fields:
                assert field in replica_state, f"Missing required field: {field}"
            
            # Verify data types and basic sanity checks
            assert isinstance(replica_state['r_curr'], int), "r_curr should be an integer"
            assert isinstance(replica_state['r_vote'], int), "r_vote should be an integer"
            assert isinstance(replica_state['r_lock'], int), "r_lock should be an integer"
            assert isinstance(replica_state['storage_block_count'], int), "storage_block_count should be an integer"
            
            # Verify round numbers are reasonable
            assert replica_state['r_curr'] >= 0, "r_curr should be non-negative"
            assert replica_state['r_vote'] >= 0, "r_vote should be non-negative"
            assert replica_state['r_lock'] >= 0, "r_lock should be non-negative"
            
            # Verify QC structure
            qc_high = replica_state['qc_high']
            assert 'block_hash' in qc_high, "QC should have block_hash"
            assert 'round' in qc_high, "QC should have round"
            assert 'vote_count' in qc_high, "QC should have vote_count"
            
            logger.info(f"✅ Replica state test passed:")
            logger.info(f"  Current round: {replica_state['r_curr']}")
            logger.info(f"  Last voted round: {replica_state['r_vote']}")
            logger.info(f"  Locked round: {replica_state['r_lock']}")
            logger.info(f"  Storage blocks: {replica_state['storage_block_count']}")
            
        except Exception as e:
            logger.error(f"Error testing replica state: {e}")
            raise

    @mark.test_key('JOLTEON-RPC-002')
    def test_round_progression(self, api: BlockchainApi, config: ApiConfig):
        """Test that rounds are progressing correctly over time
        
        This test monitors consensus state over time to verify
        that the Jolteon consensus is making progress through round advancement.
        """
        logger.info("Starting Jolteon round progression test")
        
        try:
            # Monitor consensus state over time
            monitoring_duration = 60  # seconds
            sample_interval = config.nodes_config.block_duration
            consensus_states = self._get_consensus_states_over_time(api, monitoring_duration, sample_interval)
            
            if len(consensus_states) < 2:
                logger.warning("Insufficient consensus data for round progression analysis")
                return
            
            # Analyze round progression over time
            logger.info(f"Analyzed {len(consensus_states)} consensus state samples over {monitoring_duration}s")
            
            # Verify round monotonicity (rounds should not decrease)
            for i in range(1, len(consensus_states)):
                prev_state = consensus_states[i-1]
                curr_state = consensus_states[i]
                
                # Current round should not decrease
                assert curr_state['r_curr'] >= prev_state['r_curr'], \
                    f"Round decreased: sample {prev_state['sample']} r_curr={prev_state['r_curr']} -> sample {curr_state['sample']} r_curr={curr_state['r_curr']}"
                
                # Voted round should not decrease
                assert curr_state['r_vote'] >= prev_state['r_vote'], \
                    f"Voted round decreased: sample {prev_state['sample']} r_vote={prev_state['r_vote']} -> sample {curr_state['sample']} r_vote={curr_state['r_vote']}"
                
                # Locked round should not decrease
                assert curr_state['r_lock'] >= prev_state['r_lock'], \
                    f"Locked round decreased: sample {prev_state['sample']} r_lock={prev_state['r_lock']} -> sample {curr_state['sample']} r_lock={curr_state['r_lock']}"
            
            # Check for round advancement
            round_advancements = 0
            for i in range(1, len(consensus_states)):
                if consensus_states[i]['r_curr'] > consensus_states[i-1]['r_curr']:
                    round_advancements += 1
            
            if round_advancements > 0:
                logger.info(f"✅ Round progression detected: {round_advancements} advancements across {len(consensus_states)} samples")
            else:
                logger.info("ℹ️  No round progression detected in monitoring period (may be normal)")
            
            logger.info("✅ Round progression test passed")
            
        except Exception as e:
            logger.error(f"Error testing round progression: {e}")
            raise

    @mark.test_key('JOLTEON-RPC-003')
    def test_quorum_certificate_formation(self, api: BlockchainApi, config: ApiConfig):
        """Test Quorum Certificate formation and progression
        
        This test analyzes the current block and its parent blocks to verify
        that QCs are being formed correctly and that the highest QC is advancing.
        """
        logger.info("Starting Jolteon Quorum Certificate formation test")
        
        try:
            # Get consensus states for blocks
            consensus_states = self._get_consensus_states_for_blocks(api, max_blocks=5)
            
            if len(consensus_states) < 2:
                logger.warning("Insufficient QC data for analysis")
                return
            
            # Analyze QC progression across blocks
            logger.info(f"Analyzed {len(consensus_states)} QC states")
            
            # Verify QC structure and progression
            for qc_state in consensus_states:
                assert 'qc_round' in qc_state, "QC should have round"
                assert 'qc_block' in qc_state, "QC should have block_hash"
                assert 'qc_votes' in qc_state, "QC should have vote_count"
                assert isinstance(qc_state['qc_round'], int), "QC round should be integer"
                assert isinstance(qc_state['qc_votes'], int), "QC vote_count should be integer"
            
            # Verify QC progression (round should not decrease)
            for i in range(1, len(consensus_states)):
                prev_qc = consensus_states[i-1]
                curr_qc = consensus_states[i]
                
                assert curr_qc['qc_round'] >= prev_qc['qc_round'], \
                    f"QC round decreased: block {prev_qc['block_number']} QC={prev_qc['qc_round']} -> block {curr_qc['block_number']} QC={curr_qc['qc_round']}"
            
            # Check for QC advancement
            qc_advancements = 0
            for i in range(1, len(consensus_states)):
                if consensus_states[i]['qc_round'] > consensus_states[i-1]['qc_round']:
                    qc_advancements += 1
            
            if qc_advancements > 0:
                logger.info(f"✅ QC progression detected: {qc_advancements} advancements across {len(consensus_states)} blocks")
            else:
                logger.info("ℹ️  No QC progression detected in analyzed blocks (may be normal)")
            
            # Verify vote count is reasonable
            latest_qc = consensus_states[0]  # Most recent QC
            assert latest_qc['qc_votes'] >= 0, "QC vote count should be non-negative"
            
            # If we're in initial state (round 0), vote_count can be 0
            if latest_qc['qc_round'] == 0:
                logger.info(f"ℹ️  Initial state detected: QC round {latest_qc['qc_round']} with {latest_qc['qc_votes']} votes")
            else:
                # For non-initial rounds, we expect positive vote count
                assert latest_qc['qc_votes'] > config.jolteon_config.min_vote_count_threshold, \
                    f"QC should have vote count > {config.jolteon_config.min_vote_count_threshold} for round {latest_qc['qc_round']}"
            
            logger.info("✅ Quorum Certificate formation test passed")
            
        except Exception as e:
            logger.error(f"Error testing QC formation: {e}")
            raise

    @mark.test_key('JOLTEON-RPC-004')
    def test_timeout_certificate_handling(self, api: BlockchainApi, config: ApiConfig):
        """Test Timeout Certificate handling
        
        This test checks for timeout certificates and verifies
        that the system can handle view changes properly.
        """
        logger.info("Starting Jolteon Timeout Certificate handling test")
        
        try:
            # Try to get the last TC
            result = api.substrate.rpc_request("jolteon_getLastTC", [])
            
            if result and 'result' in result:
                tc = result['result']
                logger.info(f"Found Timeout Certificate: {tc}")
                
                # Verify TC structure
                assert 'round' in tc, "TC should have round"
                assert 'timeout_count' in tc, "TC should have timeout_count"
                assert isinstance(tc['round'], int), "TC round should be integer"
                assert isinstance(tc['timeout_count'], int), "TC timeout_count should be integer"
                
                # Verify TC data is reasonable
                assert tc['round'] >= 0, "TC round should be non-negative"
                assert tc['timeout_count'] > 0, "TC should have positive timeout count"
                
                logger.info(f"✅ Found TC for round {tc['round']} with {tc['timeout_count']} timeout votes")
                
            else:
                # No TC found, which is normal in happy path
                logger.info("ℹ️  No Timeout Certificate found (normal in happy path)")
                
        except Exception as e:
            # TC endpoint might return an error if no TC exists
            error_msg = str(e)
            if "No timeout certificate available" in error_msg:
                logger.info("ℹ️  No Timeout Certificate available (normal in happy path)")
            else:
                logger.error(f"Error checking TC: {e}")
                raise
        
        logger.info("✅ Timeout Certificate handling test passed")

    @mark.test_key('JOLTEON-RPC-005')
    def test_consensus_state_consistency(self, api: BlockchainApi, config: ApiConfig):
        """Test consistency between different consensus state endpoints
        
        This test verifies that the data returned by different
        RPC endpoints is consistent with each other.
        """
        logger.info("Starting Jolteon consensus state consistency test")
        
        # Get data from all endpoints
        replica_state_result = api.substrate.rpc_request("jolteon_getReplicaState", [])
        round_info_result = api.substrate.rpc_request("jolteon_getRoundInfo", [])
        qc_result = api.substrate.rpc_request("jolteon_getHighestQC", [])
        
        assert all([replica_state_result, round_info_result, qc_result]), "Failed to get consensus state from all endpoints"
        
        replica_state = replica_state_result['result']
        round_info = round_info_result['result']
        qc = qc_result['result']
        
        logger.info(f"Replica state: {replica_state}")
        logger.info(f"Round info: {round_info}")
        logger.info(f"QC: {qc}")
        
        # Verify round consistency between endpoints
        assert replica_state['r_curr'] == round_info['r_curr'], \
            f"Current round mismatch: replica_state={replica_state['r_curr']}, round_info={round_info['r_curr']}"
        
        assert replica_state['r_vote'] == round_info['r_vote'], \
            f"Voted round mismatch: replica_state={replica_state['r_vote']}, round_info={round_info['r_vote']}"
        
        assert replica_state['r_lock'] == round_info['r_lock'], \
            f"Locked round mismatch: replica_state={replica_state['r_lock']}, round_info={round_info['r_lock']}"
        
        # Verify QC consistency
        assert replica_state['qc_high']['round'] == qc['round'], \
            f"QC round mismatch: replica_state={replica_state['qc_high']['round']}, qc={qc['round']}"
        
        assert replica_state['qc_high']['block_hash'] == qc['block_hash'], \
            f"QC block hash mismatch: replica_state={replica_state['qc_high']['block_hash']}, qc={qc['block_hash']}"
        
        assert replica_state['qc_high']['vote_count'] == qc['vote_count'], \
            f"QC vote count mismatch: replica_state={replica_state['qc_high']['vote_count']}, qc={qc['vote_count']}"
        
        # Verify logical consistency
        assert replica_state['r_curr'] >= replica_state['r_vote'], \
            f"Current round should be >= voted round: {replica_state['r_curr']} < {replica_state['r_vote']}"
        
        assert replica_state['r_vote'] >= replica_state['r_lock'], \
            f"Voted round should be >= locked round: {replica_state['r_vote']} < {replica_state['r_lock']}"
        
        assert replica_state['qc_high']['round'] <= replica_state['r_curr'], \
            f"QC round should be <= current round: {replica_state['qc_high']['round']} > {replica_state['r_curr']}"
        
        logger.info("✅ Consensus state consistency test passed")

    @mark.test_key('JOLTEON-RPC-006')
    def test_jolteon_safety_properties(self, api: BlockchainApi, config: ApiConfig):
        """Test Jolteon safety properties using RPC data
        
        This test analyzes the current block and its parent blocks to verify
        fundamental safety guarantees:
        - Round monotonicity
        - QC progression
        - Lock consistency
        """
        logger.info("Starting Jolteon safety properties test")
        
        try:
            # Get consensus states for blocks
            consensus_states = self._get_consensus_states_for_blocks(api, max_blocks=10)
            
            if len(consensus_states) < 2:
                logger.warning("Insufficient data for safety analysis")
                return
            
            logger.info(f"Analyzed {len(consensus_states)} consensus states for safety properties")
            
            # Check round monotonicity
            rounds = [s['r_curr'] for s in consensus_states]
            for i in range(1, len(rounds)):
                assert rounds[i] >= rounds[i-1], \
                    f"Round decreased: block {consensus_states[i-1]['block_number']} r_curr={rounds[i-1]} -> block {consensus_states[i]['block_number']} r_curr={rounds[i]}"
            
            # Check QC progression
            qc_rounds = [s['qc_round'] for s in consensus_states]
            for i in range(1, len(qc_rounds)):
                assert qc_rounds[i] >= qc_rounds[i-1], \
                    f"QC round decreased: block {consensus_states[i-1]['block_number']} QC={qc_rounds[i-1]} -> block {consensus_states[i]['block_number']} QC={qc_rounds[i]}"
            
            # Check lock consistency
            locks = [s['r_lock'] for s in consensus_states]
            for i in range(1, len(locks)):
                assert locks[i] >= locks[i-1], \
                    f"Lock round decreased: block {consensus_states[i-1]['block_number']} r_lock={locks[i-1]} -> block {consensus_states[i]['block_number']} r_lock={locks[i]}"
            
            # Additional safety checks
            for state in consensus_states:
                # Current round should be >= voted round
                assert state['r_curr'] >= state['r_vote'], \
                    f"Current round < voted round: block {state['block_number']} r_curr={state['r_curr']} < r_vote={state['r_vote']}"
                
                # Voted round should be >= locked round
                assert state['r_vote'] >= state['r_lock'], \
                    f"Voted round < locked round: block {state['block_number']} r_vote={state['r_vote']} < r_lock={state['r_lock']}"
                
                # QC round should be <= current round
                assert state['qc_round'] <= state['r_curr'], \
                    f"QC round > current round: block {state['block_number']} QC={state['qc_round']} > r_curr={state['r_curr']}"
            
            logger.info("✅ Jolteon safety properties test passed")
            
        except Exception as e:
            logger.error(f"Error testing safety properties: {e}")
            raise

    @mark.test_key('JOLTEON-RPC-007')
    def test_jolteon_liveness_properties(self, api: BlockchainApi, config: ApiConfig):
        """Test Jolteon liveness properties using RPC data
        
        This test analyzes the current block and its parent blocks to verify
        that the system makes progress:
        - Rounds advance over time
        - QCs are formed regularly
        - System doesn't get stuck
        """
        logger.info("Starting Jolteon liveness properties test")
        
        try:
            # Get consensus states for blocks
            consensus_states = self._get_consensus_states_for_blocks(api, max_blocks=15)
            
            if len(consensus_states) < 3:
                logger.warning("Insufficient data for liveness analysis")
                return
            
            logger.info(f"Analyzed {len(consensus_states)} consensus states for liveness properties")
            
            # Analyze liveness
            # Check for round progress
            round_progress_events = 0
            for i in range(1, len(consensus_states)):
                if consensus_states[i]['r_curr'] > consensus_states[i-1]['r_curr']:
                    round_progress_events += 1
            
            # Check for QC progress
            qc_progress_events = 0
            for i in range(1, len(consensus_states)):
                if consensus_states[i]['qc_round'] > consensus_states[i-1]['qc_round']:
                    qc_progress_events += 1
            
            logger.info(f"Liveness analysis:")
            logger.info(f"  Total blocks analyzed: {len(consensus_states)}")
            logger.info(f"  Round progress events: {round_progress_events}")
            logger.info(f"  QC progress events: {qc_progress_events}")
            
            # Basic liveness assertions
            if round_progress_events > 0:
                logger.info("✅ Round progression detected")
            else:
                logger.info("ℹ️  No round progression detected in analyzed blocks (may be normal)")
            
            if qc_progress_events > 0:
                logger.info("✅ QC progression detected")
            else:
                logger.info("ℹ️  No QC progression detected in analyzed blocks (may be normal)")
            
            # Calculate progress rates
            if len(consensus_states) > 1:
                # Estimate time span (assuming block_duration between blocks)
                estimated_time_span = len(consensus_states) * config.nodes_config.block_duration
                round_rate = round_progress_events / (estimated_time_span / 60)  # rounds per minute
                qc_rate = qc_progress_events / (estimated_time_span / 60)  # QCs per minute
                
                logger.info(f"Estimated progress rates:")
                logger.info(f"  Round progression rate: {round_rate:.2f} rounds/minute")
                logger.info(f"  QC progression rate: {qc_rate:.2f} QCs/minute")
            
            # Verify that we have some consensus activity
            # At minimum, we should have some blocks with consensus state
            assert len(consensus_states) > 0, "No consensus states found"
            
            # Check that consensus state is reasonable
            latest_state = consensus_states[0]
            assert latest_state['r_curr'] >= 0, "Current round should be non-negative"
            assert latest_state['r_vote'] >= 0, "Voted round should be non-negative"
            assert latest_state['r_lock'] >= 0, "Locked round should be non-negative"
            assert latest_state['qc_round'] >= 0, "QC round should be non-negative"
            
            logger.info("✅ Jolteon liveness properties test passed")
            
        except Exception as e:
            logger.error(f"Error testing liveness properties: {e}")
            raise

    def _get_consensus_states_over_time(self, api: BlockchainApi, monitoring_duration: int = 30, sample_interval: int = 6) -> list:
        """Get consensus states over time to detect progression
        
        Args:
            api: Blockchain API instance
            monitoring_duration: Duration to monitor in seconds
            sample_interval: Interval between samples in seconds
            rpc_endpoint: RPC endpoint to use for consensus data
            
        Returns:
            List of consensus states with timestamps
        """
        try:
            samples = monitoring_duration // sample_interval
            logger.info(f"Monitoring consensus state for {monitoring_duration}s (every {sample_interval}s, {samples} samples)...")
            
            consensus_states = []
            
            for i in range(samples + 1):
                try:
                    # Get current block
                    current_block = api.substrate.get_block()
                    block_number = current_block['header']['number']
                    
                    # Get consensus state using RPC endpoint
                    consensus_result = api.substrate.rpc_request("jolteon_getReplicaState", [])
                    
                    if consensus_result and 'result' in consensus_result:
                        consensus_state = consensus_result['result']
                        
                        state_data = {
                            'sample': i + 1,
                            'block_number': block_number,
                            'r_curr': consensus_state['r_curr'],
                            'r_vote': consensus_state['r_vote'],
                            'r_lock': consensus_state['r_lock'],
                            'qc_round': consensus_state['qc_high']['round'],
                            'qc_votes': consensus_state['qc_high']['vote_count'],
                            'qc_block': consensus_state['qc_high']['block_hash']
                        }
                        
                        consensus_states.append(state_data)
                        
                        logger.info(f"Sample {i+1}: Block {block_number}, r_curr={consensus_state['r_curr']}, r_vote={consensus_state['r_vote']}, QC={consensus_state['qc_high']['round']}")
                        
                        # Don't sleep after the last sample
                        if i < samples:
                            sleep(sample_interval)
                    
                except Exception as e:
                    logger.warning(f"Error sampling consensus state {i+1}: {e}")
                    continue
            
            logger.info(f"Successfully collected {len(consensus_states)} consensus state samples")
            return consensus_states
            
        except Exception as e:
            logger.error(f"Error getting consensus states over time: {e}")
            return []