import logging
from src.blockchain_api import BlockchainApi
from src.pc_epoch_calculator import PartnerChainEpochCalculator
from src.partner_chain_rpc import DParam
from config.api_config import ApiConfig
from pytest import fixture, mark, skip
from src.db.models import Candidates, PermissionedCandidates, StakeDistributionCommittee
from sqlalchemy import select
from sqlalchemy.orm import Session
import numpy as np


class TestCommitteeDistribution:
    @fixture(scope="class")
    def p_candidates_available(self, api: BlockchainApi):
        candidates = {}

        def _p_candidates_available(mc_epoch):
            if mc_epoch not in candidates:
                candidates[mc_epoch] = api.get_permissioned_candidates(mc_epoch, valid_only=True)
            return candidates[mc_epoch]

        return _p_candidates_available

    @fixture(scope="class")
    def p_candidates_seats(
        self,
        current_mc_epoch,
        get_pc_epoch_committee,
        p_candidates_available,
        pc_epoch_calculator: PartnerChainEpochCalculator,
    ):
        seats = {}

        def _p_candidates_seats(pc_epoch):
            if pc_epoch not in seats:
                mc_epoch = pc_epoch_calculator.find_mc_epoch(pc_epoch, current_mc_epoch)
                p_candidates = p_candidates_available(mc_epoch)
                pc_pub_keys = [candidate['sidechainPublicKey'] for candidate in p_candidates]
                committee = get_pc_epoch_committee(pc_epoch)
                count = sum(1 for member in committee if member['sidechainPubKey'] in pc_pub_keys)
                seats[pc_epoch] = count
            return seats[pc_epoch]

        return _p_candidates_seats

    @fixture(scope="class")
    def t_candidates_available(self, api: BlockchainApi):
        candidates = {}

        def _t_candidates_available(mc_epoch):
            if mc_epoch not in candidates:
                candidates[mc_epoch] = api.get_trustless_candidates(mc_epoch, valid_only=True)
            return candidates[mc_epoch]

        return _t_candidates_available

    @fixture(scope="class")
    def t_candidates_seats(
        self,
        current_mc_epoch,
        get_pc_epoch_committee,
        t_candidates_available,
        pc_epoch_calculator: PartnerChainEpochCalculator,
    ):
        seats = {}

        def _t_candidates_seats(pc_epoch):
            if pc_epoch not in seats:
                mc_epoch = pc_epoch_calculator.find_mc_epoch(pc_epoch, current_mc_epoch)
                t_candidates = t_candidates_available(mc_epoch)
                pc_pub_keys = [candidate['sidechainPubKey'] for spo in t_candidates.values() for candidate in spo]
                committee = get_pc_epoch_committee(pc_epoch)
                count = sum(1 for member in committee if member['sidechainPubKey'] in pc_pub_keys)
                seats[pc_epoch] = count
            return seats[pc_epoch]

        return _t_candidates_seats

    @mark.committee_distribution
    @mark.ariadne
    @mark.xdist_group("governance_action")
    @mark.usefixtures("governance_skey_with_cli")
    def test_update_d_param(
        self,
        api: BlockchainApi,
        config: ApiConfig,
        current_mc_epoch,
    ):
        """
        * get DParam for n + 2 mc epoch
        * generate new DParam and update it
        * confirm that DParam was updated
        """
        if not config.nodes_config.d_param_min or not config.nodes_config.d_param_max:
            skip("Cannot test d-param update when min/max parameters are not set")

        p_floor = config.nodes_config.d_param_min.permissioned_candidates_number
        p_ceil = config.nodes_config.d_param_max.permissioned_candidates_number
        t_floor = config.nodes_config.d_param_min.trustless_candidates_number
        t_ceil = config.nodes_config.d_param_max.trustless_candidates_number

        current_d_param = api.get_d_param(current_mc_epoch + 2)

        if (
            p_floor == p_ceil == current_d_param.permissioned_candidates_number
            and t_floor == t_ceil == current_d_param.trustless_candidates_number
        ):
            skip("Cannot generate new d-param when min and max are equal to current d-param")

        new_d_param = current_d_param
        while new_d_param == current_d_param:
            new_d_param = DParam(np.random.randint(p_floor, p_ceil + 1), np.random.randint(t_floor, t_ceil + 1))

        logging.info(f"Updating d-param to {new_d_param}")
        result, mc_epoch = api.update_d_param(
            new_d_param.permissioned_candidates_number, new_d_param.trustless_candidates_number
        )
        assert result, "D-param transaction id is empty. Check command output for errors."

        # FIXME: ETCM-8945 - create and use wait_for_transaction function instead of wait_for_next_pc_block
        api.wait_for_next_pc_block()
        actual_d_param = api.get_d_param(mc_epoch)
        actual_p = actual_d_param.permissioned_candidates_number
        actual_t = actual_d_param.trustless_candidates_number
        assert (
            new_d_param.permissioned_candidates_number == actual_p
            and new_d_param.trustless_candidates_number == actual_t
        ), "D-param update did not take effect"

    @mark.test_key('ETCM-7150')
    @mark.committee_distribution
    @mark.ariadne
    def test_epoch_committee_ratio_complies_with_dparam(
        self,
        pc_epoch,
        d_param_cache,
        pc_epoch_calculator: PartnerChainEpochCalculator,
        current_mc_epoch,
        p_candidates_available,
        t_candidates_available,
        p_candidates_seats,
        t_candidates_seats,
    ):
        """Verify committee ratio in given mc epoch equals d-parameter ratio.
        Ariadne v2 introduced guarantees resulting in slot assignment that now has P entries for permissioned
        candidates and R entries for registered candidates, except there are no available candidates of one type,
        in which case all of the slots are assigned to the other type.
        """
        mc_epoch = pc_epoch_calculator.find_mc_epoch(pc_epoch, current_mc_epoch)
        p_candidates_available = p_candidates_available(mc_epoch)
        t_candidates_available = t_candidates_available(mc_epoch)
        d_param_cache: DParam = d_param_cache(mc_epoch)

        if d_param_cache.permissioned_candidates_number == 0 or d_param_cache.trustless_candidates_number == 0:
            skip("Cannot test ratio when P or T is 0.")
        if not p_candidates_available or not t_candidates_available:
            skip("Cannot test ratio when there are no available candidates.")

        expected_ratio = d_param_cache.permissioned_candidates_number / d_param_cache.trustless_candidates_number
        ratio = p_candidates_seats(pc_epoch) / t_candidates_seats(pc_epoch)
        logging.info(f"Ariadne observed ratio: {ratio}, expected ratio: {expected_ratio}")
        assert expected_ratio == ratio

    @mark.ariadne
    @mark.committee_distribution
    def test_epoch_p_candidates_seats(
        self,
        config: ApiConfig,
        pc_epoch,
        p_candidates_available,
        t_candidates_available,
        d_param_cache,
        p_candidates_seats,
        pc_epoch_calculator: PartnerChainEpochCalculator,
        current_mc_epoch,
    ):
        """Test that the number of permissioned candidates seats in the committee is equal to the P in DParam."""
        if pc_epoch < config.initial_pc_epoch:
            skip("Cannot query committee before initial epoch.")
        if pc_epoch == config.initial_pc_epoch:
            skip("Initial committee is set in chain-spec, not DParam.")
            
        mc_epoch = pc_epoch_calculator.find_mc_epoch(pc_epoch, current_mc_epoch)
        p_candidates_available = p_candidates_available(mc_epoch)
        t_candidates_available = t_candidates_available(mc_epoch)
        d_param_cache: DParam = d_param_cache(mc_epoch)

        # Calculate expected P-candidates seats based on total committee size
        total_seats = d_param_cache.permissioned_candidates_number + d_param_cache.trustless_candidates_number
        expected_p_candidates = int(total_seats * (d_param_cache.permissioned_candidates_number / total_seats))
        
        # Adjust for special cases
        if not t_candidates_available:
            expected_p_candidates = total_seats
        if not p_candidates_available:
            expected_p_candidates = 0
            
        actual_p_candidates = p_candidates_seats(pc_epoch)
        assert expected_p_candidates == actual_p_candidates, f"Expected {expected_p_candidates} P-candidates, got {actual_p_candidates}"

    @mark.ariadne
    @mark.committee_distribution
    def test_epoch_t_candidates_seats(
        self,
        config: ApiConfig,
        pc_epoch,
        p_candidates_available,
        t_candidates_available,
        d_param_cache,
        t_candidates_seats,
        pc_epoch_calculator: PartnerChainEpochCalculator,
        current_mc_epoch,
    ):
        """Test that the number of trustless candidates seats in the committee is equal to the T in DParam."""
        if pc_epoch < config.initial_pc_epoch:
            skip("Cannot query committee before initial epoch.")
        if pc_epoch == config.initial_pc_epoch:
            skip("Initial committee is set in chain-spec, not DParam.")
            
        mc_epoch = pc_epoch_calculator.find_mc_epoch(pc_epoch, current_mc_epoch)
        p_candidates_available = p_candidates_available(mc_epoch)
        t_candidates_available = t_candidates_available(mc_epoch)
        d_param_cache: DParam = d_param_cache(mc_epoch)

        # Calculate expected T-candidates seats based on total committee size
        total_seats = d_param_cache.permissioned_candidates_number + d_param_cache.trustless_candidates_number
        expected_t_candidates = total_seats - int(total_seats * (d_param_cache.permissioned_candidates_number / total_seats))
        
        # Adjust for special cases
        if not p_candidates_available:
            expected_t_candidates = total_seats
        if not t_candidates_available:
            expected_t_candidates = 0
            
        actual_t_candidates = t_candidates_seats(pc_epoch)
        assert expected_t_candidates == actual_t_candidates, f"Expected {expected_t_candidates} T-candidates, got {actual_t_candidates}"

    @mark.test_key('ETCM-7032')
    @mark.committee_distribution
    @mark.ariadne
    def test_epoch_committee_size_complies_with_dparam(
        self, config: ApiConfig, get_pc_epoch_committee, pc_epoch, pc_epoch_calculator, current_mc_epoch, d_param_cache
    ):
        """Test that committee size complies with d-parameter.
        1. Calculate expected committee size based on numPermissionedCandidates + numRegisteredCandidates
        (RPC:partner_chain_getAriadneParameters)
        2. Get epoch committee length (RPC:partner_chain_getEpochCommittee)
        3. Assert both numbers are equal
        """
        if pc_epoch < config.initial_pc_epoch:
            skip("Cannot query committee before initial epoch.")
        if pc_epoch == config.initial_pc_epoch:
            skip("Initial committee is set in chain-spec, not DParam.")
        mc_epoch = pc_epoch_calculator.find_mc_epoch(pc_epoch, current_mc_epoch)
        d_param_cache: DParam = d_param_cache(mc_epoch)
        max_validators = config.max_validators
        total_committee_size = d_param_cache.permissioned_candidates_number + d_param_cache.trustless_candidates_number
        expected_committee_size = min(total_committee_size, max_validators)

        committee = get_pc_epoch_committee(pc_epoch)
        assert len(committee) == expected_committee_size, f"Committee size mismatch for pc epoch {pc_epoch}."

    @mark.test_key('ETCM-7027')
    @mark.ariadne
    @mark.committee_distribution
    def test_mc_epoch_committee_participation_total_number(
        self, api: BlockchainApi, config: ApiConfig, mc_epoch, get_total_attendance_for_mc_epoch
    ):
        """Test that the total number of committee participations in a MC epoch equals the expected number."""
        # Get expected total attendance
        expected_total_attendance = api.get_committee_seats(mc_epoch) * config.pc_epochs_in_mc_epoch_count
        
        # Get actual total attendance
        actual_total_attendance = get_total_attendance_for_mc_epoch(mc_epoch)
        
        # Log the values for debugging
        logging.info(f"Expected total attendance: {expected_total_attendance}")
        logging.info(f"Actual total attendance: {actual_total_attendance}")
        
        # Allow for some variance due to network conditions
        variance = expected_total_attendance * 0.1  # 10% variance
        assert abs(expected_total_attendance - actual_total_attendance) <= variance, \
            f"Expected total attendance {expected_total_attendance}, got {actual_total_attendance}"

    @mark.test_key('ETCM-7028')
    @mark.ariadne
    @mark.probability
    @mark.committee_distribution
    def test_mc_epoch_committee_participation_probability(
        self, mc_epoch, update_committee_expected_attendance, config: ApiConfig, db: Session
    ):
        """
        1. [Precondition] Update expected attendance for each committee member in given mc epoch
        2. Get all candidates for given mc epoch
        3. Assert all candidates have expected attendance within a threshold
        4. Assert sum of probabilities is equal to 1
        """
        if mc_epoch < config.deployment_mc_epoch:
            skip("Cannot query committee before initial epoch.")
        if mc_epoch == config.deployment_mc_epoch:
            skip("On deployment day tolerance may be exceeded.")
        update_committee_expected_attendance(mc_epoch)
        candidates = db.query(StakeDistributionCommittee).filter_by(mc_epoch=mc_epoch).all()
        for candidate in candidates:
            target = candidate.expected_attendance
            target = round(target)  # round target to avoid near-zero attendance, e.g. 0.0001
            tolerance = max(target * config.committee_participation_tolerance, 2)
            actual = candidate.actual_attendance
            assert (
                target - tolerance <= actual <= target + tolerance
            ), f"Incorrect attendance for {candidate.pc_pub_key}"

    @mark.ariadne
    @mark.committee_distribution
    def test_guaranteed_seats(self, mc_epoch, update_committee_expected_attendance, config: ApiConfig, db: Session):
        if mc_epoch < config.deployment_mc_epoch:
            skip("Cannot query committee before initial epoch.")
        if mc_epoch == config.deployment_mc_epoch:
            skip("On deployment day tolerance may be exceeded.")
        update_committee_expected_attendance(mc_epoch)
        candidates = db.query(StakeDistributionCommittee).filter_by(mc_epoch=mc_epoch).all()

        for candidate in candidates:
            expected_guaranteed_seats = candidate.guaranteed_seats
            assert candidate.actual_attendance >= expected_guaranteed_seats


