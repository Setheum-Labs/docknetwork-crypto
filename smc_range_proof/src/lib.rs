#![cfg_attr(not(feature = "std"), no_std)]
#![allow(non_snake_case)]

//! Implements the following range proof and set-membership protocols.
//! 1. Set membership protocol using BB signature. Described in Fig.1 of the paper [1]. [Code](/src/ccs_set_membership)
//! 2. Range proof protocol as described in Fig.3 of the paper [1]. Considers a perfect-range, i.e. range of the form `[0, u^l)`
//! where `u` is the base and the upper bound is a power of the base. [Code](src/ccs_range_proof/perfect_range.rs)
//! 3. Range proof protocol as described in section 4.4 of the paper [1]. Considers an arbitrary range `[min, max)`. [Code](src/ccs_range_proof/arbitrary_range.rs)
//! 4. Range proof using sumsets, based on Protocol 2 from the paper [2]. [Code](src/smc_range_proof.rs)
//! 5. Implements the Keyed-Verification of the above protocols where the verifier knows the secret key of the BB sig. This makes
//! the proof generation and verification more efficient by removing the need for pairings. This idea is taken from this PhD. thesis.
//!
//! Above protocols use a pairing based signature called the [BB signature](src/bb_sig.rs).
//!
//! References:
//!
//! [1]: [Efficient Protocols for Set Membership and Range Proofs](https://link.springer.com/chapter/10.1007/978-3-540-89255-7_15)
//!
//! [2]: [Additive Combinatorics and Discrete Logarithm Based Range Protocols](https://eprint.iacr.org/2009/469)

#[macro_use]
pub mod common;
pub mod bb_sig;
pub mod ccs_range_proof;
pub mod ccs_set_membership;
mod cls_range_proof;
pub mod error;

pub mod prelude {
    pub use crate::{
        bb_sig::{PublicKeyG2, SecretKey, SignatureParams, SignatureParamsWithPairing},
        ccs_range_proof::{
            CCSArbitraryRangeProof, CCSArbitraryRangeProofProtocol,
            CCSArbitraryRangeProofWithKVProtocol, CCSArbitraryRangeWithKVProof,
        },
        ccs_set_membership::setup::{
            SetMembershipCheckParams, SetMembershipCheckParamsWithPairing,
        },
        cls_range_proof::{
            CLSRangeProof, CLSRangeProofProtocol, CLSRangeProofWithKV, CLSRangeProofWithKVProtocol,
        },
        common::MemberCommitmentKey,
        error::SmcRangeProofError,
    };
}
