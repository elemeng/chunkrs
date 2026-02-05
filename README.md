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

## API Overview

### Core Types

| Type | Description |
|------|-------------|
| `Chunker` | Main entry point for chunking operations |
| `Chunk` | A single chunk with data, offset, and optional hash |
| `ChunkHash` | 32-byte BLAKE3 hash identifying chunk content |
| `ChunkConfig` | Configuration for chunk size bounds and hashing |
| `ChunkError` | Error type for chunking operations |

### Chunker Methods

#### Synchronous

```rust
use chunkrs::{Chunker, ChunkConfig};

let chunker = Chunker::new(ChunkConfig::default());

// From any io::Read implementation
let file = std::fs::File::open("data.bin")?;
for chunk in chunker.chunk(file) {
    let chunk = chunk?;
    // Process chunk
}

// From in-memory data
let data: Vec<u8> = vec![0u8; 1024 * 1024];
let chunks = chunker.chunk_bytes(&data);
```

#### Asynchronous

```rust
use futures_util::StreamExt;
use chunkrs::{chunk_async, ChunkConfig};

async fn process<R: futures_io::AsyncRead + Unpin>(reader: R) -> Result<(), chunkrs::ChunkError> {
    let mut stream = chunk_async(reader, ChunkConfig::default());

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        // Process chunk
    }
    
    Ok(())
}
```

## Detailed Usage

### Synchronous Chunking

#### From File

```rust
use std::fs::File;
use chunkrs::{Chunker, ChunkConfig, ChunkError};

fn chunk_file(path: &str) -> Result<Vec<chunkrs::Chunk>, ChunkError> {
    let file = File::open(path)?;
    let chunker = Chunker::new(ChunkConfig::default());
    
    let mut chunks = Vec::new();
    for chunk in chunker.chunk(file) {
        chunks.push(chunk?);
    }
    
    Ok(chunks)
}
```

#### From Memory

```rust
use chunkrs::{Chunker, ChunkConfig};

fn chunk_memory(data: &[u8]) -> Vec<chunkrs::Chunk> {
    let chunker = Chunker::new(ChunkConfig::default());
    chunker.chunk_bytes(data)
}
```

#### Custom Configuration

```rust
use chunkrs::{ChunkConfig, HashConfig};

// Large chunks for large files
let config = ChunkConfig::new(
    64 * 1024,    // min: 64 KiB
    256 * 1024,   // avg: 256 KiB
    1024 * 1024,  // max: 1 MiB
)?;

// Disable hashing (boundary detection only)
let config = ChunkConfig::default()
    .with_hash_config(HashConfig::disabled());
```

### Asynchronous Chunking

#### With Tokio

```rust
use futures_util::StreamExt;
use tokio::fs::File;
use tokio_util::compat::TokioAsyncReadCompatExt;
use chunkrs::{chunk_async, ChunkConfig};

async fn chunk_file_async(path: &str) -> Result<(), chunkrs::ChunkError> {
    // Open file with tokio
    let file = File::open(path).await
        .map_err(|e| chunkrs::ChunkError::Io(e))?;
    
    // Convert to futures_io::AsyncRead
    let reader = file.compat();
    
    let mut stream = chunk_async(reader, ChunkConfig::default());
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        println!("Chunk at offset {}: {} bytes", 
            chunk.offset.unwrap_or(0), 
            chunk.len()
        );
    }
    
    Ok(())
}
```

#### With Async-std

```rust
use futures_util::StreamExt;
use async_std::fs::File;
use chunkrs::{chunk_async, ChunkConfig};

async fn chunk_file_async(path: &str) -> Result<(), chunkrs::ChunkError> {
    let file = File::open(path).await
        .map_err(|e| chunkrs::ChunkError::Io(e))?;
    
    let mut stream = chunk_async(file, ChunkConfig::default());
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        // Process chunk
    }
    
    Ok(())
}
```

### Working with Chunks

