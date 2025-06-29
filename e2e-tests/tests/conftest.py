import os
import json
import logging
import subprocess
from omegaconf import OmegaConf
from pytest import fixture, skip, Config, Metafunc, UsageError
from src.log_filter import sensitive_filter
from src.blockchain_api import BlockchainApi, Wallet
from src.blockchain_types import BlockchainTypes
from src.pc_epoch_calculator import PartnerChainEpochCalculator
from src.partner_chain_rpc import PartnerChainRpc
from src.run_command import Runner
from config.api_config import ApiConfig
from sqlalchemy import create_engine
from sqlalchemy.orm import Session
from src.db.models import Base
from filelock import FileLock
from typing import Generator
import time

_config: ApiConfig = None
partner_chain_rpc_api: PartnerChainRpc = None
partner_chain_epoch_calc: PartnerChainEpochCalculator = None


def pytest_addoption(parser):
    parser.addoption("--env", action="store", default="local", help="Target node environment")
    parser.addoption(
        "--blockchain",
        action="store",
        default="substrate",
        help="Blockchain network type",
        choices=(BlockchainTypes._member_map_),
    )
    parser.addoption(
        "--ci-run", action="store_true", help="Overrides config values specific for executing from ci runner"
    )
    parser.addoption("--decrypt", action="store_true", help="Decrypts secrets and keys files")
    parser.addoption("--multisig", action="store_true", help="Use multisig governance authority")

    # command line args that can override config options
    # NOTE: do not add default values so config defaults are used
    parser.addoption("--node-host", action="store", help="Overrides node host")
    parser.addoption("--node-port", action="store", help="Overrides node port")
    parser.addoption("--deployment-mc-epoch", action="store", type=int, help="Deployment main chain epoch.")
    parser.addoption("--init-timestamp", action="store", type=int, help="Initial timestamp of the main chain.")

    # committee tests parametrization
    parser.addoption(
        "--latest-mc-epoch",
        action="store_true",
        help="Parametrize committee tests to verify whole last MC epoch. "
        + "Transforms pc_epoch param to range of SC epochs for last MC epoch. ",
    )
    parser.addoption(
        "--mc-epoch",
        action="store",
        type=int,
        default=None,
        help="MC epoch that parametrizes committee tests, default: <last_mc_epoch>. "
        + "Translates pc_epoch param to range of SC epochs for given MC epoch.",
    )
    parser.addoption(
        "--pc-epoch",
        action="store",
        type=int,
        default=None,
        help="SC epoch that parametrizes committee tests, default: <last_pc_epoch>.",
    )


def pytest_configure(config: Config):
    # Check config options
    latest_mc_epoch = config.getoption("--latest-mc-epoch")
    mc_epoch = config.getoption("--mc-epoch")
    pc_epoch = config.getoption("--pc-epoch")
    if sum([bool(latest_mc_epoch), bool(mc_epoch), bool(pc_epoch)]) > 1:
        raise UsageError("Options --latest-mc-epoch, --mc-epoch, and --pc-epoch are mutually exclusive.")

    # Mask sensitive data in logs
    paramiko_logger = logging.getLogger("paramiko")
    paramiko_logger.setLevel(logging.ERROR)
    logging.getLogger().addFilter(sensitive_filter)

    # Create one log file for each worker
    worker_id = os.environ.get("PYTEST_XDIST_WORKER")
    if worker_id is not None:
        logging.basicConfig(
            format=config.getini("log_file_format"),
            filename=f"logs/debug_{worker_id}.log",
            level=config.getini("log_file_level"),
            datefmt=config.getini("log_file_date_format"),
        )

    # Create objects needed for collection phase
    blockchain = config.getoption("blockchain")
    global _config
    _config = load_config(
        blockchain,
        config.getoption("env"),
        config.getoption("--ci-run"),
        config.getoption("--node-host"),
        config.getoption("--node-port"),
        config.getoption("--deployment-mc-epoch"),
        config.getoption("--init-timestamp"),
    )

    # Set main chain security param to be able to skip tests that would run for too long
    config.security_param = _config.main_chain.security_param

    global partner_chain_rpc_api, partner_chain_epoch_calc
    partner_chain_rpc_api = PartnerChainRpc(_config.nodes_config.node.rpc_url)
    partner_chain_epoch_calc = PartnerChainEpochCalculator(_config)


