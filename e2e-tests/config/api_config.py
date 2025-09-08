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
class JolteonConfig:
    """Jolteon consensus-specific configuration parameters"""
    round_progression_multiplier: int = 5  # Multiplier for block_duration when waiting for round progression
    qc_advancement_multiplier: int = 7     # Multiplier for block_duration when waiting for QC advancement
    safety_monitoring_multiplier: int = 10 # Multiplier for block_duration for safety monitoring
    liveness_monitoring_multiplier: int = 20 # Multiplier for block_duration for liveness monitoring
    check_interval_multiplier: int = 2     # Multiplier for block_duration for check intervals
    commit_latency_threshold: int = 30     # Maximum acceptable commit latency in seconds
    min_vote_count_threshold: int = 0      # Minimum vote count for non-initial rounds


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
class KubernetesConfig:
    pod: str = MISSING
    namespace: str = MISSING
    container: str = MISSING


@dataclass
class DockerConfig:
    container: str = MISSING


@dataclass
class RunnerConfig:
    copy_secrets: bool = False
    workdir: Optional[str] = None
    docker: Optional[DockerConfig] = None
    kubernetes: Optional[KubernetesConfig] = None


@dataclass
class Tool:
    path: str = MISSING
    runner: RunnerConfig = SI("${..runner}")


@dataclass
class Tools:
    runner: RunnerConfig = MISSING
    cardano_cli: Tool = MISSING
    node: Tool = MISSING


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
    tools: Tools = MISSING


@dataclass
class ApiConfig:
    committee_epoch_slippage: int = MISSING
    committee_participation_tolerance: float = MISSING
    max_validators: int = MISSING
    deployment_version: str = MISSING
    test_environment: str = MISSING
    main_chain: MainChainConfig = MISSING
    timeouts: Timeout = MISSING
    keys_path: Optional[str] = None
    poll_intervals: PollInterval = MISSING
    jolteon_config: JolteonConfig = MISSING
    nodes_config: NodesApiConfig = MISSING
    stack_config: StackApiConfig = MISSING
    deployment_mc_epoch: int = MISSING
    init_timestamp: int = MISSING
    initial_pc_epoch: Optional[int] = None
