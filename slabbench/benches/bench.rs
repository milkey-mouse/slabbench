use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use slab::Slab;
use stable_vec::StableVec;

// Define a trait that abstracts over both Slab and StableVec operations
trait Slabbable<T: Default> {
    #[inline(always)]
    fn new_with_capacity(capacity: usize) -> Self where Self: Sized;
    
    #[inline(always)]
    fn insert(&mut self, value: T) -> usize;
    
    #[inline(always)]
    fn insert_at(&mut self, key: usize, value: T) -> Option<T>;
    
    #[inline(always)]
    fn remove(&mut self, key: usize) -> Option<T>;
    
    #[inline(always)]
    fn get(&self, key: usize) -> Option<&T>;
    
    #[inline(always)]
    fn contains(&self, key: usize) -> bool;
}

// Wrapper type for Slab to implement our trait
struct SlabWrapper<T>(Slab<T>);

impl<T: Default> Slabbable<T> for SlabWrapper<T> {
    #[inline(always)]
    fn new_with_capacity(capacity: usize) -> Self {
        Self(Slab::with_capacity(capacity))
    }
    
    #[inline(always)]
    fn insert(&mut self, value: T) -> usize {
        self.0.insert(value)
    }
    
    #[inline(always)]
    fn insert_at(&mut self, key: usize, value: T) -> Option<T> {
        let old_value = if self.0.contains(key) {
            Some(self.0.remove(key))
        } else {
            None
        };
        
        // Grow the slab if needed
        while self.0.capacity() <= key {
            self.0.reserve(key - self.0.capacity() + 1);
        }
        
        // Now use a vacant entry to get a key
        let vacant = self.0.vacant_entry();
        
        // If the key is not what we want, we need a more complex solution
        if vacant.key() != key {
            // Insert the value at whatever key we got
            let temp_key = self.0.insert(value);
            
            // If we got a different key than desired, we need to set up the desired key
            if temp_key != key {
                // Fill all slots up to and including our desired key with vacant entries
                // This essentially "reserves" the slots
                for i in self.0.capacity()..=key {
                    self.0.insert(Default::default());
                }
                
                // Remove the temporary value
                let temp_value = self.0.remove(temp_key);
                
                // Now that we have ensured the exact key exists and is vacant,
                // we can insert our value there
                self.0.insert(temp_value);
            }
        } else {
            // The vacant entry key matches what we want - easy case
            vacant.insert(value);
        }
        
        old_value
    }
    
    #[inline(always)]
    fn remove(&mut self, key: usize) -> Option<T> {
        self.0.try_remove(key)
    }
    
    #[inline(always)]
    fn get(&self, key: usize) -> Option<&T> {
        self.0.get(key)
    }
    
    #[inline(always)]
    fn contains(&self, key: usize) -> bool {
        self.0.contains(key)
    }
}

// Wrapper type for StableVec to implement our trait
struct StableVecWrapper<T>(StableVec<T>);

impl<T: Default> Slabbable<T> for StableVecWrapper<T> {
    #[inline(always)]
    fn new_with_capacity(capacity: usize) -> Self {
        Self(StableVec::with_capacity(capacity))
    }
    
    #[inline(always)]
    fn insert(&mut self, value: T) -> usize {
        self.0.push(value)
    }
    
    #[inline(always)]
    fn insert_at(&mut self, key: usize, value: T) -> Option<T> {
        self.0.insert(key, value)
    }
    
    #[inline(always)]
    fn remove(&mut self, key: usize) -> Option<T> {
        self.0.remove(key)
    }
    
    #[inline(always)]
    fn get(&self, key: usize) -> Option<&T> {
        self.0.get(key)
    }
    
    #[inline(always)]
    fn contains(&self, key: usize) -> bool {
        self.0.has_element_at(key)
    }
}

// Mixed Workload Benchmarks
// These are the most important benchmarks as they simulate real-world usage