def pytest_sessionstart(session):
    # set partner chain status on main thread
    if not hasattr(session.config, 'workerinput'):
        session.config.partner_chain_status = partner_chain_rpc_api.partner_chain_get_status().result


def pytest_configure_node(node):
    # set partner chain status on worker threads
    node.workerinput["partner_chain_status"] = node.config.partner_chain_status


def pytest_generate_tests(metafunc: Metafunc):
    if "mc_epoch" in metafunc.fixturenames or "pc_epoch" in metafunc.fixturenames:
        latest_mc_epoch_cmd_line_option = metafunc.config.getoption("--latest-mc-epoch")
        mc_epoch_cmd_line_option = metafunc.config.getoption("--mc-epoch")
        pc_epoch_cmd_line_option = metafunc.config.getoption("--pc-epoch")

        is_worker_thread = getattr(metafunc.config, 'workerinput', False)
        if is_worker_thread:
            current_mc_epoch = is_worker_thread.get("partner_chain_status")["mainchain"]["epoch"]
            current_pc_epoch = is_worker_thread.get("partner_chain_status")["sidechain"]["epoch"]
        else:
            current_mc_epoch = metafunc.config.partner_chain_status["mainchain"]["epoch"]
            current_pc_epoch = metafunc.config.partner_chain_status["sidechain"]["epoch"]

        if not _config.initial_pc_epoch:
            logging.warning("Initial SC epoch is not set in config. Searching via RPC...")
            __set_initial_pc_epoch()
        elif (
            partner_chain_epoch_calc.find_mc_epoch(_config.initial_pc_epoch, current_mc_epoch)
            != _config.deployment_mc_epoch
        ):
            logging.error(
                f"Initial SC epoch {_config.initial_pc_epoch} doesn't belong to MC deployment epoch "
                + f"{_config.deployment_mc_epoch}. Searching via RPC and overwriting..."
            )
            __set_initial_pc_epoch()

        if latest_mc_epoch_cmd_line_option:
            last_mc_epoch = current_mc_epoch - 1
            mc_epoch = last_mc_epoch
            pc_epochs = partner_chain_epoch_calc.find_pc_epochs(last_mc_epoch)
        elif mc_epoch_cmd_line_option:
            mc_epoch = mc_epoch_cmd_line_option
            pc_epochs = partner_chain_epoch_calc.find_pc_epochs(mc_epoch_cmd_line_option)
        elif pc_epoch_cmd_line_option:
            mc_epoch = partner_chain_epoch_calc.find_mc_epoch(pc_epoch_cmd_line_option, current_mc_epoch)
            pc_epochs = [pc_epoch_cmd_line_option]
        else:
            last_mc_epoch = current_mc_epoch - 1
            last_pc_epoch = current_pc_epoch - 1
            mc_epoch = last_mc_epoch
            pc_epochs = [last_pc_epoch]

    if "pc_epoch" in metafunc.fixturenames:
        logging.info(f"Parameterizing {metafunc.definition.name} with SC epochs {pc_epochs}.")
        metafunc.parametrize("pc_epoch", pc_epochs)
    elif "mc_epoch" in metafunc.fixturenames:
        logging.info(f"Parameterizing {metafunc.definition.name} with MC epoch {mc_epoch}.")
        metafunc.parametrize("mc_epoch", [mc_epoch])


