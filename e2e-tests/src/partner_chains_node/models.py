from dataclasses import dataclass


@dataclass
class RegistrationSignatures:
    spo_public_key: str
    spo_signature: str
    sidechain_public_keys: str
    sidechain_signature: str


@dataclass
class AddressAssociationSignature:
    partner_chain_address: str
    signature: str
    stake_public_key: str


@dataclass
class BlockProducerMetadataSignature:
    cross_chain_pub_key: str
    cross_chain_pub_key_hash: str
    encoded_message: str
    encoded_metadata: str
    signature: str


@dataclass
class VFunction:
    cbor: str
    script_path: str
    script_hash: str
    address: str
    reference_utxo: str

    def __repr__(self) -> str:
        return (
            f"VFunction(script_hash={self.script_hash}, address={self.address}, reference_utxo={self.reference_utxo})"
        )


@dataclass
class Reserve:
    token: str
    v_function: VFunction
