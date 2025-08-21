import logging
import random
from config.api_config import ApiConfig
from src.blockchain_api import BlockchainApi
from src.partner_chains_node.models import VFunction
from pytest import fixture, mark, skip


pytestmark = [mark.xdist_group(name="governance_action")]

INITIAL_RESERVE_DEPOSIT = 1000


@fixture(scope="module", autouse=True)
def init_reserve(api: BlockchainApi, genesis_utxo, payment_key):
    response = api.partner_chains_node.smart_contracts.reserve.init(genesis_utxo, payment_key)
    return response


@fixture(scope="module", autouse=True)
def native_token_initial_balance(
    api: BlockchainApi, governance_address, reserve_asset_id, mint_token, wait_until, config: ApiConfig
):
    balance = api.get_mc_balance(governance_address, reserve_asset_id)
    if balance < INITIAL_RESERVE_DEPOSIT:
        mint_token(INITIAL_RESERVE_DEPOSIT)
        wait_until(
            lambda: api.get_mc_balance(governance_address, reserve_asset_id) == balance + INITIAL_RESERVE_DEPOSIT,
            timeout=config.timeouts.main_chain_tx,
        )
        balance = balance + INITIAL_RESERVE_DEPOSIT
    logging.info(f"Native token initial balance: {balance}")
    return balance


@fixture(scope="module")
def create_reserve(api: BlockchainApi, reserve, genesis_utxo, payment_key):
    response = api.partner_chains_node.smart_contracts.reserve.create(
        genesis_utxo,
        v_function_hash=reserve.v_function.script_hash,
        initial_deposit=INITIAL_RESERVE_DEPOSIT,
        token=reserve.token,
        payment_key=payment_key,
    )
    logging.info(f"Reserve created with initial deposit of {INITIAL_RESERVE_DEPOSIT} tokens")
    yield response
    logging.info("Cleaning up reserve (handover)...")
    api.partner_chains_node.smart_contracts.reserve.handover(genesis_utxo, payment_key)


@fixture(scope="class")
def reserve_initial_balance(create_reserve, api: BlockchainApi, addresses, reserve_asset_id):
    balance = api.get_mc_balance(addresses["ReserveValidator"], reserve_asset_id)
    return balance


@fixture(scope="class")
def circulation_supply_initial_balance(create_reserve, api: BlockchainApi, reserve_asset_id, addresses):
    circulation_balance = api.get_mc_balance(addresses["IlliquidCirculationSupplyValidator"], reserve_asset_id)
    return circulation_balance


class TestInitReserve:
    def test_init_reserve(self, init_reserve):
        response = init_reserve
        assert response.returncode == 0
        if response.json == []:
            skip("Reserve already initialized")


class TestCreateReserve:
    def test_enough_tokens_to_create_reserve(self, native_token_initial_balance):
        assert native_token_initial_balance >= INITIAL_RESERVE_DEPOSIT

    def test_create_reserve(self, create_reserve):
        response = create_reserve
        assert response.returncode == 0
        assert response.json

    @mark.usefixtures("create_reserve")
    def test_native_token_balance_is_smaller_by_initial_deposit(
        self, native_token_initial_balance, api: BlockchainApi, reserve_asset_id, governance_address
    ):
        native_token_current_balance = api.get_mc_balance(governance_address, reserve_asset_id)
        assert native_token_initial_balance - INITIAL_RESERVE_DEPOSIT == native_token_current_balance

    @mark.usefixtures("create_reserve")
    def test_reserve_balance_is_equal_to_initial_deposit(self, api: BlockchainApi, reserve_asset_id, addresses):
        reserve_balance = api.get_mc_balance(addresses["ReserveValidator"], reserve_asset_id)
        assert INITIAL_RESERVE_DEPOSIT == reserve_balance


class TestReleaseFunds:
    @fixture(scope="class")
    def amount_to_release(self, reserve_initial_balance):
        return random.randint(1, min(reserve_initial_balance, 100))

    @fixture(scope="class", autouse=True)
    def release_funds(
        self,
        amount_to_release,
        circulation_supply_initial_balance,
        api: BlockchainApi,
        v_function: VFunction,
        genesis_utxo,
        payment_key,
    ):
        logging.info(f"Releasing {amount_to_release} tokens from reserve...")
        response = api.partner_chains_node.smart_contracts.reserve.release(
            genesis_utxo,
            reference_utxo=v_function.reference_utxo,
            amount=amount_to_release,
            payment_key=payment_key
        )
        return response

    def test_release_funds(self, release_funds):
        response = release_funds
        assert response.returncode == 0
        assert response.json

    def test_circulation_supply_balance_after_release(
        self,
        circulation_supply_initial_balance,
        amount_to_release,
        api: BlockchainApi,
        reserve_asset_id,
        addresses,
    ):
        circulation = api.get_mc_balance(addresses["IlliquidCirculationSupplyValidator"], reserve_asset_id)
        assert circulation_supply_initial_balance + amount_to_release == circulation

    def test_reserve_balance_after_release(
        self, reserve_initial_balance, amount_to_release, api: BlockchainApi, reserve_asset_id, addresses
    ):
        reserve_balance = api.get_mc_balance(addresses["ReserveValidator"], reserve_asset_id)
        assert reserve_initial_balance - amount_to_release == reserve_balance

    def test_observe_released_funds(self, api: BlockchainApi, amount_to_release):
        observed_transfer = api.subscribe_token_transfer()
        assert observed_transfer == amount_to_release


