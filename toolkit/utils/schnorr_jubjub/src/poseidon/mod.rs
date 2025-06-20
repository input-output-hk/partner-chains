// Copied code from midnight-circuits
pub mod constants;

pub mod poseidon_cpu;

pub mod round_skips;

use constants::{PoseidonField, WIDTH};
pub use poseidon_cpu::*;

// Implementation of Poseidon over JubJub (using midnight-circuits implementation).
// Need to figure out if we can assume that the bytes are canonical, i.e., if we are always hashing something that
// was a Scalar field element.

use alloc::vec::Vec;
use ark_ed_on_bls12_381::{Fq as Scalar};
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};

use hash256_std_hasher::Hash256StdHasher;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_core::Hasher;
use sp_runtime::RuntimeDebug;
use sp_runtime::{StateVersion, traits::Hash};
use sp_runtime_interface::pass_by::{AllocateAndReturnPointer, PassAs, PassFatPointerAndDecode};
use sp_runtime_interface::runtime_interface;
use sp_trie::{LayoutV0, LayoutV1, TrieConfiguration};
use sp_runtime::DeserializeOwned;

#[derive(Debug)]
pub enum PoseidonError {
    /// Error produced when trying to convert bytes to field element
    NotCanonical,
}

#[derive(PartialEq, Eq, Clone, RuntimeDebug, TypeInfo, Serialize, Deserialize)]
pub struct PoseidonJubjub;

sp_core::impl_maybe_marker_std_or_serde!(
	/// A type that implements Serialize when in std environment or serde feature is activated.
	trait MaybeSerialize: Serialize;

	/// A type that implements Serialize, DeserializeOwned and Debug when in std environment or serde feature is activated.
	trait MaybeSerializeDeserialize: DeserializeOwned, Serialize;
);

impl PoseidonJubjub {
    /// Converts a byte slice into a vector of `Scalar` field elements suitable
    /// for Poseidon hashing.
    ///
    /// Poseidon operates over field elements, so this function transforms raw
    /// bytes into `Scalar`s.
    ///
    /// - If `format_scalars` is `true`, the input is assumed to already contain
    ///   valid field elements. Each 32-byte chunk is interpreted as a canonical
    ///   `Scalar`.
    ///
    /// - If `format_scalars` is `false`, the input is processed in 31-byte
    ///   chunks to ensure that each resulting value falls within the canonical
    ///   range of the field.
    ///
    /// Returns a `Vec<Scalar>` on success, or an `Error` if canonical
    /// conversion fails.
    pub fn msg_from_bytes(msg: &[u8], format_scalars: bool) -> Result<Vec<Scalar>, PoseidonError> {
        let chunk_len = if format_scalars { 32 } else { 31 };

        msg.chunks(chunk_len)
            .map(|scalar| {
                let mut bytes = [0u8; 32];
                bytes[..scalar.len()].copy_from_slice(scalar);
                Scalar::deserialize_compressed(&bytes[..]).map_err(|_| PoseidonError::NotCanonical)
            })
            .collect::<Result<Vec<_>, PoseidonError>>()
    }
}

impl Hasher for PoseidonJubjub {
    type Out = sp_core::H256;
    type StdHasher = Hash256StdHasher;
    const LENGTH: usize = 32;

    fn hash(s: &[u8]) -> Self::Out {
        // TODO: we are assuming false - would be ideal if we could assume true
        let msg = Self::msg_from_bytes(s, false).expect("Conversion should not fail.");

        let out_fr = Self::hash(&msg);

        let mut out = [0u8; 32];
        out_fr.serialize_compressed(out.as_mut_slice());
        out.into()
    }
}

#[runtime_interface]
pub trait PoseidonTrie {
    /// A trie root formed from the iterated items using Poseidon
    fn poseidon_jubjub_root(
        input: PassFatPointerAndDecode<Vec<(Vec<u8>, Vec<u8>)>>,
        version: PassAs<StateVersion, u8>,
    ) -> AllocateAndReturnPointer<H256, 32> {
        match version {
            StateVersion::V0 => LayoutV0::<PoseidonJubjub>::trie_root(input),
            StateVersion::V1 => LayoutV1::<PoseidonJubjub>::trie_root(input),
        }
    }

    /// A trie root formed from the enumerated items using Poseidon
    fn poseidon_jubjub_ordered_root(
        input: PassFatPointerAndDecode<Vec<Vec<u8>>>,
        version: PassAs<StateVersion, u8>,
    ) -> AllocateAndReturnPointer<H256, 32> {
        match version {
            StateVersion::V0 => LayoutV0::<PoseidonJubjub>::ordered_trie_root(input),
            StateVersion::V1 => LayoutV1::<PoseidonJubjub>::ordered_trie_root(input),
        }
    }
}

impl Hash for PoseidonJubjub {
    type Output = H256;

    fn ordered_trie_root(input: Vec<Vec<u8>>, version: StateVersion) -> Self::Output {
        poseidon_trie::poseidon_jubjub_ordered_root(input, version)
    }

    fn trie_root(input: Vec<(Vec<u8>, Vec<u8>)>, version: StateVersion) -> Self::Output {
        poseidon_trie::poseidon_jubjub_root(input, version)
    }
}
