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
    const COUNT: usize = 1 << 8;
    let mut bitslab  = SlabMap::<usize, usize>::with_capacity(COUNT);

    let mut indices = Vec::new();
    for i in 0..COUNT {
        let q = bitslab.insert(i, i*40000);
        println!("inserted: {:?}:{:?} -> {:?}", i, i*40000, q);
        indices.push(q);
    }

    for i in indices {
        let q = bitslab.remove(&i);
        println!("removed: {:?} -> {:?}", i, q);
    }
}