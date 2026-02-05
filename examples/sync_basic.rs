//! Basic synchronous chunking example.
//!
//! Run with:
//!     cargo run --example sync_basic

use std::io;

use chunkrs::{ChunkConfig, Chunker};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create some sample data
    let data = vec![0u8; 1024 * 1024]; // 1 MB of zeros
    let cursor = io::Cursor::new(&data);

    // Create chunker with default config
    let chunker = Chunker::new(ChunkConfig::default());

    println!("Chunking {} bytes of data...\n", data.len());

    let mut total_chunks = 0;
    let mut total_bytes = 0;

    for chunk in chunker.chunk(cursor) {
        let chunk = chunk?;
        total_chunks += 1;
        total_bytes += chunk.len();

        if let Some(hash) = chunk.hash {
            println!(
                "Chunk {}: offset={}, len={}, hash={}",
                total_chunks,
                chunk.offset.unwrap_or(0),
                chunk.len(),
                &hash.to_hex()[..16]
            );
        } else {
            println!(
                "Chunk {}: offset={}, len={}",
                total_chunks,
                chunk.offset.unwrap_or(0),
                chunk.len()
            );
        }
    }

    println!("\nTotal: {} chunks, {} bytes", total_chunks, total_bytes);
    println!("Average chunk size: {} bytes", total_bytes / total_chunks);

    Ok(())
}
