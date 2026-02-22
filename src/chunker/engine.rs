//! Core chunking engine - Chunker with streaming API.
//!
//! - [`Chunker`] - Stateful CDC engine
//! - `push()` - Feed data in any size
//! - `finish()` - Flush remaining data
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
//! let chunks1 = chunker.push(Bytes::from(&b"first"[..]));
//! let chunks2 = chunker.push(Bytes::from(&b"second"[..]));
//! let final_chunk = chunker.finish();
//! # Ok::<(), chunkrs::ChunkError>(())
//! ```

use bytes::Bytes;

use crate::cdc::FastCdc;
use crate::chunk::Chunk;
use crate::config::ChunkConfig;

/// A chunker that processes streaming byte data into content-defined chunks.
///
/// # Streaming API
///
/// - Call `push()` with data in any size
/// - Returns complete chunks and unprocessed bytes
/// - Feed unprocessed bytes back in the next `push()` call
/// - Call `finish()` when stream ends
///
/// # Determinism
///
/// Identical byte streams produce identical chunk boundaries, regardless of:
/// - How many bytes are pushed at once
/// - Call timing
/// - Number of `push()` calls
///
/// # Zero-Copy
///
/// Chunk data is zero-copy sliced from input `Bytes`.
///
/// # Example
///
/// ```
/// use chunkrs::{Chunker, ChunkConfig};
/// use bytes::Bytes;
///
/// let mut chunker = Chunker::new(ChunkConfig::default());
///
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
/// if let Some(final_chunk) = chunker.finish() {
///     all_chunks.push(final_chunk);
/// }
///
/// println!("Produced {} chunks", all_chunks.len());
/// # Ok::<(), chunkrs::ChunkError>(())
/// ```
#[derive(Debug)]
pub struct Chunker {
    cdc: FastCdc,
    pending: Option<Bytes>,
    offset: u64,
    config: ChunkConfig,
}

impl Chunker {
    /// Creates a new chunker with the given configuration.
    pub fn new(config: ChunkConfig) -> Self {
        #[cfg(feature = "keyed-cdc")]
        let key = config.keyed_gear_table_key();
        #[cfg(not(feature = "keyed-cdc"))]
        let key = None;

        Self {
            cdc: FastCdc::with_key(
                config.min_size(),
                config.avg_size(),
                config.max_size(),
                config.normalization_level(),
                key,
            ),
            pending: None,
            offset: 0,
            config,
        }
    }

    /// Computes hash for the given data if hashing is enabled.
    fn compute_hash(&self, data: &[u8]) -> Option<crate::chunk::ChunkHash> {
        if !self.config.hash_config().enabled {
            return None;
        }
        #[cfg(feature = "hash-blake3")]
        return Some(crate::hash::Blake3Hasher::hash(data));
        #[cfg(not(feature = "hash-blake3"))]
        return None;
    }

    /// Creates a new Chunk with the given data, offset, and hash.
    fn create_chunk(&self, data: Bytes, offset: u64) -> Chunk {
        let hash = self.compute_hash(data.as_ref());
        Chunk {
            data,
            offset: Some(offset),
            hash,
        }
    }

    /// Pushes data into the chunker and returns complete chunks.
    ///
    /// Returns `(Vec<Chunk>, Bytes)` where the second element must be fed back in the next call.
    pub fn push(&mut self, data: Bytes) -> (Vec<Chunk>, Bytes) {
        let mut chunks = Vec::new();
        let mut new_chunk_start = 0;

        for (i, &byte) in data.iter().enumerate() {
            if self.cdc.update(byte) {
                let chunk_data = if let Some(ref pending) = self.pending {
                    crate::util::combine_bytes(pending, &data[new_chunk_start..=i])
                } else {
                    data.slice(new_chunk_start..=i)
                };

                let chunk_offset = self.offset;
                chunks.push(self.create_chunk(chunk_data, chunk_offset));

                let chunk_len =
                    self.pending.as_ref().map_or(0, |p| p.len()) + (i + 1 - new_chunk_start);
                self.offset += chunk_len as u64;
                new_chunk_start = i + 1;
                self.pending = None;
            }
        }

        if new_chunk_start < data.len() {
            let remaining = data.slice(new_chunk_start..);
            if let Some(pending) = self.pending.take() {
                self.pending = Some(crate::util::combine_bytes(&pending, remaining.as_ref()));
            } else {
                self.pending = Some(remaining);
            }
        }

        (chunks, self.pending.clone().unwrap_or_default())
    }

    /// Finalizes the chunker and returns the final chunk if any.
    pub fn finish(&mut self) -> Option<Chunk> {
        if let Some(pending) = self.pending.take() {
            if pending.is_empty() {
                return None;
            }

            let chunk_offset = self.offset;
            let chunk = self.create_chunk(pending, chunk_offset);

            self.offset += chunk.len() as u64;
            Some(chunk)
        } else {
            None
        }
    }

    /// Resets the chunker state for a new stream.
    pub fn reset(&mut self) {
        self.cdc.reset();
        self.pending = None;
        self.offset = 0;
    }

    /// Returns the current offset in the stream.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Returns the number of pending bytes.
    pub fn pending_len(&self) -> usize {
        self.pending.as_ref().map(|b| b.len()).unwrap_or(0)
    }

    /// Returns the configuration.
    pub fn config(&self) -> &ChunkConfig {
        &self.config
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new(ChunkConfig::default())
    }
}