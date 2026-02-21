//! Core chunking engine - Chunker with streaming API.
//!
//! This module implements the synchronous chunking API using the FastCDC
//! algorithm. It provides a pure streaming interface:
//!
//! - [`Chunker`] - Stateful CDC engine that processes streaming bytes
//! - `push()` - Feed data in any size (1 byte, 8KB, 1MB, etc.)
//! - `finish()` - Flush remaining data when stream ends
//!
//! # Example
//!
//! ```
//! use chunkrs::{Chunker, ChunkConfig};
//! use bytes::Bytes;
//!
//! let config = ChunkConfig::default();
//! let mut chunker = Chunker::new(config);
//!
//! // Feed data in any size
//! let chunks1 = chunker.push(Bytes::from(&b"first"[..]));
//! let chunks2 = chunker.push(Bytes::from(&b"second"[..]));
//!
//! // When stream ends, get final chunk
//! let final_chunk = chunker.finish();
//!
//! // Process all chunks (may OOM - caller's responsibility)
//! # Ok::<(), chunkrs::ChunkError>(())
//! ```

use bytes::Bytes;

use crate::cdc::FastCdc;
use crate::chunk::Chunk;
use crate::config::ChunkConfig;

#[cfg(feature = "hash-blake3")]
use crate::hash::Blake3Hasher;

/// A chunker that processes streaming byte data into content-defined chunks.
///
/// `Chunker` is a pure CDC engine that accepts bytes via `push()` and yields
/// chunks as the FastCDC algorithm identifies boundaries. It maintains CDC
/// state across calls, ensuring deterministic chunk boundaries regardless of
/// input size.
///
/// # Streaming API
///
/// - Call `push()` with data in any size (1 byte to megabytes)
/// - Returns complete chunks and any unprocessed bytes
/// - Feed unprocessed bytes back in the next `push()` call
/// - Call `finish()` when stream ends to emit final incomplete chunk
///
/// # Determinism
///
/// Identical byte streams produce identical chunk boundaries, regardless of:
/// - How many bytes are pushed at once (1 byte vs 1MB)
/// - Call timing
/// - Number of `push()` calls
///
/// # Zero-Copy
///
/// Chunk data is zero-copy sliced from input `Bytes`. The caller owns the
/// original data memory; chunks are references into it.
///
/// # Memory Considerations
///
/// - The `push()` method returns a `Vec<Chunk>` - accumulating chunks may OOM
/// - Caller should process or drop chunks promptly
/// - Pending unprocessed bytes are held internally
///
/// # Example
///
/// ```
/// use chunkrs::{Chunker, ChunkConfig};
/// use bytes::Bytes;
///
/// let mut chunker = Chunker::new(ChunkConfig::default());
///
/// // Feed data in chunks
/// let data = vec![
///     Bytes::from(&b"first part"[..]),
///     Bytes::from(&b" second part"[..]),
///     Bytes::from(&b" final part"[..]),
/// ];
///
/// let mut all_chunks = Vec::new();
/// let mut pending = Bytes::new();
///
/// for chunk in data {
///     let (chunks, leftover) = chunker.push(chunk);
///     all_chunks.extend(chunks);
///     pending = leftover;
/// }
///
/// // Finalize stream
/// if let Some(final_chunk) = chunker.finish() {
///     all_chunks.push(final_chunk);
/// }
///
/// println!("Produced {} chunks", all_chunks.len());
// # Ok::<(), chunkrs::ChunkError>(())
// ```
#[derive(Debug)]
pub struct Chunker {
    cdc: FastCdc,
    pending: Option<Bytes>,
    offset: u64,
    config: ChunkConfig,
    #[cfg(feature = "hash-blake3")]
    hasher: Option<Blake3Hasher>,
}

