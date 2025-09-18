from config.api_config import ApiConfig
from src.blockchain_api import BlockchainApi

class TestBridge:

    def test_lock_without_spending_ics_utxo(self, api: BlockchainApi, genesis_utxo, payment_key, reserve_asset_id, node_1_aura_pub_key, wait_until, config: ApiConfig):
        amount = 1
        balance_before = api.get_pc_balance(node_1_aura_pub_key)
        assert balance_before > 0, "Test account is not over existential limit"
        api.partner_chains_node.smart_contracts.bridge(genesis_utxo, amount, node_1_aura_pub_key, payment_key, spend_ics_utxo = False)
        wait_until(
            lambda: api.get_pc_balance(node_1_aura_pub_key) == balance_before + amount,
            timeout=config.timeouts.main_chain_tx * config.main_chain.security_param * 2,
        )

    def test_lock_with_spending_ics_utxo(self, api: BlockchainApi, genesis_utxo, payment_key, reserve_asset_id, node_1_aura_pub_key, wait_until, config: ApiConfig):
        amount = 1
        balance_before = api.get_pc_balance(node_1_aura_pub_key)
        assert balance_before > 0, "Test account is not over existential limit"
        api.partner_chains_node.smart_contracts.bridge(genesis_utxo, amount, node_1_aura_pub_key, payment_key, spend_ics_utxo = True)
        wait_until(
            lambda: api.get_pc_balance(node_1_aura_pub_key) == balance_before + amount,
            timeout=config.timeouts.main_chain_tx * config.main_chain.security_param * 2,
        )
