pub mod accumulator;
#[macro_use]
pub mod bbs_plus;
pub mod bbs_23;
pub mod bound_check_bpp;
pub mod bound_check_legogroth16;
pub mod bound_check_smc;
pub mod bound_check_smc_with_kv;
pub mod inequality;
pub mod ps_signature;
pub mod r1cs_legogorth16;
pub mod saver;
pub mod schnorr;

use core::borrow::Borrow;

use crate::error::ProofSystemError;
use ark_ec::{pairing::Pairing, AffineRepr};
use ark_ff::PrimeField;
use ark_std::{format, io::Write};
use itertools::{EitherOrBoth, Itertools};

use crate::{
    statement_proof::StatementProof,
    sub_protocols::{
        bound_check_bpp::BoundCheckBppProtocol,
        bound_check_legogroth16::BoundCheckLegoGrothProtocol,
        bound_check_smc::BoundCheckSmcProtocol,
        bound_check_smc_with_kv::BoundCheckSmcWithKVProtocol, inequality::InequalityProtocol,
        r1cs_legogorth16::R1CSLegogroth16Protocol,
    },
};
use accumulator::{AccumulatorMembershipSubProtocol, AccumulatorNonMembershipSubProtocol};

/// Various sub-protocols that are executed to create a `StatementProof` which are then combined to
/// form a `Proof`
#[derive(Clone, Debug, PartialEq)]
pub enum SubProtocol<'a, E: Pairing, G: AffineRepr> {
    /// For BBS+ signature in group G1
    PoKBBSSignatureG1(self::bbs_plus::PoKBBSSigG1SubProtocol<'a, E>),
    AccumulatorMembership(AccumulatorMembershipSubProtocol<'a, E>),
    AccumulatorNonMembership(AccumulatorNonMembershipSubProtocol<'a, E>),
    PoKDiscreteLogs(self::schnorr::SchnorrProtocol<'a, G>),
    /// For verifiable encryption using SAVER
    Saver(self::saver::SaverProtocol<'a, E>),
    /// For range proof using LegoGroth16
    BoundCheckLegoGroth16(BoundCheckLegoGrothProtocol<'a, E>),
    R1CSLegogroth16Protocol(R1CSLegogroth16Protocol<'a, E>),
    PSSignaturePoK(ps_signature::PSSignaturePoK<'a, E>),
    /// For BBS signature in group G1
    PoKBBSSignature23G1(bbs_23::PoKBBSSigG1SubProtocol<'a, E>),
    /// For range proof using Bulletproofs++
    BoundCheckBpp(BoundCheckBppProtocol<'a, G>),
    /// For range proof using set-membership check
    BoundCheckSmc(BoundCheckSmcProtocol<'a, E>),
    /// For range proof using set-membership check with keyed verification
    BoundCheckSmcWithKV(BoundCheckSmcWithKVProtocol<'a, E>),
    /// To prove inequality of a signed message with a public value
    Inequality(InequalityProtocol<'a, G>),
}

macro_rules! delegate {
    ($self: ident $($tt: tt)+) => {{
        $crate::delegate_indexed! {
            $self =>
                PoKBBSSignatureG1,
                AccumulatorMembership,
                AccumulatorNonMembership,
                PoKDiscreteLogs,
                Saver,
                BoundCheckLegoGroth16,
                R1CSLegogroth16Protocol,
                PSSignaturePoK,
                PoKBBSSignature23G1,
                BoundCheckBpp,
                BoundCheckSmc,
                BoundCheckSmcWithKV,
                Inequality
            : $($tt)+
        }
    }};
}

pub trait ProofSubProtocol<E: Pairing, G: AffineRepr<ScalarField = E::ScalarField>> {
    fn challenge_contribution(&self, target: &mut [u8]) -> Result<(), ProofSystemError>;
    fn gen_proof_contribution(
        &mut self,
        challenge: &E::ScalarField,
    ) -> Result<StatementProof<E, G>, ProofSystemError>;
}

impl<'a, E: Pairing, G: AffineRepr<ScalarField = E::ScalarField>> SubProtocol<'a, E, G> {
    pub fn challenge_contribution<W: Write>(&self, writer: W) -> Result<(), ProofSystemError> {
        delegate!(self.challenge_contribution(writer))
    }

    pub fn gen_proof_contribution(
        &mut self,
        challenge: &E::ScalarField,
    ) -> Result<StatementProof<E, G>, ProofSystemError> {
        delegate!(self.gen_proof_contribution(challenge))
    }
}

/// Merges indexed messages sorted by index with indexed blindings sorted by index.
/// Messages which don't have corresponding blindings will be blinded randomly.
/// In case blinding has an index that isn't present in the messages iterator,
/// `invalid_blinding_idx` will be set to this index and iteration will be aborted.
fn merge_indexed_messages_with_blindings<'a, M, B, R: 'a>(
    indexed_msgs_sorted_by_index: impl IntoIterator<Item = (impl Borrow<usize>, M)> + 'a,
    indexed_blindings_sorted_by_index: impl IntoIterator<Item = (impl Borrow<usize>, B)> + 'a,
    mut map_randomly_blinded_msg: impl FnMut(M) -> R + 'a,
    mut map_msg_with_blinding: impl FnMut(M, B) -> R + 'a,
    invalid_blinding_idx: &'a mut Option<usize>,
) -> impl Iterator<Item = (usize, R)> + 'a {
    indexed_msgs_sorted_by_index
        .into_iter()
        .map(|(idx, msg)| (*idx.borrow(), msg))
        .merge_join_by(
            indexed_blindings_sorted_by_index
                .into_iter()
                .map(|(idx, msg)| (*idx.borrow(), msg)),
            |(m_idx, _), (b_idx, _)| m_idx.cmp(b_idx),
        )
        .map(move |either| {
            let item = match either {
                EitherOrBoth::Left((idx, msg)) => (idx, map_randomly_blinded_msg(msg)),
                EitherOrBoth::Both((idx, message), (_, blinding)) => {
                    (idx, (map_msg_with_blinding)(message, blinding))
                }
                EitherOrBoth::Right((idx, _)) => {
                    invalid_blinding_idx.replace(idx);

                    return None;
                }
            };

            Some(item)
        })
        .take_while(Option::is_some)
        .flatten()
}

pub fn validate_bounds(min: u64, max: u64) -> Result<(), ProofSystemError> {
    if max <= min {
        return Err(ProofSystemError::BoundCheckMaxNotGreaterThanMin);
    }
    Ok(())
}

pub fn enforce_and_get_u64<F: PrimeField>(val: &F) -> Result<u64, ProofSystemError> {
    let m = val.into_bigint();
    let limbs: &[u64] = m.as_ref();
    for i in 1..limbs.len() {
        if limbs[i] != 0 {
            return Err(ProofSystemError::UnsupportedValue(format!(
                "Only supports 64 bit values Bulletproofs++ range proof but found {}",
                val
            )));
        }
    }
    Ok(limbs[0])
}

pub fn should_use_cls(min: u64, max: u64) -> bool {
    assert!(max > min);
    let diff = max - min;
    let bits = diff.ilog2();
    bits < 20
}
