//! Async chunking with parallel processing example.
//!
//! Demonstrates using Chunker in async contexts for parallel processing.
//! Multiple chunkers can run concurrently for different streams.
//!
//! Run with:
//!     cargo run --example async_tokio

use bytes::Bytes;
use chunkrs::{ChunkConfig, Chunker};
use rand::{rngs::StdRng, Rng, SeedableRng};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create multiple data streams with random data
    let mut rng = StdRng::from_entropy();
    let mut stream1 = vec![0u8; 50_000];
    let mut stream2 = vec![0u8; 50_000];
    let mut stream3 = vec![0u8; 50_000];
    rng.fill(stream1.as_mut_slice());
    rng.fill(stream2.as_mut_slice());
    rng.fill(stream3.as_mut_slice());
    let streams = vec![stream1, stream2, stream3];

    println!("Processing {} streams concurrently...\n", streams.len());

    let config = ChunkConfig::default();

    // Process each stream in parallel
    let handles: Vec<_> = streams
        .into_iter()
        .enumerate()
        .map(|(stream_id, data)| {
            let config = config;
            tokio::task::spawn_blocking(move || process_stream(stream_id, data, config))
        })
        .collect();

    // Wait for all streams to complete
    for handle in handles {
        let (stream_id, chunk_count, total_bytes) = handle.await??;
        println!(
            "Stream {}: {} chunks, {} bytes",
            stream_id, chunk_count, total_bytes
        );
    }

    Ok(())
}

fn process_stream(
    stream_id: usize,
    data: Vec<u8>,
    config: ChunkConfig,
) -> Result<(usize, usize, usize), String> {
    let mut chunker = Chunker::new(config);
    let mut chunk_count = 0;
    let mut total_bytes = 0;
    let mut pending = Bytes::new();

    // Process in batches
    let batch_size = 8192;
    let mut offset = 0;

    while offset < data.len() {
        let end = (offset + batch_size).min(data.len());
        let batch = Bytes::copy_from_slice(&data[offset..end]);

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
            chunk_count += 1;
            total_bytes += chunk.len();
        }

        pending = leftover;
        offset = end;
    }

    // Finalize
    if let Some(final_chunk) = chunker.finish() {
        chunk_count += 1;
        total_bytes += final_chunk.len();
    }

    Ok((stream_id, chunk_count, total_bytes))
}