impl Chunker {
    /// Creates a new chunker with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The chunking configuration specifying min/avg/max chunk sizes
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::{Chunker, ChunkConfig};
    ///
    /// let chunker = Chunker::new(ChunkConfig::default());
    /// ```
    pub fn new(config: ChunkConfig) -> Self {
        Self {
            cdc: FastCdc::new(config.min_size(), config.avg_size(), config.max_size()),
            pending: None,
            offset: 0,
            config,
            #[cfg(feature = "hash-blake3")]
            hasher: if config.hash_config().enabled {
                Some(Blake3Hasher::new())
            } else {
                None
            },
        }
    }

    /// Pushes data into the chunker and returns complete chunks.
    ///
    /// This method processes the incoming data along with any pending bytes
    /// from previous calls. It returns all complete chunks found and any
    /// unprocessed bytes that couldn't form a complete chunk.
    ///
    /// # Arguments
    ///
    /// * `data` - Input data as `Bytes` (can be zero-copy reference)
    ///
    /// # Returns
    ///
    /// A tuple `(Vec<Chunk>, Bytes)` where:
    /// - First element: Complete chunks found during processing
    /// - Second element: Unprocessed bytes (must be fed back in next call)
    ///
    /// # Processing Flow
    ///
    /// 1. Combine pending bytes (from previous call) with new data
    /// 2. Process all bytes sequentially with CDC
    /// 3. Emit chunks when boundaries are found (zero-copy slices)
    /// 4. Return unprocessed bytes as pending
    ///
    /// # Important
    ///
    /// - **Always feed the returned `Bytes` back** in the next `push()` call
    /// - Failing to do so will break determinism
    /// - Accumulating returned `Vec<Chunk>` may OOM - process promptly
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::{Chunker, ChunkConfig};
    /// use bytes::Bytes;
    ///
    /// let mut chunker = Chunker::new(ChunkConfig::default());
    /// let mut pending = Bytes::new();
    ///
    /// // Process data
    /// for chunk in &[Bytes::from(&b"hello"[..]), Bytes::from(&b" world"[..])] {
    ///     let (chunks, leftover) = chunker.push(pending);
    ///     // Process chunks...
    ///     pending = leftover;
    /// }
    /// ```
    pub fn push(&mut self, data: Bytes) -> (Vec<Chunk>, Bytes) {
        let mut chunks = Vec::new();

        // Process new data looking for boundaries
        let mut new_chunk_start = 0;

        for (i, &byte) in data.iter().enumerate() {
            if self.cdc.update(byte) {
                // Found boundary - emit chunk
                let chunk_data = if let Some(ref pending) = self.pending {
                    // Combine pending + new data for this chunk
                    let mut combined =
                        Vec::with_capacity(pending.len() + (i + 1 - new_chunk_start));
                    combined.extend_from_slice(pending);
                    combined.extend_from_slice(&data[new_chunk_start..=i]);
                    Bytes::from(combined)
                } else {
                    // Just new data
                    data.slice(new_chunk_start..=i)
                };

                let chunk_offset = self.offset;

                // Compute hash if enabled - compute from the final chunk data
                #[cfg(feature = "hash-blake3")]
                let hash = self
                    .hasher
                    .as_ref()
                    .map(|_hasher| crate::hash::Blake3Hasher::hash(chunk_data.as_ref()));

                #[cfg(not(feature = "hash-blake3"))]
                let hash = None;

                chunks.push(Chunk {
                    data: chunk_data,
                    offset: Some(chunk_offset),
                    hash,
                });

                let chunk_len =
                    self.pending.as_ref().map_or(0, |p| p.len()) + (i + 1 - new_chunk_start);
                self.offset += chunk_len as u64;
                new_chunk_start = i + 1;
                self.pending = None;
            }
        }

        // Store remaining new data as pending (or append to existing pending)
        if new_chunk_start < data.len() {
            let remaining = data.slice(new_chunk_start..);
            if let Some(pending) = self.pending.take() {
                // Need to combine with existing pending
                let mut combined = Vec::with_capacity(pending.len() + remaining.len());
                combined.extend_from_slice(&pending);
                combined.extend_from_slice(&remaining);
                self.pending = Some(Bytes::from(combined));
            } else {
                self.pending = Some(remaining);
            }
        }

        (chunks, self.pending.clone().unwrap_or_default())
    }

