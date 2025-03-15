# SlabBench

> **DISCLAIMER**: This project was a quick experiment authored entirely by AI. The benchmark results may not be reliable and warrant further inspection/auditing if used for critical decisions.

A specialized benchmarking comparison between the Rust crates `slab` and `stable-vec`, focusing on realistic mixed workloads.

## Overview

This project compares the performance characteristics of two stable-index collections: `slab` and `stable-vec`. These data structures provide similar functionality but use fundamentally different implementation approaches, which leads to different performance tradeoffs.

## Data Structures and Algorithms

Both crates provide stable-index collections with O(1) insertion and removal, but use fundamentally different implementation strategies:

### Slab

- **Core Data Structure**: A contiguous `Vec<Entry<T>>` where `Entry` is an enum: either `Occupied(T)` or `Vacant(usize)`
- **Vacant Slot Management**: Maintains a linked list of vacant slots. Each `Vacant(usize)` entry points to the next vacant slot
- **Free Slot Allocation**: O(1) - Simply dequeue the first vacant slot from the linked list
- **Element Removal**: O(1) - Mark the slot as vacant and add it to the front of the vacant list
- **Memory Overhead**: Each vacant slot requires an entire `usize` (8 bytes on 64-bit) to store the next index

### StableVec

- **Core Data Structure**: Offers pluggable implementations, with the default using a bit vector to track occupied slots
- **Vacant Slot Management**: Uses a bit vector where each bit indicates whether a slot is occupied
- **Free Slot Allocation**: Requires scanning the bit vector for a zero bit, which is O(n) in worst case but potentially O(1) with intrinsics
- **Element Removal**: O(1) - Just flip the corresponding bit
- **Memory Overhead**: Only 1 bit per slot, which is significantly less than Slab's approach

## Expected Asymptotic Performance

| Operation            | Slab             | StableVec (Default) | Notes                                      |
|----------------------|------------------|---------------------|--------------------------------------------|
| Insert new element   | O(1) amortized   | O(1) amortized      | Both need to grow the underlying storage   |
| Insert in vacant slot| O(1)             | O(1)~O(n)           | Slab has guaranteed O(1), StableVec might need to search for a vacant slot |
| Get element          | O(1)             | O(1)                | Both have direct array access              |
| Remove element       | O(1)             | O(1)                | Both implement efficient removal           |
| Iteration            | O(n)             | O(n)                | Both need to skip empty slots              |
| Memory usage for empty slots | High (8 bytes) | Low (1 bit)  | StableVec wins for sparse data structures  |

## Mixed Workload Benchmarks

The benchmarks focus on real-world usage patterns:

1. **Standard Mixed Workload**: Insert, get, remove, insert again
2. **High Churn Workload**: Repeated cycles of insertion and removal 
3. **Sparse Access Workload**: Operations on a structure with many gaps
4. **Compaction Workload**: Performance after compaction operations

### High Churn Benchmark Design

We developed a sophisticated benchmark to simulate real-world high-churn scenarios with three distinct patterns:

1. **Uniform**: Removes every third element
2. **Clustered**: Removes 25% of elements from contiguous sections
3. **Random**: Removes elements using a deterministic but pseudo-random pattern

Each benchmark:
- Performs 20 cycles of removal, reinsertion, and fresh insertions
- Tracks and reuses indices for realistic reference management
- Alternates between inserting at specific indices and appending
- Includes random access patterns to simulate real usage

## Running Benchmarks

```bash
cargo bench
```

Benchmark results are stored in `target/criterion/` with interactive HTML reports.

## Benchmark Results

Below are the benchmark results from comparing `slab` and `stable-vec` across different workloads and collection sizes.

### Standard Mixed Workload

The standard mixed workload simulates realistic usage with insertions, lookups, removals, and iterations.

| Collection Size | Slab           | StableVec      | StableVec vs Slab |
|-----------------|----------------|----------------|-------------------|
| 1,000           | 3.92 µs        | 3.97 µs        | 1.3% slower       |
| 10,000          | 39.97 µs       | 39.08 µs       | 2.2% faster       |
| 100,000         | 717.34 µs      | 571.18 µs      | 20.4% faster      |

For small collections, both implementations show similar performance. However, as collection size grows, `StableVec` demonstrates a clear advantage, being about 20% faster for collections of 100,000 elements.

### High Churn Workload

The high churn workload tests repeated cycles of removal and insertion with different patterns.

#### Uniform Pattern (every third element removed)

