//! Async chunking with Tokio example.
//!
//! Run with:
//!     cargo run --example async_tokio --features async-io

use std::env;

use futures_util::StreamExt;

use chunkrs::Chunk;
use chunkrs::{ChunkConfig, Chunker, chunk_async};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "Cargo.toml".to_string());

    println!("Async chunking file: {}\n", path);

    // Method 1: Using chunk_async with in-memory data
    let data = tokio::fs::read(&path).await?;
    println!("File size: {} bytes\n", data.len());

    // Use a reference to the data slice which implements futures_io::AsyncRead
    let reader: &[u8] = &data[..];

    let config = ChunkConfig::default();
    let mut stream = chunk_async(reader, config);

    let mut total_chunks = 0;
    let mut total_bytes = 0;

    while let Some(chunk) = stream.next().await {
        let chunk: Chunk = chunk?;
        total_chunks += 1;
        total_bytes += chunk.len();

        if let Some(hash) = chunk.hash {
            let hex = hash.to_hex();
            println!(
                "Chunk {}: offset={:>10}, len={:>8}, hash={}",
                total_chunks,
                chunk.offset.unwrap_or(0),
                chunk.len(),
                &hex[..16.min(hex.len())]
            );
        } else {
            println!(
                "Chunk {}: offset={:>10}, len={:>8}",
                total_chunks,
                chunk.offset.unwrap_or(0),
                chunk.len()
            );
        }
    }

    println!("\nTotal: {} chunks, {} bytes", total_chunks, total_bytes);
    if total_chunks > 0 {
        println!("Average chunk size: {} bytes", total_bytes / total_chunks);
    }

    // Method 2: Using sync Chunker with tokio::task::spawn_blocking
    println!("\n--- Using spawn_blocking ---\n");

    let data_clone: Vec<u8> = data.clone();
    let chunks = tokio::task::spawn_blocking(move || {
        let chunker = Chunker::new(ChunkConfig::default());
        chunker.chunk_bytes(data_clone)
    })
    .await?;

    println!("Spawn blocking result: {} chunks", chunks.len());

    Ok(())
}
