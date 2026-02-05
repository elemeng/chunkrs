# chunkrs

Streaming Content-Defined Chunking (CDC) for Rust.

`chunkrs` transforms byte streams into content-defined chunks with optional strong hashes. Designed as a small, composable primitive for:

- Delta synchronization
- Deduplication
- Backup systems
- Content-addressable storage

## Features

- **Streaming-first**: Operates on byte streams without full-file buffering
- **Zero-copy**: Uses `bytes` for efficient buffer management
- **Deterministic**: Identical inputs produce identical chunk hashes
- **Async support**: Runtime-agnostic async I/O via `futures-io`
- **Allocator-conscious**: Thread-local buffer pools minimize allocation
- **No unsafe code**: `#![forbid(unsafe_code)]`

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
chunkrs = "0.1"
```

### Features

- `default`: Enables `hash-blake3`
- `hash-blake3`: BLAKE3 hashing for chunk identity
- `async-io`: Async streaming support (runtime-agnostic)

## Usage

### Sync

```rust
use std::fs::File;
use chunkrs::{Chunker, ChunkConfig, ChunkError};

fn main() -> Result<(), ChunkError> {
    let file = File::open("data.bin")?;
    let chunker = Chunker::new(ChunkConfig::default());

    for chunk in chunker.chunk(file) {
        let chunk = chunk?;
        println!("chunk {} bytes", chunk.data.len());
    }
    Ok(())
}
```

### Async

```rust
use futures_util::StreamExt;
use chunkrs::{chunk_async, ChunkConfig};
use futures_io::AsyncRead;

async fn demo<R: AsyncRead + Unpin>(reader: R) -> Result<(), chunkrs::ChunkError> {
    let mut stream = chunk_async(reader, ChunkConfig::default());

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        println!("chunk {} bytes", chunk.data.len());
    }
    Ok(())
}
```

## Configuration

```rust
use chunkrs::ChunkConfig;

let config = ChunkConfig::default()
    .with_min_size(4096)
    .with_avg_size(16384)
    .with_max_size(65536);
```

## Design Philosophy

`chunkrs` intentionally:

- Does NOT manage files or paths
- Does NOT manage concurrency
- Does NOT persist chunks
- Does NOT assume storage devices

It only does one thing: **Read bytes → yield chunks**

## License

Licensed under either of:

- MIT license ([LICENSE](LICENSE) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Issues and pull requests are welcome!