def __set_initial_pc_epoch():
    deployment_mc_epoch = _config.deployment_mc_epoch
    pc_epochs = partner_chain_epoch_calc.find_pc_epochs(deployment_mc_epoch, start_from_initial_pc_epoch=False)

    # extend the range by -1:0 in case the initial pc epoch is the first one for given mc epoch
    # to be able to find the transition from "earlier than the Initial Epoch" error into valid response
    pc_epochs_to_search = range(pc_epochs.start - 1, pc_epochs.stop)
    low, high = pc_epochs_to_search.start, pc_epochs_to_search.stop - 1
    while low <= high:
        mid = (low + high) // 2
        response = partner_chain_rpc_api.partner_chain_get_epoch_committee(mid)

        if response.error and "earlier than the Initial Epoch" in response.error.message:
            low = mid + 1  # Search in the later epochs
        else:
            high = mid - 1  # Potential initial SC epoch, check earlier epochs

    if low < pc_epochs_to_search.stop and high >= pc_epochs_to_search.start:
        _config.initial_pc_epoch = low
        logging.info(f"Initial SC epoch set to {_config.initial_pc_epoch}.")
    else:
        _config.initial_pc_epoch = pc_epochs.start
        logging.error(
            f"Initial SC epoch not found. Is deployment_mc_epoch {deployment_mc_epoch} correct? "
            f"Falling back to first SC epoch ({pc_epochs.start}) of MC epoch {deployment_mc_epoch}."
        )


def pytest_make_parametrize_id(val, argname):
    if argname == "mc_epoch":
        return f"mc_epoch:{val}"
    if argname == "pc_epoch":
        return f"pc_epoch:{val}"


def pytest_collection_modifyitems(items):
    for item in items:
        for marker in item.iter_markers(name="test_key"):
            test_key = marker.args[0]
            item.user_properties.append(("test_key", test_key))


def pytest_runtest_logstart(nodeid, location):
    logging.info(f"Starting test: {nodeid}")


def pytest_runtest_logfinish(nodeid, location):
    logging.info(f"Finished test: {nodeid}")


def pytest_runtest_makereport(item, call):
    if call.when == 'call' and item.obj.__doc__:
        item.user_properties.append(('test_summary', item.name))


@fixture(scope="session")
def nodes_env(request):
    return request.config.getoption("--env")


@fixture(scope="session")
def blockchain(request):
    return request.config.getoption("--blockchain")


@fixture(scope="session")
def ci_run(request):
    return request.config.getoption("--ci-run")


@fixture(scope="session")
def decrypt(request):
    return request.config.getoption("--decrypt")


@fixture(scope="session")
def multisig(request):
    return request.config.getoption("--multisig")


def load_config(blockchain, nodes_env, ci_run, node_host, node_port, deployment_mc_epoch, init_timestamp):
    default_config_path = f"{os.getcwd()}/config/config.json"
    assert os.path.isfile(default_config_path), f"Config file not found {default_config_path}"
    default_config = OmegaConf.load(default_config_path)

    blockchain_config_path = f"{os.getcwd()}/config/{blockchain}/{nodes_env}_nodes.json"
    assert os.path.isfile(blockchain_config_path), f"Config file not found {blockchain_config_path}"
    blockchain_config = OmegaConf.load(blockchain_config_path)

    stack_config_path = f"{os.getcwd()}/config/{blockchain}/{nodes_env}_stack.json"
    assert os.path.isfile(stack_config_path), f"Config file not found {stack_config_path}"
    stack_config = OmegaConf.load(stack_config_path)

    schema = OmegaConf.structured(ApiConfig)
    config: ApiConfig = OmegaConf.merge(schema, default_config, blockchain_config, stack_config)

    ci_config_path = f"{os.getcwd()}/config/{blockchain}/{nodes_env}-ci.json"
    if ci_run and os.path.isfile(ci_config_path):
        ci_config = OmegaConf.load(ci_config_path)
        config = OmegaConf.merge(config, ci_config)

    # command line arguments that override config values
    if node_host:
        config.nodes_config.node.host = node_host
    if node_port:
        config.nodes_config.node.port = node_port
    if deployment_mc_epoch:
        config.deployment_mc_epoch = deployment_mc_epoch
    if init_timestamp:
        config.main_chain.init_timestamp = init_timestamp

    # register resolvers for custom interpolations
    # example: ${pc_epochs_in_mc_epoch_count:${..main_chain.epoch_length},${.block_duration},${.slots_in_epoch}}
    OmegaConf.register_new_resolver(
        "pc_epochs_in_mc_epoch_count",
        lambda mc_epoch_length, block_duration, slots_in_epoch: int(mc_epoch_length / block_duration / slots_in_epoch),
    )
    OmegaConf.register_new_resolver(
        "partner_chain_main_cli_network", lambda network: "testnet" if network.startswith("--testnet") else "mainnet"
    )

    return config


