use std::time::Instant;

use rice_golomb::{BitVec, Encoder};
use vers_vecs::EliasFanoVec;

const N: usize = 1 << 18;
const MAX_BITS: u32 = 46;
const Q: usize = 10_000;
const BATCH_SIZES: &[usize] = &[1, 10, 100, 1000, 10_000, 50_000, 100_000, N];

struct RiceStats {
    total_bytes: usize,
    varint_bytes: usize,
    bit_bytes: usize,
    bit_len: usize,
}

struct EliasFanoStats {
    total_bytes: usize,
}

fn encode_rice_golomb(values: &[u64]) -> (RiceStats, Vec<u8>) {
    let deltas: Vec<u64> = std::iter::once(0)
        .chain(values.iter().copied())
        .zip(values.iter().copied())
        .map(|(prev, next)| next - prev)
        .collect();

    let mut store = BitVec::<u64>::new();
    for &delta in &deltas {
        Encoder::<u64, MAX_BITS>::encode(&delta, &mut store);
    }

    let bit_len = store.len();
    let needed_bytes = bit_len.div_ceil(8);
    let limbs = store.into_vec();

    let mut buf = Vec::with_capacity(10 + needed_bytes);

    let mut varint_buf = [0u8; 10];
    let varint_bytes = tiny_varint::encode(bit_len as u64, &mut varint_buf).unwrap();
    buf.extend_from_slice(&varint_buf[..varint_bytes]);

    let limb_bytes: Vec<u8> = limbs.iter().flat_map(|l| l.to_le_bytes()).collect();
    buf.extend_from_slice(&limb_bytes[..needed_bytes]);

    let stats = RiceStats {
        total_bytes: buf.len(),
        varint_bytes,
        bit_bytes: needed_bytes,
        bit_len,
    };

    (stats, buf)
}

fn reconstruct_bitvec(buf: &[u8]) -> (BitVec<u64>, usize) {
    let (bit_len, varint_bytes) = tiny_varint::decode::<u64>(buf).unwrap();
    let bit_len = bit_len as usize;
    let needed_bytes = bit_len.div_ceil(8);

    let limb_u64_count = bit_len.div_ceil(64);
    let total_limb_bytes = limb_u64_count * 8;

    let mut padded = vec![0u8; total_limb_bytes];
    padded[..needed_bytes].copy_from_slice(&buf[varint_bytes..varint_bytes + needed_bytes]);

    let limbs: Vec<u64> = padded
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
        .collect();

    let store = BitVec::<u64>::from_vec(limbs);

    (store, bit_len)
}

fn decode_rice_golomb(buf: &[u8]) -> Vec<u64> {
    let (store, bit_len) = reconstruct_bitvec(buf);
    let bits = &store.as_bitslice()[..bit_len];

    let mut result = Vec::with_capacity(N);
    let mut ptr = 0;
    let mut last: u64 = 0;

    while let Some((decoded_diff, skip)) = bits
        .get(ptr..)
        .and_then(Encoder::<u64, MAX_BITS>::decode_and_skip)
    {
        let decoded = decoded_diff + last;
        result.push(decoded);
        ptr += skip;
        last = decoded;
    }

    result
}

fn query_rice_golomb_batch(buf: &[u8], queries: &[u64]) -> usize {
    let (store, bit_len) = reconstruct_bitvec(buf);
    let bits = &store.as_bitslice()[..bit_len];

    let mut found = 0;
    let mut query_idx = 0;
    let mut ptr = 0;
    let mut last: u64 = 0;

    while query_idx < queries.len() {
        let Some((decoded_diff, skip)) = bits
            .get(ptr..)
            .and_then(Encoder::<u64, MAX_BITS>::decode_and_skip)
        else {
            break;
        };

        let decoded = decoded_diff + last;
        ptr += skip;
        last = decoded;

        while query_idx < queries.len() && queries[query_idx] < decoded {
            query_idx += 1;
        }

        if query_idx < queries.len() && queries[query_idx] == decoded {
            found += 1;
            query_idx += 1;
        }
    }

    found
}

fn encode_elias_fano(values: &[u64]) -> (EliasFanoStats, Vec<u8>) {
    let ef = EliasFanoVec::from_slice(values);
    let bytes = bincode::serialize(&ef).unwrap();

    let stats = EliasFanoStats {
        total_bytes: bytes.len(),
    };

    (stats, bytes)
}

fn decode_elias_fano(bytes: &[u8]) -> EliasFanoVec {
    bincode::deserialize(bytes).unwrap()
}

fn query_elias_fano(set: &EliasFanoVec, queries: &[u64]) -> usize {
    let mut found = 0;

    for &elem in queries {
        if set.predecessor(elem).is_some_and(|got| got == elem) {
            let _rank = set.rank(elem);
            found += 1;
        }
    }

    found
}

