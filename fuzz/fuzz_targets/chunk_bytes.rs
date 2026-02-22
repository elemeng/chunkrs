#![no_main]

//! Fuzz target for chunkrs.
//!
//! Tests:
//! - Chunk boundary detection with various configurations
//! - Size constraints (min/avg/max)
//! - Offset tracking correctness
//! - Determinism guarantees
//! - Hash consistency
//! - Data integrity preservation

use bytes::Bytes;
use chunkrs::{ChunkConfig, Chunker};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: Vec<u8>| {
    // Test with various chunk configurations
    let configs = vec![
        // Small chunks - high boundary detection frequency
        ChunkConfig::new(4, 16, 64).unwrap(),
        // Medium chunks
        ChunkConfig::new(64, 256, 1024).unwrap(),
        // Large chunks
        ChunkConfig::new(256, 4096, 16384).unwrap(),
        // Default config
        ChunkConfig::default(),
    ];

    for config in configs {
        test_chunking(&data, config);
        test_determinism(&data, config);
    }

    // Test with hashing enabled
    if cfg!(feature = "hash-blake3") {
        test_hashing(&data);
    }

    // Test with keyed-cdc feature
    #[cfg(feature = "keyed-cdc")]
    test_keyed_cdc(&data);
});

/// Test basic chunking correctness for a single configuration
fn test_chunking(data: &[u8], config: ChunkConfig) {
    let mut chunker = Chunker::new(config);
    let (chunks, _) = chunker.push(Bytes::from(data.to_vec()));
    let final_chunk = chunker.finish();
    let all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();

    // Verify: all chunks are within min/max bounds
    for (i, chunk) in all_chunks.iter().enumerate() {
        assert!(
            chunk.len() <= config.max_size(),
            "Chunk {} size {} exceeds max {}",
            i,
            chunk.len(),
            config.max_size()
        );
        // Only enforce min_size for chunks that are not the last one
        if i < all_chunks.len() - 1 {
            assert!(
                chunk.len() >= config.min_size(),
                "Chunk {} size {} below min {}",
                i,
                chunk.len(),
                config.min_size()
            );
        }
    }

    // Verify: total bytes match input
    let total_bytes: usize = all_chunks.iter().map(|c| c.len()).sum();
    assert_eq!(
        total_bytes, data.len(),
        "Total output bytes {} != input bytes {}",
        total_bytes,
        data.len()
    );

    // Verify: offsets are correct
    let mut expected_offset = 0u64;
    for (i, chunk) in all_chunks.iter().enumerate() {
        assert_eq!(
            chunk.offset,
            Some(expected_offset),
            "Chunk {} offset mismatch: expected {}, got {:?}",
            i,
            expected_offset,
            chunk.offset
        );
        expected_offset += chunk.len() as u64;
    }

    // Verify: data integrity - recombine chunks and compare
    if !all_chunks.is_empty() {
        let recombined: Vec<u8> = all_chunks
            .iter()
            .flat_map(|c| c.data.as_ref().to_vec())
            .collect();
        assert_eq!(
            recombined, data,
            "Recombined data does not match original input"
        );
    }
}

/// Test determinism: same input produces identical output
fn test_determinism(data: &[u8], config: ChunkConfig) {
    // First run
    let mut chunker1 = Chunker::new(config);
    let (chunks1, _) = chunker1.push(Bytes::from(data.to_vec()));
    let final1 = chunker1.finish();
    let all_chunks1: Vec<_> = chunks1.into_iter().chain(final1).collect();

    // Second run
    let mut chunker2 = Chunker::new(config);
    let (chunks2, _) = chunker2.push(Bytes::from(data.to_vec()));
    let final2 = chunker2.finish();
    let all_chunks2: Vec<_> = chunks2.into_iter().chain(final2).collect();

    // Verify: same number of chunks
    assert_eq!(
        all_chunks1.len(),
        all_chunks2.len(),
        "Determinism broken: different chunk counts"
    );

    // Verify: identical chunks
    for (i, (c1, c2)) in all_chunks1.iter().zip(all_chunks2.iter()).enumerate() {
        assert_eq!(
            c1.data, c2.data,
            "Chunk {} data mismatch on deterministic run",
            i
        );
        assert_eq!(
            c1.offset, c2.offset,
            "Chunk {} offset mismatch on deterministic run",
            i
        );
        assert_eq!(
            c1.len(), c2.len(),
            "Chunk {} length mismatch on deterministic run",
            i
        );
        if c1.hash.is_some() && c2.hash.is_some() {
            assert_eq!(
                c1.hash, c2.hash,
                "Chunk {} hash mismatch on deterministic run",
                i
            );
        }
    }
}

/// Test hashing consistency when enabled
fn test_hashing(data: &[u8]) {
    let config_with_hash = ChunkConfig::default().with_hash_config(chunkrs::HashConfig::enabled());
    let mut chunker = Chunker::new(config_with_hash);
    let (chunks, _) = chunker.push(Bytes::from(data.to_vec()));
    let final_chunk = chunker.finish();
    let all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();

    // All chunks should have hashes
    for (i, chunk) in all_chunks.iter().enumerate() {
        assert!(
            chunk.hash.is_some(),
            "Chunk {} missing hash when hashing enabled",
            i
        );
    }

    // Verify: same content produces same hash
    if !data.is_empty() {
        let mut chunker2 = Chunker::new(config_with_hash);
        let (chunks2, _) = chunker2.push(Bytes::from(data.to_vec()));
        let final2 = chunker2.finish();
        let all_chunks2: Vec<_> = chunks2.into_iter().chain(final2).collect();

        for (i, (c1, c2)) in all_chunks.iter().zip(all_chunks2.iter()).enumerate() {
            assert_eq!(
                c1.hash, c2.hash,
                "Chunk {} hash mismatch on same content",
                i
            );
        }
    }
}

/// Test keyed-cdc feature
#[cfg(feature = "keyed-cdc")]
fn test_keyed_cdc(data: &[u8]) {
    let key = [0u8; 32];
    let config_with_key = ChunkConfig::default().with_key(key);
    let mut chunker = Chunker::new(config_with_key);
    let (chunks, _) = chunker.push(Bytes::from(data.to_vec()));
    let final_chunk = chunker.finish();
    let all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();

    // Verify: basic correctness still holds with keyed mode
    let total_bytes: usize = all_chunks.iter().map(|c| c.len()).sum();
    assert_eq!(
        total_bytes, data.len(),
        "Keyed mode: total bytes mismatch"
    );

    // Verify: offsets are correct
    let mut expected_offset = 0u64;
    for (i, chunk) in all_chunks.iter().enumerate() {
        assert_eq!(
            chunk.offset,
            Some(expected_offset),
            "Keyed mode: chunk {} offset mismatch",
            i
        );
        expected_offset += chunk.len() as u64;
    }
}