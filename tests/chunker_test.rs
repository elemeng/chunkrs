// Integration tests for the Chunker streaming API
// Tests cover: push/finish semantics, determinism, hashing, edge cases

use bytes::Bytes;
use chunkrs::{ChunkConfig, Chunker, HashConfig};

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[test]
fn test_empty_input() {
    let mut chunker = Chunker::default();
    let (chunks, pending) = chunker.push(Bytes::new());

    assert!(chunks.is_empty(), "Empty input should produce no chunks");
    assert!(
        pending.is_empty(),
        "Empty input should have no pending bytes"
    );
    assert!(
        chunker.finish().is_none(),
        "finish() on empty state should return None"
    );
}

#[test]
fn test_small_data_below_min_size() {
    let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());

    // Data smaller than min_size (4 bytes)
    let (chunks, pending) = chunker.push(Bytes::from(vec![0xAA; 3]));

    assert!(
        chunks.is_empty(),
        "Data below min_size should not produce chunks"
    );
    assert_eq!(pending.len(), 3, "All data should be pending");

    let final_chunk = chunker.finish().expect("finish() should emit pending data");
    assert_eq!(
        final_chunk.len(),
        3,
        "Final chunk should contain all pending data"
    );
}

#[test]
fn test_data_at_min_size_boundary() {
    let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());

    // Data exactly at min_size
    let (chunks, pending) = chunker.push(Bytes::from(vec![0xAB; 4]));

    // At min_size, we might or might not get a chunk depending on CDC
    assert!(
        chunks.is_empty() || pending.is_empty(),
        "Data at min_size should either chunk or pend"
    );
}

#[test]
fn test_large_data_finds_boundaries() {
    let config = ChunkConfig::new(4, 16, 64).unwrap();
    let mut chunker = Chunker::new(config);

    let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    let (chunks, _pending) = chunker.push(Bytes::from(data.clone()));
    let final_chunk = chunker.finish();

    let all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();

    assert!(
        !all_chunks.is_empty(),
        "Large data should produce at least one chunk"
    );

    let total_output: usize = all_chunks.iter().map(|c| c.len()).sum();
    assert_eq!(
        total_output,
        data.len(),
        "Output bytes must match input bytes"
    );
}

// ============================================================================
// Streaming and Push/FINISH Semantics
// ============================================================================

#[test]
fn test_streaming_data_in_batches() {
    let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());

    // Simulate streaming data in 4 batches totaling 1000 bytes
    let batches = vec![
        Bytes::from(&[0xAAu8; 256][..]),
        Bytes::from(&[0xBBu8; 256][..]),
        Bytes::from(&[0xCCu8; 256][..]),
        Bytes::from(&[0xDDu8; 232][..]),
    ];

    let mut all_chunks = Vec::new();
    let mut _pending = Bytes::new();

    for batch in batches {
        let (chunks, _leftover) = chunker.push(batch);
        all_chunks.extend(chunks);
        _pending = _leftover;
    }

    if let Some(final_chunk) = chunker.finish() {
        all_chunks.push(final_chunk);
    }

    let total_len: usize = all_chunks.iter().map(|c| c.len()).sum();
    assert_eq!(total_len, 1000, "Streaming must preserve total byte count");
}

#[test]
fn test_pending_bytes_handling() {
    let mut chunker = Chunker::new(ChunkConfig::new(8, 16, 64).unwrap());

    // First push: data below min_size
    let (chunks1, pending1) = chunker.push(Bytes::from(&b"small"[..]));
    assert!(chunks1.is_empty());
    assert!(!pending1.is_empty());

    // Second push: more data to complete chunk
    let (chunks2, pending2) = chunker.push(Bytes::from(&b" additional data"[..]));

    // Should now have chunks
    assert!(
        !chunks2.is_empty() || !pending2.is_empty(),
        "After combining with pending, should have chunks or new pending"
    );
}

#[test]
fn test_multiple_finish_calls() {
    let mut chunker = Chunker::new(ChunkConfig::new(1, 2, 4).unwrap());

    let (chunks, _) = chunker.push(Bytes::from(&b"test"[..]));
    assert!(!chunks.is_empty(), "Should have at least one chunk");

    let final1 = chunker.finish();
    assert!(
        final1.is_none(),
        "First finish() after push should return None (already emitted)"
    );

    let final2 = chunker.finish();
    assert!(final2.is_none(), "Second finish() should return None");
}

