# chunkrs

[![Crates.io](https://img.shields.io/crates/v/chunkrs)](https://crates.io/crates/chunkrs) [![Documentation](https://docs.rs/chunkrs/badge.svg)](https://docs.rs/chunkrs) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE) [![Rust Version](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://blog.rust-lang.org/2024/02/28/Rust-1.85.0.html) [![Unsafe Forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

> **Deterministic, streaming Content-Defined Chunking (CDC) for Rust**

`chunkrs` is a high-performance, portable infrastructure library for FastCDC chunking and cryptographic hashing.

> **Bytes in → Chunks & hashes out.**

Zero-copy streaming. Async-agnostic. Excellent for any chunking and hashing use case.

## Features

- **Streaming API**: `push()`/`finish()` pattern for processing data in any batch size
- **Deterministic-by-design**: Identical bytes produce identical chunk boundaries and hashes, regardless of batching or execution timing
- **Zero-copy**: Efficient `Bytes` slicing from input with minimal allocations
- **FastCDC algorithm**: Byte-by-byte gear hash rolling with configurable min/avg/max sizes
- **BLAKE3 identity**: Cryptographic chunk hashing (optional, feature-gated)
- **Strictly safe**: `#![forbid(unsafe_code)]` - zero unsafe code throughout
- **Minimal API**: Only 6 public types - `Chunker`, `Chunk`, `ChunkHash`, `ChunkConfig`, `HashConfig`, `ChunkError`
- **Well-tested**: Comprehensive unit tests, integration tests, and fuzzing

## Recent Improvements

- **Optimized Bytes handling**: Eliminated unnecessary allocations in examples and tests
- **Consolidated helpers**: Extracted duplicate code into reusable helper methods
- **Simplified architecture**: Removed unused hasher state for better performance
- **Fixed examples**: All examples now use random data for realistic testing
- **Removed duplicates**: Consolidated duplicate code and fuzz targets
- **Clean warnings**: All compiler warnings resolved

## Architecture

chunkrs processes **one logical byte stream at a time** with byte-by-byte serial CDC:

```text
┌───────────────┐     ┌──────────────────┐      ┌──────────────────┐ 
│ Input Bytes   │     │ Push-based       │      │ Serial CDC State │
│ (any source)  │────▶│ Streaming API    │────▶ │ (FastCDC rolling │ 
│               │     │ push()/finish()  │      │   hash, byte-by- │             
└───────────────┘     └──────────────────┘      │   byte)          │ 
                                                     └──────────────────┘ 
    ┌─────────────┐       ┌───────────────────┐
    │             │       │ Chunk {           │
──▶ │ Chunk      │────▶  │   data: Bytes,    │
    │ Stream      │       │   offset: u64,    │
    │             │       │   hash: ChunkHash │
    └─────────────┘       │ }                 │
                          └───────────────────┘   
```

## When to Use chunkrs

| Scenario | Recommendation |
|----------|---------------|
| Delta sync (rsync-style) | ✅ Perfect fit |
| Backup tools | ✅ Ideal for single-stream chunking |
| Deduplication (CAS) | ✅ Use with your own index |
| NVMe Gen4/5 saturation | ✅ 3–5 GB/s per core |
| Distributed dedup | ✅ Stateless, easy to distribute |
| Any other CDC use case | ✅ Likely fits |

## Quick Start

```toml
[dependencies]
chunkrs = "0.8"
```

```rust
use chunkrs::{Chunker, ChunkConfig};
use bytes::Bytes;

fn main() {
    let mut chunker = Chunker::new(ChunkConfig::default());
    let mut pending = Bytes::new();

    // Feed data in any size (streaming)
    for chunk in &[Bytes::from(&b"first part"[..]), 
                    Bytes::from(&b"second part"[..])] {
        let (chunks, leftover) = chunker.push(chunk);
        // Process complete chunks...
        for chunk in chunks {
            println!("offset: {:?}, len: {}, hash: {:?}", 
                chunk.offset, chunk.len(), chunk.hash);
        }
        pending = leftover;
    }

    // Finalize stream
    if let Some(final_chunk) = chunker.finish() {
        println!("Final chunk: offset: {:?}, len: {}, hash: {:?}", 
            final_chunk.offset, final_chunk.len(), final_chunk.hash);
    }
}
```

**What's in a Chunk:**

Each `Chunk` contains:

- **`data`**: `Bytes` — the actual chunk payload (zero-copy reference when possible)
- **`offset`**: `Option<u64>` — byte position in the original stream
- **`hash`**: `Option<ChunkHash>` — BLAKE3 hash for content identity (if enabled)

## API Overview

### Core Types

| Type | Description |
|------|-------------|
| `Chunker` | Stateful CDC engine with streaming push()/finish() API |
| `Chunk` | Content-addressed block with `Bytes` payload and optional BLAKE3 hash |
| `ChunkHash` | 32-byte BLAKE3 hash identifying chunk content |
| `ChunkConfig` | Min/avg/max chunk sizes and hash configuration |
| `ChunkError` | Error type for chunking operations |

### Streaming API

The `Chunker` provides a streaming API:

```rust
use chunkrs::{Chunker, ChunkConfig};
use bytes::Bytes;

let mut chunker = Chunker::new(ChunkConfig::default());
let mut pending = Bytes::new();

// Feed data in any size (1 byte to megabytes)
let (chunks, leftover) = chunker.push(Bytes::from(&b"data"[..]));

// Process complete chunks immediately
for chunk in chunks {
    // chunk.data: Bytes - the chunk payload
    // chunk.offset: Option<u64> - position in original stream
    // chunk.hash: Option<ChunkHash> - BLAKE3 hash (if enabled)
}

// Feed leftover back in next push
pending = leftover;

// When stream ends, get final chunk
if let Some(final_chunk) = chunker.finish() {
    // Process final chunk
}
```

### Determinism

The same input produces identical chunks regardless of how data is fed:

```rust
let data: Vec<u8> = vec![0u8; 10000];

// All at once
let mut chunker1 = Chunker::new(ChunkConfig::default());
let (chunks1, _) = chunker1.push(Bytes::from(data.clone()));
let final1 = chunker1.finish();

// In 100-byte chunks
let mut chunker2 = Chunker::new(ChunkConfig::default());
let mut all_chunks2 = Vec::new();
for chunk in data.chunks(100) {
    let (chunks, _) = chunker2.push(Bytes::from(chunk));
    all_chunks2.extend(chunks);
}
let final2 = chunker2.finish();

// Same chunks, same hashes
assert_eq!(chunks1.len() + final1.is_some() as usize, 
           all_chunks2.len() + final2.is_some() as usize);
```

## Configuration

### Chunk Sizes

Choose based on your deduplication granularity needs:

```rust
use chunkrs::ChunkConfig;

// Small files / high dedup (8 KiB average)
let small = ChunkConfig::new(2 * 1024, 8 * 1024, 32 * 1024)?;

// Default (16 KiB average) - good general purpose
let default = ChunkConfig::default();

// Large files / high throughput (256 KiB average)  
let large = ChunkConfig::new(64 * 1024, 256 * 1024, 1024 * 1024)?;
```

### Hash Configuration

```rust
use chunkrs::{ChunkConfig, HashConfig};

// With BLAKE3 (default)
let with_hash = ChunkConfig::default();

// Boundary detection only (faster, no content identity)
let no_hash = ChunkConfig::default().with_hash_config(HashConfig::disabled());
```

## Performance

**Throughput targets on modern hardware:**

| Storage | Single-core CDC | Bottleneck |
|---------|----------------|------------|
| NVMe Gen4 | ~3–5 GB/s | CPU (hashing) |
| NVMe Gen5 | ~3–5 GB/s | CDC algorithm |
| SATA SSD | ~500 MB/s | Storage |
| 10 Gbps LAN | ~1.2 GB/s | Network |
| HDD | ~200 MB/s | Seek latency |

**Memory usage:**

- Per stream: `O(pending_bytes)` - typically minimal as pending is flushed on boundaries
- Zero-copy: Chunk data references input `Bytes` without copying
- Caller controls memory management (buffer pools, reuse, etc.)

**To saturate NVMe Gen5:**
Process multiple files concurrently by running multiple `Chunker` instances. Do not attempt to parallelize within a single file—this destroys deduplication ratios.

## Determinism Guarantees

chunkrs guarantees **exact determinism**:

- **Boundary determinism**: Identical byte streams produce identical chunk boundaries at identical byte positions
- **Hash determinism**: Identical byte streams produce identical `ChunkHash` (BLAKE3) values
- **Batch independence**: Results are identical regardless of input batch sizes (1 byte vs 1MB vs streaming)
- **Serial consistency**: Rolling hash state is strictly maintained across all `push()` calls

**What this means:**
You can re-chunk a file on Tuesday with different batch sizes and get bit-identical chunks to Monday's run. This is essential for delta sync correctness.

## Safety & Correctness

- **No unsafe code**: `#![forbid(unsafe_code)]`
- **Comprehensive testing**: Unit tests, doc tests, and property-based tests ensure:
  - Determinism invariants
  - Batch equivalence (chunking whole vs chunked yields same results)
  - No panics on edge cases (empty files, single byte, max-size boundaries)

## Algorithm

**Boundary Detection**: [FastCDC](https://www.usenix.org/conference/atc16/technical-sessions/presentation/xia) (Gear hash rolling hash)

- Byte-by-byte polynomial rolling hash via lookup table
- Dual-mask normalization (small/large chunk detection)
- Configurable min/avg/max constraints

**Chunk Identity**: BLAKE3 (when enabled)

- Incremental hashing for streaming
- 32-byte cryptographic digests

## Cargo Features

| Feature | Description | Default |
|---------|-------------|---------|
| `hash-blake3` | BLAKE3 chunk hashing | ✅ |

```toml
# Default: sync + hashing
[dependencies]
chunkrs = "0.8"

# Minimal: sync only, no hashing
[dependencies]
chunkrs = { version = "0.8", default-features = false }
```

## Roadmap

**Current:** 0.8.0 — Core API stable, comprehensive feature set, seeking production feedback.

```text
Note: bumped version to 0.8.0 because design, APIs, features are almost matured.
```

### Implemented ✅

**Core Functionality:**

- FastCDC rolling hash, push/finish streaming API, zero-copy, BLAKE3 hashing, deterministic chunking

**Quality & Safety:**

- Comprehensive unit tests + doctests, fuzzing, no `unsafe`
- Documentation and examples
- Benchmarks

### Planned Enhancements

**0.9.x — Production Hardening:**

- Extended cross-platform testing (Windows, macOS, Linux variants)
- Additional fuzzing targets for edge cases
- Miri validation for memory safety
- Performance profiling and optimization for specific workloads
- Enhanced error messages with context

**1.0.0 — Stable Release:**

- Alternative hash algorithms (xxHash for speed, SHA-256 for compatibility)
- Formal SemVer commitment with MSRV policy
- Comprehensive integration guide and production deployment patterns

**Post-1.0 — Additive Features Only:**

- SIMD optimizations (AVX2/AVX-512) for rolling hash
- Hardware-accelerated hashing (BLAKE3 SIMD, SHA-NI)
- Advanced CDC algorithm variants (e.g., pattern-aware chunking)
- `no_std` support for embedded environments

### Non-Goals

These features are intentionally out of scope:

- **Networking**: Handle in application layer
- **Encryption**: Pre-encrypt or post-encrypt at application layer
- **Compression**: Apply compression before or after chunking
- **Deduplication indexing**: Use companion crates (CAS index implementations)
- **Distributed coordination**: Manage at application level

### Feedback & Contributions

We're actively seeking feedback on:

- Real-world deployment patterns and performance characteristics
- Edge cases and failure modes in production
- Integration patterns with storage systems and databases
- Feature requests that align with CDC use cases

Open issues or discussions at [GitHub Issues](https://github.com/elemeng/chunkrs/issues). Issues and pull requests are welcome.

- Refer [ARCHITECTURE.md](ARCHITECTURE.md) for **Design** and **implementation** details.
- See [CHANGELOG.md](CHANGELOG.md) for version history.

## Acknowledgments

This crate implements the FastCDC algorithm described in:

> Wen Xia, Yukun Zhou, Hong Jiang, Dan Feng, Yu Hua, Yuchong Hu, Yuchong Zhang, Qing Liu,  
> **"FastCDC: a Fast and Efficient Content-Defined Chunking Approach for Data Deduplication"**,  
> in Proceedings of USENIX Annual Technical Conference (USENIX ATC'16), Denver, CO, USA, June 22–24, 2016, pages: 101-114.  
> [Paper Link](https://www.usenix.org/conference/atc16/technical-sessions/presentation/xia)

> Wen Xia, Xiangyu Zou, Yukun Zhou, Hong Jiang, Chuanyi Liu, Dan Feng, Yu Hua, Yuchong Hu, Yuchong Zhang,  
> **"The Design of Fast Content-Defined Chunking for Data Deduplication based Storage Systems"**,  
> IEEE Transactions on Parallel and Distributed Systems (TPDS), 2020.

This crate is inspired by the original [fastcdc](https://crates.io/crates/fastcdc) crate but focuses on a modernized API with streaming-first design, strict determinism, and allocation-conscious internals.

## License

MIT License — see [LICENSE](LICENSE)
