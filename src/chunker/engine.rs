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