use rice_golomb::{BitVec, Encoder};
use vers_vecs::EliasFanoVec;

const N: usize = 1 << 18;
const MAX_BITS: u32 = 46;

struct RiceStats {
    total_bytes: usize,
    varint_bytes: usize,
    limb_bytes: usize,
    limb_count: usize,
    bit_len: usize,
}

struct EliasFanoStats {
    total_bytes: usize,
}

fn main() {
    let values = {
        let mut values: Vec<u64> = (0..N).map(|_| rand::random::<u64>()).collect();
        values.sort_unstable();
        values
    };

    for i in 1..values.len() {
        assert!(
            values[i - 1] < values[i],
            "duplicate value detected at index {i}; re-run the benchmark"
        );
    }

    let max_value = *values.last().unwrap();
    let original_size = N * 8;

    let rice = encode_rice_golomb(&values);
    let ef = encode_elias_fano(&values);

    eprintln!("dataset:    N = {N} values, max = {max_value:#018x}");
    eprintln!();
    eprintln!(
        "original:   {original_size:>10} bytes  ({:.2} bits/value)",
        (original_size * 8) as f64 / N as f64
    );
    eprintln!(
        "rice-golomb:{:>10} bytes  ({:.2} bits/value)  [varint: {} B, limbs: {} B ({} x u64), bitstream: {} bits]",
        rice.total_bytes,
        (rice.total_bytes * 8) as f64 / N as f64,
        rice.varint_bytes,
        rice.limb_bytes,
        rice.limb_count,
        rice.bit_len,
    );
    eprintln!(
        "elias-fano: {:>10} bytes  ({:.2} bits/value)",
        ef.total_bytes,
        (ef.total_bytes * 8) as f64 / N as f64
    );
    eprintln!();
    eprintln!(
        "rice-golomb ratio: {:.4}x  ({:.2}% of original)",
        rice.total_bytes as f64 / original_size as f64,
        rice.total_bytes as f64 / original_size as f64 * 100.0
    );
    eprintln!(
        "elias-fano ratio:  {:.4}x  ({:.2}% of original)",
        ef.total_bytes as f64 / original_size as f64,
        ef.total_bytes as f64 / original_size as f64 * 100.0
    );
}

fn encode_rice_golomb(values: &[u64]) -> RiceStats {
    let deltas = std::iter::once(0)
        .chain(values.iter().copied())
        .zip(values.iter().copied())
        .map(|(prev, next)| next - prev);

    let mut store = BitVec::<u64>::new();
    for delta in deltas {
        Encoder::<u64, MAX_BITS>::encode(&delta, &mut store);
    }

    let bit_len = store.len();
    let limbs = store.into_vec();

    let mut varint_buf = [0u8; 10];
    let varint_bytes = tiny_varint::encode(bit_len as u64, &mut varint_buf).unwrap();

    let limb_bytes = limbs.len() * 8;

    RiceStats {
        total_bytes: varint_bytes + limb_bytes,
        varint_bytes,
        limb_bytes,
        limb_count: limbs.len(),
        bit_len,
    }
}

fn encode_elias_fano(values: &[u64]) -> EliasFanoStats {
    let ef = EliasFanoVec::from_slice(values);
    let ef_bytes = bincode::serialize(&ef).unwrap();

    EliasFanoStats {
        total_bytes: ef_bytes.len(),
    }
}
