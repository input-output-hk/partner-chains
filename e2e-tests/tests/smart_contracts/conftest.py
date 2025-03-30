import os
from pytest import fixture


@fixture(scope="session", autouse=True)
def setup_test_environment(request):
    """Setup additional test environment variables needed for multisig tests"""
    # Mark the multisig_governance tests for execution
    os.environ["PYTEST_ADDOPTS"] = os.environ.get("PYTEST_ADDOPTS", "") + " -m multisig_governance"

    # Setting xdist_group_order to ensure governance_cleanup runs last
    os.environ["PYTEST_XDIST_GROUP_ORDER"] = "governance_setup,multisig_operation,governance_cleanup"

    def cleanup():
        """Cleanup any test-specific environment variables when done"""
        if "PYTEST_XDIST_GROUP_ORDER" in os.environ:
            del os.environ["PYTEST_XDIST_GROUP_ORDER"]

    request.addfinalizer(cleanup)