fn main() {
    let mut values: Vec<u64> = (0..N).map(|_| rand::random::<u64>()).collect();
    values.sort_unstable();

    for i in 1..values.len() {
        assert!(
            values[i - 1] < values[i],
            "duplicate value detected at index {i}; re-run the benchmark"
        );
    }

    let max_value = *values.last().unwrap();
    let original_size = N * 8;

    let t0 = Instant::now();
    let (rice_stats, rice_buf) = encode_rice_golomb(&values);
    let rice_build = t0.elapsed();

    let t1 = Instant::now();
    let decoded_rice = decode_rice_golomb(&rice_buf);
    let rice_decode = t1.elapsed();

    let t2 = Instant::now();
    let (ef_stats, ef_buf) = encode_elias_fano(&values);
    let ef_build = t2.elapsed();

    let t3 = Instant::now();
    let decoded_ef = decode_elias_fano(&ef_buf);
    let ef_wire_decode = t3.elapsed();

    let queries: Vec<u64> = (0..Q)
        .map(|_| values[(rand::random::<u32>() % N as u32) as usize])
        .collect();

    let t4 = Instant::now();
    let found = query_elias_fano(&decoded_ef, &queries);
    let ef_query = t4.elapsed();

    assert_eq!(
        found, Q,
        "elias-fano query: not all present values were found"
    );

    assert_eq!(decoded_rice, values, "rice-golomb round-trip failed");
    assert_eq!(decoded_ef.len(), N, "elias-fano length mismatch");
    assert_eq!(
        decoded_ef.get(0),
        Some(values[0]),
        "elias-fano get(0) failed"
    );
    assert_eq!(
        decoded_ef.get(N - 1),
        Some(*values.last().unwrap()),
        "elias-fano get(N-1) failed"
    );

    eprintln!("dataset:    N = {N} values, max = {max_value:#018x}");
    eprintln!();
    eprintln!(
        "original:   {original_size:>10} bytes  ({:.2} bits/value)",
        (original_size * 8) as f64 / N as f64
    );
    eprintln!(
        "rice-golomb:{:>10} bytes  ({:.2} bits/value)  [varint: {} B, bitstream: {} B ({} bits)]",
        rice_stats.total_bytes,
        (rice_stats.total_bytes * 8) as f64 / N as f64,
        rice_stats.varint_bytes,
        rice_stats.bit_bytes,
        rice_stats.bit_len,
    );
    eprintln!(
        "elias-fano: {:>10} bytes  ({:.2} bits/value)",
        ef_stats.total_bytes,
        (ef_stats.total_bytes * 8) as f64 / N as f64
    );
    eprintln!();
    eprintln!(
        "rice-golomb ratio: {:.4}x  ({:.2}% of original)",
        rice_stats.total_bytes as f64 / original_size as f64,
        rice_stats.total_bytes as f64 / original_size as f64 * 100.0
    );
    eprintln!(
        "elias-fano ratio:  {:.4}x  ({:.2}% of original)",
        ef_stats.total_bytes as f64 / original_size as f64,
        ef_stats.total_bytes as f64 / original_size as f64 * 100.0
    );
    eprintln!();
    eprintln!("timing:");
    eprintln!(
        "  rice-golomb build:       {:.2} ms",
        rice_build.as_secs_f64() * 1000.0
    );
    eprintln!(
        "  rice-golomb full decode: {:.2} ms  (full value reconstruction)",
        rice_decode.as_secs_f64() * 1000.0
    );
    eprintln!(
        "  elias-fano build:        {:.2} ms",
        ef_build.as_secs_f64() * 1000.0
    );
    eprintln!(
        "  elias-fano wire decode:  {:.2} ms  (deserialize to in-memory structure)",
        ef_wire_decode.as_secs_f64() * 1000.0
    );
    eprintln!(
        "  elias-fano queries:      {:.2} ms  ({} random access: predecessor + rank, ~{:.2} µs/query)",
        ef_query.as_secs_f64() * 1000.0,
        Q,
        ef_query.as_secs_f64() * 1_000_000.0 / Q as f64,
    );

    eprintln!();
    eprintln!("crossover analysis (rice-golomb batch scan vs elias-fano wire decode + queries):");
    eprintln!(
        "  {:>8}  {:>14}  {:>14}  winner",
        "Q", "rice-golomb", "elias-fano"
    );
    eprintln!(
        "  {:>8}  {:>14}  {:>14}  ---",
        "---", "(cold scan)", "(wire + query)"
    );

    for &batch_q in BATCH_SIZES {
        let mut batch_queries: Vec<u64> = (0..batch_q)
            .map(|_| values[(rand::random::<u32>() % N as u32) as usize])
            .collect();
        batch_queries.sort_unstable();
        batch_queries.dedup();

        let t = Instant::now();
        let found_rice = query_rice_golomb_batch(&rice_buf, &batch_queries);
        let rice_batch_time = t.elapsed();

        let t = Instant::now();
        let found_ef = query_elias_fano(&decoded_ef, &batch_queries);
        let ef_query_time = t.elapsed();
        let ef_cold = ef_wire_decode + ef_query_time;

        assert_eq!(
            found_rice,
            batch_queries.len(),
            "rice batch mismatch at Q={batch_q}"
        );
        assert_eq!(
            found_ef,
            batch_queries.len(),
            "ef batch mismatch at Q={batch_q}"
        );

        let winner = if rice_batch_time < ef_cold {
            "rice-golomb"
        } else {
            "elias-fano"
        };

        eprintln!(
            "  {:>8}  {:>10.2} ms  {:>10.2} ms  {}",
            batch_queries.len(),
            rice_batch_time.as_secs_f64() * 1000.0,
            ef_cold.as_secs_f64() * 1000.0,
            winner,
        );
    }
}