@fixture(scope="session")
def config():
    return _config


@fixture(scope="session")
def secrets(blockchain, nodes_env, decrypt, ci_run):
    path = f"{os.getcwd()}/secrets/{blockchain}/{nodes_env}/{nodes_env}.json"
    assert os.path.isfile(path), f"Secrets file not found {path}"
    if decrypt:
        decrypted_data = subprocess.check_output(["sops", "--decrypt", path], encoding="utf-8")
        secrets = OmegaConf.create(json.loads(decrypted_data))
    else:
        secrets = OmegaConf.load(path)

    ci_path = f"{os.getcwd()}/secrets/{blockchain}/{nodes_env}/{nodes_env}-ci.json"
    if ci_run and os.path.isfile(ci_path):
        secrets = secrets_ci(secrets, decrypt, ci_path)

    return secrets


def secrets_ci(secrets, decrypt, ci_path):
    """Override secrets with values specific for ci run."""
    if decrypt:
        decrypted_data = subprocess.check_output(["sops", "--decrypt", ci_path], encoding="utf-8")
        ci_secrets = OmegaConf.create(json.loads(decrypted_data))
    else:
        ci_secrets = OmegaConf.load(ci_path)

    secrets = OmegaConf.merge(secrets, ci_secrets)
    return secrets


@fixture(scope="session", autouse=True)
def decrypt_keys(tmp_path_factory, config, blockchain, nodes_env, decrypt, ci_run):
    if decrypt:
        root_tmp_dir = tmp_path_factory.getbasetemp().parent
        fn = root_tmp_dir / "secrets"
        with FileLock(str(fn) + ".lock"):
            if fn.is_file():
                yield
            else:
                keys_path = config.keys_path or f"secrets/{blockchain}/{nodes_env}/keys"
                # TODO this should use the existence of .decrypted files to determine if decryption is necessary
                #      instead of relying on tmp/secrets file
                subprocess.check_output(
                    [
                        f"find {keys_path} -type f -not -path '*/preprodSPO/*' -not -name '*.decrypted' -exec "
                        f"sh -c \"sops -d '{{}}' > '{{}}.decrypted'\" \;"  # noqa: W605
                    ],
                    shell=True,
                )
                # write secrets lock
                fn.write_text("keys decrypted")
                yield
                subprocess.check_output(
                    ["find secrets -type f -name '*.decrypted' -exec rm {} \;"],  # noqa: W605
                    shell=True,
                )
                # clean up secrets lock
                os.remove(fn)
    else:
        # the yield statement is needed on both if/else sides because that's how the fixture communicates setup is done
        yield


@fixture(scope="session")
def init_db(tmp_path_factory, worker_id, secrets):
    """Creates db engine, and initializes db tables if they don't exist."""
    engine = create_engine(secrets["db"]["url"])

    if worker_id == "master":
        Base.metadata.create_all(engine)
        return engine

    root_tmp_dir = tmp_path_factory.getbasetemp().parent
    fn = root_tmp_dir / "db"
    with FileLock(str(fn) + ".lock"):
        if not fn.is_file():
            Base.metadata.create_all(engine)
            fn.write_text("db tables created")
    return engine


@fixture(scope="session")
def init_db_sync(secrets):
    """Creates db engine to the mainchain database"""
    return create_engine(secrets["dbSync"]["url"])


@fixture(scope="session")
def db(init_db) -> Generator[Session, None, None]:
    with Session(init_db) as session:
        yield session


@fixture(scope="session")
def db_sync(init_db_sync) -> Generator[Session, None, None]:
    with Session(init_db_sync) as session:
        yield session


@fixture(scope="session")
def api(blockchain, config, secrets, db_sync) -> Generator[BlockchainApi, None, None]:
    class_name = BlockchainTypes.__getitem__(blockchain).value
    api: BlockchainApi = class_name(config, secrets, db_sync)
    yield api
    api.close()


@fixture(scope="function", autouse=True)
def teardown(request, api: BlockchainApi):
    """Close api connection after each test to avoid idle connections and BrokenPipeError.
    Skip teardown for test_blocks.py to speed up the execution.
    """
    yield
    if request.node.fspath.basename != "test_blocks.py":
        api.close()