// ============================================================================
// Offset Tracking
// ============================================================================

#[test]
fn test_chunk_offset_tracking() {
    let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());
    let data: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();

    let (chunks, _pending) = chunker.push(Bytes::from(data.clone()));
    let final_chunk = chunker.finish();

    let all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();
    let mut expected_offset = 0u64;

    for (i, chunk) in all_chunks.iter().enumerate() {
        assert_eq!(
            chunk.offset,
            Some(expected_offset),
            "Chunk {} offset should be {}",
            i,
            expected_offset
        );
        expected_offset += chunk.len() as u64;
    }

    assert_eq!(
        expected_offset,
        data.len() as u64,
        "Final offset should equal total bytes processed"
    );
}

#[test]
fn test_offset_resets_after_reset() {
    let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());

    // Process first stream
    let (_chunks, _) = chunker.push(Bytes::from(&b"first"[..]));
    chunker.finish();
    assert!(
        chunker.offset() > 0,
        "Offset should be > 0 after processing"
    );

    // Reset and process second stream
    chunker.reset();
    let (chunks2, _) = chunker.push(Bytes::from(&b"second"[..]));
    let final_chunk = chunker.finish();
    let all: Vec<_> = chunks2.into_iter().chain(final_chunk).collect();

    assert!(all.first().is_some(), "Should have chunks after reset");
    assert_eq!(
        all.first().unwrap().offset,
        Some(0),
        "Offset should restart at 0 after reset"
    );
}

// ============================================================================
// Size Constraints
// ============================================================================

#[test]
fn test_max_size_enforces_boundary() {
    // Small max_size to force boundary quickly
    let mut chunker = Chunker::new(ChunkConfig::new(2, 4, 8).unwrap());

    let data = Bytes::from(vec![0xFF; 20]);
    let (chunks, _) = chunker.push(data);

    assert!(!chunks.is_empty(), "Should produce chunks");
    assert!(
        chunks[0].len() <= 8,
        "First chunk should not exceed max_size"
    );
}

#[test]
fn test_exact_max_size_boundary() {
    let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 32).unwrap());

    // Push exactly max_size bytes
    let data = Bytes::from(vec![0u8; 32]);
    let (chunks, _pending) = chunker.push(data);
    let final_chunk = chunker.finish();

    let all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();
    assert!(!all_chunks.is_empty());
    assert!(
        all_chunks[0].len() <= 32,
        "Chunk should not exceed max_size"
    );
}

// ============================================================================
// Determinism
// ============================================================================

#[test]
fn test_determinism_across_push_sizes() {
    let data: Vec<u8> = (0..500).map(|i| (i % 256) as u8).collect();
    let config = ChunkConfig::new(4, 16, 64).unwrap();

    // Push all at once
    let mut chunker1 = Chunker::new(config);
    let (chunks1, _pending1) = chunker1.push(Bytes::from(data.clone()));
    let final1 = chunker1.finish();
    let offsets1: Vec<_> = chunks1
        .iter()
        .chain(final1.iter())
        .map(|c| c.offset.unwrap())
        .collect();

    // Push in small chunks (10 bytes each)
    let mut chunker2 = Chunker::new(config);
    let mut chunks2 = Vec::new();
    for chunk in data.chunks(10) {
        let (chunks, _leftover) = chunker2.push(Bytes::copy_from_slice(chunk));
        chunks2.extend(chunks);
    }
    let final2 = chunker2.finish();
    let offsets2: Vec<_> = chunks2
        .iter()
        .chain(final2.iter())
        .map(|c| c.offset.unwrap())
        .collect();

    assert_eq!(
        offsets1, offsets2,
        "Chunk boundaries must be identical regardless of push size"
    );
}

