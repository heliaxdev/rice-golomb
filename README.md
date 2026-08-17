# rice-golomb

A small Rust library for Rice-Golomb coding, a form of entropy coding used in
lossless compression. It works well on values whose magnitude tends to be
small, which makes it a common choice for residual signals in audio, image,
and other media codecs.

## Usage

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
rice-golomb = "0.1"
```

Then encode and decode a value, picking a remainder width that fits the
distribution of your data. Wider remainders favour larger values; narrower
ones favour smaller ones.

```rust
use rice_golomb::{Encoder, BitVec};

let mut store: BitVec<usize> = BitVec::new();

Encoder::<u32, 5>::encode(&42, &mut store);

let decoded: u32 = Encoder::<u32, 5>::decode(&store).unwrap();
assert_eq!(decoded, 42);
```

## Status

This is an early, minimal release intended for experimentation and internal
use at Heliax. The API may change between versions.

## License

MIT, see `LICENSE`.
