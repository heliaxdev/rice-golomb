use std::ops;

use bitvec::field::BitField;
use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::store::BitStore;
use bitvec::vec::BitVec;

use crate::encodable::{Encodable, LengthLimited};

#[allow(private_bounds)]
pub trait Rice: Sealed + Copy {
    fn rice_encode_quotient<const MAX_BITS: u32, S: BitStore>(self, store: &mut BitVec<S, Lsb0>);
    fn rice_encode_remainder<const MAX_BITS: u32, S: BitStore>(self, store: &mut BitVec<S, Lsb0>);

    fn rice_decode_quotient<const MAX_BITS: u32, S: BitStore>(
        store: &BitSlice<S, Lsb0>,
    ) -> Option<(Self, usize)>;
    fn rice_decode_remainder<const MAX_BITS: u32, S: BitStore>(
        store: &BitSlice<S, Lsb0>,
    ) -> Option<Self>;

    fn rice_encode<const MAX_BITS: u32, S: BitStore>(self, store: &mut BitVec<S, Lsb0>) {
        self.rice_encode_quotient::<MAX_BITS, _>(store);
        self.rice_encode_remainder::<MAX_BITS, _>(store);
    }

    fn rice_decode<const MAX_BITS: u32, S: BitStore>(
        store: &BitSlice<S, Lsb0>,
    ) -> Option<(Self, usize)>
    where
        Self: ops::Shl<u32, Output = Self> + ops::BitOr<Output = Self>,
    {
        let (quot, skip) = Self::rice_decode_quotient::<MAX_BITS, _>(store)?;
        let rem = Self::rice_decode_remainder::<MAX_BITS, _>(store.get(skip..)?)?;
        Some(((quot << MAX_BITS) | rem, skip + MAX_BITS as usize))
    }
}

trait RiceAux {
    fn to_unary(self) -> Self;

    fn quotient<const MAX_BITS: u32>(self) -> Self;
    fn remainder<const MAX_BITS: u32>(self) -> Self;
}

trait Sealed {}

macro_rules! max_for_bits {
    ($max_bits:expr) => {
        !(const { !(0) } << $max_bits)
    };
}

macro_rules! impl_rice_int {
    ($int:ty) => {
        impl Sealed for $int {}

        impl RiceAux for $int {
            fn to_unary(self) -> Self {
                assert!(
                    self < const { <$int>::BITS as $int },
                    "{self} encoded as unary would exceed {} bits, requiring exactly {} bits",
                    <$int>::BITS,
                    <$int>::BITS + 1,
                );

                max_for_bits!(self)
            }

            fn quotient<const MAX_BITS: u32>(self) -> Self {
                const {
                    assert!(MAX_BITS < Self::BITS, "max bits exceeds capacity");
                }

                // divide by 2^MAX_BITS
                self >> MAX_BITS
            }

            fn remainder<const MAX_BITS: u32>(self) -> Self {
                const {
                    assert!(MAX_BITS < Self::BITS, "max bits exceeds capacity");
                }

                // truncate to specified max bits
                self & const { max_for_bits!(MAX_BITS) }
            }
        }

        impl Rice for $int {
            fn rice_encode_quotient<const MAX_BITS: u32, S: BitStore>(
                self,
                store: &mut BitVec<S, Lsb0>,
            ) {
                let binary_quotient = self.quotient::<MAX_BITS>();

                if binary_quotient < const { <$int>::BITS as $int } {
                    let unary_quotient = binary_quotient.to_unary();

                    LengthLimited::limit(unary_quotient, (binary_quotient + 1) as usize)
                        .unwrap()
                        .encode(store);

                    return;
                }

                // bit quotients exceeding <$int>::BITS (including sentinel bit)
                for _ in 0..binary_quotient {
                    store.push(true);
                }

                store.push(false);
            }

            fn rice_encode_remainder<const MAX_BITS: u32, S: BitStore>(
                self,
                store: &mut BitVec<S, Lsb0>,
            ) {
                let binary_remainder = self.remainder::<MAX_BITS>();

                LengthLimited::limit(binary_remainder, MAX_BITS as usize)
                    .unwrap()
                    .encode(store);
            }

            fn rice_decode_quotient<const MAX_BITS: u32, S: BitStore>(
                store: &BitSlice<S, Lsb0>,
            ) -> Option<($int, usize)> {
                let mut rsp = 0;

                store.iter().enumerate().find_map(|(i, bit_is_set)| {
                    if !*bit_is_set {
                        return Some((rsp, i + 1));
                    }

                    rsp += 1;

                    None
                })
            }

            fn rice_decode_remainder<const MAX_BITS: u32, S: BitStore>(
                store: &BitSlice<S, Lsb0>,
            ) -> Option<$int> {
                let max_pos = (MAX_BITS as usize).min(store.len());

                Some(store[..max_pos].load::<$int>().remainder::<MAX_BITS>())
            }
        }
    };
}

impl_rice_int!(i8);
impl_rice_int!(i16);
impl_rice_int!(i32);
impl_rice_int!(i64);
impl_rice_int!(i128);
impl_rice_int!(isize);