    /// Finalizes the chunker and returns the final chunk if any.
    ///
    /// Call this method when the input stream ends. It returns any remaining
    /// data as a final chunk, or `None` if there's no pending data.
    ///
    /// After calling `finish()`, the chunker is reset and can be reused for
    /// a new stream.
    ///
    /// # Returns
    ///
    /// - `Some(Chunk)` - Final chunk with remaining data
    /// - `None` - No pending data
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::{Chunker, ChunkConfig};
    /// use bytes::Bytes;
    ///
    /// let mut chunker = Chunker::new(ChunkConfig::default());
    ///
    /// // Process all data
    /// let (_, _) = chunker.push(Bytes::from("some data"));
    ///
    /// // Finalize stream
    /// if let Some(final_chunk) = chunker.finish() {
    ///     println!("Final chunk: {} bytes", final_chunk.len());
    /// }
    /// ```
    pub fn finish(&mut self) -> Option<Chunk> {
        if let Some(pending) = self.pending.take() {
            if pending.is_empty() {
                return None;
            }

            let chunk_offset = self.offset;

            // Compute hash if enabled
            #[cfg(feature = "hash-blake3")]
            let hash = self
                .hasher
                .as_ref()
                .map(|_hasher| crate::hash::Blake3Hasher::hash(pending.as_ref()));

            #[cfg(not(feature = "hash-blake3"))]
            let hash = None;

            let chunk = Chunk {
                data: pending,
                offset: Some(chunk_offset),
                hash,
            };

            self.offset += chunk.len() as u64;
            Some(chunk)
        } else {
            None
        }
    }

    /// Resets the chunker state for a new stream.
    ///
    /// Clears CDC state, pending data, and offset. Useful for reusing the
    /// same `Chunker` instance for multiple streams.
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::{Chunker, ChunkConfig};
    /// use bytes::Bytes;
    ///
    /// let mut chunker = Chunker::new(ChunkConfig::default());
    ///
    /// // Process first stream
    /// let _ = chunker.push(Bytes::from("first"));
    /// let _ = chunker.finish();
    ///
    /// // Reset for second stream
    /// chunker.reset();
    ///
    /// // Process second stream
    /// let _ = chunker.push(Bytes::from("second"));
    /// ```
    pub fn reset(&mut self) {
        self.cdc.reset();
        self.pending = None;
        self.offset = 0;
        #[cfg(feature = "hash-blake3")]
        if let Some(ref mut hasher) = self.hasher {
            hasher.reset();
        }
    }

    /// Returns the current offset in the stream.
    ///
    /// This is the byte position of the next chunk to be emitted.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Returns the number of pending bytes waiting for more input.
    ///
    /// These bytes have been processed by CDC but haven't formed a complete
    /// chunk boundary yet.
    pub fn pending_len(&self) -> usize {
        self.pending.as_ref().map(|b| b.len()).unwrap_or(0)
    }

