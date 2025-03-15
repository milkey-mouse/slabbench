use slab::Slab;
use stable_vec::StableVec;

/// Helper function to create a pre-populated slab
pub fn create_slab_with_elements(count: usize) -> (Slab<usize>, Vec<usize>) {
    let mut slab = Slab::new();
    let keys: Vec<_> = (0..count).map(|i| slab.insert(i)).collect();
    (slab, keys)
}

/// Helper function to create a pre-populated stable_vec
pub fn create_stable_vec_with_elements(count: usize) -> (StableVec<usize>, Vec<usize>) {
    let mut stable_vec = StableVec::new();
    let keys: Vec<_> = (0..count).map(|i| {
        let key = stable_vec.push(i);
        key
    }).collect();
    (stable_vec, keys)
}