| Collection Size | Slab           | StableVec      | StableVec vs Slab |
|-----------------|----------------|----------------|-------------------|
| 1,000           | 42.21 µs       | 47.47 µs       | 12.5% slower      |
| 5,000           | 239.13 µs      | 246.60 µs      | 3.1% slower       |
| 10,000          | 523.26 µs      | 502.04 µs      | 4.1% faster       |
| 50,000          | 3,387.1 µs     | 3,199.4 µs     | 5.5% faster       |

#### Clustered Pattern (25% of elements from contiguous sections)

| Collection Size | Slab           | StableVec      | StableVec vs Slab |
|-----------------|----------------|----------------|-------------------|
| 1,000           | 32.00 µs       | 38.69 µs       | 20.9% slower      |
| 5,000           | 170.78 µs      | 194.94 µs      | 14.1% slower      |
| 10,000          | 342.45 µs      | 390.78 µs      | 14.1% slower      |
| 50,000          | 2,248.9 µs     | 2,605.3 µs     | 15.8% slower      |

#### Random Pattern (pseudo-random removal)

| Collection Size | Slab           | StableVec      | StableVec vs Slab |
|-----------------|----------------|----------------|-------------------|
| 1,000           | 37.03 µs       | 43.76 µs       | 18.2% slower      |
| 5,000           | 187.53 µs      | 221.06 µs      | 17.9% slower      |
| 10,000          | 390.89 µs      | 452.54 µs      | 15.8% slower      |
| 50,000          | 1,987.6 µs     | 2,880.4 µs     | 44.9% slower      |

In high churn scenarios, `Slab` generally outperforms `StableVec` for smaller collections and clustered/random access patterns. The performance difference appears most significant with random patterns at larger collection sizes.

### Sparse Access Workload 

The sparse access workload tests operations on structures with many gaps (90% of elements removed).

| Collection Size | Slab           | StableVec      | StableVec vs Slab |
|-----------------|----------------|----------------|-------------------|
| 1,000           | 962.95 ns      | 1,092.7 ns     | 13.5% slower      |
| 10,000          | 9.57 µs        | 9.63 µs        | 0.6% slower       |

In sparse access scenarios, both implementations show comparable performance, though `Slab` maintains a slight advantage.

### Compaction Workload

The compaction workload tests performance after removing half the elements and calling `shrink_to_fit()`.

| Collection Size | Slab           | StableVec      | StableVec vs Slab |
|-----------------|----------------|----------------|-------------------|
| 1,000           | 885.81 ns      | 572.67 ns      | 35.4% faster      |
| 10,000          | 6.07 µs        | 3.56 µs        | 41.4% faster      |

`StableVec` significantly outperforms `Slab` in compaction scenarios, showing 35-41% better performance.

## Conclusion

Based on the benchmark results, we can draw the following conclusions:

1. **Standard Mixed Workload**: For general use cases with a mix of operations, `StableVec` shows better performance at larger collection sizes, while both implementations are comparable for smaller collections.

2. **High Churn Workload**: 
   - `Slab` outperforms `StableVec` in scenarios with clustered or random removal patterns, particularly at larger collection sizes.
   - For uniform removal patterns, `StableVec` becomes more competitive as collection size increases.
   - The performance gap is most pronounced for random access patterns at large collection sizes, where `Slab` is up to 44.9% faster.

3. **Sparse Access Workload**: Both implementations handle sparse collections well, with `Slab` having a slight edge.

4. **Compaction Workload**: `StableVec` significantly outperforms `Slab` for compaction operations, being 35-41% faster.

### Recommendations

- **Use `Slab` when**:
  - Your workload involves frequent, non-uniform removals (especially with random patterns)
  - You need predictable performance for high-churn scenarios
  - Memory overhead is less important than insertion/removal speed

- **Use `StableVec` when**:
  - Your collections are large (100,000+ elements)
  - Memory efficiency is important (it uses 1 bit per slot vs 8 bytes for `Slab`)
  - Your application frequently performs compaction operations
  - You have a more balanced workload with fewer removals

The choice between these implementations depends on your specific usage patterns and priorities. For most general-purpose applications, `StableVec` offers a good balance of performance and memory efficiency, while `Slab` excels in high-churn scenarios with non-uniform removal patterns.

## License

This benchmarking project is released under CC0 (public domain). Note that the libraries being benchmarked use different licenses: `slab` is licensed under [MIT](https://github.com/tokio-rs/slab/blob/master/LICENSE) and `stable-vec` is dual-licensed under [MIT](https://github.com/azriel91/stable-vec/blob/main/LICENSE-MIT) and [Apache 2.0](https://github.com/azriel91/stable-vec/blob/main/LICENSE-APACHE).
