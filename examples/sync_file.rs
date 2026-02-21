//! Streaming chunking example with variable batch sizes.
//!
//! Demonstrates that chunk boundaries are deterministic regardless of
//! input batch size (1 byte, 8KB, 1MB, etc.)
//!
//! Run with:
//!     cargo run --example sync_file

use bytes::Bytes;
use chunkrs::{ChunkConfig, Chunker};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create some test data (simulating data from any source)
    let data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();

    println!("Chunking {} bytes of data...\n", data.len());

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

    // Simulate streaming data with variable batch sizes
    let batch_sizes = [1, 100, 1024, 8192, 16384, 32768];
    let mut offset = 0;

    for batch_size in batch_sizes {
        if offset >= data.len() {
            break;
        }

        let end = (offset + batch_size).min(data.len());
        let batch = Bytes::from(data[offset..end].to_vec());

        println!("Pushing batch: {} bytes", batch.len());

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

    // Handle any remaining data
    if offset < data.len() {
        let batch = Bytes::from(data[offset..].to_vec());
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
