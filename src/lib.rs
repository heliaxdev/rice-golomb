//! Rice-Golomb coding for Rust.
//!
//! This crate implements Rice-Golomb coding, a simple and effective form of
//! entropy coding used in lossless compression. Each value is split into a
//! quotient, written in unary, and a remainder, written in fixed-width binary
//! using `EXPECTED_MAX_BITS` bits. The scheme is best suited to data whose
//! magnitudes tend to be small, such as residual signals in audio or image
//! codecs.
//!
//! All signed and unsigned integer types can be encoded and decoded directly
//! through the [`Rice`] trait. The [`Encoder`] type bundles the choice of
//! integer type and remainder width into a single generic entry point. For
//! pushing raw bits into a bit vector without Rice coding, see the
//! [`Encodable`] trait.

mod encodable;
mod rice;

use std::marker::PhantomData;
use std::ops;

use bitvec::order::Lsb0;
use bitvec::store::BitStore;

pub use self::encodable::Encodable;
pub use self::rice::Rice;

/// Bit vector stored in least-significant-bit-first order.
///
/// Backing store that encoded bits are pushed into during encoding.
pub type BitVec<S> = bitvec::vec::BitVec<S, Lsb0>;

/// Bit slice in least-significant-bit-first order.
///
/// Used to read back the bits written into a [`BitVec`].
pub type BitSlice<S> = bitvec::slice::BitSlice<S, Lsb0>;

/// Rice-Golomb encoder/decoder parameterized by integer type and remainder width.
///
/// The const parameter `EXPECTED_MAX_BITS` selects the number of bits used to
/// store the remainder portion of each coded value. Wider remainders favour
/// larger values; narrower remainders favour smaller ones. Picking a width
/// that matches the distribution of the input data is what makes Rice coding
/// compact.
///
/// The type parameter `I` selects the integer type being coded. `Encoder`
/// itself holds no state; it is purely a namespace for [`encode`](Self::encode)
/// and [`decode`](Self::decode) and can be constructed without a value.
pub struct Encoder<I, const EXPECTED_MAX_BITS: u32> {
    _marker: PhantomData<I>,
}

impl<I, const EXPECTED_MAX_BITS: u32> Encoder<I, EXPECTED_MAX_BITS> {
    /// Encode `encodable` into `store` using Rice-Golomb coding.
    ///
    /// The remainder portion uses `EXPECTED_MAX_BITS` bits.
    pub fn encode<S>(encodable: &I, store: &mut BitVec<S>)
    where
        I: Encodable + Rice,
        S: BitStore,
    {
        encodable.rice_encode::<EXPECTED_MAX_BITS, _>(store);
    }

    /// Decode a single value from `store` using Rice-Golomb coding.
    ///
    /// The remainder portion uses `EXPECTED_MAX_BITS` bits. Returns `None` if
    /// `store` does not contain enough bits to decode a full value. When
    /// decoding several values packed back to back, advance through the slice
    /// using the bit count returned by [`Rice::rice_decode`] instead of this
    /// method.
    pub fn decode<S>(store: &BitSlice<S>) -> Option<I>
    where
        I: Encodable + Rice + ops::Shl<u32, Output = I> + ops::BitOr<Output = I>,
        S: BitStore,
    {
        let (decoded, _) = I::rice_decode::<EXPECTED_MAX_BITS, _>(store)?;
        Some(decoded)
    }
}