class TestCommitteeRotation:

    @mark.test_key('ETCM-6991')
    @mark.ariadne
    @mark.committee_rotation
    def test_committee_members_rotate_over_pc_epochs(
        self, config: ApiConfig, pc_epoch_calculator: PartnerChainEpochCalculator, get_pc_epoch_committee, mc_epoch
    ):
        """Test that committee members rotate over partner_chain epoch.
        1. Get pc epochs for given mc epoch
        2. Starting from first pc epoch, get committee members and compare with committee members of subsequent pc epoch
        3. Assert members of pc epoch committee are different than those of the subsequent pc epoch.
            Otherwise continue testing members of remaining pc epochs.
        4. Fail test if no committee member changes were detected in any pair of pc epochs in mc epoch
        """
        if mc_epoch < config.deployment_mc_epoch:
            skip("Cannot query committee before initial epoch.")
        pc_epoch_range = pc_epoch_calculator.find_pc_epochs(mc_epoch)
        epochs_with_committee_rotation = self.__find_first_pc_epoch_with_committee_rotation(
            get_pc_epoch_committee, pc_epoch_range
        )
        first_epoch, second_epoch = epochs_with_committee_rotation
        logging.info(f"Rotation of committee members detected in pc epoch change from {first_epoch} to {second_epoch}")

        assert (
            epochs_with_committee_rotation is not None
        ), "No committee member rotations were detected in any of the pc epochs in mc epoch"

    def __find_first_pc_epoch_with_committee_rotation(self, get_pc_epoch_committee, pc_epoch_range):
        for current_pc_epoch in pc_epoch_range:
            next_pc_epoch = current_pc_epoch + 1
            current_committee = get_pc_epoch_committee(current_pc_epoch)
            next_committee = get_pc_epoch_committee(next_pc_epoch)
            current_keys = [member['sidechainPubKey'] for member in current_committee]
            next_keys = [member['sidechainPubKey'] for member in next_committee]

            if current_keys != next_keys:
                return (current_pc_epoch, next_pc_epoch)
        else:
            return None

    @mark.candidate_status("active")
    @mark.test_key('ETCM-6987')
    @mark.ariadne
    @mark.committee_rotation
    def test_active_trustless_candidates_were_in_committee(
        self, trustless_rotation_candidates: Candidates, get_candidate_participation: int
    ):
        """Test that active trustless candidates participated in committees

        * get a list of trustless candidates for a given mainchain epoch
        * verify that each active candidate included in committees within an mainchain epoch
        """
        for candidate in trustless_rotation_candidates:
            logging.info(
                f"Verifying if {candidate.name} is found in committee for MC epoch {candidate.next_status_epoch}"
            )
            assert get_candidate_participation(candidate) > 0, (
                f"Trustless candidate {candidate.name} not found in any committees on mc epoch "
                f"{candidate.next_status_epoch}"
            )

    @mark.candidate_status("inactive")
    @mark.test_key('ETCM-6988')
    @mark.ariadne
    @mark.committee_rotation
    def test_inactive_trustless_candidates_were_not_in_committee(
        self, trustless_rotation_candidates: Candidates, get_candidate_participation: int
    ):
        """Test that inactive trustless candidates have not participated in committees

        * get a list of trustless candidates for a given mainchain epoch
        * verify that each inactive candidate have not included in committees within an mainchain epoch
        """
        for candidate in trustless_rotation_candidates:
            logging.info(
                f"Verifying if {candidate.name} isn't found in any committee for MC epoch {candidate.next_status_epoch}"
            )
            assert get_candidate_participation(candidate) == 0, (
                f"Inactive trustless candidate {candidate.name} found in committee on mc epoch "
                f"{candidate.next_status_epoch}"
            )

    @mark.candidate_status("active")
    @mark.test_key('ETCM-6989')
    @mark.ariadne
    @mark.committee_rotation
    def test_active_permissioned_candidates_were_in_committee(
        self, permissioned_rotation_candidates: PermissionedCandidates, get_candidate_participation: int
    ):
        """Test that permissioned trustless candidates participated in committees

        * get a list of permissioned candidates for a given mainchain epoch
        * verify that each active candidate included in committees within a mainchain epoch
        """
        for candidate in permissioned_rotation_candidates:
            logging.info(
                f"Verifying if {candidate.name} is found in committee for MC epoch {candidate.next_status_epoch}"
            )
            assert get_candidate_participation(candidate) > 0, (
                f"Permissioned candidate {candidate.name} not found in any committees on mc epoch ",
                f"{candidate.next_status_epoch}",
            )

    @mark.candidate_status("inactive")
    @mark.test_key('ETCM-6990')
    @mark.ariadne
    @mark.committee_rotation
    def test_inactive_permissioned_candidates_were_not_in_committee(
        self, permissioned_rotation_candidates: PermissionedCandidates, get_candidate_participation: int
    ):
        """Test that inactive permissioned candidates have not participated in committees

        * get a list of permissioned candidates for a given mainchain epoch
        * verify that each inactive candidate have not included in committees within an mainchain epoch
        """
        for candidate in permissioned_rotation_candidates:
            logging.info(
                f"Verifying if {candidate.name} isn't found in any committee for MC epoch {candidate.next_status_epoch}"
            )
            assert get_candidate_participation(candidate) == 0, (
                f"Inactive permissioned candidate {candidate.name} found in committee on mc epoch ",
                f"{candidate.next_status_epoch}",
            )


