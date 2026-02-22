//! Streaming chunking example with variable batch sizes.
//!
//! Demonstrates that chunk boundaries are deterministic regardless of
//! input batch size (1 byte, 8KB, 1MB, etc.)
//!
//! Run with:
//!     cargo run --example sync_file

use bytes::Bytes;
use chunkrs::{ChunkConfig, Chunker};
use rand::{rngs::StdRng, Rng, SeedableRng};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create some test data (simulating data from any source)
    let mut rng = StdRng::from_entropy();
    let mut data = vec![0u8; 100_000];
    rng.fill(data.as_mut_slice());

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
        let batch = Bytes::copy_from_slice(&data[offset..end]);

        println!("Pushing batch: {} bytes", batch.len());

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
        let batch = Bytes::copy_from_slice(&data[offset..]);
        // Combine pending with remaining data
        let input = if pending.is_empty() {
            batch
        } else {
            let mut combined = Vec::with_capacity(pending.len() + batch.len());
            combined.extend_from_slice(&pending);
            combined.extend_from_slice(&batch);
            Bytes::from(combined)
        };

        let (chunks, _leftover) = chunker.push(input);
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
        // leftover is not needed since finish() will handle any pending data
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
