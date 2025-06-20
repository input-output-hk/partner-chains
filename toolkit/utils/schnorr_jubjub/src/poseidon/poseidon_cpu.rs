use alloc::vec::Vec;
use ark_ed_on_bls12_381::{Fq as Scalar};
use ark_ff::Field;
use ark_ec::AdditiveGroup;
use crate::PoseidonJubjub;

use super::{
    constants::{PoseidonField, NB_FULL_ROUNDS, NB_PARTIAL_ROUNDS, RATE, WIDTH},
    round_skips::{PreComputedRoundCPU},
};

/// Number of times the linear part of the partial rounds is skipped in the
/// Poseidon cpu implemetation (0 is the default implementation without skips at
/// all).
pub(crate) const NB_SKIPS_CPU: usize = 2;

/// Off-circuit Poseidon state.
#[derive(Clone, Debug)]
pub struct PoseidonState {
    pre_computed: PreComputedRoundCPU,
    register: [Scalar; WIDTH],
    queue: Vec<Scalar>,
    squeeze_position: usize,
    input_len: Option<usize>,
}

// Applies the MDS matrix to a state and adds the round constants. All arguments
// have length `WIDTH`. To save the addition cost, the implementation is done by
// mutating the `constants` slice, and eventually copying it into `state`.
fn linear_layer<F: PoseidonField>(state: &mut [F], constants: &mut [F]) {
    #[allow(clippy::needless_range_loop)]
    for i in 0..WIDTH {
        for j in 0..WIDTH {
            constants[i] += F::MDS[i][j] * state[j];
        }
    }
    state.copy_from_slice(constants);
}

/// A cpu version of the full round of Poseidon's permutation. Operates by
/// mutating the `state` argument (length `WIDTH`).
pub(crate) fn full_round_cpu<F: PoseidonField>(round: usize, state: &mut [F]) {
    state.iter_mut().for_each(|x| *x = x.square().square() * *x);
    let mut new_state = if round == NB_FULL_ROUNDS + NB_PARTIAL_ROUNDS - 1 {
        [F::ZERO; WIDTH]
    } else {
        F::ROUND_CONSTANTS[round + 1]
    };
    linear_layer(state, &mut new_state);
}

// A cpu version of Poseidon with `1 + NB_SKIPS_CIRCUIT` partial rounds.
fn partial_round_cpu(
    pre_computed: &PreComputedRoundCPU,
    round: usize,
    state: &mut [Scalar], // Length `WIDTH`.
) {
    pre_computed
        .partial_round_id
        .eval::<NB_SKIPS_CPU>(&pre_computed.round_constants[round], state);
}

// Alternative partial round version, without any skips.
fn partial_round_cpu_raw(round: usize, state: &mut [Scalar]) {
    state[WIDTH - 1] *= state[WIDTH - 1].square().square();
    let mut new_state = Scalar::ROUND_CONSTANTS[round + 1];
    linear_layer(state, &mut new_state)
}

/// A cpu version of the full Poseidon's permutation with partial-round skips.
pub fn permutation_cpu(pre_computed: &PreComputedRoundCPU, state: &mut [Scalar]) {
    let nb_skips = pre_computed.partial_round_id.nb_skips;
    let nb_main_partial_rounds = NB_PARTIAL_ROUNDS / (1 + nb_skips);
    let remainder_partial_rounds = NB_PARTIAL_ROUNDS % (1 + nb_skips);

    for (x, k0) in state.iter_mut().zip(Scalar::ROUND_CONSTANTS[0]) {
        *x += k0;
    }
    (0..NB_FULL_ROUNDS / 2).for_each(|round| full_round_cpu(round, state));
    (0..nb_main_partial_rounds).for_each(|round| partial_round_cpu(pre_computed, round, state));
    (NB_FULL_ROUNDS / 2 + NB_PARTIAL_ROUNDS - remainder_partial_rounds..)
        .take(remainder_partial_rounds)
        .for_each(|round| partial_round_cpu_raw(round, state));
    (NB_FULL_ROUNDS / 2 + NB_PARTIAL_ROUNDS..)
        .take(NB_FULL_ROUNDS / 2)
        .for_each(|round| {
            full_round_cpu(round, state);
        })
}