class TestCommitteeMembers:

    @mark.test_key('ETCM-7033')
    @mark.ariadne
    @mark.committee_members
    def test_there_is_at_least_one_trustless_candidate(self, api: BlockchainApi, current_mc_epoch):
        """Test that the configured d-parameter has at least one trustless candidate"""
        if api.get_d_param(current_mc_epoch).trustless_candidates_number == 0:
            skip("T==0, test is irrelevant")
        if current_mc_epoch < 3:
            skip("Stake pool is not yet initialized")
        assert len(api.get_trustless_candidates(current_mc_epoch, valid_only=True)) > 0

    @mark.test_key('ETCM-7034')
    @mark.ariadne
    @mark.committee_members
    def test_there_is_at_least_one_permissioned_candidate(self, api: BlockchainApi, current_mc_epoch):
        """Test that the configured d-parameter has at least one permissioned candidate"""
        if api.get_d_param(current_mc_epoch).permissioned_candidates_number == 0:
            skip("P==0, test is irrelevant")
        assert len(api.get_permissioned_candidates(current_mc_epoch, valid_only=True)) > 0

    @mark.test_key('ETCM-7026')
    @mark.ariadne
    @mark.committee_members
    def test_no_rogue_committee_members(
        self,
        db: Session,
        get_pc_epoch_committee,
        pc_epoch,
        pc_epoch_calculator: PartnerChainEpochCalculator,
        current_mc_epoch,
        config: ApiConfig,
        update_db_with_active_candidates,
    ):
        """Test that committee for given SC epoch does not have unexpected candidates

        * update db (if needed) with active candidates for MC epoch that given SC epoch belongs to
        * get active candidates from DB (expected result)
        * assert all committee members are active candidates (actual result)
        """
        if pc_epoch < config.initial_pc_epoch:
            skip("Cannot query committee before initial epoch.")
        elif pc_epoch == config.initial_pc_epoch:
            skip("Initial committee may be unknown.")

        mc_epoch = pc_epoch_calculator.find_mc_epoch(pc_epoch, current_mc_epoch)
        update_db_with_active_candidates(mc_epoch)

        query = select(StakeDistributionCommittee.pc_pub_key).where(StakeDistributionCommittee.mc_epoch == mc_epoch)
        active_candidates = db.scalars(query).all()
        committee = get_pc_epoch_committee(pc_epoch)
        for candidate in committee:
            pub_key = candidate['sidechainPubKey']
            assert (
                pub_key in active_candidates
            ), f"Committee member {pub_key} not an active candidate for pc epoch {pc_epoch}"

    @mark.ariadne
    @mark.test_key('ETCM-7029')
    @mark.committee_members
    def test_authorities_matching_committee(self, api: BlockchainApi, config: ApiConfig):
        """Test that authorities match validators for a given partner chain epoch

        * get current partner chain epoch
        * get epoch committee for an epoch
        * get authorities list
        * check that validators from a committee equal to the authorities
        """
        current_epoch = api.get_pc_epoch()
        committee = api.get_epoch_committee(current_epoch).result['committee']
        authorities = api.get_authorities()

        if api.get_pc_epoch() > current_epoch:
            skip("Epoch has changed while getting committee from partner chain rpc and blockchain api")

        validators_names = []

        for validator in committee:
            for key, value in config.nodes_config.nodes.items():
                if value.public_key == validator["sidechainPubKey"]:
                    validators_names.append(key)
                    break

        authorities_names = []
        for authority in authorities:
            for key, value in config.nodes_config.nodes.items():
                if value.aura_ss58_address == authority:
                    authorities_names.append(key)
                    break

        assert sorted(validators_names) == sorted(
            authorities_names
        ), f"Some validators seem offline: {set(authorities_names)-set(validators_names)}"
