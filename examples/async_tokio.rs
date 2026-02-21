//! Async chunking with parallel processing example.
//!
//! Demonstrates using Chunker in async contexts for parallel processing.
//! Multiple chunkers can run concurrently for different streams.
//!
//! Run with:
//!     cargo run --example async_tokio

use bytes::Bytes;
use chunkrs::{ChunkConfig, Chunker};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create multiple data streams
    let streams: Vec<Vec<u8>> = vec![
        (0..50_000).map(|i| (i % 256) as u8).collect(),
        (50_000..100_000).map(|i| (i % 256) as u8).collect(),
        (100_000..150_000).map(|i| (i % 256) as u8).collect(),
    ];

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
) -> Result<(usize, usize, usize), Box<dyn std::error::Error>> {
    let mut chunker = Chunker::new(config);
    let mut chunk_count = 0;
    let mut total_bytes = 0;
    let mut pending = Bytes::new();

    // Process in batches
    let batch_size = 8192;
    let mut offset = 0;

    while offset < data.len() {
        let end = (offset + batch_size).min(data.len());
        let batch = Bytes::from(data[offset..end].to_vec());

        let (chunks, leftover) = chunker.push(batch);

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
