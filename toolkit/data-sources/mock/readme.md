# Main chain follower mock

This module provides a mock for main chain follower.

It serves a set of hardcoded incoming transactions and configurable candidates data.
Other functionality is not provided yet.

To use this mock instead of the real main chain follower, please set env variable `USE_MOCK_DATA_SOURCES=true`.
Then `MOCK_REGISTRATIONS_FILE` env variable should contain the path to the candidates data file.
[Example is provided in this repository](../../res/bb-mock/default-registrations.json).
This file should contain a JSON array, which every item is an object, that contains all the data required to choose committee.
If there are N items in the list, the item with index `epoch_number mod N` defines the response for the requests for the `epoch_number`.
