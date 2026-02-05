# chunkrs — Architecture

> **A streaming, deterministic-by-design, allocation-conscious Content-Defined Chunking (CDC) engine for delta sync and deduplication systems.**
>
> `chunkrs` is a *library crate*, not a workload scheduler. It focuses on **correct, fast, single-stream chunking** and leaves concurrency orchestration, storage policy, and sync semantics to the application layer.

---

## 1. Design Goals

### Primary Goals

1. **Streaming-first**
   Operate on byte streams without requiring full-file buffering.

2. **High Throughput on Modern Hardware**
   Sustain line-rate performance on NVMe, PCIe 5.0, 100 Gbps networks, and DDR5 memory *without intra-file parallelism*.

3. **Deterministic-by-Content**
   Given identical byte streams and configuration, the produced chunk *hashes* are deterministic.

   > Exact chunk *boundaries* may vary slightly across execution strategies, but chunk identity is always defined by content hash.

4. **Zero-Copy Friendly**
   Integrate cleanly with `bytes`, `Read`, `AsyncRead`, and application-owned buffers.

5. **Allocator Discipline**
   Avoid allocator contention and allocation storms under high throughput or many small files.

6. **Std-quality API Surface**
   Small, orthogonal traits. Predictable behavior. No hidden global state.

---

## 2. Non-Goals

* Managing inter-file parallelism
* Global I/O scheduling or device throttling
* Chunk persistence or deduplication indexing
* Network sync protocols or rsync-style negotiation

These are explicitly **application responsibilities**.

---

## 3. High-Level Model

`chunkrs` processes **one logical byte stream at a time**:

```text
Byte Stream → CDC Boundary Detection → Chunk Assembly → Chunk Hashing → Output Stream
```

The engine is fully streaming:

* Input may arrive in arbitrarily sized batches
* CDC state is preserved across batch boundaries
* Output chunks are emitted incrementally

---

## 4. CDC Algorithm Choice

### Boundary Detection

`chunkrs` implements a **FastCDC-style rolling hash** for boundary detection:

* Byte-by-byte rolling hash
* Mask-based boundary check
* Configurable minimum / average / maximum chunk sizes

Rolling hash is used **only** to decide *where* chunks end — **never** as a content identifier.

### Chunk Identity

Each emitted chunk is finalized with a **strong cryptographic hash** (default: BLAKE3):

* Chunk hash defines identity
* Used for deduplication, delta sync, and verification
* Rolling hash state does *not* affect identity

---

## 5. Determinism Model

### What Is Guaranteed

* Identical byte streams + identical configuration → identical **chunk hashes**
* CDC behavior is stable within a single execution model
* CDC state is strictly serial across the logical byte stream

### What Is *Not* Guaranteed

* Bit-for-bit identical chunk *boundaries* across different execution strategies or batching patterns

This tradeoff is intentional and aligned with real-world delta sync systems, where **content hash equality** — not boundary offset — defines equivalence.

---

## 6. Streaming & Backpressure

The core API exposes chunking as a **bounded stream**:

* Internally uses a bounded buffer (2–4 chunks)
* Producers naturally block when consumers are slow
* Prevents unbounded memory growth

This design ensures:

* Safe integration with slow sinks (disk, network, object storage)
* No Rayon or async executor starvation

---

## 7. Memory Architecture

### Thread-Local Buffer Pools

To eliminate allocator contention:

* Each worker thread maintains a **thread-local buffer cache**
* Buffers are reused across chunk operations
* No global locks
* No cross-thread buffer migration

Properties:

* Zero allocation on hot paths
* NUMA-friendly
* Predictable memory footprint

> Thread-local caches are considered *local state*, not global state.

---

## 8. Execution Model

### Single-Stream Serial CDC

CDC is inherently serial over a byte stream:

* Rolling hash state at byte `n` depends on bytes `[0..n)`
* Input may be split into batches, but state persists

Therefore:

* `chunkrs` does **not** parallelize CDC within a file
* Modern CPUs are sufficient to saturate I/O bandwidth without intra-file parallelism

Parallelism is achieved by **overlapping I/O and computation**, not by fragmenting the stream.

---

## 9. Input Abstractions

`chunkrs` supports multiple input styles:

* `Read` (blocking)
* `AsyncRead` (async)
* Application-owned buffers
* Custom stream sources via traits

All inputs are normalized into a streaming byte source with preserved ordering.

---

## 10. Output Model

Each emitted chunk contains:

* Content hash
* Chunk length
* Optional offset (if provided by source)
* Borrowed or owned byte payload (zero-copy when possible)

The crate does **not**:

* Persist chunks
* Index chunks
* Decide reuse vs upload

---

## 11. Storage & I/O Policy

`chunkrs` intentionally avoids device-level assumptions:

* HDD vs SSD vs NVMe is **not inferred**
* Seek-throttling or device coordination is **not enforced**

Applications may implement:

* HDD coordinators
* Rate limiters
* Priority schedulers

and feed serialized streams into `chunkrs`.

---

## 12. Failure & Recovery Semantics

* Chunking errors are localized to the stream
* No global state is corrupted
* Partial progress is observable via emitted chunks

Checkpointing and resume logic belong to the application layer.

---

## 13. Comparison to Existing Crates

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

## 14. Crate Responsibility Boundary

**`chunkrs` guarantees:**

* Correct CDC
* Stable chunk hashes
* Efficient streaming execution

**Applications decide:**

* File traversal
* Concurrency level
* Sync protocol
* Storage backend
* Deduplication index

This boundary is intentional and enforced by design.

---

## 15. mod structure

```text
chunkrs/
├── lib.rs
│
├── chunk/
│   ├── mod.rs
│   ├── chunk.rs        # Chunk { data: Bytes, offset, hash }
│   └── hash.rs         # ChunkHash newtype
│
├── chunker/
│   ├── mod.rs
│   └── iter.rs         # Chunker + ChunkIter core engine
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
│   └── blake3.rs       # strong hash implementation
│
├── buffer/
│   ├── mod.rs
│   └── pool.rs         # thread-local buffer reuse
│
├── async_stream/
│   ├── mod.rs
│   └── stream.rs       # AsyncRead → Stream adapter
│
└── util/
    └── mod.rs          # small helpers (private)
```

## 16. Summary

`chunkrs` is a **boring, fast, correct** CDC engine.

It avoids clever parallel tricks in favor of:

* determinism-by-content
* streaming correctness
* allocator discipline
* composability

If you need orchestration — build it *around* `chunkrs`.
If you need chunking — `chunkrs` stays out of your way.
