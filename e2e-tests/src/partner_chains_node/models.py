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
