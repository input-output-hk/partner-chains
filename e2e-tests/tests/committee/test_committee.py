import logging
from src.blockchain_api import BlockchainApi
from src.pc_epoch_calculator import PartnerChainEpochCalculator
from src.partner_chain_rpc import DParam
from config.api_config import ApiConfig
from pytest import mark, skip
from src.db.models import Candidates, PermissionedCandidates, StakeDistributionCommittee
from sqlalchemy import func, select
from sqlalchemy.orm import Session
import math
import numpy as np


def calculate_d_param_tolerance(pc_epochs_in_mc_epoch, d_param_p, d_param_t):
    """
    Calculate the tolerance for the committee ratio test.
    The tolerance is calculated by running a large number of simulations
    """
    # Probability of p_cnt increment
    p_prob = d_param_p / (d_param_p + d_param_t)

    # Simulate all dice rolls at once using NumPy's capabilities
    dice_rolls = np.random.randint(1, d_param_p + d_param_t + 1, (50000, pc_epochs_in_mc_epoch))
    p_counts = np.sum(dice_rolls <= d_param_p, axis=1)

    highest_cnt = np.max(p_counts)
    observed_minimum_required_tolerance = highest_cnt / (p_prob * pc_epochs_in_mc_epoch) - 1
    d_param_tolerance = 1.1 * observed_minimum_required_tolerance
    logging.debug(
        f"Observed minimum tolerance: {observed_minimum_required_tolerance}. Tolerance to use: {d_param_tolerance}."
    )

    return d_param_tolerance


