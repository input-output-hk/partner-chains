# Ariadne Simulator

This is a tool meant for Partner Chain builders and governance authorities to run
simulation of Ariadne commitee selection in order to select best-performing values
of the D-Parameter for their actual set of permissioned and registered candidates,
as well as predict general security characteristics of the algorithm.

## Usage

The tool exposes two commands that simulate Ariadne:
- [simulate]: outputs selected committees as JSON arrays
- [analyze]: calculates various statistics for each selected committee and outputs
             them as CSV data

Both commands expect to receive as arguments JSON files containing information about
committee member candidates. For registered candidates the format is a list of objects
containing fields `key` and `stake`, eg.:
```json
[
  {
     "key": "registered-1",
     "stake": 134664494512628
  },
  {
     "key": "registered-2",
     "stake": 76499924001653
  },
  {
     "key": "registered-3",
     "stake": 75953756970290
  }
]
```
For permissioned candidates only the `key` field is expected, eg.:
```json
[
  { "key": "permissioned-0" },
  { "key": "permissioned-1" },
  { "key": "permissioned-2" }
]
```

### Analyze

This command runs a number of Ariadne selections using provided input data and outputs
the selected committees as a stream of JSON arrays of candidate IDs that can be piped
into `jq`.

Example of usage:

``` shell
./ariadne-simulator simulate \
    -r ariadne-liveness/data/stake-sorted.json \
    -p ariadne-liveness/data/permissioned-1000.json \
    -P 10 -R 10 \
    --repetitions 10 \
| jq
```

### Analyze

This command runs a number of Ariadne selections and creates a CSV file containing a
range of useful statistics about each selected committees. The output file is named
`ariadne-simulation-<timestamp>.csv`.

Example of use:

``` shell
./ariadne-simulator analyze \
    -P 7 -R 3 \
    --repetitions 1000 \
    -p ariadne-liveness/data/permissioned-1000.json \
    -r ariadne-liveness/data/stake-sorted.json
```

The following statistics are calculated for each committee:
- `total_registered_stake`: total stake of the registered candidate pool
- `total_committee_stake`: total stake of all selected committee members
- `distinct_members`: number of unique members in the committee
- `max_single_member_seats`: highest number of committee seats occupied by the same member
- `safe_offline_members`: highest number of members that can be offline without affecting
                          the consensus. This number is calculated by greedily taking members
                          with highest stake until more than 33% of seats are offline.
- `top_safe_offline_stake`: total stake of top stake candidates that can be offline without
                          affecting the consensus
- `bottom_safe_offline_stake`: total stake of lowest stake candidates that can be offline
                          without affecting the consensus
