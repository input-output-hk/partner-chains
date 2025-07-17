import logging
from src.blockchain_api import BlockchainApi
from src.db.models import Candidates
from sqlalchemy.orm import Session
from pytest import mark
from config.api_config import ApiConfig


@mark.candidate_status("inactive")
@mark.test_key('ETCM-7017')
@mark.usefixtures("candidate_skey_with_cli")
def test_register_candidate(genesis_utxo: str, candidate: Candidates, api: BlockchainApi, db: Session, config: ApiConfig):
    """Test registration of the trustless (SPO) candidate

    * register inactive SPO candidate
    * wait for next partner chain block
    * check that registered candidate appeared in the partner_chain_getAriadneParameters() response
    """
    logging.info(f"Registering {candidate}")
    result, next_status_epoch = api.register_candidate(genesis_utxo, candidate.name)
    assert result, f"Registration of candidate {candidate.name} failed."

    registered_candidate = Candidates()
    registered_candidate.name = candidate.name
    registered_candidate.next_status = "active"
    registered_candidate.next_status_epoch = next_status_epoch
    db.add(registered_candidate)
    db.commit()

    # TODO: split into separate test
    # Get registration status from RPC
    api.wait_for_next_pc_block()
    candidate_registrations = api.get_trustless_candidates(next_status_epoch, valid_only=False)
    spo_public_key = config.nodes_config.nodes[candidate.name].keys_files.spo_public_key
    registered_mc_pub_key = f"0x{api.read_cardano_key_file(spo_public_key)}"
    # FIXME: ETCM-7370 handle multiple registrations for a single spo
    registration = candidate_registrations[registered_mc_pub_key][0]
    registered_pc_pub_key = config.nodes_config.nodes[candidate.name].public_key
    registered_aura_pub_key = config.nodes_config.nodes[candidate.name].aura_public_key
    registered_grandpa_pub_key = config.nodes_config.nodes[candidate.name].grandpa_public_key
    assert (
        registered_pc_pub_key == registration["sidechainPubKey"]
    ), f"Could not find SC public key {registered_pc_pub_key} registered for MC epoch {next_status_epoch}"
    assert (
        registered_mc_pub_key == registration["mainchainPubKey"]
    ), f"Could not find MC public key {registered_mc_pub_key} registered for MC epoch {next_status_epoch}"
    assert (
        registered_aura_pub_key == registration["keys"]["aura"]
    ), f"Could not find Aura public key {registered_aura_pub_key} registered for MC epoch {next_status_epoch}"
    assert (
        registered_grandpa_pub_key == registration["keys"]["gran"]
    ), f"Could not find Grandpa public key {registered_grandpa_pub_key} registered for MC epoch {next_status_epoch}"
    assert registration[
        "isValid"
    ], f"Registered candidate {candidate.name} is not valid. Invalidity reason: {registration['invalidReasons']}."


@mark.candidate_status("active")
@mark.test_key('ETCM-7018')
@mark.usefixtures("candidate_skey_with_cli")
def test_deregister_candidate(genesis_utxo: str, candidate: Candidates, api: BlockchainApi, db: Session, config: ApiConfig):
    """Test deregistration of the trustless (SPO) candidate

    * deregister active SPO candidate
    * wait for next partner chain block
    * check that registered candidate disappeared from the partner_chian_getAriadneParameters() response
    """
    logging.info(f"Deregistering {candidate}")
    # mc_block_before_deregistration = api.get_mc_block() # TODO: https://input-output.atlassian.net/browse/ETCM-5566
    result, next_status_epoch = api.deregister_candidate(genesis_utxo, candidate.name)
    assert result, f"Deregistration of candidate {candidate.name} failed."

    deregistered_candidate = Candidates()
    deregistered_candidate.name = candidate.name
    deregistered_candidate.next_status = "inactive"
    deregistered_candidate.next_status_epoch = next_status_epoch
    db.add(deregistered_candidate)
    db.commit()

    # TODO: split into separate test
    # Get deregistration status from RPC
    api.wait_for_next_pc_block()
    candidate_registrations = api.get_trustless_candidates(next_status_epoch, valid_only=False)
    deregistered_mc_pub_key = api.read_cardano_key_file(
        config.nodes_config.nodes[candidate.name].keys_files.spo_public_key
    )

    # FIXME: ETCM-7370 handle multiple registrations for a single spo
    assert (
        deregistered_mc_pub_key not in candidate_registrations.keys()
    ), f"Found deregistered MC pubKey {deregistered_mc_pub_key} in MC epoch {next_status_epoch}"
