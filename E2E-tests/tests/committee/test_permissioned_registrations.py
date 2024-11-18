import logging
import pytest
from src.blockchain_api import BlockchainApi
from src.db.models import PermissionedCandidates
from sqlalchemy.orm import Session
from src.sidechain_main_cli import SidechainMainCliException
from pytest import mark


@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.permissioned_candidate_status("inactive")
@mark.ariadne
@mark.test_key('ETCM-7015')
@mark.registration
@mark.xdist_group(name="governance_action")
@mark.usefixtures("governance_skey_with_cli")
def test_add_permissioned_candidate(permissioned_candidate: PermissionedCandidates, api: BlockchainApi, db: Session):
    """Test addition of the permissioned candidate

    * add inactive permissioned candidate
    * check that candidate appeared in the partner_chain_getAriadneParameters() response
    """

    logging.info(f"Adding permissioned candidate {permissioned_candidate.name}")
    try:
        result, next_status_epoch = api.add_permissioned_candidate(permissioned_candidate.name)
    except SidechainMainCliException as e:
        if 'InvalidCLIParams "New candidates list is the same as the currently stored list."' in str(e):
            pytest.skip("Skipping test because the candidate is already in the list")
        else:
            raise e
    assert result, f"Addition of permissioned candidate {permissioned_candidate.name} failed."

    new_permission_candidate = PermissionedCandidates()
    new_permission_candidate.name = permissioned_candidate.name
    new_permission_candidate.next_status = "active"
    new_permission_candidate.next_status_epoch = next_status_epoch
    db.add(new_permission_candidate)
    db.commit()

    # TODO: split into separate test
    # Get operation status from RPC
    api.wait_for_next_pc_block()
    logging.debug(f"Querying ariadne params for epoch {next_status_epoch}...")
    permissioned_candidates = api.partner_chain_rpc.partner_chain_get_ariadne_parameters(next_status_epoch).result[
        "permissionedCandidates"
    ]
    new_candidate_found = False
    for candidate in permissioned_candidates:
        if (
            candidate["sidechainPublicKey"] == api.config.nodes_config.nodes[permissioned_candidate.name].public_key
            and candidate["auraPublicKey"] == api.config.nodes_config.nodes[permissioned_candidate.name].aura_public_key
            and candidate["grandpaPublicKey"]
            == api.config.nodes_config.nodes[permissioned_candidate.name].grandpa_public_key
        ):
            new_candidate_found = True
            break
    assert (
        new_candidate_found
    ), "Expected new permissioned candidate not found, keys do not match any existing candidate"


@mark.skip_blockchain("pc_evm", reason="not implemented yet")
@mark.permissioned_candidate_status("active")
@mark.ariadne
@mark.test_key('ETCM-7016')
@mark.registration
@mark.xdist_group(name="governance_action")
@mark.usefixtures("governance_skey_with_cli")
def test_remove_permissioned_candidate(permissioned_candidate: PermissionedCandidates, api: BlockchainApi, db: Session):
    """Test removal of the permissioned candidate

    * remove active permissioned candidate
    * verify that candidate was removed from the partner_chain_getAriadneParameters() response
    """

    logging.info(f"Removing permissioned candidate {permissioned_candidate.name}")
    try:
        result, next_status_epoch = api.remove_permissioned_candidate(permissioned_candidate.name)
    except SidechainMainCliException as e:
        if 'InvalidCLIParams "New candidates list is the same as the currently stored list."' in str(e):
            pytest.skip("Skipping test due to candidate already removed from list")
        else:
            raise e
    assert result, f"Removal of permissioned candidate {permissioned_candidate.name} failed."

    removed_candidate = PermissionedCandidates()
    removed_candidate.name = permissioned_candidate.name
    removed_candidate.next_status = "inactive"
    removed_candidate.next_status_epoch = next_status_epoch
    db.add(removed_candidate)
    db.commit()

    # TODO: split into separate test
    # Get deregistration status from RPC
    api.wait_for_next_pc_block()
    permissioned_candidates = api.partner_chain_rpc.partner_chain_get_ariadne_parameters(next_status_epoch).result[
        "permissionedCandidates"
    ]
    candidate_not_found = True
    for candidate in permissioned_candidates:
        if (
            candidate["sidechainPublicKey"] == api.config.nodes_config.nodes[permissioned_candidate.name].public_key
            or candidate["auraPublicKey"] == api.config.nodes_config.nodes[permissioned_candidate.name].aura_public_key
            or candidate["grandpaPublicKey"]
            == api.config.nodes_config.nodes[permissioned_candidate.name].grandpa_public_key
        ):
            candidate_not_found = False
            break
    assert candidate_not_found, "Removed permissioned candidate still found"