class TestDepositFunds:
    @fixture(scope="class")
    def native_token_balance(self, api: BlockchainApi, governance_address, reserve_asset_id):
        balance = api.get_mc_balance(governance_address, reserve_asset_id)
        logging.info(f"Native token balance: {balance}")
        return balance

    @fixture(scope="class")
    def amount_to_deposit(self, reserve_initial_balance):
        return random.randint(1, min(reserve_initial_balance, 100))

    @fixture(scope="class", autouse=True)
    def deposit_funds(self, native_token_balance, amount_to_deposit, api: BlockchainApi, genesis_utxo, payment_key):
        response = api.partner_chains_node.smart_contracts.reserve.deposit(
            genesis_utxo,
            amount=amount_to_deposit,
            payment_key=payment_key
        )
        return response

    def test_deposit_funds(self, deposit_funds):
        response = deposit_funds
        assert response.returncode == 0
        assert response.json

    def test_reserve_balance_after_deposit(
        self, reserve_initial_balance, amount_to_deposit, api: BlockchainApi, reserve_asset_id, addresses
    ):
        reserve_balance = api.get_mc_balance(addresses["ReserveValidator"], reserve_asset_id)
        assert reserve_initial_balance + amount_to_deposit == reserve_balance

    def test_native_token_balance_after_deposit(
        self,
        native_token_balance,
        amount_to_deposit,
        api: BlockchainApi,
        reserve_asset_id,
        governance_address,
    ):
        native_token = api.get_mc_balance(governance_address, reserve_asset_id)
        assert native_token_balance - amount_to_deposit == native_token


class TestUpdateVFunction:
    @fixture(scope="class")
    def new_v_function(self, v_function_factory, config: ApiConfig):
        v_function_path = config.nodes_config.reserve.v_function_updated_script_path
        v_function = v_function_factory(v_function_path)
        return v_function

    @fixture(scope="class")
    def update_v_function(self, create_reserve, new_v_function: VFunction, api: BlockchainApi, genesis_utxo, payment_key):
        response = api.partner_chains_node.smart_contracts.reserve.update_settings(
            genesis_utxo,
            v_function_hash=new_v_function.script_hash,
            payment_key=payment_key
        )
        return response

    def test_update_v_function(self, update_v_function):
        response = update_v_function
        assert response.returncode == 0
        assert response.json

    def test_release_funds_with_updated_v_function(self, api: BlockchainApi, new_v_function: VFunction, genesis_utxo, payment_key, update_v_function):
        assert update_v_function.returncode == 0, f"Update V-function failed: {update_v_function.stderr}"
        response = api.partner_chains_node.smart_contracts.reserve.release(
            genesis_utxo,
            reference_utxo=new_v_function.reference_utxo,
            amount=1,
            payment_key=payment_key
        )
        assert response.returncode == 0
        assert response.json

    def test_release_funds_with_old_v_function(self, api: BlockchainApi, v_function: VFunction, genesis_utxo, payment_key, update_v_function):
        assert update_v_function.returncode == 0, f"Update V-function failed: {update_v_function.stderr}"
        response = api.partner_chains_node.smart_contracts.reserve.release(
            genesis_utxo,
            reference_utxo=v_function.reference_utxo,
            amount=1,
            payment_key=payment_key
        )
        assert response.returncode == 1
        assert "Error" in response.stderr


class TestHandoverReserve:
    @fixture(scope="class")
    def handover_reserve(self, create_reserve, api: BlockchainApi, genesis_utxo, payment_key):
        response = api.partner_chains_node.smart_contracts.reserve.handover(genesis_utxo, payment_key)
        return response

    def test_handover_reserve(self, handover_reserve):
        response = handover_reserve
        assert response.returncode == 0
        assert response.json

    def test_reserve_balance_after_handover(self, api: BlockchainApi, reserve_asset_id, addresses, handover_reserve):
        assert handover_reserve.returncode == 0, f"Handover failed: {handover_reserve.stderr}"
        reserve_balance = api.get_mc_balance(addresses["ReserveValidator"], reserve_asset_id)
        assert reserve_balance == 0

    def test_circulation_supply_balance_after_handover(
        self,
        circulation_supply_initial_balance,
        reserve_initial_balance,
        api: BlockchainApi,
        reserve_asset_id,
        addresses,
        handover_reserve,
    ):
        assert handover_reserve.returncode == 0, f"Handover failed: {handover_reserve.stderr}"
        circulation = api.get_mc_balance(addresses["IlliquidCirculationSupplyValidator"], reserve_asset_id)
        assert circulation_supply_initial_balance + reserve_initial_balance == circulation
