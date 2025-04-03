import pytest


def pytest_collection_modifyitems(items):
    for item in items:
        # mark all tests in `tests/delegator_rewards` directory with `delegator_rewards` marker
        if "tests/delegator_rewards" in item.nodeid:
            item.add_marker(pytest.mark.delegator_rewards)
