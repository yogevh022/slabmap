use std::hint::black_box;
use ::slab::Slab;
use slabmap::{BitmapSlab, SlabMap};
mod slab;
#[macro_export]
macro_rules! timed {
    ($block:block) => {{
        let start = std::time::Instant::now();
        std::hint::black_box($block);
        let end = start.elapsed();
        end
    }};
}

fn main() {
    const COUNT: usize = 1 << 28;

    let mut bitslab = slab::BitmapSlab2::<usize>::with_capacity(COUNT);

    dbg!(bitslab.capacity);

    let mut indices_new = Vec::with_capacity(COUNT);
    let bitslab_ins_time = timed!({
        for i in 0..COUNT - 1 {
            let z = bitslab.insert(i).unwrap();
            indices_new.push(z);
        }
    });
    println!("bitslab ins: {:?}", bitslab_ins_time);

    let bitslab_rem_time = timed!({
        for ind in indices_new {
            let q = bitslab.remove(ind).unwrap();
        }
    });
    println!("bitslab rem: {:?}", bitslab_rem_time);

    // let mut slab = Slab::<usize>::with_capacity(COUNT);
    // let mut indices_slab = Vec::with_capacity(COUNT);
    //
    // let slab_ins_time = timed!({
    //     for i in 0..COUNT {
    //         let z = slab.insert(i);
    //         indices_slab.push(z);
    //     }
    // });
    // println!("slab ins: {:?}", slab_ins_time);
    //
    // let slab_rem_time = timed!({
    //     for ind in indices_slab {
    //         let q = slab.remove(ind);
    //     }
    // });
    // println!("slab rem: {:?}", slab_rem_time);

    //
    // let mut indices_old = Vec::with_capacity(COUNT);
    // let mut slabmap: BitmapSlab<u8> = BitmapSlab::with_capacity(COUNT);
    // let sm_ins = timed!({
    //     for i in 0..COUNT {
    //         let q = slabmap.insert(i as u8);
    //         indices_old.push(q);
    //     }
    // });
    // println!("bitslab ins old: {:?}", sm_ins);
    //
    // let sm_rem = timed!({
    //     for ind in indices_old {
    //         let q = slabmap.remove(ind).unwrap();
    //     }
    // });
    // println!("bitslab rem old: {:?}", sm_rem);

}