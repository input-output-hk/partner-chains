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
