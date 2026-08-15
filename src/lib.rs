pub mod encodable;
pub mod rice;

use bitvec::order::Lsb0;
use bitvec::store::BitStore;
use bitvec::vec::BitVec;

use self::encodable::Encodable;
use self::rice::Rice;

pub struct Encoder<const EXPECTED_MAX: u32>;

impl<const EXPECTED_MAX_BITS: u32> Encoder<EXPECTED_MAX_BITS> {
    pub fn encode<I, S>(encodable: &I, store: &mut BitVec<S, Lsb0>)
    where
        I: Encodable + Rice,
        S: BitStore,
    {
        encodable.rice_encode::<EXPECTED_MAX_BITS, _>(store);
    }
}
