//! Basic synchronous chunking example with streaming API.
//!
//! Run with:
//!     cargo run --example sync_basic

use bytes::Bytes;
use chunkrs::{ChunkConfig, Chunker};
use rand::{Rng, SeedableRng, rngs::StdRng};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create some sample data with varied content using random numbers
    let mut rng = StdRng::from_entropy();
    let mut data = vec![0u8; 1024 * 1024]; // 1 MB buffer
    rng.fill(data.as_mut_slice()); // Fill with random bytes

    // Create chunker with default config
    let mut chunker = Chunker::new(ChunkConfig::default());

    println!("Chunking {} bytes of data...\n", data.len());

    let mut total_chunks = 0;
    let mut total_bytes = 0;
    let mut pending = Bytes::new();

    // Simulate streaming data in batches
    let batch_size = 8 * 1024; // 8 KB batches
    for chunk in data.chunks(batch_size) {
        let batch = Bytes::copy_from_slice(chunk);
        // Combine pending with new batch
        let input = if pending.is_empty() {
            batch
        } else {
            let mut combined = Vec::with_capacity(pending.len() + batch.len());
            combined.extend_from_slice(&pending);
            combined.extend_from_slice(&batch);
            Bytes::from(combined)
        };

        let (chunks, leftover) = chunker.push(input);

        for chunk_result in chunks {
            total_chunks += 1;
            total_bytes += chunk_result.len();

            if let Some(hash) = chunk_result.hash {
                println!(
                    "Chunk {}: offset={}, len={}, hash={}",
                    total_chunks,
                    chunk_result.offset.unwrap_or(0),
                    chunk_result.len(),
                    &hash.to_hex()[..16]
                );
            } else {
                println!(
                    "Chunk {}: offset={}, len={}",
                    total_chunks,
                    chunk_result.offset.unwrap_or(0),
                    chunk_result.len()
                );
            }
        }

        pending = leftover;
    }

    // Finalize stream
    if let Some(final_chunk) = chunker.finish() {
        total_chunks += 1;
        total_bytes += final_chunk.len();

        if let Some(hash) = final_chunk.hash {
            println!(
                "Chunk {}: offset={}, len={}, hash={}",
                total_chunks,
                final_chunk.offset.unwrap_or(0),
                final_chunk.len(),
                &hash.to_hex()[..16]
            );
        } else {
            println!(
                "Chunk {}: offset={}, len={}",
                total_chunks,
                final_chunk.offset.unwrap_or(0),
                final_chunk.len()
            );
        }
    }

    println!("\nTotal: {} chunks, {} bytes", total_chunks, total_bytes);
    if total_chunks > 0 {
        println!("Average chunk size: {} bytes", total_bytes / total_chunks);
    }

    Ok(())
}
