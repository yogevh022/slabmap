use std::mem::MaybeUninit;

type Word = u64;
type AllocResult<T> = Result<T, AllocError>;
const WORD_BITS: usize = Word::BITS as usize;
const MAX_FREE_LINKS_PER_SL: usize = u8::MAX as usize - 1;
const FLI_STEP: usize = WORD_BITS * MAX_FREE_LINKS_PER_SL;

#[derive(Debug)]
pub enum AllocError {
    OutOfMemory,
}

struct FreeSlot {
    next: u8,
}

impl FreeSlot {
    pub fn has_next(&self) -> bool {
        self.next != u8::MAX
    }
}

pub struct BitmapSlab2<T> {
    capacity: usize,
    fl_step: usize,
    fl_bitmap: Word,
    sl_bitmap: Box<[Word]>,
    free_blocks: Box<[*mut FreeSlot]>,
    mem: Box<[T]>,
    mem_start_ptr: *mut Box<[T]>,
}

impl<T> BitmapSlab2<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity_per_fl_bit = (capacity as f64 / (WORD_BITS * WORD_BITS) as f64).ceil() as usize;
        let aligned_capacity = capacity_per_fl_bit * WORD_BITS * WORD_BITS;

        let mut mem = Vec::with_capacity(aligned_capacity);
        unsafe { mem.set_len(aligned_capacity) };

        let mem_start_ptr = mem.as_mut_ptr() as *mut Box<[T]>;

        Self {
            capacity: aligned_capacity,
            fl_step: capacity_per_fl_bit,
            fl_bitmap: 0,
            sl_bitmap: vec![0; WORD_BITS].into_boxed_slice(),
            free_blocks: Box::new([]),
            mem: mem.into_boxed_slice(),
            mem_start_ptr,
        }
    }

    pub fn pop_free_link(&mut self, fli: usize, sli: usize) -> usize {
        let link_head_idx = fli * FLI_STEP + sli * MAX_FREE_LINKS_PER_SL;
        let free_slot = unsafe { &mut *self.free_blocks[link_head_idx] };
        if !free_slot.has_next() {
            panic!();
        }
        let ptr_offset = link_head_idx + free_slot.next as usize;
        let next_ptr = unsafe {
            self.mem_start_ptr.add(ptr_offset) as *mut FreeSlot
        };
        self.free_blocks[link_head_idx] = next_ptr;

        ptr_offset
    }

    pub fn insert(&mut self, value: T) -> AllocResult<usize> {
        let fli = (!self.fl_bitmap).trailing_zeros() as usize;
        if fli == WORD_BITS {
            return Err(AllocError::OutOfMemory);
        }
        let sl_word = &mut self.sl_bitmap[fli];
        let sli = (!*sl_word).trailing_zeros();
        let fl_offset = fli * self.fl_step;
        let sl_byte_idx = self.sl_bitmap[fl_offset..(fl_offset + self.fl_step)]
            .iter()
            .position(|b| *b != Word::MAX)
            .unwrap();

        let byte_idx = fl_offset + sl_byte_idx;
        let sl_bitmap_slot = &mut self.sl_bitmap[byte_idx];
        let sli = (!*sl_bitmap_slot).trailing_zeros() as usize;
        *sl_bitmap_slot |= 1 << sli;

        if *sl_bitmap_slot == Word::MAX && self.sl_bitmap[fl_offset..(fl_offset + self.fl_step)]
            .iter()
            .rev()
            .all(|b| *b == Word::MAX) {
            self.fl_bitmap |= 1 << fli;
        }

        let slab_index = byte_idx * WORD_BITS + sli;
        self.mem[slab_index] = value;

        Ok(slab_index)
    }
}
