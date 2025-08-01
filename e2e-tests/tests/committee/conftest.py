import math
import logging
import random
import json
from typing import Generator, Tuple
from config.api_config import ApiConfig, Node
from pytest import fixture, mark, skip
from sqlalchemy import select, func, text
from sqlalchemy.orm import Session, aliased
from contextlib import contextmanager
from src.blockchain_api import BlockchainApi
from src.db.models import Candidates, PermissionedCandidates, StakeDistributionCommittee
from src.partner_chain_rpc import DParam
from src.pc_epoch_calculator import PartnerChainEpochCalculator


CANDIDATES_STABILITY_OFFSET_IN_MC_EPOCHS = 2


def pytest_collection_modifyitems(items):
    for item in items:
        if "tests/committee" in item.nodeid:
            item.add_marker(mark.ariadne)


def insert_candidates(db, candidates, type, mc_epoch):
    for candidate in candidates:
        db_candidate = type()
        db_candidate.name = candidate["name"]
        db_candidate.next_status = candidate["status"]
        db_candidate.next_status_epoch = mc_epoch
        db.add(db_candidate)
    db.commit()


@fixture(scope="session")
def initialize_candidates(api: BlockchainApi, db: Session, current_mc_epoch):
    candidates_count = db.scalar(select(func.count()).select_from(Candidates))
    if candidates_count != 0:
        return

    mc_epoch = current_mc_epoch - CANDIDATES_STABILITY_OFFSET_IN_MC_EPOCHS
    candidates = []
    while mc_epoch < current_mc_epoch + CANDIDATES_STABILITY_OFFSET_IN_MC_EPOCHS:
        candidates = api.get_trustless_rotation_candidates(mc_epoch)
        if candidates:
            insert_candidates(db, candidates, Candidates, mc_epoch)
        mc_epoch += 1

    # check if candidates were modified in current epoch, only then update db
    # otherwise registration tests would be skipped
    mc_epoch = current_mc_epoch + CANDIDATES_STABILITY_OFFSET_IN_MC_EPOCHS
    latest_candidates = api.get_trustless_rotation_candidates(mc_epoch)
    if latest_candidates != candidates:
        insert_candidates(db, latest_candidates, Candidates, mc_epoch)

    if not candidates:
        skip("No rotation candidates available")


@fixture(scope="session")
def initialize_permissioned_candidates(api: BlockchainApi, db: Session, current_mc_epoch):
    candidates_count = db.scalar(select(func.count()).select_from(PermissionedCandidates))
    if candidates_count != 0:
        return

    mc_epoch = current_mc_epoch - CANDIDATES_STABILITY_OFFSET_IN_MC_EPOCHS
    candidates = []
    while mc_epoch < current_mc_epoch + CANDIDATES_STABILITY_OFFSET_IN_MC_EPOCHS:
        candidates = api.get_permissioned_rotation_candidates(mc_epoch)
        if candidates:
            insert_candidates(db, candidates, PermissionedCandidates, mc_epoch)
        mc_epoch += 1

    # check if candidates were modified in current epoch, only then update db
    # otherwise registration tests would be skipped
    mc_epoch = current_mc_epoch + CANDIDATES_STABILITY_OFFSET_IN_MC_EPOCHS
    latest_candidates = api.get_permissioned_rotation_candidates(mc_epoch)
    if latest_candidates != candidates:
        insert_candidates(db, latest_candidates, PermissionedCandidates, mc_epoch)

    if not candidates:
        skip("No rotation candidates available")


