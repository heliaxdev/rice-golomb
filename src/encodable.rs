use bitvec::field::BitField;
use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::store::BitStore;
use bitvec::vec::BitVec;

pub trait Encodable {
    fn encode<T>(&self, store: &mut BitVec<T, Lsb0>)
    where
        T: BitStore;

    fn len(&self) -> usize;

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Encodable for [u8] {
    fn len(&self) -> usize {
        (*self).len() << 3
    }

    fn is_empty(&self) -> bool {
        (*self).is_empty()
    }

    fn encode<T>(&self, store: &mut BitVec<T, Lsb0>)
    where
        T: BitStore,
    {
        store.extend_from_bitslice(BitSlice::<_, Lsb0>::from_slice(self));
    }
}

macro_rules! impl_encodable_int {
    ($int:ty) => {
        impl Encodable for $int {
            fn len(&self) -> usize {
                const { Self::BITS as _ }
            }

            fn is_empty(&self) -> bool {
                false
            }

            fn encode<T>(&self, store: &mut BitVec<T, Lsb0>)
            where
                T: BitStore,
            {
                let old_len = store.len();
                let new_len = old_len + const { Self::BITS as usize };

                store.resize(new_len, false);
                store[old_len..].store(*self);
            }
        }
    };
}

impl_encodable_int!(i8);
impl_encodable_int!(i16);
impl_encodable_int!(i32);
impl_encodable_int!(i64);
impl_encodable_int!(i128);

impl_encodable_int!(u8);
impl_encodable_int!(u16);
impl_encodable_int!(u32);
impl_encodable_int!(u64);
impl_encodable_int!(u128);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extend_from_int() {
        let mut store: BitVec<usize> = BitVec::new();

        const BITMAP: u64 = 0b111010u64;

        BITMAP.encode(&mut store);

        for (i, expected_bit) in [false, true, false, true, true, true]
            .into_iter()
            .enumerate()
        {
            assert_eq!(expected_bit, *store.get(i).unwrap());
        }

        assert_eq!(<u64 as Encodable>::len(&BITMAP), store.len());
    }

    #[test]
    fn extend_from_slice() {
        let mut store: BitVec<usize> = BitVec::new();

        let bits = [1u8, 2, 3, 4, 5, 6];

        bits.encode(&mut store);

        for (outer_idx, bitmap) in bits.into_iter().enumerate() {
            for (global_idx, expected_bit) in
                (0..8).map(|i| (outer_idx * 8 + i, (bitmap >> i) & 1 == 1))
            {
                assert_eq!(expected_bit, *store.get(global_idx).unwrap());
            }
        }

        assert_eq!(<[u8] as Encodable>::len(&bits), store.len());
    }
}
