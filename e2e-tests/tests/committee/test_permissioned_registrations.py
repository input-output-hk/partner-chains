import logging
from config.api_config import Node
from src.blockchain_api import BlockchainApi
from src.db.models import PermissionedCandidates
from sqlalchemy.orm import Session
from pytest import mark
from typing import Tuple


@mark.test_key('ETCM-7015')
@mark.xdist_group(name="governance_action")
@mark.usefixtures("governance_skey_with_cli")
def test_upsert_permissioned_candidates(
    permissioned_candidates: Tuple[dict[str, Node], str], genesis_utxo, api: BlockchainApi, db: Session, write_file
):
    """Test addition of the permissioned candidate

    * add inactive permissioned candidate
    * check that candidate appeared in the partner_chain_getAriadneParameters() response
    """
    new_candidates_list, candidate_to_remove = permissioned_candidates

    logging.info(f"Setting permissioned candidates {list(new_candidates_list.keys())}, genesis utxo: {genesis_utxo}")
    candidates_file_content = "\n".join(
        f"{candidate.public_key}:{candidate.aura_public_key}:{candidate.grandpa_public_key}"
        for candidate in new_candidates_list.values()
    )
    candidates_filepath = write_file(api.partner_chains_node.run_command, candidates_file_content, is_json=False)
    result, next_status_epoch = api.upsert_permissioned_candidates(genesis_utxo, candidates_filepath)
    assert result, f"Addition of permissioned candidate {new_candidates_list} failed."

    for candidate in new_candidates_list.keys():
        new_permission_candidate = PermissionedCandidates()
        new_permission_candidate.name = candidate
        new_permission_candidate.next_status = "active"
        new_permission_candidate.next_status_epoch = next_status_epoch
        db.add(new_permission_candidate)

    removed_candidate = PermissionedCandidates()
    removed_candidate.name = candidate_to_remove
    removed_candidate.next_status = "inactive"
    removed_candidate.next_status_epoch = next_status_epoch
    db.add(removed_candidate)
    db.commit()

    # TODO: split into separate test
    expected_candidates = []
    for candidate in new_candidates_list.values():
        expected_candidates.append(
            {
                "sidechainPublicKey": candidate.public_key,
                "keys": { "aura": candidate.aura_public_key, "gran": candidate.grandpa_public_key },
                "isValid": True,
            }
        )

    # Get operation status from RPC
    api.wait_for_next_pc_block()
    logging.info(f"Querying ariadne params for epoch {next_status_epoch}...")
    rpc_permissioned_candidates = api.partner_chain_rpc.partner_chain_get_ariadne_parameters(next_status_epoch).result[
        "permissionedCandidates"
    ]
    assert compare_lists_of_dicts(
        expected_candidates, rpc_permissioned_candidates
    ), "Expected permissioned candidates not found"


def compare_lists_of_dicts(list1, list2):
    def sort_key(d):
        return tuple(sorted(d.items()))

    sorted_list1 = sorted(list1, key=sort_key)
    sorted_list2 = sorted(list2, key=sort_key)
    return sorted_list1 == sorted_list2
