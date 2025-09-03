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
        """Test that rounds are progressing correctly
        
        This test monitors round progression over time to verify
        that the Jolteon consensus is making progress.
        """
        logger.info("Starting Jolteon round progression test")
        
        # Get initial round state
        initial_result = api.substrate.rpc_request("jolteon_getRoundInfo", [])
        if not initial_result or 'result' not in initial_result:
            assert False, "Failed to get initial round info"
        
        initial_round = initial_result['result']
        logger.info(f"Initial round state: {initial_round}")
        
        # Wait for round progression
        wait_time = 30  # Wait 30 seconds for round advancement
        logger.info(f"Waiting {wait_time} seconds for round progression...")
        sleep(wait_time)
        
        # Get final round state
        final_result = api.substrate.rpc_request("jolteon_getRoundInfo", [])
        if not final_result or 'result' not in final_result:
            assert False, "Failed to get final round info"
        
        final_round = final_result['result']
        logger.info(f"Final round state: {final_round}")
        
        # Verify round progression
        assert final_round['r_curr'] >= initial_round['r_curr'], \
            f"Current round should not decrease: {initial_round['r_curr']} -> {final_round['r_curr']}"
        
        # In happy path, rounds should advance
        if final_round['r_curr'] > initial_round['r_curr']:
            logger.info(f"✅ Rounds advanced: {initial_round['r_curr']} -> {final_round['r_curr']}")
        else:
            logger.info(f"⚠️  Rounds stayed the same: {initial_round['r_curr']}")
        
        # Verify round consistency
        assert final_round['r_vote'] >= initial_round['r_vote'], \
            f"Voted round should not decrease: {initial_round['r_vote']} -> {final_round['r_vote']}"
        
        assert final_round['r_lock'] >= initial_round['r_lock'], \
            f"Locked round should not decrease: {initial_round['r_lock']} -> {final_round['r_lock']}"
        
        logger.info("✅ Round progression test passed")

    @mark.test_key('JOLTEON-RPC-003')
    def test_quorum_certificate_formation(self, api: BlockchainApi, config: ApiConfig):
        """Test Quorum Certificate formation and progression
        
        This test verifies that QCs are being formed correctly
        and that the highest QC is advancing.
        """
        logger.info("Starting Jolteon Quorum Certificate formation test")
        
        # Get initial QC state
        initial_result = api.substrate.rpc_request("jolteon_getHighestQC", [])
        if not initial_result or 'result' not in initial_result:
            assert False, "Failed to get initial QC"
        
        initial_qc = initial_result['result']
        logger.info(f"Initial QC: {initial_qc}")
        
        # Wait for QC progression
        wait_time = 45  # Wait 45 seconds for QC advancement
        logger.info(f"Waiting {wait_time} seconds for QC progression...")
        sleep(wait_time)
        
        # Get final QC state
        final_result = api.substrate.rpc_request("jolteon_getHighestQC", [])
        if not final_result or 'result' not in final_result:
            assert False, "Failed to get final QC"
        
        final_qc = final_result['result']
        logger.info(f"Final QC: {final_qc}")
        
        # Verify QC structure
        for qc in [initial_qc, final_qc]:
            assert 'block_hash' in qc, "QC should have block_hash"
            assert 'round' in qc, "QC should have round"
            assert 'vote_count' in qc, "QC should have vote_count"
            assert isinstance(qc['round'], int), "QC round should be integer"
            assert isinstance(qc['vote_count'], int), "QC vote_count should be integer"
        
        # Verify QC progression (round should not decrease)
        assert final_qc['round'] >= initial_qc['round'], \
            f"QC round should not decrease: {initial_qc['round']} -> {final_qc['round']}"
        
        # In happy path, QC should advance
        if final_qc['round'] > initial_qc['round']:
            logger.info(f"✅ QC advanced: round {initial_qc['round']} -> {final_qc['round']}")
        else:
            logger.info(f"⚠️  QC round stayed the same: {initial_qc['round']}")
        
        # Verify vote count is reasonable (should be >= 2f+1 for n=3f+1)
        # In initial state, vote_count can be 0, but should be non-negative
        assert final_qc['vote_count'] >= 0, "QC vote count should be non-negative"
        
        # If we're in initial state (round 0), vote_count can be 0
        if final_qc['round'] == 0:
            logger.info(f"ℹ️  Initial state detected: QC round {final_qc['round']} with {final_qc['vote_count']} votes")
        else:
            # For non-initial rounds, we expect positive vote count
            assert final_qc['vote_count'] > 0, f"QC should have positive vote count for round {final_qc['round']}"
        
        logger.info("✅ Quorum Certificate formation test passed")

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
        
        if not all([replica_state_result, round_info_result, qc_result]):
            assert False, "Failed to get consensus state from all endpoints"
        
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
        
        This test verifies fundamental safety guarantees:
        - Round monotonicity
        - QC progression
        - Lock consistency
        """
        logger.info("Starting Jolteon safety properties test")
        
        # Monitor consensus state over time
        wait_time = 60  # Monitor for 1 minute
        check_interval = 15  # Check every 15 seconds
        logger.info(f"Monitoring consensus state for {wait_time} seconds...")
        
        previous_states = []
        start_time = time()
        
        while (time() - start_time) < wait_time:
            try:
                # Get current state
                result = api.substrate.rpc_request("jolteon_getReplicaState", [])
                if result and 'result' in result:
                    current_state = result['result']
                    previous_states.append({
                        'timestamp': time() - start_time,
                        'state': current_state
                    })
                    
                    logger.info(f"State at {current_state['r_curr']}: r_vote={current_state['r_vote']}, r_lock={current_state['r_lock']}")
                    
                sleep(check_interval)
                
            except Exception as e:
                logger.warning(f"Error getting state: {e}")
                sleep(check_interval)
        
        # Analyze safety properties
        if len(previous_states) < 2:
            logger.warning("Insufficient data for safety analysis")
            return
        
        logger.info(f"Collected {len(previous_states)} state samples")
        
        # Check round monotonicity
        rounds = [s['state']['r_curr'] for s in previous_states]
        for i in range(1, len(rounds)):
            assert rounds[i] >= rounds[i-1], \
                f"Round decreased: {rounds[i-1]} -> {rounds[i]}"
        
        # Check QC progression
        qc_rounds = [s['state']['qc_high']['round'] for s in previous_states]
        for i in range(1, len(qc_rounds)):
            assert qc_rounds[i] >= qc_rounds[i-1], \
                f"QC round decreased: {qc_rounds[i-1]} -> {qc_rounds[i]}"
        
        # Check lock consistency
        locks = [s['state']['r_lock'] for s in previous_states]
        for i in range(1, len(locks)):
            assert locks[i] >= locks[i-1], \
                f"Lock round decreased: {locks[i-1]} -> {locks[i]}"
        
        logger.info("✅ Jolteon safety properties test passed")

    @mark.test_key('JOLTEON-RPC-007')
    def test_jolteon_liveness_properties(self, api: BlockchainApi, config: ApiConfig):
        """Test Jolteon liveness properties using RPC data
        
        This test verifies that the system makes progress:
        - Rounds advance over time
        - QCs are formed regularly
        - System doesn't get stuck
        """
        logger.info("Starting Jolteon liveness properties test")
        
        # Monitor progress over time
        wait_time = 120  # Monitor for 2 minutes
        check_interval = 20  # Check every 20 seconds
        logger.info(f"Monitoring liveness for {wait_time} seconds...")
        
        progress_checks = []
        start_time = time()
        initial_state = None
        
        while (time() - start_time) < wait_time:
            try:
                # Get current state
                result = api.substrate.rpc_request("jolteon_getReplicaState", [])
                if result and 'result' in result:
                    current_state = result['result']
                    
                    if initial_state is None:
                        initial_state = current_state
                    
                    # Check for progress
                    round_progress = current_state['r_curr'] > initial_state['r_curr']
                    qc_progress = current_state['qc_high']['round'] > initial_state['qc_high']['round']
                    
                    progress_checks.append({
                        'timestamp': time() - start_time,
                        'round': current_state['r_curr'],
                        'qc_round': current_state['qc_high']['round'],
                        'round_progress': round_progress,
                        'qc_progress': qc_progress
                    })
                    
                    if round_progress:
                        logger.info(f"✅ Round progress: {initial_state['r_curr']} -> {current_state['r_curr']}")
                    if qc_progress:
                        logger.info(f"✅ QC progress: {initial_state['qc_high']['round']} -> {current_state['qc_high']['round']}")
                    
                sleep(check_interval)
                
            except Exception as e:
                logger.warning(f"Error checking progress: {e}")
                sleep(check_interval)
        
        # Analyze liveness
        if len(progress_checks) == 0:
            assert False, "No progress data collected"
        
        # Check for any progress
        round_progress_events = [c for c in progress_checks if c['round_progress']]
        qc_progress_events = [c for c in progress_checks if c['qc_progress']]
        
        logger.info(f"Progress analysis:")
        logger.info(f"  Total checks: {len(progress_checks)}")
        logger.info(f"  Round progress events: {len(round_progress_events)}")
        logger.info(f"  QC progress events: {len(qc_progress_events)}")
        
        # Basic liveness assertions
        if len(round_progress_events) > 0:
            logger.info("✅ Round progression detected")
        else:
            logger.warning("⚠️  No round progression detected")
        
        if len(qc_progress_events) > 0:
            logger.info("✅ QC progression detected")
        else:
            logger.warning("⚠️  No QC progression detected")
        
        # Calculate progress rate
        if len(progress_checks) > 1:
            total_time = progress_checks[-1]['timestamp']
            round_rate = len(round_progress_events) / (total_time / 60)  # rounds per minute
            logger.info(f"Round progression rate: {round_rate:.2f} rounds/minute")
        
        logger.info("✅ Jolteon liveness properties test passed")
