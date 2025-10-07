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
    const COUNT: usize = 1 << 4;
    let mut bitslab  = SlabMap::<usize, usize>::with_capacity(COUNT);
    for i in 0..COUNT {
        bitslab.insert(i, i);
    }
    
    
}