//! Core chunking engine - Chunker and ChunkIter.
//!
//! This module implements the synchronous chunking API using the FastCDC
//! algorithm. It provides two main types:
//!
//! - [`Chunker`] - Configures and initiates chunking operations
//! - [`ChunkIter`] - Iterator that yields chunks from a [`std::io::Read`] source
//!
//! # Example
//!
//! ```ignore
//! use chunkrs::{Chunker, ChunkConfig};
//! use std::fs::File;
//!
//! let file = File::open("data.bin")?;
//! let chunker = Chunker::new(ChunkConfig::default());
//!
//! for chunk in chunker.chunk(file) {
//!     let chunk = chunk?;
//!     println!("Chunk: {} bytes", chunk.len());
//! }
//! # Ok::<(), chunkrs::ChunkError>(())
//! ```

use std::io::Read;

use bytes::Bytes;

use crate::buffer::Buffer;
use crate::cdc::FastCdc;
use crate::chunk::Chunk;
use crate::config::ChunkConfig;
use crate::error::ChunkError;

#[cfg(feature = "hash-blake3")]
use crate::hash::Blake3Hasher;

/// A chunker that processes byte streams into content-defined chunks.
///
/// `Chunker` is the high-level API for synchronous chunking. It holds a
/// configuration and provides methods to chunk data from various sources.
///
/// # Example
///
/// ```
/// use chunkrs::{Chunker, ChunkConfig};
/// use std::io::Cursor;
///
/// let data = b"some data to chunk";
/// let chunker = Chunker::new(ChunkConfig::default());
/// let chunks: Vec<_> = chunker.chunk(Cursor::new(&data[..])).collect::<Result<_, _>>()?;
/// assert!(!chunks.is_empty());
/// # Ok::<(), chunkrs::ChunkError>(())
/// ```
#[derive(Debug, Clone)]
pub struct Chunker {
    config: ChunkConfig,
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
    /// let config = ChunkConfig::default();
    /// let chunker = Chunker::new(config);
    /// ```
    pub fn new(config: ChunkConfig) -> Self {
        Self { config }
    }

    /// Creates a chunking iterator from a reader.
    ///
    /// This method returns an iterator that lazily reads from the reader and
    /// yields chunks as boundaries are found.
    ///
    /// # Arguments
    ///
    /// * `reader` - Any type implementing [`std::io::Read`]
    ///
    /// # Returns
    ///
    /// A [`ChunkIter`] that yields [`Result<Chunk, ChunkError>`]
    ///
    /// # Example
    ///
    /// ```ignore
    /// use chunkrs::{Chunker, ChunkConfig};
    /// use std::io::File;
    ///
    /// let file = File::open("data.bin")?;
    /// let chunker = Chunker::new(ChunkConfig::default());
    ///
    /// for chunk in chunker.chunk(file) {
    ///     let chunk = chunk?;
    ///     println!("Chunk: {} bytes", chunk.len());
    /// }
    /// # Ok::<(), chunkrs::ChunkError>(())
    /// ```
    pub fn chunk<R: Read>(self, reader: R) -> ChunkIter<R> {
        ChunkIter::new(reader, self.config)
    }

