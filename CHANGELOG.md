# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-02-06

### Added
- Initial release
- FastCDC rolling hash for content-defined chunking
- BLAKE3 hashing for chunk identity
- Sync API with `Chunker` and `ChunkIter`
- Async API with `chunk_async` (runtime-agnostic via `futures-io`)
- Configurable min/avg/max chunk sizes
- Thread-local buffer pools for efficient memory reuse
- Zero-copy friendly design using `bytes`
- `#![forbid(unsafe_code)]`