```rust
use chunkrs::Chunk;

fn process_chunk(chunk: &Chunk) {
    // Access chunk data (bytes::Bytes)
    let data: &bytes::Bytes = &chunk.data;
    
    // Get chunk offset in original stream
    if let Some(offset) = chunk.offset {
        println!("Chunk starts at byte {}", offset);
    }
    
    // Get chunk hash (if hashing enabled)
    if let Some(hash) = chunk.hash {
        println!("Chunk hash: {}", hash.to_hex());
        
        // Compare hashes
        let other_hash = chunkrs::ChunkHash::from_hex(
            "abcd1234..."
        );
    }
    
    // Chunk length
    println!("Chunk size: {} bytes", chunk.len());
}
```

### Error Handling

```rust
use chunkrs::{ChunkConfig, Chunker, ChunkError};

fn chunk_with_error_handling() -> Result<(), ChunkError> {
    let file = std::fs::File::open("data.bin")
        .map_err(ChunkError::Io)?;
    
    let chunker = Chunker::new(ChunkConfig::default());
    
    for (idx, result) in chunker.chunk(file).enumerate() {
        match result {
            Ok(chunk) => {
                println!("Chunk {}: {} bytes", idx, chunk.len());
            }
            Err(ChunkError::Io(e)) => {
                eprintln!("IO error at chunk {}: {}", idx, e);
                return Err(ChunkError::Io(e));
            }
            Err(ChunkError::ChunkTooLarge { actual, max }) => {
                eprintln!("Chunk too large: {} > {}", actual, max);
                return Err(ChunkError::ChunkTooLarge { actual, max });
            }
            Err(ChunkError::InvalidConfig { message }) => {
                eprintln!("Config error: {}", message);
                return Err(ChunkError::InvalidConfig { message });
            }
        }
    }
    
    Ok(())
}
```

## Configuration

### Chunk Size Bounds

```rust
use chunkrs::ChunkConfig;

// Small chunks (good for small files, high dedup)
let small = ChunkConfig::new(2 * 1024, 8 * 1024, 32 * 1024)?;

// Default chunks (16 KiB average)
let default = ChunkConfig::default();

// Large chunks (good for large files, less overhead)
let large = ChunkConfig::new(64 * 1024, 256 * 1024, 1024 * 1024)?;
```

### Hash Configuration

```rust
use chunkrs::{ChunkConfig, HashConfig};

// Default: hashing enabled
let with_hash = ChunkConfig::default();

// Disable hashing (faster, no chunk identity)
let no_hash = ChunkConfig::default()
    .with_hash_config(HashConfig::disabled());

// Explicitly enable
let explicit = ChunkConfig::default()
    .with_hash_config(HashConfig::enabled());
```

## Cargo Features

| Feature | Description |
|---------|-------------|
| `hash-blake3` (default) | BLAKE3 hashing for chunk identity |
| `async-io` | Async streaming support via `futures-io` |

### Feature Combinations

```toml
# Default: sync + hashing
[dependencies]
chunkrs = "0.1"

# Sync only, no hashing (lightest)
[dependencies]
chunkrs = { version = "0.1", default-features = false }

# Sync + async + hashing
[dependencies]
chunkrs = { version = "0.1", features = ["async-io"] }

# Async only, no hashing
[dependencies]
chunkrs = { version = "0.1", default-features = false, features = ["async-io"] }
```

## Algorithm Details

- **Boundary Detection**: [FastCDC](https://www.usenix.org/conference/atc16/technical-sessions/presentation/xia) - Gear hash-based rolling hash
  - Byte-by-byte rolling hash
  - Mask-based boundary check  
  - Normal and large chunk detection
  
- **Chunk Identity**: BLAKE3 (when `hash-blake3` enabled)
  - Cryptographic strength
  - Incremental hashing for streaming
  - 32-byte output

## Performance Tips

1. **Choose appropriate chunk sizes**: Larger chunks = less overhead, smaller chunks = better deduplication
2. **Disable hashing if not needed**: ~2x faster for boundary-only use cases
3. **Use `chunk_bytes` for in-memory data**: Avoids iterator overhead
4. **Use `spawn_blocking` for sync ops in async contexts**: Prevents blocking the runtime

## Minimum Supported Rust Version (MSRV)

1.85.0

## License

Licensed under the [MIT License](LICENSE).

## Contributing

Issues and pull requests are welcome at [https://github.com/elemeng/chunkrs](https://github.com/elemeng/chunkrs).
