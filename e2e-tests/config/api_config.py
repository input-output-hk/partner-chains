from dataclasses import dataclass
from omegaconf import MISSING, SI
from typing import Optional
from src.partner_chain_rpc import DParam


@dataclass
class Timeout:
    long_running_function: int = MISSING
    register_cmd: int = MISSING
    deregister_cmd: int = MISSING
    main_chain_tx: int = MISSING


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


@dataclass
class MainChainConfig:
    network: str = MISSING
    epoch_length: int = MISSING
    slot_length: int = MISSING
    active_slots_coeff: float = MISSING
    security_param: int = MISSING
    init_timestamp: int = MISSING
    block_stability_margin: int = MISSING


@dataclass
class MainchainAccount:
    mainchain_address: str
    mainchain_key: str
    mainchain_pub_key: str
    mainchain_pub_key_hash: str


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
    pod: Optional[str] = None
    namespace: Optional[str] = None
    container: Optional[str] = None


@dataclass
class Reserve:
    token_name: str = MISSING
    v_function_script_path: str = MISSING
    v_function_updated_script_path: str = MISSING


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
    governance_authority: MainchainAccount = MISSING
    additional_governance_authorities: Optional[list[MainchainAccount]] = None
    reserve: Optional[Reserve] = None
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
    keys_path: Optional[str] = None
    poll_intervals: PollInterval = MISSING
    nodes_config: NodesApiConfig = MISSING
    stack_config: StackApiConfig = MISSING
    deployment_mc_epoch: int = MISSING
    init_timestamp: int = MISSING
    initial_pc_epoch: Optional[int] = None
