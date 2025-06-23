/// Length of Poseidon's state.
pub(crate) const WIDTH: usize = 3;

/// Hash rate of Poseidon.
pub(crate) const RATE: usize = 2;

/// Number of full rounds of the Poseidon permutation.
pub(crate) const NB_FULL_ROUNDS: usize = 8;

/// Number of partial rounds of the Poseidon permutation.
pub(crate) const NB_PARTIAL_ROUNDS: usize = 60;

/// A PrimeField with the constants needed to compute Poseidon's permutation
/// (MDS matrix and round constants).
pub trait PoseidonField: ark_ff::Field
{
    /// The MDS matrix used for the linear layer at each round of Poseidon.
    const MDS: [[Self; WIDTH]; WIDTH];

    /// The constants added to Poseidon's state on every round.
    const ROUND_CONSTANTS: [[Self; WIDTH]; NB_FULL_ROUNDS + NB_PARTIAL_ROUNDS];
}

mod blstrs;
