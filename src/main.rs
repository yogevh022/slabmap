mod slab;
mod map;

use map::SlabMap;
use std::hint::black_box;


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
    const COUNT: usize = 1 << 19;
    let mut sm = SlabMap::<usize, usize>::with_capacity(COUNT);
    let q = timed!({
        for i in 0..COUNT {
            let q = sm.insert(i, i);
            black_box(q);
        }
    });

    println!("time: {:?}", q);
}
