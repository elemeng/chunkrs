# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.8.2] - 2026-02-06

### Changed

- Updated package description for clarity and conciseness
- Simplified README introduction with new tagline
- Reorganized README section order for better flow

### Improved

- More concise and impactful README messaging
- Better alignment of "When to Use" section placement

## [0.8.1] - 2026-02-06

### Changed

- Refined package description to emphasize high-performance, portable infrastructure
- Clarified streaming model: "Bytes in → Chunks & hashes out"

## [0.8.0] - 2026-02-06

### Added

- Comprehensive documentation across all modules (1,119 lines of documentation)
- Chunk helper methods: `start()`, `end()`, `range()` for offset calculations
- Detailed algorithm explanations for FastCDC implementation
- Module-level documentation explaining purpose and usage patterns
- Inline comments for complex logic and implementation details
- Roadmap section documenting current status and future plans
- Version 0.8.0 release indicating cautious development phase

### Changed

- Bumped version from 0.2.0 to 0.8.0 to reflect API maturity
- Converted documentation examples from `no_run`/`ignore` to executable doctests
- Enhanced async example to use `futures_io::AsyncRead` for runtime-agnostic documentation
- Updated all version references in README from 0.1 to 0.8
- Standardized documentation style across all modules with consistent structure

### Improved

- All public APIs now have comprehensive documentation with examples
- 40 doctests passing (was 4 passed, 1 ignored)
- Better error messages and documentation
- Clear separation between public and internal APIs in documentation
- Added "Non-Goals" section to explicitly document out-of-scope features
- Detailed version policy explaining 0.8.x, 0.9.x, and 1.0.0 expectations

### Documentation

- Added crate-level documentation in lib.rs with design philosophy
- Enhanced README with Architecture diagram and feature overview
- Documented all struct fields and method parameters
- Added examples for synchronous and asynchronous usage patterns
- Provided configuration examples for different use cases
- Documented performance characteristics and determinism guarantees

### Quality

- Maintained 45 unit tests + 40 doctests
- No unsafe code: `#![forbid(unsafe_code)]`
- No clippy warnings with `-D warnings` flag
- Follows Rust 2024 edition best practices
- Production-ready quality with comprehensive test coverage

## [0.2.0] - 2026-02-06

### Added

- Chunk helper methods: `start()`, `end()`, `range()` for offset calculations
- Unit tests for new range methods
- Doctests for public API examples

### Changed

- Improved documentation clarity and consistency

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