fn bench_standard_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("standard_mixed_workload");
    group.sample_size(20); // Reduce sample size to make benchmarks run faster
    
    // Use fewer size variants but include a larger size for stress testing
    for size in [1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(BenchmarkId::new("slab", size), size, |b, size| {
            b.iter(|| {
                let mut slab = Slab::with_capacity(*size / 2);
                let mut keys = Vec::with_capacity(*size);
                
                // Insert phase
                for i in 0..*size {
                    keys.push(slab.insert(i));
                }
                
                // Get phase
                let mut sum = 0;
                for &key in &keys {
                    if let Some(&val) = slab.get(key) {
                        sum += val;
                    }
                }
                black_box(sum);
                
                // Remove every third element
                for i in (0..keys.len()).step_by(3) {
                    slab.remove(keys[i]);
                }
                
                // Insert some new elements
                for i in 0..(*size / 4) {
                    slab.insert(i * 100);
                }
                
                // Final get phase
                sum = 0;
                for (_, &val) in slab.iter() {
                    sum += val;
                }
                black_box(sum)
            })
        });
        
        group.bench_with_input(BenchmarkId::new("stable_vec", size), size, |b, size| {
            b.iter(|| {
                let mut sv = StableVec::with_capacity(*size / 2);
                let mut keys = Vec::with_capacity(*size);
                
                // Insert phase
                for i in 0..*size {
                    keys.push(sv.push(i));
                }
                
                // Get phase
                let mut sum = 0;
                for &key in &keys {
                    if let Some(&val) = sv.get(key) {
                        sum += val;
                    }
                }
                black_box(sum);
                
                // Remove every third element
                for i in (0..keys.len()).step_by(3) {
                    sv.remove(keys[i]);
                }
                
                // Insert some new elements
                for i in 0..(*size / 4) {
                    sv.push(i * 100);
                }
                
                // Final get phase
                sum = 0;
                for (_, &val) in sv.iter() {
                    sum += val;
                }
                black_box(sum)
            })
        });
    }
    
    group.finish();
}

