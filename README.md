# chunkrs

[![Crates.io](https://img.shields.io/crates/v/chunkrs)](https://crates.io/crates/chunkrs) [![Documentation](https://docs.rs/chunkrs/badge.svg)](https://docs.rs/chunkrs) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE) [![Rust Version](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://blog.rust-lang.org/2024/02/28/Rust-1.85.0.html) [![Unsafe Forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

> **Deterministic, streaming Content-Defined Chunking (CDC) for Rust**

`chunkrs` provides byte-stream chunking for delta synchronization, deduplication, and content-addressable storage. It prioritizes **correctness, determinism, and composability** over clever parallelism tricks.

**Core principle**: *CDC is inherently serial—parallelize at the application level, not within the stream.*

## Features

- **Streaming-first**: Processes multi-GB files with constant memory (no full-file buffering)
- **Deterministic-by-design**: Identical bytes always produce identical chunk hashes, regardless of batching or execution timing
- **Zero-allocation hot path**: Thread-local buffer pools eliminate allocator contention under load
- **FastCDC algorithm**: Gear hash rolling boundary detection with configurable min/avg/max sizes
- **BLAKE3 identity**: Cryptographic chunk hashing (optional, incremental)
- **Runtime-agnostic async**: Works with Tokio, async-std, or any `futures-io` runtime
- **Strictly safe**: `#![forbid(unsafe_code)]`

## When to Use chunkrs

| Scenario | Recommendation |
|----------|---------------|
| Delta sync (rsync-style) | ✅ Perfect fit |
| Backup tools | ✅ Ideal for single-stream chunking |
| Deduplication (CAS) | ✅ Use with your own index |
| NVMe Gen4/5 saturation | ✅ 3–5 GB/s per core |
| Distributed dedup | ✅ Stateless, easy to distribute |
| Any other CDC use case | ✅ Likely fits |

## Architecture

chunkrs processes **one logical byte stream at a time** with strictly serial CDC state:

```text
┌───────────────┐     ┌──────────────┐      ┌──────────────────┐ 
│ Input Byte    │     │ I/O Batching │      │ Serial CDC State │
│ Stream        │────▶│ (8KB buffers│────▶ │ Machine          │ 
│ (any io::Read │     │  for syscall │      │ (FastCDC rolling │ 
│  or AsyncRead)│     │  efficiency) │      │   hash)          │             
└───────────────┘     └──────────────┘      └──────────────────┘ 

    ┌─────────────┐       ┌───────────────────┐
    │             │       │ Chunk {           │
──▶ │ Chunk      │────▶  │   data: Bytes,    │
    │ Stream      │       │   offset: u64,    │
    │             │       │   hash: ChunkHash │
    └─────────────┘       │ }                 │
                          └───────────────────┘   
```

## Quick Start

```toml
[dependencies]
chunkrs = "0.8"
```

```rust
use std::fs::File;
use chunkrs::{Chunker, ChunkConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("data.bin")?;
    let chunker = Chunker::new(ChunkConfig::default());

    for chunk in chunker.chunk(file) {
        let chunk = chunk?;
        println!("offset: {:?}, len: {}, hash: {:?}", 
            chunk.offset, chunk.len(), chunk.hash);
    }
    
    Ok(())
}
```

**What's in the Chunk Stream:**

Each element is a `Chunk` containing:

- **`data`**: `Bytes` — the actual chunk payload (zero-copy reference when possible) for subsequent use (e.g., writing to disk)
- **`offset`**: `Option<u64>` — byte position in the original stream
- **`hash`**: `Option<ChunkHash>` — BLAKE3 hash for content identity (if enabled)

## API Overview

### Core Types

| Type | Description |
|------|-------------|
| `Chunker` | Stateful CDC engine (maintains rolling hash across batches) |
| `Chunk` | Content-addressed block with `Bytes` payload and optional BLAKE3 hash |
| `ChunkHash` | 32-byte BLAKE3 hash identifying chunk content |
| `ChunkConfig` | Min/avg/max chunk sizes and hash configuration |
| `ChunkIter` | Iterator over chunks (sync) |
| `ChunkError` | Error type for chunking operations |

### Synchronous Usage

```rust
use chunkrs::{Chunker, ChunkConfig};

// From file
let file = std::fs::File::open("data.bin")?;
let chunker = Chunker::new(ChunkConfig::default());
for chunk in chunker.chunk(file) {
    let chunk = chunk?;
    // chunk.data: Bytes - the chunk payload
    // chunk.offset: Option<u64> - position in original stream
    // chunk.hash: Option<ChunkHash> - BLAKE3 hash (if enabled)
}

// From memory
let data: Vec<u8> = vec![0u8; 1024 * 1024];
let chunks: Vec<_> = chunker.chunk_bytes(data);
```

### Asynchronous Usage

Runtime-agnostic via `futures-io`:

```rust
use futures_util::StreamExt;
use chunkrs::{ChunkConfig, ChunkError};

async fn process<R: futures_io::AsyncRead + Unpin>(reader: R) -> Result<(), ChunkError> {
    let mut stream = chunkrs::chunk_async(reader, ChunkConfig::default());
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        // Process
    }
    Ok(())
}
```

**Tokio compatibility:**

```rust
use tokio::fs::File;
use tokio_util::compat::TokioAsyncReadCompatExt;

let file = File::open("data.bin").await?;
let stream = chunkrs::chunk_async(file.compat(), ChunkConfig::default());
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

- Constant: `O(batch_size)` typically 4–16MB per stream
- Thread-local cache: ~64MB per thread (reusable)

**To saturate NVMe Gen5:**
Process multiple files concurrently (application-level parallelism). Do not attempt to parallelize within a single file—this destroys deduplication ratios.

## Determinism Guarantees

chunkrs guarantees **content-addressable identity**:

- **Strong guarantee**: Identical byte streams produce identical `ChunkHash` (BLAKE3) values
- **Boundary stability**: For identical inputs and configurations, chunk boundaries are deterministic across different batch sizes or execution timings
- **Serial consistency**: Rolling hash state is strictly maintained across batch boundaries

**What this means:**
You can re-chunk a file on Tuesday with different I/O batch sizes and get bit-identical chunks to Monday's run. This is essential for delta sync correctness.

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
| `async-io` | Async `Stream` support via `futures-io` | ❌ |

```toml
# Default: sync + hashing
[dependencies]
chunkrs = "0.8"

# Minimal: sync only, no hashing
[dependencies]
chunkrs = { version = "0.8", default-features = false }

# Full featured: sync + async + hashing
[dependencies]
chunkrs = { version = "0.8", features = ["async-io"] }
```

## Roadmap

**Current:** 0.8.0 — Core API stable, comprehensive feature set, seeking production feedback.

### Implemented ✅

**Core Functionality:**

- FastCDC rolling hash, sync, async I/O, zero-copy, BLAKE3 hashing, thread-local buffer pools, deterministic chunking

**Quality & Safety:**

- 45 unit tests + 40 doctests, fuzzing, no `unsafe`
- documents and example
- benchmarks

### Planned Enhancements

**0.9.x — Production Hardening:**

- Extended cross-platform testing (Windows, macOS, Linux variants)
- Additional fuzzing targets for edge cases
- Miri validation for memory safety
- Performance profiling and optimization for specific workloads
- Enhanced error messages with context

**1.0.0 — Stable Release:**

- Alternative hash algorithms (xxHash for speed, SHA-256 for compatibility)
- Configurable buffer pool sizes for memory-constrained environments
- Custom allocator support for specialized use cases
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

**Reference**: [ARCHITECTURE.md](ARCHITECTURE.md) — Design and implementation details.

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
