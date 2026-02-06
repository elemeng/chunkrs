//! Async stream adapter for chunking.
//!
//! This module provides asynchronous chunking using the `futures-io::AsyncRead`
//! trait, making it runtime-agnostic and compatible with tokio, async-std,
//! smol, and other async runtimes.
//!
//! # Example
//!
//! ```ignore
//! use futures_util::StreamExt;
//! use chunkrs::{chunk_async, ChunkConfig};
//! use futures_io::AsyncRead;
//!
//! async fn demo<R: AsyncRead + Unpin>(reader: R) -> Result<(), chunkrs::ChunkError> {
//!     let mut stream = chunk_async(reader, ChunkConfig::default());
//!
//!     while let Some(chunk) = stream.next().await {
//!         let chunk = chunk?;
//!         println!("Chunk: {} bytes", chunk.len());
//!     }
//!     Ok(())
//! }
//! ```

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use futures_io::AsyncRead;
use pin_project_lite::pin_project;

use crate::cdc::FastCdc;
use crate::chunk::Chunk;
use crate::config::ChunkConfig;
use crate::error::ChunkError;

#[cfg(feature = "hash-blake3")]
use crate::hash::Blake3Hasher;

pin_project! {
    /// A stream that yields chunks from an async reader.
    ///
    /// This uses `futures_io::AsyncRead` which is runtime-agnostic.
    /// Works with tokio, async-std, smol, or any futures-compatible runtime.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use chunkrs::{chunk_async, ChunkConfig};
    /// use futures_util::StreamExt;
    /// use futures_io::AsyncRead;
    ///
    /// async fn example<R: AsyncRead + Unpin>(reader: R) -> Result<(), chunkrs::ChunkError> {
    ///     let mut stream = chunk_async(reader, ChunkConfig::default());
    ///
    ///     while let Some(chunk) = stream.next().await {
    ///         let chunk = chunk?;
    ///         println!("chunk: {} bytes", chunk.len());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub struct ChunkStream<R> {
        #[pin]
        reader: R,
        config: ChunkConfig,
        cdc: FastCdc,
        buffer: Vec<u8>,
        chunk_buffer: Vec<u8>,
        offset: u64,
        finished: bool,
        // Note: hasher is stored separately due to pin_project_lite limitations with cfg
    }
}

/// Hasher state stored outside the pinned struct.
///
/// This wrapper allows the hasher to be conditionally compiled while
/// maintaining compatibility with the pinned `ChunkStream` struct.
#[cfg(feature = "hash-blake3")]
struct HasherState {
    hasher: Option<Blake3Hasher>,
}

#[cfg(not(feature = "hash-blake3"))]
struct HasherState;

#[cfg(feature = "hash-blake3")]
impl HasherState {
    /// Creates a new hasher state based on the configuration.
    fn new(config: &ChunkConfig) -> Self {
        Self {
            hasher: if config.hash_config().enabled {
                Some(Blake3Hasher::new())
            } else {
                None
            },
        }
    }

    /// Hashes a chunk if hashing is enabled.
    fn hash_chunk(&mut self, data: &Bytes) -> Option<crate::chunk::ChunkHash> {
        self.hasher.as_mut().map(|h| {
            h.update(data);
            let hash = h.finalize();
            h.reset();
            hash
        })
    }
}

#[cfg(not(feature = "hash-blake3"))]
impl HasherState {
    /// Creates a new hasher state (no-op when hashing is disabled).
    fn new(_config: &ChunkConfig) -> Self {
        Self
    }

    /// Hashes a chunk (always returns None when hashing is disabled).
    fn hash_chunk(&mut self, _data: &Bytes) -> Option<crate::chunk::ChunkHash> {
        None
    }
}

/// Chunk stream with hasher state.
///
/// This type combines the chunk stream with optional hashing support.
/// It implements the `Stream` trait, yielding chunks asynchronously.
pub struct ChunkStreamWithHasher<R> {
    inner: ChunkStream<R>,
    hasher: HasherState,
}

impl<R> ChunkStreamWithHasher<R> {
    /// Creates a new chunk stream from an async reader.
    ///
    /// # Arguments
    ///
    /// * `reader` - An async reader implementing `AsyncRead`
    /// * `config` - The chunking configuration
    pub fn new(reader: R, config: ChunkConfig) -> Self {
        let inner = ChunkStream {
            reader,
            config,
            cdc: FastCdc::new(config.min_size(), config.avg_size(), config.max_size()),
            buffer: vec![0u8; 8192],
            chunk_buffer: Vec::with_capacity(config.max_size()),
            offset: 0,
            finished: false,
        };
        let hasher = HasherState::new(&config);
        Self { inner, hasher }
    }
}

impl<R: AsyncRead + Unpin> Stream for ChunkStreamWithHasher<R> {
    type Item = Result<Chunk, ChunkError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = &mut *self;

        if this.inner.finished {
            return Poll::Ready(None);
        }

