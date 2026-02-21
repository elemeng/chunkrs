#![no_main]

use libfuzzer_sys::fuzz_target;
use bytes::Bytes;
use chunkrs::{Chunker, ChunkConfig};

fuzz_target!(|data: Vec<u8>| {
    // Test with various chunk configurations
    let configs = vec![
        // Small chunks
        ChunkConfig::new(4, 16, 64).unwrap(),
        // Medium chunks
        ChunkConfig::new(64, 256, 1024).unwrap(),
        // Large chunks
        ChunkConfig::new(256, 4096, 16384).unwrap(),
        // Default config
        ChunkConfig::default(),
    ];

    for config in configs {
        let mut chunker = Chunker::new(config);
        let (chunks, _) = chunker.push(Bytes::from(data.clone()));
        let final_chunk = chunker.finish();
        let all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();

        // Verify: all chunks are within min/max bounds
        for (i, chunk) in all_chunks.iter().enumerate() {
            assert!(chunk.len() <= config.max_size());
            // Only enforce min_size for chunks that are not the last one
            if i < all_chunks.len() - 1 {
                assert!(chunk.len() >= config.min_size());
            }
        }

        // Verify: total bytes match input
        let total_bytes: usize = all_chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total_bytes, data.len());

        // Verify: offsets are correct
        let mut expected_offset = 0u64;
        for chunk in &all_chunks {
            assert_eq!(chunk.offset, Some(expected_offset));
            expected_offset += chunk.len() as u64;
        }

        // Verify: determinism - same input produces same chunks
        let mut chunker2 = Chunker::new(config);
        let (chunks2, _) = chunker2.push(Bytes::from(data.clone()));
        let final2 = chunker2.finish();
        let all_chunks2: Vec<_> = chunks2.into_iter().chain(final2).collect();

        assert_eq!(all_chunks.len(), all_chunks2.len());
        for (c1, c2) in all_chunks.iter().zip(all_chunks2.iter()) {
            assert_eq!(c1.data, c2.data);
            assert_eq!(c1.offset, c2.offset);
            if c1.hash.is_some() && c2.hash.is_some() {
                assert_eq!(c1.hash, c2.hash);
            }
        }
    }

    // Test with hashing enabled
    let config_with_hash = ChunkConfig::default().with_hash_config(chunkrs::HashConfig::enabled());
    let mut chunker = Chunker::new(config_with_hash);
    let (chunks, _) = chunker.push(Bytes::from(data.clone()));
    let final_chunk = chunker.finish();
    let all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();

    // All chunks should have hashes
    for chunk in &all_chunks {
        assert!(chunk.hash.is_some());
    }

    // Verify: same content produces same hash
    if !data.is_empty() {
        let mut chunker2 = Chunker::new(config_with_hash);
        let (chunks2, _) = chunker2.push(Bytes::from(data.clone()));
        let final2 = chunker2.finish();
        let all_chunks2: Vec<_> = chunks2.into_iter().chain(final2).collect();

        for (c1, c2) in all_chunks.iter().zip(all_chunks2.iter()) {
            assert_eq!(c1.hash, c2.hash);
        }
    }
});