#[test]
fn test_same_stream_same_chunks_same_hashes() {
    let data: Vec<u8> = (0..800).map(|i| (i % 256) as u8).collect();
    let config = ChunkConfig::new(4, 16, 64)
        .unwrap()
        .with_hash_config(HashConfig::enabled());

    // Test 1: All at once
    let mut chunker1 = Chunker::new(config);
    let (chunks1, _pending1) = chunker1.push(Bytes::from(data.clone()));
    let final1 = chunker1.finish();
    let all1: Vec<_> = chunks1.into_iter().chain(final1).collect();

    // Test 2: 10-byte chunks
    let mut chunker2 = Chunker::new(config);
    let mut all2 = Vec::new();
    for chunk in data.chunks(10) {
        let (chunks, _leftover) = chunker2.push(Bytes::copy_from_slice(chunk));
        all2.extend(chunks);
    }
    all2.extend(chunker2.finish());

    // Test 3: 37-byte chunks
    let mut chunker3 = Chunker::new(config);
    let mut all3 = Vec::new();
    for chunk in data.chunks(37) {
        let (chunks, _leftover) = chunker3.push(Bytes::copy_from_slice(chunk));
        all3.extend(chunks);
    }
    all3.extend(chunker3.finish());

    assert_eq!(all1.len(), all2.len(), "Same number of chunks");
    assert_eq!(all1.len(), all3.len(), "Same number of chunks");

    for (i, ((c1, c2), c3)) in all1.iter().zip(&all2).zip(&all3).enumerate() {
        assert_eq!(c1.offset, c2.offset, "Chunk {} offset mismatch (1 vs 2)", i);
        assert_eq!(c1.offset, c3.offset, "Chunk {} offset mismatch (1 vs 3)", i);
        assert_eq!(c1.len(), c2.len(), "Chunk {} length mismatch (1 vs 2)", i);
        assert_eq!(c1.len(), c3.len(), "Chunk {} length mismatch (1 vs 3)", i);
        assert_eq!(c1.hash, c2.hash, "Chunk {} hash mismatch (1 vs 2)", i);
        assert_eq!(c1.hash, c3.hash, "Chunk {} hash mismatch (1 vs 3)", i);
    }
}

// ============================================================================
// Zero-Copy Verification
// ============================================================================

#[test]
fn test_zero_copy_semantics() {
    let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());
    let original = Bytes::from(&b"hello world, zero copy test data"[..]);

    let (chunks, _pending) = chunker.push(original.clone());
    let final_chunk = chunker.finish();

    for chunk in chunks.iter().chain(final_chunk.iter()) {
        // Verify chunk data is a slice of the original
        assert!(
            chunk.data.as_ptr() >= original.as_ptr()
                && (chunk.data.as_ptr() as usize + chunk.data.len())
                    <= (original.as_ptr() as usize + original.len()),
            "Chunk data must be a slice of the original Bytes"
        );
    }
}

// ============================================================================
// Hashing Tests
// ============================================================================

#[cfg(feature = "hash-blake3")]
mod hashing_tests {
    use super::*;

    #[test]
    fn test_hashing_enabled() {
        let config = ChunkConfig::default().with_hash_config(HashConfig::enabled());
        let mut chunker = Chunker::new(config);

        let data = Bytes::from(&b"test data for hashing"[..]);
        let (chunks, _pending) = chunker.push(data);
        let final_chunk = chunker.finish();

        for (i, chunk) in chunks.iter().chain(final_chunk.iter()).enumerate() {
            assert!(
                chunk.hash.is_some(),
                "Chunk {} must have a hash when enabled",
                i
            );
        }
    }

    #[test]
    fn test_hashing_disabled() {
        let config = ChunkConfig::default().with_hash_config(HashConfig::disabled());
        let mut chunker = Chunker::new(config);

        let data = Bytes::from(&b"test data without hashing"[..]);
        let (chunks, _pending) = chunker.push(data);
        let final_chunk = chunker.finish();

        for (i, chunk) in chunks.iter().chain(final_chunk.iter()).enumerate() {
            assert!(
                chunk.hash.is_none(),
                "Chunk {} must not have a hash when disabled",
                i
            );
        }
    }

    #[test]
    fn test_hash_determinism() {
        let data: Vec<u8> = (0..300).map(|i| (i % 256) as u8).collect();
        let config = ChunkConfig::new(4, 16, 64)
            .unwrap()
            .with_hash_config(HashConfig::enabled());

        let mut chunker1 = Chunker::new(config);
        let (chunks1, _) = chunker1.push(Bytes::from(data.clone()));
        let final1 = chunker1.finish();

        let mut chunker2 = Chunker::new(config);
        let (chunks2, _) = chunker2.push(Bytes::from(data.clone()));
        let final2 = chunker2.finish();

        let mut iter1 = chunks1.into_iter().chain(final1);
        let mut iter2 = chunks2.into_iter().chain(final2);

        let mut count = 0;
        while let (Some(c1), Some(c2)) = (iter1.next(), iter2.next()) {
            assert_eq!(c1.hash, c2.hash, "Chunk {} hashes must match", count);
            count += 1;
        }
    }

