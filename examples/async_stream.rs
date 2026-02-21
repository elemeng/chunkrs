//! Async streaming chunking example.
//!
//! Demonstrates using the streaming API in an async context.
//! The Chunker itself is synchronous (no async required), but can be
//! used from async code as needed.
//!
//! Run with:
//!     cargo run --example async_stream

use bytes::Bytes;
use chunkrs::{ChunkConfig, Chunker};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create some test data
    let data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();

    println!("Async chunking {} bytes of data...\n", data.len());

    // Custom config
    let config = ChunkConfig::new(
        4 * 1024,  // min: 4 KiB
        16 * 1024, // avg: 16 KiB
        64 * 1024, // max: 64 KiB
    )
    .expect("invalid config");

    let mut chunker = Chunker::new(config);

    let mut total_chunks = 0;
    let mut total_bytes = 0;
    let mut pending = Bytes::new();

    // Simulate async data streaming in batches
    let batch_size = 8192;
    let mut offset = 0;

    while offset < data.len() {
        // Simulate async delay (e.g., waiting for network data)
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;

        let end = (offset + batch_size).min(data.len());
        let batch = Bytes::from(data[offset..end].to_vec());

        println!("Async received batch: {} bytes", batch.len());

        // Chunker.push() is synchronous - just call it
        let (chunks, leftover) = chunker.push(batch);

        for chunk in chunks {
            total_chunks += 1;
            total_bytes += chunk.len();

            println!(
                "  Chunk {}: offset={:>8}, len={:>8}",
                total_chunks,
                chunk.offset.unwrap_or(0),
                chunk.len()
            );
        }

        pending = leftover;
        offset = end;
    }

    // Finalize stream
    if let Some(final_chunk) = chunker.finish() {
        total_chunks += 1;
        total_bytes += final_chunk.len();

        println!(
            "  Chunk {}: offset={:>8}, len={:>8} (final)",
            total_chunks,
            final_chunk.offset.unwrap_or(0),
            final_chunk.len()
        );
    }

    println!("\nTotal: {} chunks, {} bytes", total_chunks, total_bytes);
    if total_chunks > 0 {
        println!("Average chunk size: {} bytes", total_bytes / total_chunks);
    }

    Ok(())
}