/// Generic benchmark function for high churn workload using the Slabbable trait
fn bench_high_churn_generic<S: Slabbable<usize>>(
    c: &mut Criterion,
    name: &str,
    sizes: &[usize],
    patterns: &[&str],
) {
    let mut group = c.benchmark_group("high_churn_workload");
    group.sample_size(30); // Increase sample size for better statistical significance
    
    for &size in sizes {
        group.throughput(Throughput::Elements(size as u64));
        
        for &pattern in patterns {
            group.bench_with_input(
                BenchmarkId::new(format!("{}_{}", name, pattern), size), 
                &(size, pattern), 
                |b, (size, pattern)| {
                    b.iter_with_setup(
                        || {
                            // Setup: initialize with capacity and tracking structures
                            let container = S::new_with_capacity(*size);
                            let active_keys = Vec::with_capacity(*size);
                            let removed_keys = Vec::with_capacity(*size / 2);
                            
                            (container, active_keys, removed_keys)
                        },
                        |(mut container, mut active_keys, mut removed_keys)| {
                            // First, fill the container halfway
                            for i in 0..(*size / 2) {
                                active_keys.push(container.insert(i));
                            }
                            
                            // Now perform high-churn operations in different patterns
                            let cycles = 20; // More cycles for more realistic workload
                            
                            for cycle in 0..cycles {
                                // Each pattern has a different removal strategy
                                match *pattern {
                                    "uniform" => {
                                        // Remove every third element
                                        let mut to_remove = Vec::new();
                                        for i in (0..active_keys.len()).step_by(3) {
                                            if i < active_keys.len() {
                                                to_remove.push(i);
                                            }
                                        }
                                        
                                        // Remove the elements from the end to avoid index shifting
                                        for i in to_remove.iter().rev() {
                                            let key = active_keys.swap_remove(*i);
                                            if let Some(val) = container.remove(key) {
                                                removed_keys.push((key, val));
                                            }
                                        }
                                    },
                                    "clustered" => {
                                        // Remove elements in clusters (25% of elements from a continuous section)
                                        if !active_keys.is_empty() {
                                            let cluster_size = active_keys.len() / 4;
                                            if cluster_size > 0 && active_keys.len() > cluster_size {
                                                let start = (cycle * 17) % (active_keys.len() - cluster_size);
                                                
                                                for _ in 0..cluster_size {
                                                    let key = active_keys.swap_remove(start);
                                                    if let Some(val) = container.remove(key) {
                                                        removed_keys.push((key, val));
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    "random" => {
                                        // Remove random elements (using a deterministic algorithm)
                                        let num_to_remove = active_keys.len() / 3;
                                        for _ in 0..num_to_remove {
                                            if !active_keys.is_empty() {
                                                let idx = (cycle * 31) % active_keys.len();
                                                let key = active_keys.swap_remove(idx);
                                                if let Some(val) = container.remove(key) {
                                                    removed_keys.push((key, val));
                                                }
                                            }
                                        }
                                    },
                                    _ => unreachable!()
                                }
                                
                                // Reinsert some elements from the removed set
                                let num_to_reinsert = removed_keys.len() / 2;
                                for _ in 0..num_to_reinsert {
                                    if !removed_keys.is_empty() {
                                        let (key, val) = removed_keys.pop().unwrap();
                                        // Alternate between inserting at specific index and adding new
                                        if cycle % 2 == 0 && !container.contains(key) {
                                            // Insert at the specific index if available
                                            if container.insert_at(key, val + 1000).is_none() {
                                                active_keys.push(key);
                                            }
                                        } else {
                                            // Insert new element, getting a new key
                                            active_keys.push(container.insert(val + 2000));
                                        }
                                    }
                                }
                                
                                // Add some fresh elements
                                let num_new = (*size / 10).max(1);
                                for i in 0..num_new {
                                    active_keys.push(container.insert(i + cycle * 1000));
                                }
                                
                                // Occasionally access elements randomly to simulate real use
                                let mut sum = 0;
                                for i in 0..active_keys.len() {
                                    if i % 5 == (cycle % 5) {
                                        if let Some(&val) = container.get(active_keys[i]) {
                                            sum += val;
                                        }
                                    }
                                }
                                black_box(sum);
                            }
                            
                            // Return the final structure for verification
                            black_box(container)
                        }
                    )
                }
            );
        }
    }
    
    group.finish();
}

fn bench_high_churn_workload(c: &mut Criterion) {
    // Define the test parameters once to ensure identical test conditions
    let sizes = [1_000, 5_000, 10_000, 50_000];
    let patterns = ["uniform", "clustered", "random"];
    
    // Run the same benchmark with both implementations
    bench_high_churn_generic::<SlabWrapper<usize>>(c, "slab", &sizes, &patterns);
    bench_high_churn_generic::<StableVecWrapper<usize>>(c, "stable_vec", &sizes, &patterns);
}

fn bench_sparse_access_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_access_workload");
    group.sample_size(20); // Reduce sample size for faster benchmarks
    
    for size in [1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(BenchmarkId::new("slab", size), size, |b, size| {
            b.iter_with_setup(
                || {
                    // Setup: create a sparse data structure by removing most elements
                    let mut slab = Slab::with_capacity(*size);
                    let mut keys = Vec::with_capacity(*size);
                    
                    for i in 0..*size {
                        keys.push(slab.insert(i));
                    }
                    
                    // Remove 90% of elements, keeping only every 10th
                    for i in 0..keys.len() {
                        if i % 10 != 0 {
                            slab.remove(keys[i]);
                        }
                    }
                    
                    (slab, keys)
                },
                |(mut slab, keys)| {
                    // Benchmark sparse access patterns
                    
                    // Random accesses across the sparse structure
                    let mut sum = 0;
                    for &key in &keys {
                        if let Some(&val) = slab.get(key) {
                            sum += val;
                        }
                    }
                    black_box(sum);
                    
                    // Iteration through sparse structure
                    sum = 0;
                    for (_, &val) in slab.iter() {
                        sum += val;
                    }
                    black_box(sum);
                    
                    // Insert some new elements in random vacant slots
                    for i in 0..(*size / 10) {
                        slab.insert(i * 100);
                    }
                }
            )
        });
        
        group.bench_with_input(BenchmarkId::new("stable_vec", size), size, |b, size| {
            b.iter_with_setup(
                || {
                    // Setup: create a sparse data structure by removing most elements
                    let mut sv = StableVec::with_capacity(*size);
                    let mut keys = Vec::with_capacity(*size);
                    
                    for i in 0..*size {
                        keys.push(sv.push(i));
                    }
                    
                    // Remove 90% of elements, keeping only every 10th
                    for i in 0..keys.len() {
                        if i % 10 != 0 {
                            sv.remove(keys[i]);
                        }
                    }
                    
                    (sv, keys)
                },
                |(mut sv, keys)| {
                    // Benchmark sparse access patterns
                    
                    // Random accesses across the sparse structure
                    let mut sum = 0;
                    for &key in &keys {
                        if let Some(&val) = sv.get(key) {
                            sum += val;
                        }
                    }
                    black_box(sum);
                    
                    // Iteration through sparse structure
                    sum = 0;
                    for (_, &val) in sv.iter() {
                        sum += val;
                    }
                    black_box(sum);
                    
                    // Insert some new elements in random vacant slots
                    for i in 0..(*size / 10) {
                        sv.push(i * 100);
                    }
                }
            )
        });
    }
    
    group.finish();
}

fn bench_compaction_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("compaction_workload");
    group.sample_size(20); // Reduce sample size for faster benchmarks
    
    for size in [1_000, 10_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        group.bench_with_input(BenchmarkId::new("slab", size), size, |b, size| {
            b.iter_with_setup(
                || {
                    // Setup: create a fragmented data structure
                    let mut slab = Slab::with_capacity(*size);
                    let mut keys = Vec::with_capacity(*size);
                    
                    for i in 0..*size {
                        keys.push(slab.insert(i));
                    }
                    
                    // Remove elements with odd indices to create fragmentation
                    for i in (1..keys.len()).step_by(2) {
                        slab.remove(keys[i]);
                    }
                    
                    slab
                },
                |mut slab| {
                    // Benchmark the compaction and operations after compaction
                    slab.shrink_to_fit();
                    
                    // Operations after compaction
                    let mut sum = 0;
                    for (_, &val) in slab.iter() {
                        sum += val;
                    }
                    black_box(sum);
                    
                    // Add some more elements after compaction
                    for i in 0..100 {
                        slab.insert(i * 200);
                    }
                }
            )
        });
        
        group.bench_with_input(BenchmarkId::new("stable_vec", size), size, |b, size| {
            b.iter_with_setup(
                || {
                    // Setup: create a fragmented data structure
                    let mut sv = StableVec::with_capacity(*size);
                    let mut keys = Vec::with_capacity(*size);
                    
                    for i in 0..*size {
                        keys.push(sv.push(i));
                    }
                    
                    // Remove elements with odd indices to create fragmentation
                    for i in (1..keys.len()).step_by(2) {
                        sv.remove(keys[i]);
                    }
                    
                    sv
                },
                |mut sv| {
                    // Benchmark the compaction and operations after compaction
                    sv.shrink_to_fit();
                    
                    // Operations after compaction
                    let mut sum = 0;
                    for (_, &val) in sv.iter() {
                        sum += val;
                    }
                    black_box(sum);
                    
                    // Add some more elements after compaction
                    for i in 0..100 {
                        sv.push(i * 200);
                    }
                }
            )
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_standard_mixed_workload,
    bench_high_churn_workload,
    bench_sparse_access_workload,
    bench_compaction_workload
);
criterion_main!(benches);