    #[test]
    fn test_hash_persists_across_chunks() {
        let config = ChunkConfig::new(4, 8, 16)
            .unwrap()
            .with_hash_config(HashConfig::enabled());
        let mut chunker = Chunker::new(config);

        let data = Bytes::from(&b"this will produce multiple chunks with hashes"[..]);
        let (chunks, _pending) = chunker.push(data);
        let final_chunk = chunker.finish();

        let all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();

        for (i, chunk) in all_chunks.iter().enumerate() {
            assert!(
                chunk.hash.is_some(),
                "All chunks (including {}) must have hashes when enabled",
                i
            );
            // Different chunks should generally have different hashes
            if i > 0 {
                assert_ne!(
                    chunk.hash,
                    all_chunks[i - 1].hash,
                    "Different chunks should have different hashes"
                );
            }
        }
    }
}

// ============================================================================
// Edge Cases and Error Conditions
// ============================================================================

#[test]
fn test_config_validation() {
    // Invalid: min > avg
    assert!(
        ChunkConfig::new(16, 8, 64).is_err(),
        "min > avg should be invalid"
    );

    // Invalid: avg > max
    assert!(
        ChunkConfig::new(4, 32, 16).is_err(),
        "avg > max should be invalid"
    );

    // Invalid: zero sizes
    assert!(
        ChunkConfig::new(0, 16, 64).is_err(),
        "zero min_size should be invalid"
    );
}

#[test]
fn test_hash_config_consistency() {
    let data: Vec<u8> = (0..100).collect();

    let config1 = ChunkConfig::new(4, 16, 64)
        .unwrap()
        .with_hash_config(HashConfig::enabled());
    let config2 = ChunkConfig::new(4, 16, 64)
        .unwrap()
        .with_hash_config(HashConfig::enabled());

    let mut chunker1 = Chunker::new(config1);
    let mut chunker2 = Chunker::new(config2);

    let (chunks1, _) = chunker1.push(Bytes::from(data.clone()));
    let final1 = chunker1.finish();
    let all1 = chunks1.into_iter().chain(final1).collect::<Vec<_>>();

    let (chunks2, _) = chunker2.push(Bytes::from(data));
    let final2 = chunker2.finish();
    let all2 = chunks2.into_iter().chain(final2).collect::<Vec<_>>();

    assert_eq!(
        all1.len(),
        all2.len(),
        "Same config should produce same number of chunks"
    );

    for (c1, c2) in all1.iter().zip(all2.iter()) {
        assert_eq!(c1.offset, c2.offset, "Offsets should match");
        assert_eq!(c1.hash, c2.hash, "Hashes should match");
    }
}

#[test]
fn test_pending_bytes_data_integrity() {
    let mut chunker = Chunker::new(ChunkConfig::new(16, 32, 64).unwrap());

    let data1 = Bytes::from(&b"partial"[..]);
    let (chunks, pending) = chunker.push(data1.clone());
    assert!(chunks.is_empty(), "Small data should not chunk");
    assert!(!pending.is_empty(), "Should have pending bytes");

    let data2 = Bytes::from(&b" completion"[..]);
    let data2_expected = data2.clone();
    let (chunks2, _) = chunker.push(data2);
    let final_chunk = chunker.finish();

    let all: Vec<_> = chunks2.into_iter().chain(final_chunk).collect();
    let total_output: usize = all.iter().map(|c| c.len()).sum();
    let total_input = data1.len() + data2_expected.len();

    assert_eq!(
        total_output, total_input,
        "Total output bytes must equal total input bytes"
    );

    let combined: Vec<u8> = all.iter().flat_map(|c| c.data.as_ref().to_vec()).collect();
    let expected: Vec<u8> = data1.iter().chain(data2_expected.iter()).copied().collect();
    assert_eq!(combined, expected, "Data content must be preserved");
}
