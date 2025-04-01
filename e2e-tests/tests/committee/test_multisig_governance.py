from pytest import fixture, mark, skip
from src.blockchain_api import BlockchainApi
from config.api_config import ApiConfig, MainchainAccount
from typing import Tuple
from config.api_config import Node
import logging
import random

pytestmark = mark.multisig_governance


@fixture(scope="module")
def governance_authority(config: ApiConfig) -> MainchainAccount:
    """Default governance authority account"""
    return config.nodes_config.governance_authority


@fixture(scope="module")
def additional_governance_authorities(config: ApiConfig) -> list[str]:
    """Additional authorities for multisig - should be set in test config"""
    if not config.nodes_config.additional_governance_authorities:
        skip("Additional governance authorities not configured")
    return config.nodes_config.additional_governance_authorities


@fixture(scope="module")
def set_governance_to_multisig(api: BlockchainApi, governance_authority, additional_governance_authorities):
    # Combine existing authority with additional authorities
    all_authorities = [governance_authority.mainchain_pub_key_hash] + list(
        map(lambda x: x.mainchain_pub_key_hash, additional_governance_authorities)
    )
    threshold = 2  # Require signatures from at least 2 authorities

    response = api.partner_chains_node.smart_contracts.governance.update(
        payment_key=governance_authority.mainchain_key,
        new_governance_authorities=all_authorities,
        new_governance_threshold=threshold,
    )

    yield response

    # Try to update governance back to original single-key
    response = api.partner_chains_node.smart_contracts.governance.update(
        payment_key=governance_authority.mainchain_key,
        new_governance_authorities=[governance_authority.mainchain_pub_key_hash],
        new_governance_threshold=1,
    )

    assert not response.stderr
    # With multisig, we should get a transaction CBOR
    assert response.transaction_cbor is not None

    # Save the transaction CBOR for later
    tx_cbor = response.transaction_cbor

    # First signer
    first_witness_response = api.partner_chains_node.smart_contracts.sign_tx(
        transaction_cbor=tx_cbor, payment_key=governance_authority.mainchain_key
    )

    assert not first_witness_response.stderr
    witness1 = first_witness_response.json["cborHex"]

    # Second signer
    second_key = additional_governance_authorities[0].mainchain_key
    second_witness_response = api.partner_chains_node.smart_contracts.sign_tx(
        transaction_cbor=tx_cbor, payment_key=second_key
    )

    assert not second_witness_response.stderr
    witness2 = second_witness_response.json["cborHex"]

    # Now submit the transaction with both witnesses
    submit_response = api.partner_chains_node.smart_contracts.assemble_and_submit_tx(
        transaction_cbor=tx_cbor, witnesses=[witness1, witness2]
    )

    assert not submit_response.stderr
    assert (
        submit_response.transaction_id is not None
    ), "Expected transaction ID after submitting with multiple signatures"

    # Verify governance is now restored to single key
    final_response = api.partner_chains_node.smart_contracts.governance.get_policy()
    assert not final_response.stderr

    final_policy = final_response.json
    assert len(final_policy["key_hashes"]) == 1, "Expected single governance authority after restoration"
    assert final_policy["threshold"] == 1, "Expected threshold to be 1 after restoration"
    assert (
        final_policy["key_hashes"][0] == governance_authority.mainchain_pub_key_hash
    ), "Expected original key to be restored"
    logging.info("Governance restored to single key successfully")


@mark.xdist_group(name="governance_action")
def test_get_governance_policy(api: BlockchainApi):
    """Verify the governance policy is initialized with the correct single authority"""
    response = api.partner_chains_node.smart_contracts.governance.get_policy()
    assert not response.stderr

    policy = response.json
    assert policy is not None, "Expected governance policy to be initialized"
    assert len(policy["key_hashes"]) == 1, "Expected single governance authority"
    assert policy["threshold"] == 1, "Expected threshold to be 1"


@mark.xdist_group(name="governance_action")
@mark.usefixtures("governance_skey_with_cli")
def test_update_governance_to_multisig(set_governance_to_multisig):
    """Test updating to multisig governance with multiple authorities"""

    response = set_governance_to_multisig
    assert not response.stderr
    # For update, we should get a transaction_id since we're updating with the current single authority
    assert response.transaction_id


