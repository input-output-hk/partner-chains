from pytest import mark, skip
from src.blockchain_api import BlockchainApi


@mark.rpc
class TestRpc:
    @mark.test_key('ETCM-7445')
    def test_get_ariadne_parameters(self, api: BlockchainApi):
        """Test sidechain_getAriadneParameters() returns data about d-parameter and candidates

        * execute sidechain_getAriadneParameters() API call for latest finished epoch
        * check that the response data has expected elements
        """
        mc_epoch = api.get_mc_epoch()
        ariadne_parameters = api.get_ariadne_parameters(mc_epoch)

        assert ariadne_parameters["dParameter"]["numPermissionedCandidates"]
        assert ariadne_parameters["dParameter"]["numRegisteredCandidates"]

        assert 'permissionedCandidates' in ariadne_parameters
        permissioned_candidates = ariadne_parameters["permissionedCandidates"]
        assert isinstance(permissioned_candidates, list)

        if permissioned_candidates:
            for candidate in permissioned_candidates:
                assert candidate["sidechainPublicKey"] is not None
                assert candidate["auraPublicKey"] is not None
                assert candidate["grandpaPublicKey"] is not None

        assert 'candidateRegistrations' in ariadne_parameters
        trustless_registrations = ariadne_parameters["candidateRegistrations"]
        assert isinstance(trustless_registrations, dict)

        if trustless_registrations:
            for entry in trustless_registrations:
                assert entry
                check_registration_data(trustless_registrations[entry])

    @mark.test_key('ETCM-7446')
    @mark.ariadne
    def test_get_epoch_committee(self, api: BlockchainApi):
        """Test sidechain_getEpochCommittee() returns committee members for a given sidechain epoch

        * get pc_epoch number from sidechain_getStatus()
        * execute sidechain_getEpochCommittee() for a latest pc_epoch
        * check that response contains committee members and sidechain epoch
        """
        epoch_number = api.get_status()["sidechain"]["epoch"]
        committee_response = api.get_epoch_committee(epoch_number).result
        assert (
            committee_response["sidechainEpoch"] == epoch_number
        ), "Epoch number mismatch at sidechain_getEpochCommittee()"
        assert committee_response["committee"], f"No committee members found for {epoch_number} epoch"
        assert all(member["sidechainPubKey"] is not None for member in committee_response["committee"])

    @mark.test_key('ETCM-7448')
    def test_get_registrations(self, api: BlockchainApi):
        """Test sidechain_getRegistrations() returns registration data for a given mainchain key and mc_epoch

        * get mc_epoch number from sidechain_getStatus()
        * get mainchain public key from sidechain_getAriadneParameters()
        * execute sidechain_getRegistrations() for a given mc_epoch and key
        * check that the response data has expected elements
        """
        mc_epoch = api.get_mc_epoch()
        mainchain_key = next(iter(api.get_ariadne_parameters(mc_epoch)["candidateRegistrations"]))
        registrations = api.get_registrations(mc_epoch, mainchain_key)

        assert isinstance(registrations, list)
        if len(registrations) > 0:
            check_registration_data(registrations)
        else:
            skip("No registrations found for {mc_epoch} epoch")


def check_registration_data(registrations):
    assert registrations[0]["sidechainPubKey"] is not None
    assert registrations[0]["sidechainAccountId"] is not None
    assert registrations[0]["mainchainPubKey"] is not None
    assert registrations[0]["crossChainPubKey"] is not None
    assert registrations[0]["auraPubKey"] is not None
    assert registrations[0]["grandpaPubKey"] is not None
    assert registrations[0]["sidechainSignature"] is not None
    assert registrations[0]["mainchainSignature"] is not None
    assert registrations[0]["crossChainSignature"] is not None
    assert isinstance(registrations[0]["isValid"], bool)
    assert isinstance(registrations[0]["stakeDelegation"], int)
    assert registrations[0]["utxo"]["utxoId"] is not None
    assert isinstance(registrations[0]["utxo"]["epochNumber"], int)
    assert isinstance(registrations[0]["utxo"]["blockNumber"], int)
    assert isinstance(registrations[0]["utxo"]["slotNumber"], int)
    assert isinstance(registrations[0]["utxo"]["txIndexWithinBlock"], int)
