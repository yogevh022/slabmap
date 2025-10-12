use std::ptr::slice_from_raw_parts_mut;
use word_bitmap;
use word_bitmap::BitMap;

type Word = u64;
type AllocResult<T> = Result<T, AllocError>;
const WORD_BITS: usize = Word::BITS as usize;

type SlotOffset = u8;
const NULL_SLOT_OFFSET: SlotOffset = SlotOffset::MAX;
const MAX_SLOT_OFFSET: usize = NULL_SLOT_OFFSET as usize - 1;

#[derive(Debug)]
pub enum AllocError {
    OutOfMemory,
}

struct FreeSlot {
    next: SlotOffset,
}

impl FreeSlot {
    pub fn has_next(&self) -> bool {
        self.next != NULL_SLOT_OFFSET
    }
}

pub struct BitmapSlab<T: Clone> {
    capacity: usize,
    sl_step: usize,
    fl_step: usize,
    fl_bitmap: BitMap<Word>,
    sl_bitmap: Box<[BitMap<Word>]>,
    free_slots: Box<[FreeSlot]>,
    mem_ptr: *mut T,
    mem: Box<[T]>,
}

impl<T: Clone> BitmapSlab<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity < WORD_BITS * WORD_BITS * MAX_SLOT_OFFSET);
        // todo mask fl instad of huge alignments?
        let sl_step = (capacity as f64 / (WORD_BITS * WORD_BITS) as f64).ceil() as usize;
        let fl_step = sl_step * WORD_BITS;
        let aligned_capacity = fl_step * WORD_BITS;

        let (mem_ptr, mem, initial_free_slots) =
            Self::initialize_mem(aligned_capacity, sl_step as u8);

        Self {
            capacity: aligned_capacity,
            sl_step,
            fl_step,
            fl_bitmap: BitMap::default(),
            sl_bitmap: vec![BitMap::default(); WORD_BITS].into_boxed_slice(),
            free_slots: initial_free_slots,
            mem_ptr,
            mem,
        }
    }

    fn initialize_mem(
        aligned_capacity: usize,
        sl_step: SlotOffset,
    ) -> (*mut T, Box<[T]>, Box<[FreeSlot]>) {
        let mut initial_free_slots: Vec<FreeSlot> = Vec::with_capacity(WORD_BITS.pow(2));
        let mem_ptr = unsafe {
            let mut mem: Vec<T> = Vec::with_capacity(aligned_capacity);
            mem.set_len(aligned_capacity);
            let mem_ptr = mem.as_mut_ptr();
            std::mem::forget(mem);
            mem_ptr
        };

        let mut mem_byte_ptr = mem_ptr as *mut u8;
        for _fl_bits in 0..WORD_BITS {
            for _sl_bits in 0..WORD_BITS {
                Self::initialize_mem_sl(&mut mem_byte_ptr, &mut initial_free_slots, sl_step);
            }
        }

        let mem = unsafe { Box::from_raw(slice_from_raw_parts_mut(mem_ptr, aligned_capacity)) };
        let initial_free_slots = initial_free_slots.into_boxed_slice();
        (mem_ptr, mem, initial_free_slots)
    }

    fn initialize_mem_sl(
        mem_byte_ptr: &mut *mut u8,
        initial_free_slots: &mut Vec<FreeSlot>,
        slots_per_sl: u8,
    ) {
        initial_free_slots.push(FreeSlot { next: 0 });
        for sli in 1..slots_per_sl {
            unsafe { **mem_byte_ptr = sli };
            *mem_byte_ptr = unsafe { mem_byte_ptr.byte_add(size_of::<T>()) };
        }
        unsafe { **mem_byte_ptr = NULL_SLOT_OFFSET };
        *mem_byte_ptr = unsafe { mem_byte_ptr.byte_add(size_of::<T>()) };
    }

    fn mapping_from_index(&self, index: usize) -> (usize, usize) {
        let fli = index / self.fl_step;
        let sli = (index % self.fl_step) / self.sl_step;
        (fli, sli)
    }

    fn mem_slot_offset_from_mapping(&self, fli: usize, sli: usize) -> usize {
        fli * self.fl_step + sli * self.sl_step
    }

    fn free_slot_index_from_mapping(fli: usize, sli: usize) -> usize {
        fli * WORD_BITS + sli
    }

    fn release_slot(&mut self, index: usize) {
        let (fli, sli) = self.mapping_from_index(index);
        let free_slot_index = Self::free_slot_index_from_mapping(fli, sli);
        let mem_slot_offset = self.mem_slot_offset_from_mapping(fli, sli);
        let sl_word = &mut self.sl_bitmap[fli];

        let free_slot = unsafe {
            // fli will never be >= WORD_BITS, because in that case fl_bitmap is max (Err)
            self.free_slots.get_unchecked_mut(free_slot_index)
        };

        // if current free head is null, set bitmap to free
        if !free_slot.has_next() {
            if sl_word.is_max() {
                self.fl_bitmap.set_bit_zero(fli);
            }
            sl_word.set_bit_zero(sli);
        }

        unsafe {
            let slot_ptr = self.mem_ptr.add(index) as *mut SlotOffset;
            *slot_ptr = free_slot.next;
        }

        free_slot.next = (index - mem_slot_offset) as u8;
    }

    fn claim_available_slot(&mut self) -> AllocResult<usize> {
        if self.fl_bitmap.is_max() {
            return Err(AllocError::OutOfMemory);
        }
        // get available mapping mapping
        let fli = self.fl_bitmap.first_zero_lsb() as usize;
        let sl_word = unsafe {
            // fli is always less than self.sl_bitmap.len()
            self.sl_bitmap.get_unchecked_mut(fli)
        };
        let sli = sl_word.first_zero_lsb() as usize;

        // pop head of available slots linked list
        let free_slot = unsafe {
            // fli will never be >= WORD_BITS, because in that case fl_bitmap is max (Err)
            let free_slot_index = Self::free_slot_index_from_mapping(fli, sli);
            self.free_slots.get_unchecked_mut(free_slot_index)
        };
        debug_assert!(free_slot.has_next());

        // hand written mem_ptr offset logic because of borrow checker
        let ptr_offset = fli * self.fl_step + sli * self.sl_step + free_slot.next as usize;
        free_slot.next = unsafe { (*(self.mem_ptr.add(ptr_offset) as *const FreeSlot)).next };

        // if new free head is null, set bitmap to used
        if !free_slot.has_next() {
            sl_word.set_bit_one(sli);
            if sl_word.is_max() {
                self.fl_bitmap.set_bit_one(fli);
            }
        }

        Ok(ptr_offset)
    }

    pub fn insert(&mut self, value: T) -> AllocResult<usize> {
        let slab_index = self.claim_available_slot()?;
        self.mem[slab_index] = value;
        Ok(slab_index)
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        let value = self.mem[index].clone();
        self.release_slot(index);
        Some(value)
    }

    pub fn get(&self, index: usize) -> &T {
        &self.mem[index]
    }

    pub fn get_mut(&mut self, index: usize) -> &mut T {
        &mut self.mem[index]
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub unsafe fn set_unsafe(&mut self, value: T, index: usize) {
        self.mem[index] = value;
    }
}
