use rustc_hash::FxHashMap;
use std::hash::Hash;

type Word = u64;
const WORD_BITS: usize = Word::BITS as usize;

pub struct BitmapSlab<T: Clone> {
    bitmap: Vec<Word>,
    data: Vec<T>,
    cursor: usize,
}

impl<T: Clone> BitmapSlab<T> {
    pub fn new() -> Self {
        let mut data = Vec::with_capacity(WORD_BITS);
        unsafe {
            data.set_len(WORD_BITS);
        }
        Self {
            bitmap: vec![0],
            data,
            cursor: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let bitmaps = (capacity as f64 / WORD_BITS as f64).ceil() as usize;
        let data_capacity = bitmaps * WORD_BITS;
        let mut data = Vec::with_capacity(data_capacity);
        unsafe {
            data.set_len(data_capacity);
        }
        Self {
            bitmap: vec![0; bitmaps],
            data,
            cursor: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if self.index_exists(index) {
            Some(&self.data[index])
        } else {
            None
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        let byte = index / WORD_BITS;
        let bit = index % WORD_BITS;
        self.cursor = self.cursor.min(byte);
        let byte = &mut self.bitmap[self.cursor];
        if Self::bitmap_bit_is_one(byte, bit) {
            Self::bitmap_set_bit_zero(byte, bit);
            Some(self.data[index].clone())
        } else {
            None
        }
    }

    pub fn insert(&mut self, value: T) -> usize {
        let Some(rel_i) = self.bitmap[self.cursor..]
            .iter()
            .position(|&b| b != Word::MAX)
        else {
            self.cursor = self.grow();
            return self.cursor;
        };
        self.cursor += rel_i;
        let byte = &mut self.bitmap[self.cursor];
        let first_available = (*byte).trailing_ones();
        Self::bitmap_set_bit_one(byte, first_available as usize);
        let index = self.cursor * WORD_BITS + first_available as usize;
        self.data[index] = value;
        index
    }

    fn index_exists(&self, index: usize) -> bool {
        let byte = index / WORD_BITS;
        let bit = index % WORD_BITS;
        Self::bitmap_bit_is_one(&self.bitmap[byte], bit)
    }

    fn bitmap_bit_is_one(byte: &Word, bit: usize) -> bool {
        *byte & (1 << bit) != 0
    }

    fn bitmap_set_bit_one(byte: &mut Word, bit: usize) {
        *byte |= 1 << bit;
    }

    fn bitmap_set_bit_zero(byte: &mut Word, bit: usize) {
        *byte &= !(1 << bit);
    }

    fn grow(&mut self) -> usize {
        let first_available = self.bitmap.len();
        self.bitmap.resize(self.bitmap.capacity(), 0);
        self.data.reserve(self.data.capacity());
        unsafe {
            self.data.set_len(self.data.capacity());
        }
        first_available * WORD_BITS
    }

    /// this function will NOT check if the index is valid
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        &self.data[index]
    }

    /// this function will NOT check if the index is valid
    pub unsafe fn remove_unchecked(&mut self, index: usize) -> T {
        let byte = index / WORD_BITS;
        let bit = index % WORD_BITS;
        Self::bitmap_set_bit_zero(&mut self.bitmap[self.cursor], bit);
        self.cursor = self.cursor.min(byte);
        self.data[index].clone()
    }

    /// this function will NOT update the free slots bitmap
    pub(crate) unsafe fn set_unsafe(&mut self, value: T, index: usize) {
        self.data[index] = value;
    }
}

pub struct SlabMap<K: Hash + Eq, V: Clone> {
    slab: BitmapSlab<V>,
    hashmap: FxHashMap<K, usize>,
}

impl<K: Hash + Eq, V: Clone> SlabMap<K, V> {
    pub fn new() -> Self {
        Self {
            slab: BitmapSlab::new(),
            hashmap: FxHashMap::default(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut hashmap = FxHashMap::default();
        hashmap.reserve(capacity);
        Self {
            slab: BitmapSlab::with_capacity(capacity),
            hashmap,
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> usize {
        match self.hashmap.get(&key) {
            Some(&idx) => unsafe {
                self.slab.set_unsafe(value, idx);
                idx
            },
            None => {
                let idx = self.slab.insert(value);
                self.hashmap.insert(key, idx);
                idx
            }
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.hashmap.get(key).and_then(|&idx| self.slab.get(idx))
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let idx = self.hashmap.remove(key)?;
        self.slab.remove(idx)
    }

    pub unsafe fn get_unchecked(&self, key: &K) -> &V {
        let idx = self.hashmap[key];
        unsafe { self.slab.get_unchecked(idx) }
    }

    pub unsafe fn remove_unchecked(&mut self, key: &K) -> V {
        let idx = self.hashmap.remove(key).unwrap();
        unsafe { self.slab.remove_unchecked(idx) }
    }
}
