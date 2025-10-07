use std::hint::black_box;
use slabmap::SlabMap;
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
    const COUNT: usize = 1 << 20;

    let mut bitslab = slab::BitmapSlab2::<usize>::with_capacity(COUNT);

    let bitslab_time = timed!({
        for i in 0..COUNT {
            // let z = bitslab.insert(i);
            let z = bitslab.q();
            black_box(z);
        }
    });

    println!("bitslab: {:?}", bitslab_time);

    let mut slabmap: SlabMap<usize, usize> = SlabMap::with_capacity(COUNT);

    let sm = timed!({
        for i in 0..COUNT {
            let q = slabmap.insert(i, i);
            black_box(q);
        }
    });

    println!("slabmap: {:?}", sm);
}