    /// Chunks an in-memory buffer.
    ///
    /// This is a convenience method for chunking data that is already in memory.
    /// It's more efficient than creating a [`std::io::Cursor`] and using
    /// [`Chunker::chunk`] for small to medium-sized data.
    ///
    /// # Arguments
    ///
    /// * `data` - Any type that can be converted to [`Bytes`]
    ///
    /// # Returns
    ///
    /// A vector of [`Chunk`] objects
    ///
    /// # Example
    ///
    /// ```
    /// use chunkrs::{Chunker, ChunkConfig};
    ///
    /// let chunker = Chunker::new(ChunkConfig::default());
    /// let chunks = chunker.chunk_bytes(&b"hello world"[..]);
    ///
    /// assert!(!chunks.is_empty());
    /// ```
    pub fn chunk_bytes(&self, data: impl Into<Bytes>) -> Vec<Chunk> {
        let data = data.into();
        let mut chunks = Vec::new();
        let mut cdc = FastCdc::new(
            self.config.min_size(),
            self.config.avg_size(),
            self.config.max_size(),
        );

        let offset = 0u64;
        let mut chunk_start = 0usize;

        #[cfg(feature = "hash-blake3")]
        let mut hasher = if self.config.hash_config().enabled {
            Some(Blake3Hasher::new())
        } else {
            None
        };

        for (i, &byte) in data.iter().enumerate() {
            if cdc.update(byte) {
                let chunk_data = data.slice(chunk_start..i + 1);

                #[cfg(feature = "hash-blake3")]
                let hash = hasher.as_mut().map(|h| {
                    h.update(&chunk_data);
                    let hash = h.finalize();
                    h.reset();
                    hash
                });
                #[cfg(not(feature = "hash-blake3"))]
                let hash: Option<crate::chunk::ChunkHash> = None;

                chunks.push(Chunk {
                    data: chunk_data,
                    offset: Some(offset + chunk_start as u64),
                    hash,
                });

                chunk_start = i + 1;
            }
        }

        // Handle trailing data (force boundary at end)
        if chunk_start < data.len() {
            let chunk_data = data.slice(chunk_start..);

            #[cfg(feature = "hash-blake3")]
            let hash = hasher.as_mut().map(|h| {
                h.update(&chunk_data);
                h.finalize()
            });
            #[cfg(not(feature = "hash-blake3"))]
            let hash: Option<crate::chunk::ChunkHash> = None;

            chunks.push(Chunk {
                data: chunk_data,
                offset: Some(offset + chunk_start as u64),
                hash,
            });
        }

        chunks
    }
}

impl Default for Chunker {
    fn default() -> Self {
        Self::new(ChunkConfig::default())
    }
}

/// An iterator that yields chunks from a reader.
///
/// `ChunkIter` reads data from a [`std::io::Read`] source incrementally and
/// yields chunks as the FastCDC algorithm identifies boundaries.
///
/// The iterator is lazy and reads in chunks of up to 8KB at a time, making it
/// efficient for streaming large data sources.
///
/// # Example
///
/// ```ignore
/// use chunkrs::{Chunker, ChunkConfig};
/// use std::io::File;
///
/// let file = File::open("data.bin")?;
/// let chunker = Chunker::new(ChunkConfig::default());
/// let mut iter = chunker.chunk(file);
///
/// while let Some(result) = iter.next() {
///     let chunk = result?;
///     println!("Chunk: {} bytes", chunk.len());
/// }
/// # Ok::<(), chunkrs::ChunkError>(())
/// ```
pub struct ChunkIter<R> {
    reader: R,
    config: ChunkConfig,
    cdc: FastCdc,
    buffer: Buffer,
    chunk_buffer: Vec<u8>,
    offset: u64,
    #[cfg(feature = "hash-blake3")]
    hasher: Option<Blake3Hasher>,
    finished: bool,
}

impl<R: Read> ChunkIter<R> {
    /// Creates a new chunk iterator.
    ///
    /// # Arguments
    ///
    /// * `reader` - The source of data to chunk
    /// * `config` - The chunking configuration
    fn new(reader: R, config: ChunkConfig) -> Self {
        let cdc = FastCdc::new(config.min_size(), config.avg_size(), config.max_size());

        #[cfg(feature = "hash-blake3")]
        let hasher = if config.hash_config().enabled {
            Some(Blake3Hasher::new())
        } else {
            None
        };

        Self {
            reader,
            config,
            cdc,
            buffer: Buffer::take(),
            chunk_buffer: Vec::with_capacity(config.max_size()),
            offset: 0,
            #[cfg(feature = "hash-blake3")]
            hasher,
            finished: false,
        }
    }