impl_rice_int!(u8);
impl_rice_int!(u16);
impl_rice_int!(u32);
impl_rice_int!(u64);
impl_rice_int!(u128);
impl_rice_int!(usize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rice_decode() {
        let mut store: BitVec<usize> = BitVec::new();

        // ---------------------------------------------------------
        // basic tests
        // ---------------------------------------------------------
        255u8.rice_encode::<7, _>(&mut store);
        assert_eq!(store.len(), 9);
        let (decoded, skip) = u8::rice_decode::<7, _>(&store).unwrap();
        assert_eq!(decoded, 255u8);
        assert_eq!(skip, 9);
        store.clear();

        1024u32.rice_encode::<10, _>(&mut store);
        assert_eq!(store.len(), 12);
        let (decoded, skip) = u32::rice_decode::<10, _>(&store).unwrap();
        assert_eq!(decoded, 1024u32);
        assert_eq!(skip, 12);
        store.clear();

        // ---------------------------------------------------------
        // 46-bit vectors
        // ---------------------------------------------------------

        0u64.rice_encode::<46, _>(&mut store);
        assert_eq!(store.len(), 47); // Q=0 (1 bit), R=0 (46 bits)
        let (decoded, skip) = u64::rice_decode::<46, _>(&store).unwrap();
        assert_eq!(decoded, 0u64);
        assert_eq!(skip, 47);
        store.clear();

        0x0000400000001337u64.rice_encode::<46, _>(&mut store);
        assert_eq!(store.len(), 48); // Q=1 (2 bits), R=0x1337 (46 bits)
        let (decoded, skip) = u64::rice_decode::<46, _>(&store).unwrap();
        assert_eq!(decoded, 0x0000400000001337u64);
        assert_eq!(skip, 48);
        store.clear();

        0x0004000000000123u64.rice_encode::<46, _>(&mut store);
        assert_eq!(store.len(), 63); // Q=16 (17 bits), R=0x0123 (46 bits)
        let (decoded, skip) = u64::rice_decode::<46, _>(&store).unwrap();
        assert_eq!(decoded, 0x0004000000000123u64);
        assert_eq!(skip, 63);
        store.clear();

        // ---------------------------------------------------------
        // continuous stream decoding; verify we can decode
        // multiple sequential elements by correctly advancing
        // via the `skip` offset
        // ---------------------------------------------------------
        let stream_values = [
            0u32,    // edge case 0
            15u32,   // fits perfectly in remainder (Q=0)
            42u32,   // Q=1, R=10
            100u32,  // Q=3, R=4
            9999u32, // large quotient
        ];

        // encode all into a single bitstream using MAX_BITS = 5
        for &v in stream_values.iter() {
            v.rice_encode::<5, _>(&mut store);
        }

        let mut offset = 0;
        for &expected in stream_values.iter() {
            // slice the bitvec from the current offset to the end
            let slice = &store[offset..];

            let (decoded, skip) = u32::rice_decode::<5, _>(slice).expect("failed to decode");

            assert_eq!(decoded, expected, "mismatch at offset {offset}");
            offset += skip;
        }

        // ensure we perfectly consumed the entire bitstream
        assert_eq!(
            offset,
            store.len(),
            "stream length mismatch after full decode"
        );
        store.clear();
    }

    #[test]
    fn encode_quotient() {
        let mut store: BitVec<usize> = BitVec::new();

        struct TestCase {
            value: u64,
            pattern: Vec<bool>,
        }

        let cases = [
            TestCase {
                value: 0x0000000000000000,
                pattern: vec![false], // 0b0
            },
            TestCase {
                value: 0x0000200000000000,
                pattern: vec![false], // 0b0
            },
            TestCase {
                value: 0x0000400000001337,
                pattern: vec![true, false], // 0b01
            },
            TestCase {
                value: 0x0000C00000ABCDEF,
                pattern: vec![true, true, true, false], // 0b0111
            },
            TestCase {
                value: 0x0004000000000123,
                pattern: std::iter::repeat_n(true, 16)
                    .chain(std::iter::once(false))
                    .collect(), // 0b01111111111111111
            },
        ];

        for test_case in cases {
            test_case.value.rice_encode_quotient::<46, _>(&mut store);

            for (i, expected_bit) in test_case.pattern.into_iter().enumerate() {
                assert_eq!(expected_bit, *store.get(i).unwrap());
            }

            store.clear();
        }
    }

    #[test]
    fn encode_remainder() {
        let mut store: BitVec<usize> = BitVec::new();

        struct TestCase {
            value: u64,
            expected_remainder: u64,
        }

        let cases = [
            TestCase {
                value: 0x0000000000000000,
                expected_remainder: 0x000000000000,
            },
            TestCase {
                value: 0x0000200000000000,
                expected_remainder: 0x200000000000,
            },
            TestCase {
                value: 0x0000400000001337,
                expected_remainder: 0x000000001337,
            },
            TestCase {
                value: 0x0000C00000ABCDEF,
                expected_remainder: 0x000000ABCDEF,
            },
            TestCase {
                value: 0x0004000000000123,
                expected_remainder: 0x000000000123,
            },
        ];

        for test_case in cases {
            test_case.value.rice_encode_remainder::<46, _>(&mut store);

            assert_eq!(store.len(), 46, "remainder must be exactly 46 bits");

            for i in 0..46 {
                let expected_bit = (test_case.expected_remainder & (1u64 << i)) != 0;

                assert_eq!(
                    expected_bit,
                    *store.get(i).unwrap(),
                    "mismatch at bit {i} for value {:#018X}",
                    test_case.value
                );
            }

            store.clear();
        }
    }

    #[test]
    fn unary() {
        let x = 0u64.to_unary();
        assert_eq!(x, 0b0, "got {x:b}");

        let x = 1u64.to_unary();
        assert_eq!(x, 0b1, "got {x:b}");

        let x = 2u64.to_unary();
        assert_eq!(x, 0b11, "got {x:b}");

        let x = 3u64.to_unary();
        assert_eq!(x, 0b111, "got {x:b}");

        let x = 4u64.to_unary();
        assert_eq!(x, 0b1111, "got {x:b}");

        let x = 63u64.to_unary();
        assert_eq!(x, !(1 << 63), "got {x:b}");
    }
}