        loop {
            // Try to find a boundary in existing data
            if !this.inner.chunk_buffer.is_empty() {
                // Scan for boundary
                let mut found = None;
                for (i, &byte) in this.inner.chunk_buffer.iter().enumerate() {
                    if this.inner.cdc.update(byte) {
                        found = Some(i + 1);
                        break;
                    }
                }

                if let Some(len) = found {
                    let chunk = this.emit_chunk(len);
                    return Poll::Ready(Some(Ok(chunk)));
                }

                // No boundary found - check if we've exceeded max size
                if this.inner.chunk_buffer.len() >= this.inner.config.max_size() {
                    // Force a boundary at max_size
                    let len = this.inner.config.max_size();
                    let chunk = this.emit_chunk(len);
                    return Poll::Ready(Some(Ok(chunk)));
                }
            }

            // Read more data using AsyncRead into buffer
            let buf = &mut this.inner.buffer[..];
            match Pin::new(&mut this.inner.reader).poll_read(cx, buf) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(e)) => {
                    this.inner.finished = true;
                    return Poll::Ready(Some(Err(ChunkError::Io(e))));
                }
                Poll::Ready(Ok(n)) => {
                    if n == 0 {
                        // End of stream - emit remaining data if any
                        if !this.inner.chunk_buffer.is_empty() {
                            let len = this.inner.chunk_buffer.len();
                            let chunk = this.emit_chunk(len);
                            this.inner.finished = true;
                            return Poll::Ready(Some(Ok(chunk)));
                        }
                        this.inner.finished = true;
                        return Poll::Ready(None);
                    }
                    this.inner
                        .chunk_buffer
                        .extend_from_slice(&this.inner.buffer[..n]);
                }
            }
        }
    }
}

impl<R> ChunkStreamWithHasher<R> {
    /// Processes the chunk buffer and returns a chunk.
    ///
    /// This internal method extracts a chunk from the buffer, computes its
    /// hash if enabled, and updates the offset.
    fn emit_chunk(&mut self, len: usize) -> Chunk {
        let data = Bytes::copy_from_slice(&self.inner.chunk_buffer[..len]);
        let chunk_offset = self.inner.offset;

        let hash = self.hasher.hash_chunk(&data);

        // Keep any remaining data
        if len < self.inner.chunk_buffer.len() {
            self.inner.chunk_buffer.copy_within(len.., 0);
            self.inner
                .chunk_buffer
                .truncate(self.inner.chunk_buffer.len() - len);
        } else {
            self.inner.chunk_buffer.clear();
        }

        self.inner.offset += len as u64;

        Chunk {
            data,
            offset: Some(chunk_offset),
            hash,
        }
    }
}

/// Creates a chunk stream from an async reader.
///
/// Uses `futures_io::AsyncRead` for runtime-agnostic async I/O.
/// This works with any async runtime (tokio, async-std, smol, etc.).
///
/// # Runtime Compatibility
///
/// For tokio users, you can use `tokio_util::compat` to convert
/// `tokio::io::AsyncRead` to `futures_io::AsyncRead`:
///
/// ```ignore
/// use tokio_util::compat::TokioAsyncReadCompatExt;
/// use chunkrs::{chunk_async, ChunkConfig};
///
/// let tokio_reader = tokio::fs::File::open("file").await?;
/// let stream = chunk_async(tokio_reader.compat(), ChunkConfig::default());
/// ```
///
/// # Example
///
/// ```ignore
/// use chunkrs::{chunk_async, ChunkConfig};
/// use futures_util::StreamExt;
/// use futures_io::AsyncRead;
///
/// async fn demo<R: AsyncRead + Unpin>(reader: R) -> Result<(), chunkrs::ChunkError> {
///     let mut stream = chunk_async(reader, ChunkConfig::default());
///
///     while let Some(chunk) = stream.next().await {
///         let chunk = chunk?;
///         println!("chunk {}", chunk.data.len());
///     }
///     Ok(())
/// }
/// ```
///
/// # Arguments
///
/// * `reader` - An async reader implementing `AsyncRead`
/// * `config` - The chunking configuration
///
/// # Returns
///
/// A `ChunkStreamWithHasher` that implements `Stream<Item = Result<Chunk, ChunkError>>`
pub fn chunk_async<R: AsyncRead>(reader: R, config: ChunkConfig) -> ChunkStreamWithHasher<R> {
    ChunkStreamWithHasher::new(reader, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chunk_stream_empty() {
        let reader: &[u8] = &[];
        let stream = ChunkStreamWithHasher::new(reader, ChunkConfig::default());
        let chunks: Vec<_> = futures_util::StreamExt::collect(stream).await;
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn test_chunk_stream_small_data() {
        let data: Vec<u8> = vec![0xAAu8; 100];
        let reader: &[u8] = &data;
        let stream = ChunkStreamWithHasher::new(reader, ChunkConfig::new(4, 16, 64).unwrap());

        let chunks: Vec<_> = futures_util::StreamExt::collect(stream).await;
        let chunks: Vec<_> = chunks.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        let total_len: usize = chunks.iter().map(|c: &Chunk| c.len()).sum();
        assert_eq!(total_len, data.len());
    }

    #[tokio::test]
    #[cfg(feature = "hash-blake3")]
    async fn test_chunk_stream_with_hashes() {
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let config = ChunkConfig::default().with_hash_config(crate::config::HashConfig::enabled());

        let reader: &[u8] = &data;
        let stream = ChunkStreamWithHasher::new(reader, config);

        let chunks: Vec<_> = futures_util::StreamExt::collect(stream).await;
        let chunks: Vec<_> = chunks.into_iter().collect::<Result<Vec<_>, _>>().unwrap();

        for chunk in &chunks {
            assert!(chunk.hash.is_some());
        }
    }
}
