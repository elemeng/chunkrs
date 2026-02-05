//! Async streaming chunking with tokio-util compat.
//!
//! Run with:
//!     cargo run --example async_stream --features async-io

use std::env;

use futures_util::StreamExt;
use tokio::fs::File;
use tokio_util::compat::TokioAsyncReadCompatExt;

use chunkrs::Chunk;
use chunkrs::{ChunkConfig, chunk_async};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "Cargo.toml".to_string());

    println!("Async streaming chunker: {}\n", path);

    // Open file with tokio and convert to futures_io::AsyncRead
    let file = File::open(&path).await?;
    let metadata = file.metadata().await?;
    println!("File size: {} bytes\n", metadata.len());

    // Use tokio_util::compat to convert AsyncRead
    let reader = file.compat();

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
        }
    }

    println!("\nTotal: {} chunks, {} bytes", total_chunks, total_bytes);

    Ok(())
}