@fixture(scope="session", autouse=True)
def check_mc_sync_progress(api: BlockchainApi, decrypt_keys) -> Wallet:
    logging.info("Checking if cardano node is fully synced")
    sync_progress = api.get_mc_sync_progress()
    if float(sync_progress) != 100.00:
        logging.warning(f"Main chain node is not fully synced yet. Current status: {sync_progress}%")


@fixture(scope="session")
def current_mc_epoch(api: BlockchainApi) -> int:
    epoch = api.get_mc_epoch()
    logging.info(f"Setting current MC epoch {epoch} with session scope.")
    return epoch


@fixture(scope="session")
def current_pc_epoch(api: BlockchainApi) -> int:
    epoch = api.get_pc_epoch()
    logging.info(f"Setting current SC epoch {epoch} with session scope.")
    return epoch


@fixture(scope="session")
def initial_pc_epoch(api: BlockchainApi, config: ApiConfig) -> int:
    initial_pc_epoch = api.get_initial_pc_epoch()
    if not config.initial_pc_epoch:
        logging.info(f"Setting initial SC epoch {initial_pc_epoch}.")
        config.initial_pc_epoch = initial_pc_epoch
    elif config.initial_pc_epoch != initial_pc_epoch:
        logging.error(
            f"Initial epoch in config {config.initial_pc_epoch} doesn't match the actual one {initial_pc_epoch}. "
            "Overriding."
        )
        config.initial_pc_epoch = initial_pc_epoch
    return initial_pc_epoch


@fixture(scope="session")
def pc_epoch_calculator(config: ApiConfig) -> PartnerChainEpochCalculator:
    return PartnerChainEpochCalculator(config)


@fixture
def new_wallet(api: BlockchainApi) -> Wallet:
    return api.new_wallet()


@fixture(scope="session")
def get_wallet(api: BlockchainApi, secrets) -> Wallet:
    faucet = secrets["wallets"]["faucet-0"]
    address = faucet["address"]
    public_key = faucet["public_key"]
    secret = faucet["secret_seed"]
    scheme = faucet["scheme"]
    return api.get_wallet(address=address, public_key=public_key, secret=secret, scheme=scheme)

@fixture(scope="session")
def genesis_utxo(api: BlockchainApi):
	return api.get_params()["genesis_utxo"]

@fixture(scope="session")
def get_scripts(genesis_utxo, api: BlockchainApi):
    return api.partner_chains_node.smart_contracts.get_scripts(genesis_utxo).json


@fixture(scope="session")
def addresses(get_scripts):
    return get_scripts["addresses"]


@fixture(scope="session")
def policy_ids(get_scripts):
    return get_scripts["policyIds"]


@fixture(scope="session")
def full_mc_epoch_has_passed_since_deployment(config: ApiConfig, current_mc_epoch):
    logging.info("Checking if full MC epoch has elapsed since deployment")
    if current_mc_epoch < config.deployment_mc_epoch + 2:
        return False
    return True


@fixture(autouse=True)
def skip_on_new_chain(request, full_mc_epoch_has_passed_since_deployment):
    skip_marker = request.node.get_closest_marker("skip_on_new_chain")
    if skip_marker and not full_mc_epoch_has_passed_since_deployment:
        skip("Test requires at least one full MC epoch that has passed in order to verify data for the past epoch.")


@fixture(scope="session")
def wait_until():
    """Generic wait function until <condition> is True.

    Arguments:
        condition {function} -- function name or lambda, e.g. lambda x: x + 1 == 2, x = 1
        args {Any} -- position args used by <condition>

    Keyword Arguments:
        timeout {int} -- timeout in seconds (default: {20})
        poll_interval {int} -- poll interval in seconds (default: {3})

    Returns:
        Any -- returns <condition> result, None if timed out.
    """

    def _wait_until(condition, *args, timeout=20, poll_interval=3):
        start = time.time()
        logging.info(f"WAIT UNTIL: {condition}. TIMEOUT: {timeout}, POLL_INTERVAL: {poll_interval}")
        while time.time() - start < timeout:
            result = condition(*args)
            if result:
                return result
            time.sleep(poll_interval)
        raise TimeoutError(f"WAIT UNTIL function TIMED OUT after {timeout}s on {condition} with args {args}.")

    yield _wait_until


