# chunkrs

[![Crates.io](https://img.shields.io/crates/v/chunkrs)](https://crates.io/crates/chunkrs)
[![Documentation](https://docs.rs/chunkrs/badge.svg)](https://docs.rs/chunkrs)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://blog.rust-lang.org/2024/02/28/Rust-1.85.0.html)

> Streaming Content-Defined Chunking (CDC) for Rust

`chunkrs` transforms byte streams into content-defined chunks with optional strong hashes. Designed as a small, composable primitive for delta synchronization, deduplication, backup systems, and content-addressable storage.

## Features

- **Streaming-first**: Operates on byte streams without full-file buffering
- **Zero-copy friendly**: Uses `bytes::Bytes` for efficient buffer management  
- **Deterministic**: Identical inputs produce identical chunk hashes
- **FastCDC algorithm**: Gear hash-based rolling hash for boundary detection
- **BLAKE3 hashes**: Cryptographic chunk identity (optional)
- **Async support**: Runtime-agnostic via `futures-io`
- **No unsafe code**: `#![forbid(unsafe_code)]`

## Quick Start

```toml
[dependencies]
chunkrs = "0.1"
```

```rust
use std::fs::File;
use chunkrs::{Chunker, ChunkConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("data.bin")?;
    let chunker = Chunker::new(ChunkConfig::default());

    for chunk in chunker.chunk(file) {
        let chunk = chunk?;
        println!("chunk: {} bytes, hash: {:?}", chunk.len(), chunk.hash);
    }
    
    Ok(())
}
```

## Usage

### Synchronous

```rust
use std::fs::File;
use chunkrs::{Chunker, ChunkConfig};

let file = File::open("data.bin")?;
let chunker = Chunker::new(ChunkConfig::default());

for chunk in chunker.chunk(file) {
    let chunk = chunk?;
    println!("chunk: {} bytes", chunk.len());
}
```

### Asynchronous

```rust
use futures_util::StreamExt;
use chunkrs::{chunk_async, ChunkConfig};

async fn process<R: futures_io::AsyncRead + Unpin>(reader: R) -> Result<(), chunkrs::ChunkError> {
    let mut stream = chunk_async(reader, ChunkConfig::default());

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        println!("chunk: {} bytes", chunk.len());
    }
    
    Ok(())
}
```

## Configuration

```rust
use chunkrs::ChunkConfig;

let config = ChunkConfig::new(
    4 * 1024,   // min: 4 KiB
    16 * 1024,  // avg: 16 KiB  
    64 * 1024,  // max: 64 KiB
)?;

// Or use builder pattern
let config = ChunkConfig::default()
    .with_min_size(4096)
    .with_avg_size(16384)
    .with_max_size(65536);
```

## Cargo Features

| Feature | Description |
|---------|-------------|
| `hash-blake3` (default) | BLAKE3 hashing for chunk identity |
| `async-io` | Async streaming support via `futures-io` |

Disable default features for boundary detection only:

```toml
[dependencies]
chunkrs = { version = "0.1", default-features = false }
```

## Algorithm

- **Boundary Detection**: [FastCDC](https://www.usenix.org/conference/atc16/technical-sessions/presentation/xia) - Gear hash-based rolling hash
- **Chunk Identity**: BLAKE3 (when `hash-blake3` enabled)

## Minimum Supported Rust Version (MSRV)

1.85.0

## License

Licensed under the [MIT License](LICENSE).

## Contributing

Issues and pull requests are welcome at [https://github.com/elemeng/chunkrs](https://github.com/elemeng/chunkrs).