@fixture
def candidate(request, initialize_candidates, api: BlockchainApi, config: ApiConfig, db: Session) -> Candidates:
    """Parameterized fixture to get the first 'active' or 'inactive' candidate.

    Use @pytest.mark.candidate_status() to pass data ('active' or 'inactive' only).

    Will skip the test if:
        - there are no rotation candidates in config
        case 'inactive':
            - there are no inactive candidates (or all are pending registration which means that test was already
              executed for that candidate in current mc epoch)
        case 'active':
            - there are no active candidates (or all are pending deregistration which means that test was already
              executed for that candidate in current mc epoch)

    Fixture is using nested query to get candidate with given status.
    Firstly, subquery gets latest (max) effective epoch for candidate. Effective epoch means lower or equal to current.
    Secondly, main query selects all candidates if they have matching status for above epoch.

    At this point, if no candidates were found we skip the test.

    However, if some were found we need to filter out candidates that are pending (de)registration.
    Candidates that are pending (de)registration had been already picked by this fixture in previous
    executions for current mc epoch, and we don't want to (de)register the same candidate twice.
    While multiple registrations for the same candidate would work, deregistration can be performed only once.
    """
    rotation_candidates = [name for name, node in config.nodes_config.nodes.items() if node.rotation_candidate]
    if not rotation_candidates:
        skip("No rotation candidates available")

    candidate_status = request.node.get_closest_marker("candidate_status").args[0]
    current_epoch = api.get_mc_epoch()

    committee_for_query = aliased(Candidates, name='query')
    committee_for_subquery = aliased(Candidates, name='subquery')
    subquery = (
        select(func.max(committee_for_subquery.next_status_epoch))
        .where(committee_for_subquery.name == committee_for_query.name)
        .where(committee_for_subquery.next_status_epoch <= current_epoch + 1)
    ).scalar_subquery()
    query = (
        select(committee_for_query)
        .where(committee_for_query.next_status_epoch == subquery)
        .where(committee_for_query.next_status == candidate_status)
        .where(committee_for_query.name.in_(rotation_candidates))
    )
    candidates = db.scalars(query).all()
    if not candidates:
        skip(f"No {candidate_status} candidates available.")

    candidates_names = [candidate.name for candidate in candidates]
    pending_candidates_names = []
    query_for_pending = (
        select(Candidates)
        .where(Candidates.name.in_(candidates_names))
        .where(Candidates.next_status_epoch > current_epoch + 1)
        .order_by(Candidates.id.desc())
    )
    pending_candidates = db.scalars(query_for_pending).all()
    pending_candidates_names = [candidate.name for candidate in pending_candidates]

    available_candidates = [candidate for candidate in candidates if candidate.name not in pending_candidates_names]

    if not available_candidates:
        skip(f"No {candidate_status} candidates available without a pending status")

    return available_candidates[0]


@fixture
def permissioned_candidates(
    initialize_permissioned_candidates, api: BlockchainApi, config: ApiConfig
) -> Tuple[dict[str, Node], str]:
    """
    Creates a tuple of new permissioned candidates to set and the name of the candidate to remove.

    * get permissioned candidates from config
    * get <current_mc_epoch+2> candidates from RPC
    * delete one active candidate to test rotation
    * if no current active candidates, delete one from config list and return the rest

    Returns:
        Tuple[dict[str, Node], str]: config <Node> objects, name of the candidate to remove
    """
    candidates = {name: node for name, node in config.nodes_config.nodes.items() if node.permissioned_candidate}
    if not candidates:
        skip("No permissioned candidates available in config.")

    current_epoch = api.get_mc_epoch()
    active_candidates = api.get_permissioned_candidates(current_epoch + 2, valid_only=True)

    if active_candidates:
        candidate_to_remove = random.choice(active_candidates)
        candidate_name_to_remove = next(
            (name for name, node in candidates.items() if node.aura_public_key == candidate_to_remove["keys"]["aura"]),
            None,
        )
        if not candidate_name_to_remove:
            raise ValueError(
                f"Could not find candidate with aura key {candidate_to_remove['keys']['aura']} in config."
            )
        del candidates[candidate_name_to_remove]
    else:
        candidate_name_to_remove = random.choice(list(candidates.keys()))
        del candidates[candidate_name_to_remove]

    return candidates, candidate_name_to_remove


@fixture
def trustless_rotation_candidates(request, mc_epoch, db: Session, config: ApiConfig) -> Candidates:
    """Parameterized fixture to get all the 'active' or 'inactive' rotation (trustless) candidates
    for given mc epoch. Use @pytest.mark.candidate_status() to pass data ('active' or 'inactive' only).
    """
    all_rotation_candidates = [name for name, node in config.nodes_config.nodes.items() if node.rotation_candidate]
    candidate_status = request.node.get_closest_marker("candidate_status").args[0]

    query = (
        select(Candidates)
        .where(Candidates.name.in_(all_rotation_candidates))
        .where(Candidates.next_status == candidate_status)
        .where(Candidates.next_status_epoch == mc_epoch)
    )
    rotation_candidates = db.scalars(query).all()

    if not rotation_candidates:
        skip(f"No {candidate_status} trustless candidates for MC epoch {mc_epoch}.")

    return rotation_candidates