// A cpu implementation of the sponge operations, building on the Poseidon's
// permutation.
impl PoseidonJubjub {
    pub(crate) fn init(input_len: Option<usize>) -> PoseidonState {
        let mut register = [Scalar::ZERO; WIDTH];
        register[RATE] = Scalar::from(input_len.map(|l| l as u128).unwrap_or(1 << 64));
        let pre_computed = PreComputedRoundCPU::init();
        PoseidonState {
            pre_computed,
            register,
            queue: Vec::new(),
            squeeze_position: 0,
            input_len,
        }
    }

    pub(crate) fn absorb(state: &mut PoseidonState, inputs: &[Scalar]) {
        state.queue.extend(inputs);
        state.squeeze_position = 0;
    }

    pub(crate) fn squeeze(state: &mut PoseidonState) -> Scalar {
        if state.squeeze_position > 0 {
            // If `input_len` was specified, we only allow 1 squeeze.
            if state.input_len.is_some() {
                panic!("Attempting to squeeze multiple times a fixed-size Poseidon sponge (CPU).")
            };
            debug_assert!(state.queue.is_empty());
            let output = state.register[state.squeeze_position % RATE];
            state.squeeze_position = (state.squeeze_position + 1) % RATE;
            return output;
        }

        match state.input_len {
            None => {
                let padding = Scalar::from(state.queue.len() as u64);
                state.queue.push(padding);
            }
            Some(len) => {
                if state.queue.len() != len {
                    panic!("Inconsistent lengths in fixed-size Poseidon sponge (CPU). Expected: {}, found: {}.", len, state.queue.len())
                };
            }
        }

        for chunk in state.queue.chunks(RATE) {
            for (entry, value) in state.register.iter_mut().zip(chunk.iter()) {
                *entry += value;
            }
            permutation_cpu(&state.pre_computed, &mut state.register);
        }

        state.queue = Vec::new();
        state.squeeze_position = 1 % RATE;
        state.register[0]
    }

    pub(crate) fn hash(inputs: &[Scalar]) -> Scalar {
        let mut state = Self::init(Some(inputs.len()));
        Self::absorb(&mut state, inputs);
        Self::squeeze(&mut state)
    }
}

#[cfg(test)]
mod tests {
    use rand::SeedableRng;
    use rand_chacha::ChaCha12Rng;
    use ark_ff::UniformRand;

    use super::*;

    // A version of Poseidon's permutation, without round skips. Has been tested
    // against the previous version of Poseidon (replaced since Merge request #521).
    fn permutation_cpu_raw(state: &mut [Scalar]) {
        for (x, k0) in state.iter_mut().zip(Scalar::ROUND_CONSTANTS[0]) {
            *x += k0;
        }
        for round in 0..NB_FULL_ROUNDS / 2 {
            full_round_cpu(round, state);
        }
        for round in (NB_FULL_ROUNDS / 2..).take(NB_PARTIAL_ROUNDS) {
            partial_round_cpu_raw(round, state);
        }
        for round in (NB_FULL_ROUNDS / 2 + NB_PARTIAL_ROUNDS..).take(NB_FULL_ROUNDS / 2) {
            full_round_cpu(round, state);
        }
    }
    // Tests the performances of the cpu version of Poseidon. In debug mode, also
    // tests the consistency between the version with and without round skips.
    fn consistency_cpu(nb_samples: usize) {
        let pre_computed = PreComputedRoundCPU::init();
        let mut rng = ChaCha12Rng::seed_from_u64(0xf007ba11);
        (0..nb_samples)
            .for_each(|_| {
                let input: [Scalar; WIDTH] =
                    core::array::from_fn(|_| Scalar::rand(&mut rng));
                let mut res1 = input;
                let mut res2 = input;
                permutation_cpu_raw(&mut res1);
                permutation_cpu(&pre_computed, &mut res2);
                if res1 != res2 {
                    panic!("=> Inconsistencies between the cpu implementations of the permutations.\n\nOn input x = {:?},\n\npermutation_cpu_no_skip(x) = {:?}\n\npermutation_cpu_with_skips(x) = {:?}\n", input, res1, res2)
                }
            });
    }

    #[test]
    fn cpu_test() {
        // Testing cpu performances. In debug mode, also tests the consistency between
        // the optimised and non-optimised cpu implementations of the permutation.
        consistency_cpu(1);
    }
}