@mark.xdist_group(name="governance_action")
def test_verify_multisig_policy(api: BlockchainApi, additional_governance_authorities):
    """Verify the governance policy has been updated to multisig"""
    response = api.partner_chains_node.smart_contracts.governance.get_policy()
    assert not response.stderr

    policy = response.json
    assert policy is not None, "Expected governance policy to be initialized"

    # Verify the correct number of authorities
    expected_count = 1 + len(additional_governance_authorities)
    assert len(policy["key_hashes"]) == expected_count, f"Expected {expected_count} governance authorities"

    # Verify threshold
    assert policy["threshold"] == 2, "Expected threshold to be 2"


@mark.xdist_group(name="governance_action")
@mark.usefixtures("governance_skey_with_cli")
def test_multisig_upsert_d_parameter(api: BlockchainApi, governance_authority, additional_governance_authorities):
    """Test a multisig operation that modifies the D parameter"""
    # Try to update D parameter

    # TODO: Extract logic to calculate unique DParam
    response = api.partner_chains_node.smart_contracts.update_d_param(
        permissioned_candidates_count=random.randint(1, 5),
        registered_candidates_count=random.randint(1, 5),
        payment_key=governance_authority.mainchain_key,
    )

    assert not response.stderr
    # We should get a transaction CBOR instead of a transaction ID
    assert response.transaction_cbor is not None
    assert response.transaction_id is not None

    # Save the transaction CBOR for later
    tx_cbor = response.transaction_cbor

    # First signer
    first_witness_response = api.partner_chains_node.smart_contracts.sign_tx(
        transaction_cbor=tx_cbor, payment_key=governance_authority.mainchain_key
    )

    assert not first_witness_response.stderr
    witness1 = first_witness_response.json["cborHex"]

    # Now use a second authority to sign (assuming we have access to its key)
    # For testing, you may need to set up a second key or mock this
    second_key = additional_governance_authorities[0].mainchain_key  # Replace with actual key file path when available
    second_witness_response = api.partner_chains_node.smart_contracts.sign_tx(
        transaction_cbor=tx_cbor, payment_key=second_key
    )

    assert not second_witness_response.stderr
    witness2 = second_witness_response.json["cborHex"]

    # Now submit the transaction with both witnesses
    submit_response = api.partner_chains_node.smart_contracts.assemble_and_submit_tx(
        transaction_cbor=tx_cbor, witnesses=[witness1, witness2]
    )

    assert not submit_response.stderr
    assert (
        submit_response.transaction_id is not None
    ), "Expected transaction ID after submitting with multiple signatures"


@mark.xdist_group(name="governance_action")
@mark.usefixtures("governance_skey_with_cli")
def test_multisig_upsert_permissioned_candidates(
    api: BlockchainApi,
    permissioned_candidates: Tuple[dict[str, Node], str],
    governance_authority,
    additional_governance_authorities,
):
    """Test a multisig operation that modifies the permissioned candidates list"""
    # Get node configuration for permissioned candidates, but remove one, so that the list is different
    new_candidates_list, candidate_to_remove = permissioned_candidates

    # Skip if no permissioned candidates are configured
    if not permissioned_candidates:
        skip("No permissioned candidates configured")

    # Try to update permissioned candidates
    response = api.partner_chains_node.smart_contracts.upsert_permissioned_candidates(
        governance_key=governance_authority.mainchain_key, new_candidates_list=new_candidates_list
    )

    assert not response.stderr
    # With multisig, we should get a transaction CBOR instead of a transaction ID
    assert response.transaction_cbor is not None
    assert response.transaction_id is not None

    # Save the transaction CBOR for later
    tx_cbor = response.transaction_cbor

    # First signer
    first_witness_response = api.partner_chains_node.smart_contracts.sign_tx(
        transaction_cbor=tx_cbor, payment_key=governance_authority.mainchain_key
    )

    assert not first_witness_response.stderr
    witness1 = first_witness_response.json["cborHex"]

    # Second signer
    second_key = additional_governance_authorities[0].mainchain_key  # Replace with actual key file path when available
    second_witness_response = api.partner_chains_node.smart_contracts.sign_tx(
        transaction_cbor=tx_cbor, payment_key=second_key
    )

    assert not second_witness_response.stderr
    witness2 = second_witness_response.json["cborHex"]

    # Now submit the transaction with both witnesses
    submit_response = api.partner_chains_node.smart_contracts.assemble_and_submit_tx(
        transaction_cbor=tx_cbor, witnesses=[witness1, witness2]
    )

    assert not submit_response.stderr
    assert (
        submit_response.transaction_id is not None
    ), "Expected transaction ID after submitting with multiple signatures"
