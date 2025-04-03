# Partner Chains Mock Data Sources

This module provides mocks of data sources required by Partner Chains features.

It serves a set of hardcoded incoming transactions and configurable candidates data.
Other functionality is not provided yet.

When using the mocks, `MOCK_REGISTRATIONS_FILE` env variable should contain the path to the candidates data file.
[Example is provided in this repository](../../res/bb-mock/default-registrations.json).
This file should contain a JSON array, which every item is an object, that contains all the data required to choose committee.
If there are N items in the list, the item with index `epoch_number mod N` defines the response for the requests for the `epoch_number`.
