use crate::slab::BitmapSlab;
use rustc_hash::FxHashMap;
use std::hash::Hash;

pub struct SlabMap<K: Hash + Eq, V: Clone> {
    slab: BitmapSlab<V>,
    hashmap: FxHashMap<K, usize>,
}

impl<K: Hash + Eq, V: Clone> SlabMap<K, V> {
    pub fn with_capacity(capacity: usize) -> Self {
        let slab = BitmapSlab::with_capacity(capacity);
        let mut hashmap = FxHashMap::default();
        hashmap.reserve(slab.capacity());
        Self { slab, hashmap }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.hashmap
            .iter()
            .map(|(k, &idx)| (k, unsafe { self.slab.get_unchecked(idx) }))
    }

    pub fn insert(&mut self, key: K, value: V) -> usize {
        match self.hashmap.get(&key) {
            Some(&idx) => unsafe {
                self.slab.set_unsafe(value, idx);
                idx
            },
            None => {
                let idx = self.slab.insert(value).unwrap();
                self.hashmap.insert(key, idx);
                idx
            }
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.hashmap.get(key).and_then(|&idx| self.slab.get(idx))
    }

    pub fn remove(&mut self, key: &K) -> Option<(usize, V)> {
        let idx = self.hashmap.remove(key)?;
        self.slab.remove(idx).map(|v| (idx, v))
    }

    pub unsafe fn get_unchecked(&self, key: &K) -> &V {
        let idx = self.hashmap[key];
        unsafe { self.slab.get_unchecked(idx) }
    }
}