@fixture
def permissioned_rotation_candidates(request, mc_epoch, db: Session, config: ApiConfig) -> PermissionedCandidates:
    """Parameterized fixture to get all the 'active' or 'inactive' rotation (permissioned) candidates
    for given mc epoch. Use @pytest.mark.candidate_status() to pass data ('active' or 'inactive' only).
    If there are multiple entries for the same candidate, the one with the highest ID is returned.
    """
    all_rotation_candidates = [name for name, node in config.nodes_config.nodes.items() if node.permissioned_candidate]
    candidate_status = request.node.get_closest_marker("candidate_status").args[0]

    # Create an alias for the table to use in a subquery
    latest_entries = (
        db.query(func.max(PermissionedCandidates.id).label("latest_id"))
        .group_by(PermissionedCandidates.name)
        .where(PermissionedCandidates.name.in_(all_rotation_candidates))
        .where(PermissionedCandidates.next_status_epoch == mc_epoch)
        .subquery()
    )

    # Query the latest statuses using a join
    rotation_candidates = (
        db.query(PermissionedCandidates)
        .join(latest_entries, PermissionedCandidates.id == latest_entries.c.latest_id)
        .where(PermissionedCandidates.next_status == candidate_status)
        .all()
    )

    if not rotation_candidates:
        skip(f"No {candidate_status} permissioned candidates for MC epoch {mc_epoch}.")

    return rotation_candidates


@fixture
def get_total_attendance_for_mc_epoch(update_committee_attendance, db: Session) -> int:
    def _inner(mc_epoch, db=db):
        logging.info(
            f"Getting total attendance for MC epoch {mc_epoch} from db table {StakeDistributionCommittee.__tablename__}"
        )
        update_committee_attendance(mc_epoch)
        query = select(StakeDistributionCommittee).where(StakeDistributionCommittee.mc_epoch == mc_epoch)
        candidates = db.scalars(query).all()
        logging.info(f"Candidates found: {candidates}")
        total_attendance = 0
        candidate: StakeDistributionCommittee
        for candidate in candidates:
            total_attendance += candidate.actual_attendance
        return total_attendance

    return _inner


@fixture
def get_candidate_participation(update_committee_attendance, db: Session, config: ApiConfig) -> int:
    """Parameterized fixture to get candidate's participation in committees for a main chain epoch.
    Use get_candidate_participation(candidate: Candidates) to pass data.
    """

    def _inner(candidate: Candidates, db=db, config=config):
        logging.info(f"Getting attendance of {candidate.name} from db table {StakeDistributionCommittee.__tablename__}")
        update_committee_attendance(candidate.next_status_epoch)
        query = (
            select(StakeDistributionCommittee)
            .where(StakeDistributionCommittee.mc_epoch == candidate.next_status_epoch)
            .where(StakeDistributionCommittee.pc_pub_key == config.nodes_config.nodes[candidate.name].public_key)
        )
        candidate = db.scalars(query).first()
        logging.debug(f"Found candidate: {candidate}")
        if candidate:
            return candidate.actual_attendance
        else:
            return 0

    return _inner


@contextmanager
def db_lock(db: Session, lock_name: str):
    try:
        db.execute(text(f"SELECT pg_advisory_lock(hashtext('{lock_name}'))"))
        yield
    finally:
        db.execute(text(f"SELECT pg_advisory_unlock(hashtext('{lock_name}'))"))


@fixture
def update_db_with_active_candidates(db: Session, api: BlockchainApi) -> int:
    """
    This fixture retrieves active candidates for given mc epoch,
    and stores them in db with some additional data (keys, stake, etc.).
    """

    def _inner(mc_epoch):
        lock_name = f"update_db_with_active_candidates_{mc_epoch}"
        with db_lock(db, lock_name):
            logging.debug(f"Updating db with active candidates for MC epoch {mc_epoch}.")
            query = select(StakeDistributionCommittee).where(StakeDistributionCommittee.mc_epoch == mc_epoch)
            candidates = db.scalars(query).all()
            if candidates:
                # TODO: permissioned are known upfront, but trustless are not so we might need to update them
                logging.debug(f"Some entries already exist in db for MC epoch {mc_epoch}. Skipping update.")
                return

            d_param = api.get_d_param(mc_epoch)
            permissioned_candidates_number = d_param.permissioned_candidates_number
            trustless_candidates_number = d_param.trustless_candidates_number

            if permissioned_candidates_number > 0:
                permissioned_candidates = api.get_permissioned_candidates(mc_epoch, valid_only=True)
                for candidate in permissioned_candidates:
                    candidate_db = StakeDistributionCommittee()
                    candidate_db.mc_epoch = mc_epoch
                    candidate_db.mc_vkey = "permissioned"
                    candidate_db.sc_pub_key = candidate["sidechainPublicKey"]
                    candidate_db.pc_pub_key = candidate["sidechainPublicKey"]
                    db.add(candidate_db)

            if trustless_candidates_number > 0:
                active_candidates = api.get_trustless_candidates(mc_epoch, valid_only=True)
                for active_candidate in active_candidates:
                    # This will be more than 1 if the same SPO registered multiple PC keys
                    for active_spo in active_candidates[active_candidate]:
                        candidate_db = StakeDistributionCommittee()
                        candidate_db.mc_epoch = mc_epoch
                        candidate_db.mc_vkey = active_candidate[2:]
                        candidate_db.pool_id = api.cardano_cli.get_stake_pool_id(cold_vkey=active_candidate[2:])
                        candidate_db.stake_delegation = api.cardano_cli.get_stake_snapshot_of_pool(
                            candidate_db.pool_id
                        )["pools"][candidate_db.pool_id]["stakeGo"]
                        candidate_db.pc_pub_key = active_spo["sidechainPubKey"]
                        db.add(candidate_db)

            db.commit()

    return _inner


