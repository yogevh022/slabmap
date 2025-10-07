use std::ops::Deref;
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

pub struct BitmapSlab2<T: Clone> {
    pub(crate) capacity: usize,
    sl_step: usize,
    fl_step: usize,
    fl_bitmap: BitMap<Word>,
    sl_bitmap: Box<[BitMap<Word>]>,
    free_slots: Box<[FreeSlot]>,
    mem_ptr: *mut T,
    pub mem: Box<[T]>, // fixme
}

impl<T: Clone> BitmapSlab2<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        // fixme mask fl instead of aligning with huge values
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

    fn set_bitmap_free(&mut self, fli: usize, sli: usize) {
        let sl_word = &mut self.sl_bitmap[fli];
        if sl_word.is_max() {
            self.fl_bitmap.set_bit_zero(fli);
        }
        sl_word.set_bit_zero(sli);
    }

    fn free_slot(&mut self, index: usize) {
        let fli = index / (WORD_BITS * WORD_BITS);
        let sli = index % WORD_BITS;
        let layers_offset = fli * WORD_BITS + sli;

        let free_slot = &mut self.free_slots[layers_offset];

        unsafe {
            let slot_ptr = self.mem_ptr.add(index) as *mut SlotOffset;
            *slot_ptr = free_slot.next;
        }

        free_slot.next = (index - layers_offset) as u8;
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
            self.free_slots.get_unchecked_mut(fli * WORD_BITS + sli)
        };
        debug_assert!(free_slot.has_next());

        let ptr_offset = fli * self.fl_step + sli * self.sl_step + free_slot.next as usize;
        free_slot.next = unsafe { (*(self.mem_ptr.add(ptr_offset) as *const FreeSlot)).next };

        // if new head is null, set bitmap to used
        if !free_slot.has_next() {
            sl_word.set_bit_one(sli);
            if sl_word.is_max() {
                self.fl_bitmap.set_bit_one(fli);
            }
        }

        Ok(ptr_offset / size_of::<T>())
    }

    pub fn insert(&mut self, value: T) -> AllocResult<usize> {
        let slab_index = self.claim_available_slot()?;
        self.mem[slab_index] = value;
        Ok(slab_index)
    }

    pub fn remove(&mut self, index: usize) -> AllocResult<T> {
        let value = self.mem[index].clone();
        self.free_slot(index);
        Ok(value)
    }
}