class TestCommitteeDistribution:

    @mark.committee_distribution
    @mark.ariadne
    @mark.xdist_group("governance_action")
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
        assert result, "D-param update failed"

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
    @mark.probability
    def test_epoch_committee_ratio_complies_with_dparam(
        self, db: Session, config: ApiConfig, mc_epoch, d_param_cache, update_committee_attendance, api: BlockchainApi
    ):
        """Test that committee ratio complies with d-parameter.
        1. Get attendance of permissioned and trustless committee members in given mc epoch from DB
        2. Assert the ratio is within a threshold of d-parameter ratio
        """
        if mc_epoch < config.deployment_mc_epoch:
            skip("Cannot query committee before initial epoch.")
        update_committee_attendance(mc_epoch)

        p_candidates_attendance = (
            db.query(func.sum(StakeDistributionCommittee.actual_attendance))
            .where(StakeDistributionCommittee.mc_epoch == mc_epoch)
            .where(StakeDistributionCommittee.mc_vkey == "permissioned")
            .scalar()
        ) or 0
        t_candidates_attendance = (
            db.query(func.sum(StakeDistributionCommittee.actual_attendance))
            .where(StakeDistributionCommittee.mc_epoch == mc_epoch)
            .where(StakeDistributionCommittee.mc_vkey != "permissioned")
            .scalar()
        ) or 0
        logging.info(f"Permissioned candidates attendance: {p_candidates_attendance}")
        logging.info(f"Trustless candidates attendance: {t_candidates_attendance}")

        p_candidates_available = api.get_permissioned_candidates(mc_epoch, valid_only=True)
        t_candidates_available = api.get_trustless_candidates(mc_epoch, valid_only=True)
        d_param_cache: DParam = d_param_cache(mc_epoch)

        if d_param_cache.permissioned_candidates_number == 0 or d_param_cache.trustless_candidates_number == 0:
            skip("Cannot test ratio when P or T is 0.")
        if not p_candidates_available or not t_candidates_available:
            skip("Cannot test ratio when there are no available candidates.")

        expected_ratio = d_param_cache.permissioned_candidates_number / d_param_cache.trustless_candidates_number
        ratio = p_candidates_attendance / t_candidates_attendance
        logging.info(f"Ariadne observed ratio: {ratio}, expected ratio: {expected_ratio}")

        pc_epochs_in_mc_epoch = config.nodes_config.pc_epochs_in_mc_epoch_count
        d_param_expected_tolerance = calculate_d_param_tolerance(
            pc_epochs_in_mc_epoch,
            d_param_cache.permissioned_candidates_number,
            d_param_cache.trustless_candidates_number,
        )
        tolerance = expected_ratio * d_param_expected_tolerance

        difference = abs(ratio - expected_ratio)
        assert (
            difference <= tolerance
        ), f"Difference {difference} not within tolerance {tolerance} of expected ratio {expected_ratio}"

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
        """
        This test is run for the last epoch that has been completed.
        All pc epochs of that mc epoch will be queried for committee members.
        Their attendance will be counted and asserted to be equal to the number of epochs
        times the committee size.
        """
        if mc_epoch < config.deployment_mc_epoch:
            skip("Cannot query committee before initial epoch.")
        if mc_epoch == config.deployment_mc_epoch:
            skip("Initial committee may be different than the rest")

        logging.info(f"Counting committee members participation for mc epoch {mc_epoch}")
        actual_total_attendance = get_total_attendance_for_mc_epoch(mc_epoch)
        pc_epochs_in_mc_epoch_count = config.nodes_config.pc_epochs_in_mc_epoch_count
        assert api.get_committee_seats(mc_epoch) * pc_epochs_in_mc_epoch_count == actual_total_attendance

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
        probability_sum = 0
        for candidate in candidates:
            target = candidate.expected_attendance
            tolerance = target * config.committee_participation_tolerance
            actual = candidate.actual_attendance
            assert (
                target - tolerance <= actual <= target + tolerance
            ), f"Incorrect attendance for {candidate.pc_pub_key}"

            probability_sum += candidate.probability

        assert math.isclose(
            probability_sum, 1, rel_tol=1e-9
        ), f"Sum of probabilities is not equal to 1: {probability_sum}"


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

    @mark.test_key("ETCM-7236")
    @mark.committee_rotation
    @mark.active_flow
    def test_committee_was_selected_by_previous_committee(
        self, pc_epoch, config: ApiConfig, get_pc_epoch_committee, get_pc_epoch_signatures
    ):
        """
        * Query epoch n-1 for the next committee pub keys
        * Query epoch n for the committee members' pub keys
        * Assert both are equal
        """
        if pc_epoch <= config.initial_pc_epoch:
            skip("Cannot query committee before initial epoch.")

        expected_committee_public_keys = get_pc_epoch_signatures(pc_epoch - 1)['nextCommitteePubKeys']
        expected_committee_public_keys.sort()
        actual_committee_public_keys = [member['sidechainPubKey'] for member in get_pc_epoch_committee(pc_epoch)]
        actual_committee_public_keys.sort()
        assert actual_committee_public_keys == expected_committee_public_keys, (
            f"Current committee for epoch {pc_epoch} does not match ",
            f"expected next committee from epoch {pc_epoch - 1}",
        )

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

    @mark.test_key('ETCM-7030')
    @mark.committee_members
    @mark.active_flow
    def test_epoch_signatures_are_made_by_all_committee_members(
        self, config: ApiConfig, get_pc_epoch_committee, get_pc_epoch_signatures, pc_epoch
    ):
        """Test that partner chain epoch is signed by current committee members
        1. Create unique list of public keys selected as committee (RPC:partner_chain_getEpochCommittee)
        2. Create unique list of public keys that have signed the epoch (RPC:partner_chain_getEpochSignatures)
        3. Assert both lists are equal
        """
        if pc_epoch <= config.initial_pc_epoch:
            skip("Cannot query committee before initial epoch.")
        committee = get_pc_epoch_committee(pc_epoch)
        signatures = get_pc_epoch_signatures(pc_epoch)["signatures"]
        committee_public_keys = set(item["sidechainPubKey"] for item in committee)
        signature_public_keys = set(item["committeeMember"] for item in signatures)

        assert (
            committee_public_keys == signature_public_keys
        ), f"Epoch {pc_epoch} has wrong signatures, diff: {committee_public_keys - signature_public_keys}."

    @mark.test_key('ETCM-7031')
    @mark.committee_members
    @mark.active_flow
    def test_epoch_signatures_are_not_empty(self, config: ApiConfig, get_pc_epoch_signatures, pc_epoch):
        if pc_epoch <= config.initial_pc_epoch:
            skip("Cannot query committee before initial epoch.")
        signatures = get_pc_epoch_signatures(pc_epoch)["signatures"]
        for signature in signatures:
            assert signature["signature"]

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
