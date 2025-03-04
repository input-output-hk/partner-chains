mod header_verification {
	use crate::Header;
	use hex_literal::hex;
	use parity_scale_codec::Encode;
	use sp_consensus_aura::sr25519::AuthorityId as AuraId;
	use sp_consensus_aura::sr25519::AuthoritySignature as AuraSignature;
	use sp_core::crypto::Ss58Codec;
	use sp_core::H256;
	use sp_runtime::traits::{BlakeTwo256, Hash};
	use sp_runtime::RuntimeAppPublic;

	#[test]
	fn test_header_hashing() {
		let json_header = "{
            \"parentHash\": \"0x3daf82d4a574425dd416d6ca1644d582f875a41f7578cc6cb60ba66abdff4f01\",
            \"number\": \"0x5b1\",
            \"stateRoot\": \"0x291edbe5a31b3a8844df818d8ebff2e37ce2a80f3fd3a311a60f92cd0bee0858\",
            \"extrinsicsRoot\": \"0x61956e6d49c23796b0b3031482d9feacfa9d4cfd0a7b14810a440523e77723fc\",
            \"digest\": {
            \"logs\": [
            \"0x0661757261201e61fd1000000000\",
            \"0x066d6373682861626162616261626161\"
            ]
        }
        }";

		// This is the last log of the header. Its suffix is the signature of header hash. It was removed from the header, so we can get correct hash.
		// "0x056175726101019ee2ad67e2646c4d8331787b22d3ca793a491a5ec6d4def9d526de3a6f3ffb0adafc42111ef5743f4213692b0f30301f4ab97a8cb9bb8d5f7e5d7f0090287085"
		let header: Header = serde_json::from_str(json_header).unwrap();
		let public_key =
			AuraId::from_ss58check("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY").unwrap();
		let header_hash = calculate_header_hash(header);
		let signature_bytes = hex!(
			"9ee2ad67e2646c4d8331787b22d3ca793a491a5ec6d4def9d526de3a6f3ffb0adafc42111ef5743f4213692b0f30301f4ab97a8cb9bb8d5f7e5d7f0090287085"
		);
		let aura_signature: AuraSignature =
			AuraSignature::try_from(signature_bytes.as_ref()).unwrap();
		let verification_result = AuraId::verify(&public_key, &header_hash.0, &aura_signature);
		assert!(verification_result);
	}

	fn calculate_header_hash(header: Header) -> H256 {
		let encoded_header = header.encode();
		assert_eq!(
			hex::encode(&encoded_header),
			"3daf82d4a574425dd416d6ca1644d582f875a41f7578cc6cb60ba66abdff4f01c516291edbe5a31b3a8844df818d8ebff2e37ce2a80f3fd3a311a60f92cd0bee085861956e6d49c23796b0b3031482d9feacfa9d4cfd0a7b14810a440523e77723fc080661757261201e61fd1000000000066d6373682861626162616261626161"
		);
		let header_hash = BlakeTwo256::hash(&encoded_header);
		assert_eq!(header_hash, header.hash());
		assert_eq!(
			hex::encode(header_hash.0),
			"0b05da57679f89c11a6d8d4b007e78397a3c3b680ff487fa19d5700c5f308453"
		);
		header_hash
	}
}
