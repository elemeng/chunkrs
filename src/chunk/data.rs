//! The Chunk type - represents a content-defined chunk.

use bytes::Bytes;
use std::fmt;

use super::ChunkHash;

/// A content-defined chunk with metadata.
///
/// # Example
///
/// ```
/// use chunkrs::{Chunk, ChunkHash};
/// use bytes::Bytes;
///
/// let chunk = Chunk {
///     data: Bytes::from_static(b"hello world"),
///     offset: Some(0),
///     hash: None,
/// };
///
/// assert_eq!(chunk.data.len(), 11);
/// ```
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The chunk data (may be owned or borrowed).
    pub data: Bytes,

    /// The offset in the original stream (if available).
    pub offset: Option<u64>,

    /// The content hash of this chunk (if computed).
    pub hash: Option<ChunkHash>,
}

impl Chunk {
    /// Creates a new chunk with the given data.
    pub fn new(data: impl Into<Bytes>) -> Self {
        Self {
            data: data.into(),
            offset: None,
            hash: None,
        }
    }

    /// Creates a new chunk with an offset.
    pub fn with_offset(data: impl Into<Bytes>, offset: u64) -> Self {
        Self {
            data: data.into(),
            offset: Some(offset),
            hash: None,
        }
    }

    /// Creates a new chunk with a hash.
    pub fn with_hash(data: impl Into<Bytes>, hash: ChunkHash) -> Self {
        Self {
            data: data.into(),
            offset: None,
            hash: Some(hash),
        }
    }

    /// Sets the offset.
    pub fn set_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Sets the hash.
    pub fn set_hash(mut self, hash: ChunkHash) -> Self {
        self.hash = Some(hash);
        self
    }

    /// Returns the length of the chunk data.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the chunk has no data.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns a reference to the chunk data.
    pub fn data(&self) -> &Bytes {
        &self.data
    }

    /// Returns the offset, if set.
    pub fn offset(&self) -> Option<u64> {
        self.offset
    }

    /// Returns the hash, if computed.
    pub fn hash(&self) -> Option<ChunkHash> {
        self.hash
    }

    /// Returns the start offset (0 if not set).
    pub fn start(&self) -> u64 {
        self.offset.unwrap_or(0)
    }

    /// Returns the end offset (exclusive).
    pub fn end(&self) -> u64 {
        self.start() + self.data.len() as u64
    }

    /// Returns the chunk as a range.
    pub fn range(&self) -> std::ops::Range<u64> {
        self.start()..self.end()
    }

    /// Consumes the chunk and returns the underlying data.
    pub fn into_data(self) -> Bytes {
        self.data
    }

    /// Splits the chunk into (data, hash).
    pub fn into_parts(self) -> (Bytes, Option<ChunkHash>) {
        (self.data, self.hash)
    }
}

impl From<Bytes> for Chunk {
    fn from(data: Bytes) -> Self {
        Self::new(data)
    }
}

impl From<Vec<u8>> for Chunk {
    fn from(data: Vec<u8>) -> Self {
        Self::new(data)
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Chunk({} bytes", self.len())?;
        if let Some(offset) = self.offset {
            write!(f, " @ {}", offset)?;
        }
        if let Some(hash) = self.hash {
            write!(f, ", hash={}", hash)?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let chunk = Chunk::new(&b"hello"[..]);
        assert_eq!(chunk.len(), 5);
        assert!(!chunk.is_empty());
    }

    #[test]
    fn test_empty() {
        let chunk = Chunk::new(&b""[..]);
        assert!(chunk.is_empty());
    }

    #[test]
    fn test_with_offset() {
        let chunk = Chunk::with_offset(&b"hello"[..], 100);
        assert_eq!(chunk.offset(), Some(100));
    }

    #[test]
    fn test_with_hash() {
        let hash = ChunkHash::new([0u8; 32]);
        let chunk = Chunk::with_hash(&b"hello"[..], hash);
        assert_eq!(chunk.hash(), Some(hash));
    }

    #[test]
    fn test_builder_pattern() {
        let hash = ChunkHash::new([0u8; 32]);
        let chunk = Chunk::new(&b"hello"[..]).set_offset(100).set_hash(hash);

        assert_eq!(chunk.len(), 5);
        assert_eq!(chunk.offset(), Some(100));
        assert_eq!(chunk.hash(), Some(hash));
    }

    #[test]
    fn test_from_bytes() {
        let bytes = Bytes::from_static(b"test");
        let chunk: Chunk = bytes.into();
        assert_eq!(chunk.len(), 4);
    }

    #[test]
    fn test_display() {
        let chunk = Chunk::with_offset(&b"hello"[..], 100);
        let s = format!("{}", chunk);
        assert!(s.contains("5 bytes"));
        assert!(s.contains("@ 100"));
    }

    #[test]
    fn test_start_with_offset() {
        let chunk = Chunk::with_offset(&b"hello"[..], 100);
        assert_eq!(chunk.start(), 100);
    }

    #[test]
    fn test_start_without_offset() {
        let chunk = Chunk::new(&b"hello"[..]);
        assert_eq!(chunk.start(), 0);
    }

    #[test]
    fn test_end() {
        let chunk = Chunk::with_offset(&b"hello"[..], 100);
        assert_eq!(chunk.end(), 105);
    }

    #[test]
    fn test_end_without_offset() {
        let chunk = Chunk::new(&b"hello"[..]);
        assert_eq!(chunk.end(), 5);
    }

    #[test]
    fn test_range() {
        let chunk = Chunk::with_offset(&b"hello"[..], 100);
        assert_eq!(chunk.range(), 100..105);
    }

    #[test]
    fn test_range_without_offset() {
        let chunk = Chunk::new(&b"hello"[..]);
        assert_eq!(chunk.range(), 0..5);
    }
}
