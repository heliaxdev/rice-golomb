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
    /// using the bit count returned by [`Encoder::decode_and_skip`] instead of
    /// calling this method.
    pub fn decode<S>(store: &BitSlice<S>) -> Option<I>
    where
        I: Encodable + Rice + ops::Shl<u32, Output = I> + ops::BitOr<Output = I>,
        S: BitStore,
    {
        let (decoded, _) = I::rice_decode::<EXPECTED_MAX_BITS, _>(store)?;
        Some(decoded)
    }

    /// Decode a single value from `store` using Rice-Golomb coding,
    /// and return the number of bits to skip in a [`BitSlice`] stream.
    ///
    /// The remainder portion uses `EXPECTED_MAX_BITS` bits. Returns `None` if
    /// `store` does not contain enough bits to decode a full value.
    pub fn decode_and_skip<S>(store: &BitSlice<S>) -> Option<(I, usize)>
    where
        I: Encodable + Rice + ops::Shl<u32, Output = I> + ops::BitOr<Output = I>,
        S: BitStore,
    {
        I::rice_decode::<EXPECTED_MAX_BITS, _>(store)
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use super::*;

    #[test]
    fn u64_value_stream_compaction() {
        type BackingInt = usize;

        type RegInt = u64;
        const MAX_BITS: u32 = 8;

        let mut store = BitVec::<BackingInt>::new();

        let sorted_elements = [
            20, 21, 26, 36, 49, 55, 62, 79, 135, 282, 291, 340, 525, 537, 569, 644, 665, 759, 841,
            975, 2518, 3475, 4577, 4750, 4998, 5499, 5566, 5952, 7035, 7939, 8506, 9240,
        ];

        for elem in sorted_elements {
            Encoder::<RegInt, MAX_BITS>::encode(&elem, &mut store);
            assert_eq!(elem, Encoder::<RegInt, MAX_BITS>::decode(&store).unwrap(),);
            eprintln!("encoded len of {elem} => {}", store.len());
            store.clear();
        }

        for diff in iter::once(0)
            .chain(sorted_elements.iter().copied())
            .zip(sorted_elements.iter().copied())
            .map(|(prev, next)| next - prev)
        {
            Encoder::<RegInt, MAX_BITS>::encode(&diff, &mut store);
        }

        assert!(sorted_elements.len() * 64 > store.len());

        let mut ptr = 0;
        let mut decoded_elements = Vec::with_capacity(sorted_elements.len());
        let mut last = None;

        while let Some((decoded_diff, skip)) = store
            .get(ptr..)
            .and_then(|store| Encoder::<RegInt, MAX_BITS>::decode_and_skip(store))
        {
            let decoded = decoded_diff + last.unwrap_or_default();

            decoded_elements.push(decoded);

            ptr += skip;
            last = Some(decoded);
        }

        assert_eq!(sorted_elements.len(), decoded_elements.len());
        assert_eq!(&sorted_elements[..], decoded_elements);
    }
}
