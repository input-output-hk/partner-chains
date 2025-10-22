from src.blockchain_api import BlockchainApi, Wallet
import logging
from pytest import mark


@mark.xdist_group("faucet_tx")
def test_delegator_can_associate_pc_address(genesis_utxo, api: BlockchainApi, new_wallet: Wallet, get_wallet: Wallet):
    logging.info("Starting address association test...")
    logging.debug(f"Partner Chain address: {new_wallet.address}")
    logging.debug(f"Genesis UTXO: {genesis_utxo}")
    
    logging.info("Generating Cardano stake keys...")
    stake_skey, stake_vkey = api.cardano_cli.generate_stake_keys()
    skey_hex = stake_skey["cborHex"][4:]
    vkey_hex = stake_vkey["cborHex"][4:]
    logging.debug(f"Generated stake signing key (hex): {skey_hex[:20]}...")
    logging.debug(f"Generated stake verification key (hex): {vkey_hex[:20]}...")
    
    logging.info("Signing address association...")
    signature = api.sign_address_association(genesis_utxo, new_wallet.public_key, skey_hex)
    logging.debug(f"Address association signature created")
    logging.debug(f"Signature partner chain address: {signature.partner_chain_address}")
    logging.debug(f"Signature stake public key: {signature.stake_public_key[:20]}...")
    
    assert signature.partner_chain_address == new_wallet.address, \
        f"Partner chain address mismatch: expected {new_wallet.address}, got {signature.partner_chain_address}"
    assert signature.signature, f"Address association signature is empty for PC address {new_wallet.address}"
    assert signature.stake_public_key == f"0x{vkey_hex}", \
        f"Stake public key mismatch: expected 0x{vkey_hex}, got {signature.stake_public_key}"

    logging.info("Submitting address association transaction...")
    logging.debug(f"Using wallet: {get_wallet.address}")
    tx = api.submit_address_association(signature, wallet=get_wallet)
    logging.debug(f"Address association transaction submitted with hash: {tx.hash}")
    assert tx.hash, f"Failed to submit address association transaction for PC address {new_wallet.address}"

    logging.info("Verifying address association on-chain...")
    vkey_hash = api.cardano_cli.get_stake_key_hash(vkey_hex)
    logging.debug(f"Stake public key hash: {vkey_hash}")
    
    address_association = api.get_address_association(vkey_hash)
    logging.debug(f"Retrieved address association: {address_association}")
    
    assert address_association == new_wallet.address, \
        f"Address association verification failed: expected {new_wallet.address}, got {address_association} for stake key hash {vkey_hash}"
    
    logging.info("Address association test completed successfully!")
