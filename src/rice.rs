use bitvec::order::Lsb0;
use bitvec::store::BitStore;
use bitvec::vec::BitVec;

use crate::encodable::{Encodable, LengthLimited};

#[allow(private_bounds)]
pub trait Rice: Sealed + Copy {
    fn rice_encode_quotient<const MAX_BITS: u32, S: BitStore>(self, store: &mut BitVec<S, Lsb0>);
    fn rice_encode_remainder<const MAX_BITS: u32, S: BitStore>(self, store: &mut BitVec<S, Lsb0>);

    fn rice_encode<const MAX_BITS: u32, S: BitStore>(self, store: &mut BitVec<S, Lsb0>) {
        self.rice_encode_quotient::<MAX_BITS, _>(store);
        self.rice_encode_remainder::<MAX_BITS, _>(store);
    }
}

trait RiceAux {
    fn to_unary(self) -> Self;

    fn quotient<const MAX_BITS: u32>(self) -> Self;
    fn remainder<const MAX_BITS: u32>(self) -> Self;
}

trait Sealed {}

macro_rules! impl_rice_int {
    ($int:ty) => {
        impl Sealed for $int {}

        impl RiceAux for $int {
            fn to_unary(self) -> Self {
                const HIGH_BIT: $int = 1 << const { <$int>::BITS - 1 };

                assert!((self & HIGH_BIT) != HIGH_BIT, "high bit cannot be set");

                (1 << self) - 1
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
                self & ((1 << MAX_BITS) - 1)
            }
        }

        impl Rice for $int {
            fn rice_encode_quotient<const MAX_BITS: u32, S: BitStore>(
                self,
                store: &mut BitVec<S, Lsb0>,
            ) {
                let binary_quotient = self.quotient::<MAX_BITS>();
                let unary_quotient = binary_quotient.to_unary();

                LengthLimited::limit(unary_quotient, (binary_quotient + 1) as usize)
                    .unwrap()
                    .encode(store);
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