@fixture
def update_committee_attendance(
    update_db_with_active_candidates,
    db: Session,
    get_pc_epoch_committee,
    config: ApiConfig,
    pc_epoch_calculator: PartnerChainEpochCalculator,
):

    def _inner(mc_epoch):
        if mc_epoch < config.deployment_mc_epoch:
            skip("Cannot query committee before initial epoch.")

        update_db_with_active_candidates(mc_epoch)
        query = (
            select(StakeDistributionCommittee)
            .where(StakeDistributionCommittee.mc_epoch == mc_epoch)
            .where(StakeDistributionCommittee.actual_attendance.is_(None))
        )
        candidates = db.scalars(query).all()
        if not candidates:
            logging.debug(f"Attendance for MC epoch {mc_epoch} was already calculated. Skipping update.")
            return
        logging.debug(f"Updating attendance of candidates {candidates}")
        pc_epochs_range = pc_epoch_calculator.find_pc_epochs(mc_epoch)
        for pc_epoch in pc_epochs_range:
            committee = get_pc_epoch_committee(pc_epoch)
            candidate: StakeDistributionCommittee
            for candidate in candidates:
                attendance = sum(1 for member in committee if member["sidechainPubKey"] == candidate.pc_pub_key)
                if not candidate.actual_attendance:
                    candidate.actual_attendance = 0
                candidate.actual_attendance += attendance
        db.commit()

    return _inner


@fixture
def update_committee_expected_attendance(
    update_committee_attendance,
    api: BlockchainApi,
    d_param_cache,
    db: Session,
    pc_epoch_calculator: PartnerChainEpochCalculator,
):
    def _inner(mc_epoch):
        update_committee_attendance(mc_epoch)
        # Count permissioned candidates and total stake of trustless candidates
        total_stake = 0
        permissioned_candidates_count = 0
        candidates = (
            db.query(StakeDistributionCommittee)
            .where(StakeDistributionCommittee.mc_epoch == mc_epoch)
            .where(StakeDistributionCommittee.expected_attendance.is_(None))
            .all()
        )
        if not candidates:
            logging.debug(f"Expected attendance for MC epoch {mc_epoch} was already calculated. Skipping update.")
            return
        else:
            candidates = (
                db.query(StakeDistributionCommittee).where(StakeDistributionCommittee.mc_epoch == mc_epoch).all()
            )
        logging.debug(f"Updating expected attendance of candidates {candidates}")
        candidate: StakeDistributionCommittee
        for candidate in candidates:
            if candidate.mc_vkey == "permissioned":
                permissioned_candidates_count += 1
            if candidate.stake_delegation:
                total_stake += candidate.stake_delegation

        # Get seats for permissioned and trustless candidates
        d_param: DParam = d_param_cache(mc_epoch)
        d_param_p = d_param.permissioned_candidates_number
        d_param_t = d_param.trustless_candidates_number
        total_committee_seats = d_param_p + d_param_t
        permissioned_seats = d_param_p
        trustless_seats = d_param_t

        # If there are no permissioned or trustless candidates, the committee will be filled with the other type
        trustless_candidates = api.get_trustless_candidates(mc_epoch, valid_only=True)
        if not trustless_candidates:
            permissioned_seats += d_param_t
        permissioned_candidates = api.get_permissioned_candidates(mc_epoch, valid_only=True)
        if not permissioned_candidates:
            trustless_seats += d_param_p

        # Update probabilities and expected attendance for each candidate
        epochs_num = len(pc_epoch_calculator.find_pc_epochs(mc_epoch))
        for candidate in candidates:
            if candidate.mc_vkey == "permissioned":
                expected_attendance = permissioned_seats / permissioned_candidates_count
                guaranteed_seats = math.floor(expected_attendance)
            else:
                expected_attendance = trustless_seats * (candidate.stake_delegation / total_stake)
                guaranteed_seats = math.floor(expected_attendance)
            total_expected_attendance = epochs_num * expected_attendance
            candidate.expected_attendance = total_expected_attendance
            candidate.guaranteed_seats = epochs_num * guaranteed_seats
            db.commit()

    yield _inner