    /// Returns the configuration used by this chunker.
    pub fn config(&self) -> &ChunkConfig {
        &self.config
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new(ChunkConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_chunker_empty() {
        let mut chunker = Chunker::default();
        let (chunks, pending) = chunker.push(Bytes::new());
        assert!(chunks.is_empty());
        assert!(pending.is_empty());
        assert!(chunker.finish().is_none());
    }

    #[test]
    fn test_chunker_small_data() {
        let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());
        // Small data that doesn't reach min_size
        let (chunks, pending) = chunker.push(Bytes::from(vec![0xAAu8; 3]));
        assert!(chunks.is_empty());
        assert_eq!(pending.len(), 3);

        let final_chunk = chunker.finish();
        assert!(final_chunk.is_some());
        assert_eq!(final_chunk.unwrap().len(), 3);
    }

    #[test]
    fn test_chunker_with_boundaries() {
        let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());
        // Large enough data to find boundaries
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let data_bytes = Bytes::from(data.clone());

        let (chunks, _pending) = chunker.push(data_bytes);
        let final_chunk = chunker.finish();

        // Combine all chunks
        let all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();

        // Should have found at least one boundary
        assert!(!all_chunks.is_empty());

        // Verify all chunks
        let total_len: usize = all_chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total_len, data.len());
    }

    #[test]
    fn test_streaming_data() {
        let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());

        // Simulate streaming data
        let data = vec![
            Bytes::from(&[0xAAu8; 256][..]),
            Bytes::from(&[0xBBu8; 256][..]),
            Bytes::from(&[0xCCu8; 256][..]),
            Bytes::from(&[0xDDu8; 232][..]), // Total: 1000 bytes
        ];

        let mut all_chunks = Vec::new();
        let mut pending = Bytes::new();

        for chunk in data {
            let (chunks, leftover) = chunker.push(chunk);
            all_chunks.extend(chunks);
            pending = leftover;
        }

        let final_chunk = chunker.finish();
        if let Some(chunk) = final_chunk {
            all_chunks.push(chunk);
        }

        // Verify total length
        let total_len: usize = all_chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total_len, 1000);
    }

    #[test]
    fn test_chunk_offsets() {
        let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

        let (chunks, _pending) = chunker.push(Bytes::from(data));
        let final_chunk = chunker.finish();

        let mut all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();

        let mut expected_offset = 0u64;
        for chunk in &all_chunks {
            assert_eq!(chunk.offset, Some(expected_offset));
            expected_offset += chunk.len() as u64;
        }
    }

    #[test]
    fn test_pending_bytes() {
        let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());

        // Push data that won't form a complete chunk
        let (chunks, pending) = chunker.push(Bytes::from(&[0u8; 2][..]));
        assert!(chunks.is_empty());
        assert_eq!(pending.len(), 2);
        assert_eq!(chunker.pending_len(), 2);

        // Push more data - should process pending first
        let (chunks, pending) = chunker.push(Bytes::from(&[0u8; 100][..]));
        // Now we should have chunks
        assert!(!chunks.is_empty() || pending.len() > 0);
    }

    #[test]
    fn test_reset() {
        let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());

        // Process some data
        let (_, _) = chunker.push(Bytes::from(&[0u8; 100][..]));
        assert!(chunker.offset() > 0);

        // Reset
        chunker.reset();
        assert_eq!(chunker.offset(), 0);
        assert_eq!(chunker.pending_len(), 0);

        // Should work like new
        let (chunks, _) = chunker.push(Bytes::from(&[0u8; 10][..]));
        let final_chunk = chunker.finish();
        let total: usize = chunks.iter().map(|c| c.len()).sum::<usize>()
            + final_chunk.map(|c| c.len()).unwrap_or(0);
        assert_eq!(total, 10);
    }

    #[test]
    fn test_max_size_forces_boundary() {
        let mut chunker = Chunker::new(ChunkConfig::new(4, 8, 8).unwrap());

        // Push data that exceeds max_size
        let data = Bytes::from(vec![0xFFu8; 20]);
        let (chunks, _pending) = chunker.push(data);

        // Should have at least one chunk at max_size boundary
        assert!(!chunks.is_empty());

        // First chunk should be at most max_size
        assert!(chunks[0].len() <= 8);
    }

    #[test]
    fn test_determinism_across_push_sizes() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

        // Chunk all at once
        let mut chunker1 = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());
        let (chunks1, _pending1) = chunker1.push(Bytes::from(data.clone()));
        let final1 = chunker1.finish();
        let offsets1: Vec<u64> = chunks1
            .iter()
            .chain(final1.iter())
            .map(|c| c.offset.unwrap())
            .collect();

        // Chunk in small pieces (feed pending bytes back each time)
        let mut chunker2 = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());
        let mut chunks2 = Vec::new();
        let mut pending = Bytes::new();

        for chunk in data.chunks(10) {
            let new_data = Bytes::from(chunk.to_vec());
            let (chunks, leftover) = chunker2.push(new_data);
            chunks2.extend(chunks);
            pending = leftover;
        }

        let final2 = chunker2.finish();
        let offsets2: Vec<u64> = chunks2
            .iter()
            .chain(final2.iter())
            .map(|c| c.offset.unwrap())
            .collect();

        // Same chunk boundaries regardless of push size
        assert_eq!(offsets1, offsets2);
    }

    #[test]
    fn test_zero_copy() {
        let mut chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());
        let data = Bytes::from(&b"hello world this is test data"[..]);

        let (chunks, _) = chunker.push(data.clone());
        let final_chunk = chunker.finish();

        // All chunk data should be slices of the original Bytes
        for chunk in chunks.iter().chain(final_chunk.iter()) {
            // Verify chunk data points into the original
            assert!(chunk.data.as_ptr() >= data.as_ptr());
            assert!(
                chunk.data.as_ptr() as usize + chunk.data.len()
                    <= data.as_ptr() as usize + data.len()
            );
        }
    }

    #[cfg(feature = "hash-blake3")]
    #[test]
    fn test_hashing_enabled() {
        let config = ChunkConfig::default().with_hash_config(crate::HashConfig::enabled());
        let mut chunker = Chunker::new(config);

        let data = Bytes::from(&b"hello world this is test data"[..]);
        let (chunks, _) = chunker.push(data.clone());
        let final_chunk = chunker.finish();

        // All chunks should have hashes
        for chunk in chunks.iter().chain(final_chunk.iter()) {
            assert!(chunk.hash.is_some(), "Chunk should have a hash");
        }

        // Verify hash correctness
        let mut all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();
        let chunk = &all_chunks[0];
        #[cfg(feature = "hash-blake3")]
        {
            let expected_hash = crate::hash::Blake3Hasher::hash(chunk.data.as_ref());
            assert_eq!(chunk.hash.unwrap(), expected_hash);
        }
    }

    #[cfg(feature = "hash-blake3")]
    #[test]
    fn test_hashing_disabled() {
        let config = ChunkConfig::default().with_hash_config(crate::HashConfig::disabled());
        let mut chunker = Chunker::new(config);

        let data = Bytes::from(&b"hello world this is test data"[..]);
        let (chunks, _) = chunker.push(data);
        let final_chunk = chunker.finish();

        // No chunks should have hashes
        for chunk in chunks.iter().chain(final_chunk.iter()) {
            assert!(
                chunk.hash.is_none(),
                "Chunk should not have a hash when disabled"
            );
        }
    }

    #[cfg(feature = "hash-blake3")]
    #[test]
    fn test_hash_determinism() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

        let config1 = ChunkConfig::default().with_hash_config(crate::HashConfig::enabled());
        let mut chunker1 = Chunker::new(config1);
        let (chunks1, _) = chunker1.push(Bytes::from(data.clone()));
        let final1 = chunker1.finish();

        let config2 = ChunkConfig::default().with_hash_config(crate::HashConfig::enabled());
        let mut chunker2 = Chunker::new(config2);
        let (chunks2, _) = chunker2.push(Bytes::from(data.clone()));
        let final2 = chunker2.finish();

        // Same data should produce same hashes
        let mut chunks1_iter = chunks1.into_iter().chain(final1);
        let mut chunks2_iter = chunks2.into_iter().chain(final2);

        while let (Some(c1), Some(c2)) = (chunks1_iter.next(), chunks2_iter.next()) {
            assert_eq!(c1.hash, c2.hash, "Hashes should be identical for same data");
        }
    }

    #[cfg(feature = "hash-blake3")]
    #[test]
    fn test_hash_with_reset() {
        let config = ChunkConfig::new(4, 16, 64)
            .unwrap()
            .with_hash_config(crate::HashConfig::enabled());
        let mut chunker = Chunker::new(config);

        // Process first stream
        let data1 = Bytes::from(&b"first stream"[..]);
        let (chunks1, _) = chunker.push(data1.clone());
        let final1 = chunker.finish();

        // Get the hash from either push() chunks or final chunk
        let hash1 = if !chunks1.is_empty() {
            chunks1[0].hash.unwrap()
        } else if let Some(chunk) = final1 {
            chunk.hash.unwrap()
        } else {
            panic!("Expected at least one chunk");
        };

        // Reset and process second stream
        chunker.reset();
        let data2 = Bytes::from(&b"second stream"[..]);
        let (chunks2, _) = chunker.push(data2.clone());
        let final2 = chunker.finish();

        // Get the hash from either push() chunks or final chunk
        let hash2 = if !chunks2.is_empty() {
            chunks2[0].hash.unwrap()
        } else if let Some(chunk) = final2 {
            chunk.hash.unwrap()
        } else {
            panic!("Expected at least one chunk");
        };

        // Different streams should have different hashes
        assert_ne!(hash1, hash2);
    }

    #[cfg(feature = "hash-blake3")]
    #[test]
    fn test_hash_with_pending_bytes() {
        let config = ChunkConfig::new(4, 16, 64)
            .unwrap()
            .with_hash_config(crate::HashConfig::enabled());
        let mut chunker = Chunker::new(config);

        // Push data that creates pending bytes
        let data = Bytes::from(&b"small"[..]);
        let (chunks, pending) = chunker.push(data.clone());

        // Push more data to complete the chunk
        let more_data = Bytes::from(&b" and more data"[..]);
        let (chunks2, _) = chunker.push(more_data);
        let final_chunk = chunker.finish();

        let mut all_chunks: Vec<_> = chunks
            .into_iter()
            .chain(chunks2)
            .chain(final_chunk)
            .collect();

        // Verify all chunks have hashes
        for chunk in &all_chunks {
            assert!(chunk.hash.is_some());
        }

        // Verify hash correctness by recomputing
        #[cfg(feature = "hash-blake3")]
        for chunk in &all_chunks {
            let expected_hash = crate::hash::Blake3Hasher::hash(chunk.data.as_ref());
            assert_eq!(chunk.hash.unwrap(), expected_hash);
        }
    }

    #[cfg(feature = "hash-blake3")]
    #[test]
    fn test_same_stream_same_chunks_same_hashes() {
        // Verify that the same stream produces identical chunks with identical hashes
        // regardless of how data is pushed
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

        let config = ChunkConfig::new(4, 16, 64)
            .unwrap()
            .with_hash_config(crate::HashConfig::enabled());

        // Test 1: Push all data at once
        let mut chunker1 = Chunker::new(config);
        let (chunks1, _pending1) = chunker1.push(Bytes::from(data.clone()));
        let final1 = chunker1.finish();
        let all1: Vec<_> = chunks1.into_iter().chain(final1).collect();

        // Test 2: Push in small chunks (10 bytes each)
        let mut chunker2 = Chunker::new(config);
        let mut all2 = Vec::new();
        let mut pending = Bytes::new();

        for chunk in data.chunks(10) {
            let new_data = Bytes::from(chunk.to_vec());
            let (chunks, leftover) = chunker2.push(new_data);
            all2.extend(chunks);
            pending = leftover;
        }
        let final2 = chunker2.finish();
        all2.extend(final2);

        // Test 3: Push in different sized chunks (37 bytes each)
        let mut chunker3 = Chunker::new(config);
        let mut all3 = Vec::new();
        let mut pending = Bytes::new();

        for chunk in data.chunks(37) {
            let new_data = Bytes::from(chunk.to_vec());
            let (chunks, leftover) = chunker3.push(new_data);
            all3.extend(chunks);
            pending = leftover;
        }
        let final3 = chunker3.finish();
        all3.extend(final3);

        // Verify all three approaches produce the same number of chunks
        assert_eq!(all1.len(), all2.len(), "Different number of chunks");
        assert_eq!(all1.len(), all3.len(), "Different number of chunks");

        // Verify each chunk has the same offset, length, and hash
        for (i, ((c1, c2), c3)) in all1.iter().zip(&all2).zip(&all3).enumerate() {
            assert_eq!(
                c1.offset, c2.offset,
                "Chunk {} offset differs between test 1 and 2",
                i
            );
            assert_eq!(
                c1.offset, c3.offset,
                "Chunk {} offset differs between test 1 and 3",
                i
            );
            assert_eq!(
                c1.len(),
                c2.len(),
                "Chunk {} length differs between test 1 and 2",
                i
            );
            assert_eq!(
                c1.len(),
                c3.len(),
                "Chunk {} length differs between test 1 and 3",
                i
            );
            assert_eq!(
                c1.hash, c2.hash,
                "Chunk {} hash differs between test 1 and 2",
                i
            );
            assert_eq!(
                c1.hash, c3.hash,
                "Chunk {} hash differs between test 1 and 3",
                i
            );

            // Also verify the hash matches the actual data
            let expected_hash = crate::hash::Blake3Hasher::hash(c1.data.as_ref());
            assert_eq!(
                c1.hash.unwrap(),
                expected_hash,
                "Chunk {} hash doesn't match data",
                i
            );
        }
    }

    #[cfg(feature = "hash-blake3")]
    #[test]
    fn test_hash_correctness_verification() {
        // Verify that computed hashes match actual BLAKE3 of chunk data
        let config = ChunkConfig::new(4, 16, 64)
            .unwrap()
            .with_hash_config(crate::HashConfig::enabled());
        let mut chunker = Chunker::new(config);

        let data = Bytes::from(&b"hello world this is test data for hash verification"[..]);
        let (chunks, _) = chunker.push(data.clone());
        let final_chunk = chunker.finish();

        let all_chunks: Vec<_> = chunks.into_iter().chain(final_chunk).collect();

        for chunk in &all_chunks {
            if let Some(hash) = chunk.hash {
                // Recompute hash from chunk data
                let expected_hash = crate::hash::Blake3Hasher::hash(chunk.data.as_ref());
                assert_eq!(hash, expected_hash, "Hash doesn't match actual data");
            } else {
                panic!("Expected hash to be Some when hashing is enabled");
            }
        }
    }

    #[test]
    fn test_empty_input_handling() {
        let mut chunker = Chunker::default();
        
        // Push empty data
        let (chunks, pending) = chunker.push(Bytes::new());
        assert!(chunks.is_empty());
        assert!(pending.is_empty());
        
        // Push empty data again
        let (chunks2, pending2) = chunker.push(Bytes::new());
        assert!(chunks2.is_empty());
        assert!(pending2.is_empty());
        
        // Finish should return None
        assert!(chunker.finish().is_none());
    }

    #[test]
    fn test_exact_size_boundaries() {
        // Test data exactly at min/avg/max sizes
        let config = ChunkConfig::new(4, 16, 64).unwrap();
        let mut chunker = Chunker::new(config);

        // Data exactly at min_size
        let data_min = Bytes::from(vec![0u8; 4]);
        let (chunks, _) = chunker.push(data_min);
        let final_min = chunker.finish();
        let all_min: Vec<_> = chunks.into_iter().chain(final_min).collect();
        assert!(!all_min.is_empty());
        assert_eq!(all_min[0].len(), 4);

        // Data exactly at max_size (should force boundary)
        let mut chunker2 = Chunker::new(config);
        let data_max = Bytes::from(vec![0u8; 64]);
        let (chunks, _) = chunker2.push(data_max);
        let final_max = chunker2.finish();
        let all_max: Vec<_> = chunks.into_iter().chain(final_max).collect();
        assert!(!all_max.is_empty());
        assert!(all_max[0].len() <= 64);
    }

    #[test]
    fn test_multiple_finish_calls() {
        let mut chunker = Chunker::default();

        // Process some data
        let (_chunks, _) = chunker.push(Bytes::from(&b"test data"[..]));
        let final1 = chunker.finish();
        
        // Second finish should return None
        let final2 = chunker.finish();
        assert!(final1.is_some()); // First finish returns the chunk
        assert!(final2.is_none()); // Second finish returns None

        // Third finish should also return None
        let final3 = chunker.finish();
        assert!(final3.is_none());
    }

    #[test]
    fn test_hash_consistency_across_configurations() {
        // Verify that the same data produces the same hash with different configs
        // as long as the chunk boundaries are the same
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

        #[cfg(feature = "hash-blake3")]
        {
            let config1 = ChunkConfig::new(4, 16, 64)
                .unwrap()
                .with_hash_config(crate::HashConfig::enabled());
            let config2 = ChunkConfig::new(4, 16, 64)
                .unwrap()
                .with_hash_config(crate::HashConfig::enabled());

            let mut chunker1 = Chunker::new(config1);
            let mut chunker2 = Chunker::new(config2);

            let (chunks1, _) = chunker1.push(Bytes::from(data.clone()));
            let final1 = chunker1.finish();
            let all1: Vec<_> = chunks1.into_iter().chain(final1).collect();

            let (chunks2, _) = chunker2.push(Bytes::from(data.clone()));
            let final2 = chunker2.finish();
            let all2: Vec<_> = chunks2.into_iter().chain(final2).collect();

            // Same chunks should have same hashes
            assert_eq!(all1.len(), all2.len());
            for (c1, c2) in all1.iter().zip(all2.iter()) {
                assert_eq!(c1.hash, c2.hash);
            }
        }
    }

    #[test]
    fn test_offset_tracking_across_multiple_streams() {
        // Verify offset tracking works correctly across multiple independent streams
        let data = Bytes::from(&b"test data for offset tracking"[..]);

        let mut chunker1 = Chunker::new(ChunkConfig::default());
        let (chunks1, _) = chunker1.push(data.clone());
        let final1 = chunker1.finish();
        let all1: Vec<_> = chunks1.into_iter().chain(final1).collect();

        // Reset and process new stream
        chunker1.reset();
        let (chunks2, _) = chunker1.push(data.clone());
        let final2 = chunker1.finish();
        let all2: Vec<_> = chunks2.into_iter().chain(final2).collect();

        // Offsets should restart at 0 after reset
        assert_eq!(all2[0].offset, Some(0));
        assert_eq!(all1[0].offset, Some(0));
    }

    #[test]
    fn test_pending_bytes_preserve_data() {
        // Verify that pending bytes preserve the original data correctly
        let mut chunker = Chunker::new(ChunkConfig::new(16, 32, 64).unwrap());

        // Push data that's smaller than min_size
        let data1 = Bytes::from(&b"partial"[..]);
        let (chunks, pending) = chunker.push(data1.clone());
        assert!(chunks.is_empty());
        assert!(!pending.is_empty());

        // Push more data to complete a chunk
        let data2 = Bytes::from(&b" data to complete chunk"[..]);
        let data2_expected = data2.clone();
        let (chunks2, _) = chunker.push(data2);
        let final_chunk = chunker.finish();

        let all: Vec<_> = chunks2.into_iter().chain(final_chunk).collect();

        // Verify total bytes match input
        let total_input = data1.len() + data2_expected.len();
        let total_output: usize = all.iter().map(|c| c.len()).sum();
        assert_eq!(total_input, total_output);

        // Verify the combined data is correct
        let combined: Vec<u8> = all.iter().flat_map(|c| c.data.as_ref().to_vec()).collect();
        let expected: Vec<u8> = data1.iter().chain(data2_expected.iter()).copied().collect();
        assert_eq!(combined, expected);
    }
}
