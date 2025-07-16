from pytest import mark
from config.api_config import ApiConfig
from src.blockchain_api import BlockchainApi
from src.partner_chains_node.models import VFunction


pytestmark = [mark.xdist_group(name="governance_action")]


def test_enough_funds_for_minting(transaction_input):
    assert transaction_input(), "Not enough funds to mint token"


def test_mint_tokens_for_reserve(
    api: BlockchainApi, governance_address: str, reserve_asset_id, mint_token, wait_until, config: ApiConfig
):
    initial_balance = api.get_mc_balance(governance_address, reserve_asset_id)
    tokens_to_mint = 1000
    result = mint_token(tokens_to_mint)
    assert "txhash" in result
    assert wait_until(
        lambda: api.get_mc_balance(governance_address, reserve_asset_id) == initial_balance + tokens_to_mint,
        timeout=config.timeouts.main_chain_tx,
    )


def test_enough_funds_for_tx_with_reference_script(transaction_input):
    assert transaction_input(), "Not enough funds to pay for transaction with reference script"


def test_attach_v_function_as_reference_script(v_function: VFunction):
    assert v_function.reference_utxo, "V-function reference UTXO is not set"
