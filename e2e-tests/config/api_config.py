from dataclasses import dataclass
from omegaconf import MISSING, SI
from typing import Optional
from src.partner_chain_rpc import DParam


@dataclass
class Timeout:
    long_running_function: int = MISSING
    register_cmd: int = MISSING
    deregister_cmd: int = MISSING
    burn_cmd: int = MISSING
    burn_tx_visible_in_pc_rpc: int = MISSING
    claim_cmd: int = MISSING


@dataclass
class PollInterval:
    transaction_finalization: int = MISSING


@dataclass
class KeysFiles:
    cardano_payment_key: str = MISSING
    spo_signing_key: str = MISSING
    spo_public_key: str = MISSING
    partner_chain_signing_key: str = MISSING


@dataclass
class Node:
    scheme: str = SI("${...default_scheme}")
    host: str = MISSING
    port: int = SI("${...default_port}")
    url: str = SI("${.scheme}://${.host}:${.port}")
    rpc_scheme: str = SI("${...default_rpc_scheme}")
    rpc_url: str = SI("${.rpc_scheme}://${.host}:${.port}")
    aura_ss58_address: str = MISSING
    pool_id: str = MISSING
    public_key: str = MISSING
    aura_public_key: str = MISSING
    grandpa_public_key: str = MISSING
    rotation_candidate: bool = False
    permissioned_candidate: bool = False
    cardano_payment_addr: str = MISSING
    keys_files: Optional[KeysFiles] = None
    block_rewards_id: str = MISSING


@dataclass
class NativeToken:
    total_accrued_function_script_hash: str = MISSING
    total_accrued_function_address: str = MISSING
    policy_id: str = MISSING
    asset_name: str = MISSING
    token: str = SI("${.policy_id}.${.asset_name}")


@dataclass
class MainChainConfig:
    network: str = MISSING
    epoch_length: int = MISSING
    slot_length: int = MISSING
    active_slots_coeff: float = MISSING
    security_param: int = MISSING
    init_timestamp: int = MISSING
    block_stability_margin: int = MISSING
    native_token: Optional[NativeToken] = None


@dataclass
class MainchainAccount:
    mainchain_address: str
    mainchain_key: str


@dataclass
class TransferAccount:
    mainchain_address: str
    mainchain_key: str
    mainchain_address_bech32: str = MISSING
    partner_chain_address: str = MISSING
    partner_chain_key: str = MISSING


@dataclass
class SSH:
    username: str = MISSING
    host: str = MISSING
    port: int = MISSING
    host_keys_path: Optional[str] = None
    private_key_path: Optional[str] = None


@dataclass
class Tool:
    cli: str = MISSING
    ssh: Optional[SSH] = None
    shell: Optional[str] = SI("${...tools_shell}")


@dataclass
class TxTypeFee:
    send: int = MISSING
    lock: int = MISSING


@dataclass
class Fees:
    ECDSA: TxTypeFee = MISSING
    SR25519: TxTypeFee = MISSING
    ED25519: TxTypeFee = MISSING


@dataclass
class NodesApiConfig:
    default_scheme: str = MISSING
    default_rpc_scheme: str = MISSING
    default_port: int = MISSING
    nodes: dict[str, Node] = MISSING
    block_duration: int = MISSING
    slots_in_epoch: int = MISSING
    pc_epochs_in_mc_epoch_count: int = SI(
        "${pc_epochs_in_mc_epoch_count:${..main_chain.epoch_length},${.block_duration},${.slots_in_epoch}}"
    )
    token_conversion_rate: int = MISSING
    selected_node: str = MISSING
    node: Node = MISSING
    token_policy_id: str = MISSING
    d_param_min: Optional[DParam] = None
    d_param_max: Optional[DParam] = None
    active_transfer_account: TransferAccount = MISSING
    passive_transfer_account: TransferAccount = MISSING
    negative_test_transfer_account: TransferAccount = MISSING
    random_mc_account: MainchainAccount = MISSING
    invalid_mc_skey: MainchainAccount = MISSING
    governance_authority: MainchainAccount = MISSING
    fees: Fees = MISSING
    network: str = SI("${partner_chain_main_cli_network:${..main_chain.network}}")


@dataclass
class StackApiConfig:
    ogmios_scheme: str = "http"
    ogmios_host: str = MISSING
    ogmios_port: int = MISSING
    ogmios_url: str = SI("${.ogmios_scheme}://${.ogmios_host}:${.ogmios_port}")
    tools: dict[str, Tool] = MISSING
    tools_host: str = MISSING
    tools_shell: Optional[str] = None
    ssh: Optional[SSH] = None


@dataclass
class ApiConfig:
    genesis_utxo: str = MISSING
    atms_kind: str = MISSING
    committee_epoch_slippage: int = MISSING
    committee_participation_tolerance: float = MISSING
    max_validators: int = MISSING
    deployment_version: str = MISSING
    test_environment: str = MISSING
    main_chain: MainChainConfig = MISSING
    timeouts: Timeout = MISSING
    poll_intervals: PollInterval = MISSING
    nodes_config: NodesApiConfig = MISSING
    stack_config: StackApiConfig = MISSING
    deployment_mc_epoch: int = MISSING
    init_timestamp: int = MISSING
    initial_pc_epoch: Optional[int] = None
    block_encoding_suffix_grandpa: str = MISSING
    block_encoding_suffix_aura: str = MISSING
