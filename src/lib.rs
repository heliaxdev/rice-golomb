mod encodable;
mod rice;

use std::marker::PhantomData;
use std::ops;

use bitvec::order::Lsb0;
use bitvec::store::BitStore;

pub use self::encodable::Encodable;
pub use self::rice::Rice;

pub type BitVec<S> = bitvec::vec::BitVec<S, Lsb0>;
pub type BitSlice<S> = bitvec::slice::BitSlice<S, Lsb0>;

pub struct Encoder<I, const EXPECTED_MAX_BITS: u32> {
    _marker: PhantomData<I>,
}

impl<I, const EXPECTED_MAX_BITS: u32> Encoder<I, EXPECTED_MAX_BITS> {
    pub fn encode<S>(encodable: &I, store: &mut BitVec<S>)
    where
        I: Encodable + Rice,
        S: BitStore,
    {
        encodable.rice_encode::<EXPECTED_MAX_BITS, _>(store);
    }

    pub fn decode<S>(store: &BitSlice<S>) -> Option<I>
    where
        I: Encodable + Rice + ops::Shl<u32, Output = I> + ops::BitOr<Output = I>,
        S: BitStore,
    {
        let (decoded, _) = I::rice_decode::<EXPECTED_MAX_BITS, _>(store)?;
        Some(decoded)
    }
}