@fixture(scope="session", autouse=True)
def d_param_dict() -> dict[int, DParam]:
    return {}


@fixture(scope="session")
def d_param_cache(api: BlockchainApi, d_param_dict: dict[int, DParam]):
    def _inner(mc_epoch):
        if mc_epoch not in d_param_dict.keys():
            d_param_dict[mc_epoch] = api.get_d_param(mc_epoch)
        return d_param_dict[mc_epoch]

    return _inner


@fixture(scope="session")
def get_pc_epoch_committee(api: BlockchainApi) -> Generator[dict, None, None]:
    """
    Fixture that stores the return of RPC endpoint partner_chain_getEpochCommittee in a dictionary
    """
    committees_dict = {}

    def _get_pc_epoch_committee(epoch):
        if epoch not in committees_dict.keys():
            result = api.get_epoch_committee(epoch).result
            if result is None:
                raise ValueError(f"API call returned None for epoch {epoch}")
            committee = result.get("committee")
            if committee is None:
                raise ValueError(f"Committee not found in API result for epoch {epoch}")
            committees_dict[epoch] = committee
        return committees_dict[epoch]

    yield _get_pc_epoch_committee


@fixture(scope="session", autouse=False)
def get_block_authorship_keys_dict(config: ApiConfig) -> dict:
    """
    Fixture that creates a dictionary with the PC public key as key
    and the block authoring public key as the value for all nodes
    """
    block_authorship_keys = {}
    for member in config.nodes_config.nodes:
        block_authorship_keys[config.nodes_config.nodes[member].public_key] = config.nodes_config.nodes[
            member
        ].aura_public_key
    return block_authorship_keys


@fixture(scope="session")
def get_pc_epoch_blocks(api: BlockchainApi) -> Generator[dict, None, None]:
    """
    Fixture that stores the blocks of an epoch in a dictionary
    """
    blocks_dict = {}

    def _get_pc_epoch_blocks(epoch):
        if epoch not in blocks_dict.keys():
            block_range = api.get_pc_epoch_blocks(epoch)
            if type(block_range) is not range:
                logging.error(f"Could not get block range for epoch {epoch}")
                block_range = range(0, 0)
            blocks_dict[epoch] = {}
            blocks_dict[epoch]["range"] = block_range
            logging.info(f"Getting blocks for epoch {epoch}...")
            for block in block_range:
                blocks_dict[epoch][block] = api.get_block(block)
        return blocks_dict[epoch]

    yield _get_pc_epoch_blocks


@fixture
def candidate_skey_with_cli(config: ApiConfig, api: BlockchainApi, candidate: Candidates, write_file):
    """
    Securely copy the candidate's Cardano payment key (a secret key used by the smart-contracts to pay fees)
    to a temporary file on the remote machine and update the path in the configuration.
    The temporary file is deleted after the test completes.

    This fixture is executed only if:
    - you call it directly in test or other fixture
    - tools.node.runner.files.copy_secrets is set to true in the config file `<env>_stack.json`

    WARNING: This fixture copies secret file to a remote host and should be used with caution.

    :param config: The API configuration object.
    :param candidate: The candidate to register/deregister.
    """
    if config.stack_config.tools.node.runner.copy_secrets:
        runner = api.partner_chains_node.run_command
        path = config.nodes_config.nodes[candidate.name].keys_files.cardano_payment_key
        with open(path, "r") as f:
            content = json.load(f)
            filepath = write_file(runner, content)
        config.nodes_config.nodes[candidate.name].keys_files.cardano_payment_key = filepath
        yield
        config.nodes_config.nodes[candidate.name].keys_files.cardano_payment_key = path
    else:
        yield