    /// Processes the chunk buffer and returns a chunk.
    ///
    /// This internal method extracts a chunk of the specified length from
    /// the buffer, computes its hash if enabled, and updates the offset.
    fn emit_chunk(&mut self, len: usize) -> Chunk {
        let data = Bytes::copy_from_slice(&self.chunk_buffer[..len]);
        let chunk_offset = self.offset;

        #[cfg(feature = "hash-blake3")]
        let hash = self.hasher.as_mut().map(|h| {
            h.update(&data);
            let hash = h.finalize();
            h.reset();
            hash
        });
        #[cfg(not(feature = "hash-blake3"))]
        let hash: Option<crate::chunk::ChunkHash> = None;

        // Keep any remaining data
        if len < self.chunk_buffer.len() {
            self.chunk_buffer.copy_within(len.., 0);
            self.chunk_buffer.truncate(self.chunk_buffer.len() - len);
        } else {
            self.chunk_buffer.clear();
        }

        self.offset += len as u64;

        Chunk {
            data,
            offset: Some(chunk_offset),
            hash,
        }
    }
}

impl<R: Read> Iterator for ChunkIter<R> {
    type Item = Result<Chunk, ChunkError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            // Try to find a boundary in existing data
            if !self.chunk_buffer.is_empty() {
                // Scan for boundary
                let mut found = None;
                for (i, &byte) in self.chunk_buffer.iter().enumerate() {
                    if self.cdc.update(byte) {
                        found = Some(i + 1);
                        break;
                    }
                }

                if let Some(len) = found {
                    return Some(Ok(self.emit_chunk(len)));
                }

                // No boundary found - check if we've exceeded max size
                if self.chunk_buffer.len() >= self.config.max_size() {
                    // Force a boundary at max_size
                    let len = self.config.max_size();
                    return Some(Ok(self.emit_chunk(len)));
                }
            }

            // Read more data
            self.buffer.clear();
            let mut temp_buf = vec![0u8; 8192];
            match self.reader.read(&mut temp_buf) {
                Ok(0) => {
                    // End of stream - emit remaining data if any
                    if !self.chunk_buffer.is_empty() {
                        let len = self.chunk_buffer.len();
                        let chunk = self.emit_chunk(len);
                        self.finished = true;
                        return Some(Ok(chunk));
                    }
                    self.finished = true;
                    return None;
                }
                Ok(n) => {
                    temp_buf.truncate(n);
                    self.chunk_buffer.extend_from_slice(&temp_buf);
                }
                Err(e) => {
                    self.finished = true;
                    return Some(Err(e.into()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_chunker_empty() {
        let chunker = Chunker::default();
        let chunks = chunker.chunk_bytes(&b""[..]);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunker_small_data() {
        let chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());
        // Small data that doesn't reach min_size
        let chunks: Vec<u8> = vec![0xAAu8; 3];
        let chunks = chunker.chunk_bytes(chunks);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 3);
    }

    #[test]
    fn test_chunker_with_boundaries() {
        let chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());
        // Large enough data to find boundaries
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let chunks = chunker.chunk_bytes(data.clone());

        // Should have found at least one boundary
        assert!(!chunks.is_empty());

        // Verify all chunks
        let total_len: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total_len, data.len());
    }

    #[test]
    #[cfg(feature = "hash-blake3")]
    fn test_chunker_with_hashes() {
        let config = ChunkConfig::default().with_hash_config(crate::config::HashConfig::enabled());
        let chunker = Chunker::new(config);

        let data = b"hello world this is some test data";
        let chunks = chunker.chunk_bytes(&data[..]);

        // All chunks should have hashes
        for chunk in &chunks {
            assert!(chunk.hash.is_some());
        }
    }

    #[test]
    fn test_chunker_iterator() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());

        let chunks: Vec<_> = chunker
            .chunk(Cursor::new(&data))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let total_len: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total_len, data.len());
    }

    #[test]
    fn test_chunk_offsets() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let chunker = Chunker::new(ChunkConfig::new(4, 16, 64).unwrap());

        let chunks = chunker.chunk_bytes(data);

        let mut expected_offset = 0u64;
        for chunk in &chunks {
            assert_eq!(chunk.offset, Some(expected_offset));
            expected_offset += chunk.len() as u64;
        }
    }
}
