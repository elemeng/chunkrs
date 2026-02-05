//! File chunking example.
//!
//! Run with:
//!     cargo run --example sync_file -- /path/to/file

use std::env;
use std::fs::File;

use chunkrs::{ChunkConfig, Chunker};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "Cargo.toml".to_string());

    println!("Chunking file: {}\n", path);

    let file = File::open(&path)?;
    let metadata = file.metadata()?;
    println!("File size: {} bytes\n", metadata.len());

    // Custom config for larger chunks
    let config = ChunkConfig::new(
        8 * 1024,   // min: 8 KiB
        32 * 1024,  // avg: 32 KiB
        128 * 1024, // max: 128 KiB
    )
    .expect("invalid config");

    let chunker = Chunker::new(config);

    let mut total_chunks = 0;
    let mut total_bytes = 0;

    for chunk in chunker.chunk(file) {
        let chunk = chunk?;
        total_chunks += 1;
        total_bytes += chunk.len();

        if let Some(hash) = chunk.hash {
            println!(
                "Chunk {}: offset={:>10}, len={:>8}, hash={}",
                total_chunks,
                chunk.offset.unwrap_or(0),
                chunk.len(),
                hash.to_hex()
            );
        }
    }

    println!("\nTotal: {} chunks, {} bytes", total_chunks, total_bytes);
    if total_chunks > 0 {
        println!("Average chunk size: {} bytes", total_bytes / total_chunks);
    }

    Ok(())
}
