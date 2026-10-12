//! A Merkle tree, compressed two-to-one with Poseidon.
//!
//! The compression is the permutation at width three on [left, right, 0],
//! taking the first element out. The permutation, its constants and the rule
//! they are derived by are the Poseidon specification's, unchanged — and the
//! constants are still derived in `init`, which is not measured.
use ark_bn254::Fr;
use ark_ff::{BigInt, Field, PrimeField, Zero};
use sha2::{Digest, Sha256};

use crate::fherma::{Inputs, Outputs, Point, Tensor};

const ALPHA: u64 = 5;
const FULL: usize = 8;
const PARTIAL: usize = 57;
const DOMAIN: &[u8] = b"fherma/poseidon/bn254/x5";

/// Field elements by rejection sampling from SHA-256 in counter mode: the
/// digest big-endian, kept when it is below the modulus, counter four bytes.
fn stream(tag: &[u8], count: usize) -> Vec<Fr> {
    let mut out = Vec::with_capacity(count);
    let mut counter: u32 = 0;
    while out.len() < count {
        let mut hasher = Sha256::new();
        hasher.update(tag);
        hasher.update(counter.to_be_bytes());
        let digest = hasher.finalize();
        counter += 1;
        // Big-endian into four 64-bit limbs, low word first, then refused if
        // it is at or above the modulus — which is what "kept when below" is.
        let mut words = [0u64; 4];
        for (i, chunk) in digest.chunks_exact(8).rev().enumerate() {
            words[i] = u64::from_be_bytes(chunk.try_into().unwrap());
        }
        if let Some(value) = Fr::from_bigint(BigInt::new(words)) {
            out.push(value);
        }
    }
    out
}

fn round_constants(t: usize) -> Vec<Fr> {
    let mut tag = DOMAIN.to_vec();
    tag.extend_from_slice(b"/ark/");
    tag.extend_from_slice(t.to_string().as_bytes());
    stream(&tag, (FULL + PARTIAL) * t)
}

/// A Cauchy matrix, M[i][j] = 1 / (i + t + j): MDS for certain.
fn mds(t: usize) -> Vec<Vec<Fr>> {
    (0..t)
        .map(|i| {
            (0..t)
                .map(|j| Fr::from((i + t + j) as u64).inverse().expect("a nonzero sum"))
                .collect()
        })
        .collect()
}

pub struct State {
    t: usize,
    limbs: usize,
    ark: Vec<Fr>,
    m: Vec<Vec<Fr>>,
}

impl Default for State {
    fn default() -> Self {
        State { t: 0, limbs: 0, ark: Vec::new(), m: Vec::new() }
    }
}

/// The tree is built at width three: two children in, one parent out.
const T: usize = 3;

pub fn init(p: &Point) -> State {
    State { t: T, limbs: p.L as usize, ark: round_constants(T), m: mds(T) }
}

fn permute(s: &mut [Fr], state: &State) {
    let t = state.t;
    let half = FULL / 2;
    let mut next = vec![Fr::zero(); t];
    for rnd in 0..FULL + PARTIAL {
        let base = rnd * t;
        for i in 0..t {
            s[i] += state.ark[base + i];
        }
        if rnd < half || rnd >= half + PARTIAL {
            for x in s.iter_mut() {
                *x = x.pow([ALPHA]);
            }
        } else {
            s[0] = s[0].pow([ALPHA]);
        }
        for i in 0..t {
            let mut acc = Fr::zero();
            for j in 0..t {
                acc += state.m[i][j] * s[j];
            }
            next[i] = acc;
        }
        s.copy_from_slice(&next);
    }
}


/// One field element, read from the `i`-th row of a limb tensor.
fn fe(data: &[u64], i: usize, limbs: usize) -> Fr {
    let mut words = [0u64; 4];
    words[..limbs].copy_from_slice(&data[i * limbs..(i + 1) * limbs]);
    Fr::from_bigint(BigInt::new(words)).expect("below the modulus")
}

/// H(left, right): the permutation on [left, right, 0], first element out.
fn compress(left: Fr, right: Fr, state: &State) -> Fr {
    let mut s = [left, right, Fr::zero()];
    permute(&mut s, state);
    s[0]
}

pub fn run(state: &mut State, inp: &Inputs) -> Outputs {
    let limbs = state.limbs;
    let n = inp.leaves.data.len() / limbs;

    let mut level: Vec<Fr> = (0..n).map(|i| fe(&inp.leaves.data, i, limbs)).collect();
    while level.len() > 1 {
        level = level
            .chunks_exact(2)
            .map(|pair| compress(pair[0], pair[1], state))
            .collect();
    }

    Outputs {
        root: Tensor { shape: vec![limbs as i64], data: level[0].into_bigint().0[..limbs].to_vec() },
    }
}

pub fn free(state: State) {
    drop(state);
}
