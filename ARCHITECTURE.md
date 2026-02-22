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

### Primary Goals

1. **Streaming-first**
   Operate on byte streams without requiring full-file buffering.

2. **High Throughput on Modern Hardware**
   Sustain line-rate performance on NVMe, PCIe 5.0, 100 Gbps networks, and DDR5 memory *without intra-file parallelism*.

3. **Deterministic-by-Content**
   Given identical byte streams and configuration, the produced chunk *boundaries* and *hashes* are deterministic.

4. **Zero-Copy Friendly**
   Integrate cleanly with `bytes` for zero-copy chunk data slicing.

5. **Allocator Discipline**
   Avoid allocator contention and allocation storms under high throughput or many small files.

6. **Std-quality API Surface**
   Small, orthogonal traits. Predictable behavior. No hidden global state.

7. **Memory Safety**
   Zero unsafe code. `#![forbid(unsafe_code)]` enforced throughout.

---

## 2. Non-Goals

* Managing inter-file parallelism
* Global I/O scheduling or device throttling
* Chunk persistence or deduplication indexing
* Network sync protocols or rsync-style negotiation

These are explicitly **application responsibilities**.

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

## 5. Streaming API

The core API exposes chunking as a **push-based streaming interface**:

* `Chunker::push(Bytes)` - Feed data in arbitrary sizes (1 byte to megabytes)
* `Chunker::finish()` - Emit final incomplete chunk when stream ends
* Returns `(Vec<Chunk>, Bytes)` - Complete chunks and pending unprocessed bytes

### Memory Considerations

* The `push()` method returns a `Vec<Chunk>` - caller must process or drop chunks promptly
* Accumulating returned chunks may OOM on large streams - caller's responsibility
* Pending unprocessed bytes are held internally for CDC state continuity

This design ensures:

* Zero-copy chunk data via `Bytes` slicing from input
* Caller controls memory and backpressure
* Flexible integration with any data source

---

## 6. Memory Architecture

### Zero-Copy Design

The implementation uses `Bytes` from the `bytes` crate for zero-copy chunk data:

* Chunk data is sliced directly from input `Bytes` references
* No data copying when creating chunks
* Caller owns the underlying memory

### Allocator Discipline

To minimize allocations:

* No global buffer pools
* No cross-thread buffer migration
* Pending bytes are held internally only between `push()` calls

The library provides tools for efficient chunking, but memory management (thread pools, buffer reuse, etc.) is the application's responsibility.

---

## 7. Execution Model

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

## 8. Input & Output Model

`chunkrs` accepts `Bytes` as input and emits `Chunk` objects:

**Input:**

* Zero-copy slicing for efficient chunk data
* Application owns the underlying memory
* Supports any data source that can provide `Bytes` (files, network, buffers, etc.)

**Output:**

* Each chunk contains: content hash, length, optional offset, and zero-copy byte payload
* The crate does not persist, index, or manage chunks - these are application responsibilities

---

## 9. Storage & I/O Policy

`chunkrs` intentionally avoids device-level assumptions:

* HDD vs SSD vs NVMe is **not inferred**
* Seek-throttling or device coordination is **not enforced**

Applications may implement:

* HDD coordinators
* Rate limiters
* Priority schedulers

and feed serialized streams into `chunkrs`.

---

## 10. Failure & Recovery Semantics

* Chunking errors are localized to the stream
* No global state is corrupted
* Partial progress is observable via emitted chunks

Checkpointing and resume logic belong to the application layer.

---

## 11. Comparison to Existing Crates

### fastcdc

| Aspect            | fastcdc      | chunkrs     |
| ----------------- | ------------ | ----------- |
| Streaming API     | Limited      | First-class |
| Async support     | No           | Yes         |
| Zero-copy         | No           | Yes         |
| Allocator control | Minimal      | Explicit    |
| Rust edition      | 2018         | 2024+       |
| API stability     | Experimental | Std-style   |

`chunkrs` is not a reimplementation for speed alone — it is a **modernization** focused on API quality, streaming correctness, and integration into real systems.

---

## 12. Module Structure

```text
chunkrs/
├── lib.rs
│
├── chunk/
│   ├── mod.rs
│   ├── data.rs         # Chunk { data: Bytes, offset, hash }
│   └── hash.rs         # ChunkHash newtype
│
├── chunker/
│   ├── mod.rs
│   └── engine.rs       # Chunker with push/finish streaming API
│
├── config/
│   └── mod.rs          # ChunkConfig + HashConfig
│
├── error/
│   └── mod.rs          # ChunkError (small, std-style)
│
├── cdc/
│   ├── mod.rs
│   └── fastcdc.rs      # rolling hash boundary detector
│
├── hash/
│   ├── mod.rs
│   └── blake3.rs       # strong hash implementation (feature-gated)
│
└── util/
    └── mod.rs          # internal utility functions
```

**Note:** `fuzz/` contains fuzz testing targets (development only) and is not part of the library structure.

## 13. Summary

`chunkrs` is a **deterministic, streaming, zero-copy** CDC engine.

It provides:

* Exact determinism - identical byte streams produce identical chunk boundaries
* Push-based streaming API - process data of arbitrary size
* Zero-copy chunk data via `Bytes` slicing
* Simple, composable design - no hidden complexity

If you need orchestration (concurrency, I/O, storage) — build it *around* `chunkrs`.
If you need chunking — `chunkrs` stays out of your way.
