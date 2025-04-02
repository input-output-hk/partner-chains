from src.blockchain_api import BlockchainApi, Wallet
import logging
from pytest import mark


@mark.xdist_group("faucet_tx")
def test_delegator_can_associate_pc_address(api: BlockchainApi, new_wallet: Wallet, get_wallet: Wallet):
    logging.info("Signing address association...")
    stake_skey, stake_vkey = api.cardano_cli.generate_stake_keys()
    skey_hex = stake_skey["cborHex"][4:]
    vkey_hex = stake_vkey["cborHex"][4:]
    signature = api.sign_address_association(new_wallet.public_key, skey_hex)
    assert signature.partner_chain_address == new_wallet.address
    assert signature.signature, "Signature is empty"
    assert signature.stake_public_key == f"0x{vkey_hex}"

    logging.info("Submitting address association...")
    tx = api.submit_address_association(signature, wallet=get_wallet)
    assert tx.hash, "Could not submit address association"

    logging.info("Verifying address association...")
    vkey_hash = api.cardano_cli.get_stake_key_hash(vkey_hex)
    logging.info(f"Stake public key hash: {vkey_hash}")
    address_association = api.get_address_association(vkey_hash)
    assert address_association == new_wallet.address, "Address association not found"
