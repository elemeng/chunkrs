# chunkrs — Architecture

> **A streaming, deterministic-by-design, allocation-conscious Content-Defined Chunking (CDC) engine for delta sync and deduplication systems.**
>
> `chunkrs` is a *library crate*, not a workload scheduler. It focuses on **correct, fast, single-stream chunking** and leaves concurrency orchestration, storage policy, and sync semantics to the application layer.

## Version: 0.8.3

---

## Overview

```Byte Stream → CDC Boundary Detection → Chunk Assembly → Chunk Hashing → Output Stream```

---

## 1. Design Goals

* **Streaming-first** - Process byte streams without full-file buffering
* **High throughput** - Saturate modern I/O without intra-file parallelism
* **Deterministic** - Identical inputs produce identical boundaries and hashes
* **Zero-copy** - Efficient `Bytes` slicing with minimal allocations
* **Allocator-disciplined** - Avoid contention under high throughput
* **Std-quality API** - Small, predictable, no hidden state
* **Memory-safe** - `#![forbid(unsafe_code)]` throughout

---

## 2. Non-Goals

`chunkrs` does not handle:

* Inter-file parallelism or thread pool management
* I/O scheduling, device throttling, or storage coordination
* Chunk persistence, deduplication indexing, or storage backends
* Network protocols, sync negotiation, or application-level logic
* HDD vs SSD vs NVMe detection or device-specific optimizations

These are **application responsibilities** - `chunkrs` provides pure CDC.

---

## 3. CDC Algorithm Choice

### Boundary Detection

`chunkrs` implements a **FastCDC-style rolling hash** for boundary detection:

* Byte-by-byte rolling hash
* Mask-based boundary check
* Configurable minimum / average / maximum chunk sizes

Rolling hash is used **only** to decide *where* chunks end — **never** as a content identifier.

### Chunk Identity

Each emitted chunk is finalized with a **strong cryptographic hash** (default: BLAKE3):

* Chunk hash defines identity
* Used for deduplication, delta sync, verification, ect.
* Rolling hash state does *not* affect identity

---

## 4. Determinism Model

### What Is Guaranteed

* Identical byte streams + identical configuration → identical **chunk boundaries**
* Identical byte streams + identical configuration → identical **chunk hashes**
* CDC behavior is byte-by-byte serial, ensuring deterministic boundaries regardless of:
  * Input batch sizes (1 byte vs 1MB vs streaming)
  * Number of `push()` calls
  * Call timing

### Implementation

The FastCDC algorithm processes each byte sequentially, maintaining rolling hash state across all calls. This ensures:

* Exact boundary determinism - same byte positions always produce same boundaries
* No dependency on execution strategy or batching patterns
* Perfect reproducibility across different streaming scenarios

---

## 5. API & Memory Model

### Streaming Interface

```rust
let mut chunker = Chunker::new(config);
let (chunks, pending) = chunker.push(data_bytes);
let final_chunk = chunker.finish();
```

* **`push(Bytes)`** - Feed data in any size (1 byte to megabytes)
* **`finish()`** - Emit final incomplete chunk when stream ends
* **Returns** - `(Vec<Chunk>, Bytes)` - Complete chunks and pending bytes

### Zero-Copy Design

* Chunk data is sliced directly from input `Bytes` - no copying
* Caller owns the underlying memory
* Pending bytes held internally only between `push()` calls

### Memory Responsibility

* Caller must process/drop chunks promptly (accumulating may OOM)
* Caller controls backpressure and memory management
* No global buffer pools or cross-thread state

---

## 6. Execution Model

### Single-Stream Serial CDC

CDC is inherently serial over a byte stream:

* Rolling hash state at byte `n` depends on bytes `[0..n)`
* Input may be split into batches via multiple `push()` calls, but state persists
* Implementation processes bytes one-by-one for exact determinism

Therefore:

* `chunkrs` does **not** parallelize CDC within a file
* Modern CPUs are sufficient to saturate I/O bandwidth without intra-file parallelism

### Application-Level Parallelism

Applications achieve parallelism by:

* Running multiple `Chunker` instances on different streams
* Using async executors (tokio) with blocking tasks
* Managing their own thread pools

The library provides pure CDC - concurrency and I/O orchestration are application responsibilities.

---

## 7. I/O Model

`chunkrs` accepts `Bytes` from any source and emits `Chunk` objects:

* **Input**: Files, network, buffers - any source providing `Bytes`
* **Output**: Chunk with hash, length, offset, and zero-copy payload
* **Errors**: Localized to stream, no global state corruption
* **Recovery**: Checkpointing/resume is application's responsibility

The crate does not persist, index, or manage chunks.

---

## 8. Comparison to fastcdc

| Aspect          | fastcdc      | chunkrs     |
| --------------- | ------------ | ----------- |
| Streaming API   | Limited      | First-class |
| Zero-copy       | No           | Yes         |
| Rust edition    | 2018         | 2024+       |
| API quality     | Experimental | Std-style   |

`chunkrs` focuses on API quality and streaming correctness, not just speed.

---

## 9. Summary

`chunkrs` is a **deterministic, streaming, zero-copy** CDC engine with a simple push/finish API. Byte-by-byte processing ensures exact determinism, while `Bytes` slicing provides zero-copy efficiency. The library handles pure CDC - orchestration, I/O, and storage are application responsibilities.

---

## Appendix: Module Structure

### Flat API Design

`chunkrs` uses a **flat API design** for "small, composable primitive" positioning. All public types are accessible directly from the crate root:

```
chunkrs::Chunk
chunkrs::ChunkHash
chunkrs::Chunker
chunkrs::ChunkConfig
chunkrs::HashConfig
chunkrs::ChunkError
```

### Module Organization

```
chunkrs/
├── lib.rs              # Public API: pub use re-exports only
├── chunk/              # Private: Chunk, ChunkHash
├── chunker/            # Private: Chunker with push/finish API
├── config/             # Private: ChunkConfig, HashConfig
├── error/              # Private: ChunkError
├── cdc/                # Private: FastCDC rolling hash
├── hash/               # Private: BLAKE3 (feature-gated)
└── util/               # Private: Internal helpers
```

### Visibility Strategy

- **Public modules**: None - modules are private for code organization only
- **Public API**: Only `pub use` re-exports in `lib.rs`
- **Internal sharing**: Private modules use `pub` for crate-local sharing
- **No `pub(crate)`**: Eliminated for cleaner boundaries

This design ensures:
- No duplicate access paths (e.g., `chunkrs::Chunk` vs `chunkrs::chunk::Chunk`)
- Minimal public API surface
- Clear separation between public API and implementation details
