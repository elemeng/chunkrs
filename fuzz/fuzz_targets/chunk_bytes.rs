#![no_main]

use libfuzzer_sys::fuzz_target;
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
        let chunker = Chunker::new(config);
        let chunks = chunker.chunk_bytes(data.clone());

        // Verify: all chunks are within min/max bounds
        for (i, chunk) in chunks.iter().enumerate() {
            assert!(chunk.len() <= config.max_size());
            // Only enforce min_size for chunks that are not the last one
            if i < chunks.len() - 1 {
                assert!(chunk.len() >= config.min_size());
            }
        }

        // Verify: total bytes match input
        let total_bytes: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total_bytes, data.len());

        // Verify: offsets are correct
        let mut expected_offset = 0u64;
        for chunk in &chunks {
            assert_eq!(chunk.offset, Some(expected_offset));
            expected_offset += chunk.len() as u64;
        }

        // Verify: determinism - same input produces same chunks
        let chunker2 = Chunker::new(config);
        let chunks2 = chunker2.chunk_bytes(data.clone());
        assert_eq!(chunks.len(), chunks2.len());
        for (c1, c2) in chunks.iter().zip(chunks2.iter()) {
            assert_eq!(c1.data, c2.data);
            assert_eq!(c1.offset, c2.offset);
            if c1.hash.is_some() && c2.hash.is_some() {
                assert_eq!(c1.hash, c2.hash);
            }
        }
    }

    // Test with hashing enabled
    let config_with_hash = ChunkConfig::default().with_hash_config(chunkrs::HashConfig::enabled());
    let chunker = Chunker::new(config_with_hash);
    let chunks = chunker.chunk_bytes(data.clone());

    // All chunks should have hashes
    for chunk in &chunks {
        assert!(chunk.hash.is_some());
    }

    // Verify: same content produces same hash
    if !data.is_empty() {
        let chunks2 = chunker.chunk_bytes(data);
        for (c1, c2) in chunks.iter().zip(chunks2.iter()) {
            assert_eq!(c1.hash, c2.hash);
        }
    }
});