@fixture(scope="session")
def write_file():
    """Write a file in a location that is available by tools used in tests, e.g. node.

    Example usage:
    ```python
    def test_something(api: BlockchainApi, write_file):
        content = {"keyHash": <key_hash>, "type": "sig"}
        filepath = write_file(api.cardano_cli.run_command , content)
        policy_id = api.cardano_cli.get_policy_id(filepath)
        assert policy_id
    ```

    The file is created using `mktemp` command, which creates a temporary file under /tmp/tmp.XXXXXXX, unless
    the tool.runner.workdir is set in the `<env>_stack.json` file, in which case it creates a temporary file in that
    directory.
    The content is passed as a string and is converted to JSON format, unless `is_json` is set to False.
    The file is created on the same host that is configured in the `<env>_stack.json` for given tool.
    The file is removed after the test completes.


    Returns:
        str: filepath available to use by CLI

    Yields:
        function: write_file callable function that takes runner and content as arguments
    """
    runners: list[Runner] = []

    def _write_file(runner: Runner, content: str, is_json: bool = True) -> str:
        temp_file = runner.mktemp()
        if is_json:
            content = json.dumps(content)
            content = content.replace('"', '\\"')
        runner.exec(f"echo '{content}' > {temp_file}")

        if runner not in runners:
            runners.append(runner)
        return temp_file

    yield _write_file

    for runner in runners:
        logging.info(f"Cleaning up temporary files created by {runner.__class__.__name__}...")
        runner.cleanup()


@fixture(scope="session")
def governance_skey_with_cli(config: ApiConfig, api: BlockchainApi, write_file):
    """
    Securely copy the governance authority's init skey (a secret key used by the smart-contracts to authorize admin
    operations) to a temporary file on the remote machine and update the path in the configuration.
    The temporary file is deleted after the test completes.

    This fixture is executed only if:
    - you call it directly in test or other fixture
    - tools.node.runner.files.copy_secrets is set to true in the config file `<env>_stack.json`

    WARNING: This fixture copies secret file to a remote host and should be used with caution.

    :param config: The API configuration object.
    """
    if config.stack_config.tools.node.runner.copy_secrets:
        runner = api.partner_chains_node.run_command
        path = config.nodes_config.governance_authority.mainchain_key
        with open(path, "r") as f:
            content = json.load(f)
            filepath = write_file(runner, content)
        config.nodes_config.governance_authority.mainchain_key = filepath
        yield
        config.nodes_config.governance_authority.mainchain_key = path
    else:
        yield


@fixture(scope="session")
def governance_authority(config: ApiConfig):
    return config.nodes_config.governance_authority


@fixture(scope="session")
def additional_governance_authorities(config: ApiConfig):
    return config.nodes_config.additional_governance_authorities


@fixture(scope="session", autouse=True)
def set_governance_to_multisig(multisig, api: BlockchainApi, genesis_utxo, governance_authority, additional_governance_authorities):
    if not multisig:
        yield
        return

    if not additional_governance_authorities:
        raise UsageError("--multisig requires additional governance authorities to be set in the config.")

    logging.info("Updating governance to use multiple authorities for the session...")
    all_authorities = [governance_authority.mainchain_pub_key_hash] + list(
        map(lambda x: x.mainchain_pub_key_hash, additional_governance_authorities)
    )
    threshold = 2

    response = api.partner_chains_node.smart_contracts.governance.update(
        genesis_utxo,
        payment_key=governance_authority.mainchain_key,
        new_governance_authorities=all_authorities,
        new_governance_threshold=threshold,
    )

    yield response

    response = api.partner_chains_node.smart_contracts.governance.update(
        genesis_utxo,
        payment_key=governance_authority.mainchain_key,
        new_governance_authorities=[governance_authority.mainchain_pub_key_hash],
        new_governance_threshold=1,
    )

    assert response.returncode == 0
    logging.info("Governance restored to single key successfully")
